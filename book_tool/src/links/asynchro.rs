//! Checks URLs concurrently, with retries that honor `Retry-After`.
//!
//! TODO Reviewed ??-??-2026.

use std::time::Duration;
use std::time::SystemTime;

use anyhow::Context as _;
use reqwest::Client;
use reqwest::header::HeaderMap;
use tokio::task::JoinSet;
use tracing::debug;
use tracing::warn;
use url::Url;

/// Result of checking a single URL.
#[derive(Debug, Clone)]
pub struct UrlCheck {
    /// Whether the final HTTP check succeeded with a `2xx` status code.
    pub success: bool,
    /// The final HTTP status code returned by the remote server, when available.
    pub status: Option<reqwest::StatusCode>,
    /// The transport-level error message when a request could not be completed.
    pub error: Option<String>,
}

impl UrlCheck {
    /// Returns the transport error message, the HTTP status line, or `no response` when neither is
    /// available.
    #[must_use]
    pub fn describe(&self) -> String {
        match (&self.error, self.status) {
            (Some(error), _) => error.clone(),
            (None, Some(status)) => format!("HTTP {status}"),
            (None, None) => "no response".to_owned(),
        }
    }
}

/// Longest pause honoured between retries, whatever the server asks for.
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// How long a single request may take before it counts as failed.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Parses `value` as delay seconds or as an HTTP date relative to `now`. Past dates yield zero
/// delay. Returns `None` for an unrecognized value.
fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let date = httpdate::parse_http_date(value).ok()?;
    Some(date.duration_since(now).unwrap_or_default())
}

/// Chooses how long to wait before retrying a throttled request.
///
/// A usable `Retry-After` header (seconds or HTTP-date) wins; a missing or malformed one falls back
/// to exponential backoff (1 s, 2 s, 4 s, ...). The result never exceeds [`MAX_RETRY_DELAY`].
fn retry_delay(headers: &HeaderMap, attempt: usize, now: SystemTime) -> Duration {
    let requested = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_retry_after(value, now));
    let backoff = Duration::from_secs(1_u64 << attempt.min(5));
    requested.unwrap_or(backoff).min(MAX_RETRY_DELAY)
}

/// Concurrent HTTP/HTTPS link checker.
#[derive(Clone)]
pub struct AsyncLinkChecker {
    /// HTTP client shared by all checks.
    client: Client,
}

impl AsyncLinkChecker {
    /// Creates an HTTP checker with the default ten-second request timeout.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built.
    pub fn new() -> anyhow::Result<Self> {
        Self::with_timeout(DEFAULT_TIMEOUT)
    }

    /// Creates a checker whose requests fail after `timeout`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn with_timeout(timeout: Duration) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("book-tooling-link-checker")
            .build()
            .context("Failed to build HTTP client for link checking")?;

        Ok(Self { client })
    }

    /// Checks `url` with `HEAD`, then tries `GET` after an unsuccessful response or a transport
    /// error. Returns the final result, including the HTTP status or transport error when
    /// available.
    pub async fn check_url(&self, url: Url) -> UrlCheck {
        debug!("Checking {}", url);
        // Try HEAD first, fall back to GET if HEAD fails or returns certain status codes
        let mut result = self.do_check(&url, true).await;
        if !result.success {
            debug!("HEAD failed for {}, trying GET", url);
            result = self.do_check(&url, false).await;
        }

        result
    }

    /// Verifies a batch of URLs concurrently.
    ///
    /// Results are returned as `(Url, UrlCheck)` pairs. The caller may sort them if
    /// stable output order matters.
    pub async fn check_urls<I>(&self, urls: I) -> Vec<(Url, UrlCheck)>
    where
        I: IntoIterator<Item = Url>,
    {
        /// Allows at most 32 URL requests to be in flight at once.
        const MAX_CONCURRENT_CHECKS: usize = 32;

        let mut pending = urls.into_iter();
        let mut in_flight = JoinSet::new();
        let mut results = Vec::new();

        loop {
            while in_flight.len() < MAX_CONCURRENT_CHECKS {
                let Some(url) = pending.next() else {
                    break;
                };

                let checker = self.clone();
                in_flight.spawn(async move {
                    let result = checker.check_url(url.clone()).await;
                    (url, result)
                });
            }

            match in_flight.join_next().await {
                Some(Ok(result)) => results.push(result),
                Some(Err(error)) => warn!("Link check task failed: {}", error),
                None => break,
            }
        }

        results
    }

    /// Requests `url` with `HEAD` or `GET`, retrying transient failures, and summarizes the
    /// outcome.
    async fn do_check(&self, url: &Url, use_head: bool) -> UrlCheck {
        /// Maximum number of attempts after the initial transient HTTP failure.
        const MAX_RETRIES: usize = 3;
        let mut retries = 0;

        loop {
            let request = if use_head {
                self.client.head(url.clone())
            } else {
                self.client.get(url.clone())
            };

            match request.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if matches!(
                        status,
                        reqwest::StatusCode::TOO_MANY_REQUESTS
                            | reqwest::StatusCode::SERVICE_UNAVAILABLE
                    ) && retries < MAX_RETRIES
                    {
                        let delay = retry_delay(resp.headers(), retries, SystemTime::now());

                        warn!(
                            "Retrying {} after {} response from {} in {}s",
                            if use_head { "HEAD" } else { "GET" },
                            status,
                            url,
                            delay.as_secs()
                        );
                        retries += 1;
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    return UrlCheck {
                        success: status.is_success(),
                        status: Some(status),
                        error: None,
                    };
                }
                Err(e) => {
                    return UrlCheck {
                        success: false,
                        status: None,
                        error: Some(e.to_string()),
                    };
                }
            }
        }
    }
}

