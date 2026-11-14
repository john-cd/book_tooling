//! Searches GitHub for `Cargo.toml` files and downloads their content.
//!
//! TODO Reviewed ??-??-2026.

use std::fmt::Write as _;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::debug;
use tracing::info;

/// GitHub code search API response payload.
#[derive(Debug, Deserialize)]
pub struct GithubSearchResponse {
    /// Matching search items returned by the GitHub API.
    pub items: Vec<GithubItem>,
}

/// One GitHub code-search result item.
#[derive(Debug, Deserialize)]
pub struct GithubItem {
    /// Repository that owns the matching file.
    pub repository: GithubRepo,
    /// Path of the matching file inside the repository.
    pub path: String,
    /// API URL that can be used to retrieve the file contents.
    pub url: String,
}

/// Minimal repository metadata embedded in GitHub search results.
#[derive(Debug, Deserialize)]
pub struct GithubRepo {
    /// Repository name in `owner/name` form.
    pub full_name: String,
}

/// Base URL of the public GitHub REST API.
pub const GITHUB_API: &str = "https://api.github.com";

/// Results requested per search page (the GitHub maximum).
const SEARCH_PAGE_SIZE: usize = 100;

/// Pages fetched at most per search; GitHub exposes no more than 1,000 code-search results.
const MAX_SEARCH_PAGES: usize = 10;

/// Request and response limits used by [`GithubCrawler`].
#[derive(Clone, Copy, Debug)]
pub struct CrawlerLimits {
    /// Maximum time from starting a request through finishing its response body.
    pub request_timeout: Duration,
    /// Maximum time allowed to establish an HTTP connection.
    pub connect_timeout: Duration,
    /// Maximum number of bytes accepted in one GitHub code-search response.
    pub max_search_response_bytes: usize,
    /// Maximum number of bytes accepted in one downloaded file response.
    pub max_content_response_bytes: usize,
}

impl Default for CrawlerLimits {
    /// Returns finite defaults suited to GitHub code-search pages and source files.
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_search_response_bytes: 16 * 1024 * 1024,
            max_content_response_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Client for interacting with GitHub API.
pub struct GithubCrawler {
    /// HTTP client preconfigured with the crawler headers.
    client: Client,
    /// Root of the GitHub REST API; overridable for tests.
    api_base: String,
    /// Deadlines and response byte ceilings applied to every request.
    limits: CrawlerLimits,
}

impl GithubCrawler {
    /// Creates a crawler using an optional GitHub personal access token.
    ///
    /// Passing a token increases the GitHub API rate limit available to the crawler.
    ///
    /// # Errors
    ///
    /// Returns an error if the authorization header or HTTP client cannot be built.
    pub fn new(token: Option<&str>) -> Result<Self> {
        Self::new_with_limits(token, CrawlerLimits::default())
    }

    /// Creates a crawler with explicit request deadlines and response size ceilings.
    ///
    /// Passing a token increases the GitHub API rate limit available to the crawler. Both
    /// deadlines and both response limits must be greater than zero.
    ///
    /// # Errors
    ///
    /// Returns an error if a limit is zero, the authorization header is invalid, or the HTTP
    /// client cannot be built.
    pub fn new_with_limits(token: Option<&str>, limits: CrawlerLimits) -> Result<Self> {
        anyhow::ensure!(
            !limits.request_timeout.is_zero(),
            "GitHub request timeout must be greater than zero"
        );
        anyhow::ensure!(
            !limits.connect_timeout.is_zero(),
            "GitHub connect timeout must be greater than zero"
        );
        anyhow::ensure!(
            limits.max_search_response_bytes > 0,
            "GitHub search response byte limit must be greater than zero"
        );
        anyhow::ensure!(
            limits.max_content_response_bytes > 0,
            "GitHub content response byte limit must be greater than zero"
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("rust-rate-crawler"),
        );
        if let Some(secret) = token
            && !secret.is_empty()
        {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("token {secret}"))
                    .context("Failed to build GitHub authorization header")?,
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(limits.request_timeout)
            .connect_timeout(limits.connect_timeout)
            .build()
            .context("Failed to build GitHub crawler HTTP client")?;

        Ok(Self {
            client,
            api_base: GITHUB_API.to_owned(),
            limits,
        })
    }

    /// Points the crawler at another API root, such as a local test server.
    #[must_use]
    pub fn with_api_base(mut self, api_base: &str) -> Self {
        api_base
            .trim_end_matches('/')
            .clone_into(&mut self.api_base);
        self
    }

    /// Builds a paginated GitHub search URL, encoding the query and appending the supplied search
    /// qualifier, such as `filename:Cargo.toml`.
    fn build_search_url(api_base: &str, query: &str, qualifier: &str, page: usize) -> String {
        let mut encoded_query = String::with_capacity(query.len());
        for byte in query.as_bytes() {
            match byte {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded_query.push(char::from(*byte));
                }
                _ => {
                    encoded_query.push('%');
                    let _unused = write!(encoded_query, "{byte:02X}");
                }
            }
        }
        format!(
            "{api_base}/search/code?q={encoded_query}+{qualifier}&per_page={SEARCH_PAGE_SIZE}&page={page}"
        )
    }

