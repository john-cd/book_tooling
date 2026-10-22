// TODO Reviewed ??-??-2026
use std::fmt::Write as _;

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

/// Client for interacting with GitHub API.
pub struct GithubCrawler {
    /// HTTP client preconfigured with the crawler headers.
    client: Client,
    /// Root of the GitHub REST API; overridable for tests.
    api_base: String,
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
            .build()
            .context("Failed to build GitHub crawler HTTP client")?;

        Ok(Self {
            client,
            api_base: GITHUB_API.to_owned(),
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

    /// Builds the GitHub code search URL for `query`, percent-encoding it and restricting results to `Cargo.toml` files.
    fn build_search_url(api_base: &str, query: &str, page: usize) -> String {
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
            "{api_base}/search/code?q={encoded_query}+filename:Cargo.toml&per_page={SEARCH_PAGE_SIZE}&page={page}"
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

    /// Searches GitHub code results for matching `Cargo.toml` files, following pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if a request fails, the API returns an error status (rate limiting is
    /// reported explicitly), or a response cannot be decoded.
    pub async fn search_cargo_tomls(&self, query: &str) -> Result<Vec<GithubItem>> {
        let mut items = Vec::new();
        for page in 1..=MAX_SEARCH_PAGES {
            let url = Self::build_search_url(&self.api_base, query, page);
            info!("Searching GitHub: {}", url);

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to send GitHub search request")?;
            let search_response: GithubSearchResponse = Self::ensure_success(response)?
                .json()
                .await
                .context("Failed to parse GitHub search response")?;

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
        // GitHub search API returns a URL to the API endpoint, we need to request with proper headers.
        let response = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3.raw")
            .send()
            .await
            .context("Failed to download content")?;

        let content = Self::ensure_success(response)?
            .text()
            .await
            .context("Failed to get response text")?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url_encoding() {
        let url = GithubCrawler::build_search_url(GITHUB_API, "anyhow&sort=stars", 2);
        assert_eq!(
            url,
            "https://api.github.com/search/code?q=anyhow%26sort%3Dstars+filename:Cargo.toml&per_page=100&page=2"
        );
    }

    #[test]
    fn test_new_rejects_invalid_token() {
        let result = GithubCrawler::new(Some("bad\nvalue"));
        assert!(
            result.is_err(),
            "crawler should reject invalid header values"
        );
    }
}