/// Tests for HTTP response handling and retry timing.
#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    /// A fixed clock reading for tests that do not use HTTP-dates.
    const EPOCH_2026: SystemTime = SystemTime::UNIX_EPOCH;

    /// Builds headers carrying a single `Retry-After` value.
    fn headers_with_retry_after(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            HeaderValue::from_str(value).expect("valid header value"),
        );
        headers
    }

    /// A numeric `Retry-After` value sets the delay.
    #[test]
    fn numeric_retry_after_is_honoured() {
        assert_eq!(
            retry_delay(&headers_with_retry_after("7"), 0, EPOCH_2026),
            Duration::from_secs(7)
        );
    }

    /// A very large `Retry-After` value is capped.
    #[test]
    fn oversized_retry_after_is_capped() {
        assert_eq!(
            retry_delay(&headers_with_retry_after("86400"), 0, EPOCH_2026),
            MAX_RETRY_DELAY
        );
    }

    /// Without a usable `Retry-After`, the delay doubles per attempt up to the cap.
    #[test]
    fn missing_or_unusable_retry_after_backs_off_exponentially() {
        assert_eq!(
            retry_delay(&HeaderMap::new(), 0, EPOCH_2026),
            Duration::from_secs(1)
        );
        assert_eq!(
            retry_delay(&HeaderMap::new(), 2, EPOCH_2026),
            Duration::from_secs(4)
        );
        assert_eq!(
            retry_delay(&HeaderMap::new(), 40, EPOCH_2026),
            Duration::from_secs(30)
        );
        assert_eq!(
            retry_delay(&headers_with_retry_after("soon"), 1, EPOCH_2026),
            Duration::from_secs(2)
        );
    }

    /// An HTTP-date `Retry-After` is measured from the supplied clock.
    #[test]
    fn http_date_retry_after_is_relative_to_now() {
        let now = httpdate::parse_http_date("Wed, 21 Oct 2026 07:27:50 GMT").expect("valid date");
        assert_eq!(
            retry_delay(
                &headers_with_retry_after("Wed, 21 Oct 2026 07:28:00 GMT"),
                0,
                now
            ),
            Duration::from_secs(10)
        );
    }

    /// An HTTP-date in the past means retry immediately; a distant one is capped.
    #[test]
    fn past_and_distant_http_dates_are_clamped() {
        let now = httpdate::parse_http_date("Wed, 21 Oct 2026 07:28:00 GMT").expect("valid date");
        assert_eq!(
            retry_delay(
                &headers_with_retry_after("Wed, 21 Oct 2026 07:00:00 GMT"),
                3,
                now
            ),
            Duration::ZERO
        );
        assert_eq!(
            retry_delay(
                &headers_with_retry_after("Thu, 22 Oct 2026 07:28:00 GMT"),
                0,
                now
            ),
            MAX_RETRY_DELAY
        );
    }

    /// A server that accepts the connection but never answers makes the check fail on timeout.
    #[tokio::test]
    async fn unresponsive_server_fails_with_a_transport_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("local address");
        let _held = std::thread::spawn(move || {
            let _connections = listener.incoming().take(2).collect::<Vec<_>>();
            std::thread::sleep(Duration::from_secs(2));
        });
        let checker = AsyncLinkChecker::with_timeout(Duration::from_millis(200)).expect("client");
        let url = Url::parse(&format!("http://{address}/")).expect("url");

        let result = checker.check_url(url).await;

        assert!(!result.success);
        assert_eq!(result.status, None);
        assert!(result.error.is_some(), "expected a transport error");
    }

    /// `describe` reports the transport error when present, otherwise the HTTP status.
    #[test]
    fn describe_prefers_transport_errors_then_status() {
        let failed = UrlCheck {
            success: false,
            status: None,
            error: Some("connection refused".to_owned()),
        };
        let missing = UrlCheck {
            success: false,
            status: Some(reqwest::StatusCode::NOT_FOUND),
            error: None,
        };
        assert_eq!(failed.describe(), "connection refused");
        assert_eq!(missing.describe(), "HTTP 404 Not Found");
    }
}