    /// Turns a non-success GitHub response into an error that names the cause.
    ///
    /// Rate limiting (HTTP 403 or 429) mentions when the quota resets, if GitHub says so.
    fn ensure_success(response: Response) -> Result<Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        if matches!(
            status,
            StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
        ) {
            let reset = response
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|value| value.to_str().ok())
                .map_or_else(String::new, |epoch| {
                    format!(" (quota resets at Unix time {epoch})")
                });
            anyhow::bail!("GitHub API rate limit exceeded or forbidden access{reset}.");
        }
        anyhow::bail!("GitHub API returned HTTP {status}.")
    }

    /// Converts request timeouts into diagnostics that include the configured deadlines.
    fn request_error(
        error: reqwest::Error,
        operation: &str,
        limits: &CrawlerLimits,
    ) -> anyhow::Error {
        if error.is_timeout() {
            anyhow::anyhow!(
                "GitHub {operation} request timed out (request deadline {:?}, connect deadline {:?}): {error}",
                limits.request_timeout,
                limits.connect_timeout
            )
        } else {
            anyhow::Error::new(error).context(format!("Failed to send GitHub {operation} request"))
        }
    }

    /// Reads a response body incrementally and stops before it can exceed `max_bytes`.
    async fn read_bounded_response(
        mut response: Response,
        response_kind: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>> {
        if let Some(content_length) = response.content_length() {
            let maximum = u64::try_from(max_bytes).unwrap_or(u64::MAX);
            anyhow::ensure!(
                content_length <= maximum,
                "GitHub {response_kind} response declares {content_length} bytes, exceeding the configured limit of {max_bytes} bytes"
            );
        }

        let initial_capacity = response
            .content_length()
            .and_then(|length| usize::try_from(length).ok())
            .unwrap_or_default()
            .min(max_bytes);
        let mut body = Vec::with_capacity(initial_capacity);
        while let Some(chunk) = response
            .chunk()
            .await
            .with_context(|| format!("Failed to read GitHub {response_kind} response body"))?
        {
            let next_length = body
                .len()
                .checked_add(chunk.len())
                .context("GitHub response body size overflowed")?;
            anyhow::ensure!(
                next_length <= max_bytes,
                "GitHub {response_kind} response exceeds the configured limit of {max_bytes} bytes"
            );
            body.extend_from_slice(&chunk);
        }

        Ok(body)
    }

    /// Searches GitHub code results for matching `Cargo.toml` files, following pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if a request fails, the API returns an error status (rate limiting is
    /// reported explicitly), or a response cannot be decoded.
    pub async fn search_cargo_tomls(&self, query: &str) -> Result<Vec<GithubItem>> {
        self.search_code(query, "filename:Cargo.toml").await
    }

    /// Searches GitHub code results for Rust source files that import the crate `crate_name`.
    ///
    /// # Errors
    ///
    /// Returns an error under the same conditions as [`Self::search_cargo_tomls`].
    pub async fn search_rust_sources(&self, crate_name: &str) -> Result<Vec<GithubItem>> {
        self.search_code(&format!("\"use {crate_name}::\""), "extension:rs")
            .await
    }

    /// Runs a GitHub code search for `query` restricted by `qualifier`, following pagination.
    async fn search_code(&self, query: &str, qualifier: &str) -> Result<Vec<GithubItem>> {
        let mut items = Vec::new();
        for page in 1..=MAX_SEARCH_PAGES {
            let url = Self::build_search_url(&self.api_base, query, qualifier, page);
            info!("Searching GitHub: {}", url);

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|error| Self::request_error(error, "search", &self.limits))?;
            let body = Self::read_bounded_response(
                Self::ensure_success(response)?,
                "search",
                self.limits.max_search_response_bytes,
            )
            .await?;
            let search_response: GithubSearchResponse =
                serde_json::from_slice(&body).context("Failed to parse GitHub search response")?;

            let fetched = search_response.items.len();
            items.extend(search_response.items);
            if fetched < SEARCH_PAGE_SIZE {
                break;
            }
        }
        Ok(items)
    }

    /// Downloads raw file content from a GitHub API content URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the API returns an error status.
    pub async fn download_content(&self, url: &str) -> Result<String> {
        debug!("Downloading content from: {}", url);
        // GitHub search API returns a URL to the API endpoint, we need to request with proper
        // headers.
        let response = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3.raw")
            .send()
            .await
            .map_err(|error| Self::request_error(error, "content", &self.limits))?;

        let bytes = Self::read_bounded_response(
            Self::ensure_success(response)?,
            "content",
            self.limits.max_content_response_bytes,
        )
        .await?;
        let content =
            String::from_utf8(bytes).context("GitHub content response was not valid UTF-8")?;
        Ok(content)
    }
}

/// Tests for GitHub pagination, response parsing, and request handling.
#[cfg(test)]
mod tests {
    use super::*;

    /// The query is percent-encoded and paging parameters are appended.
    #[test]
    fn test_build_search_url_encoding() {
        let url = GithubCrawler::build_search_url(
            GITHUB_API,
            "anyhow&sort=stars",
            "filename:Cargo.toml",
            2,
        );
        assert_eq!(
            url,
            "https://api.github.com/search/code?q=anyhow%26sort%3Dstars+filename:Cargo.toml&per_page=100&page=2"
        );
    }

    /// A token that is not a valid header value is rejected.
    #[test]
    fn test_new_rejects_invalid_token() {
        let result = GithubCrawler::new(Some("bad\nvalue"));
        assert!(
            result.is_err(),
            "crawler should reject invalid header values"
        );
    }
}
