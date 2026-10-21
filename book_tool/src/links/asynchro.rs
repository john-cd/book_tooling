// TODO Reviewed ??-??-2026
use std::time::Duration;

use anyhow::Context;
use moka::future::Cache;
use reqwest::Client;
use tokio::task::JoinSet;
use tracing::debug;
use tracing::warn;
use url::Url;

#[derive(Debug, Clone)]
/// Result of checking a single URL.
pub struct UrlCheck {
    /// Whether the final HTTP check succeeded with a `2xx` status code.
    pub success: bool,
    /// The final HTTP status code returned by the remote server, when available.
    pub status: Option<reqwest::StatusCode>,
    /// The transport-level error message when a request could not be completed.
    pub error: Option<String>,
}

#[derive(Clone)]
/// Concurrent HTTP/HTTPS link checker with in-memory result caching.
pub struct AsyncLinkChecker {
    client: Client,
    cache: Cache<Url, UrlCheck>,
}

impl AsyncLinkChecker {
    /// Creates a new checker with a shared Reqwest client and a small URL cache.
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("book-tooling-link-checker")
            .build()
            .context("Failed to build HTTP client for link checking")?;

        let cache = Cache::new(1000);

        Ok(Self { client, cache })
    }

    /// Verifies a single URL, using the cache when possible.
    ///
    /// The checker attempts a `HEAD` request first and falls back to `GET` when needed.
    pub async fn check_url(&self, url: Url) -> UrlCheck {
        if let Some(cached) = self.cache.get(&url).await {
            debug!("Cache hit for {}", url);
            return cached;
        }

        debug!("Checking {}", url);
        // Try HEAD first, fall back to GET if HEAD fails or returns certain status codes
        let mut result = self.do_check(&url, true).await;
        if !result.success {
            debug!("HEAD failed for {}, trying GET", url);
            result = self.do_check(&url, false).await;
        }

        self.cache.insert(url, result.clone()).await;
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

    async fn do_check(&self, url: &Url, use_head: bool) -> UrlCheck {
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
                        let retry_after = resp
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|value| value.to_str().ok())
                            .and_then(|value| value.parse::<u64>().ok())
                            .unwrap_or(1);

                        warn!(
                            "Retrying {} after {} response from {} in {}s",
                            if use_head { "HEAD" } else { "GET" },
                            status,
                            url,
                            retry_after
                        );
                        retries += 1;
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
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
