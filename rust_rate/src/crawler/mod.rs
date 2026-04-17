use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
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

/// Client for interacting with GitHub API.
pub struct GithubCrawler {
    client: Client,
}

impl GithubCrawler {
    /// Creates a crawler using an optional GitHub personal access token.
    ///
    /// Passing a token increases the GitHub API rate limit available to the crawler.
    pub fn new(token: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("rust-rate-crawler"),
        );
        if let Some(ref t) = token {
            if !t.is_empty() {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("token {}", t)).unwrap(),
                );
            }
        }

        let client = Client::builder().default_headers(headers).build().unwrap();

        Self { client }
    }

    fn build_search_url(&self, query: &str) -> String {
        let mut encoded_query = String::with_capacity(query.len());
        for b in query.as_bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded_query.push(*b as char)
                }
                _ => {
                    encoded_query.push('%');
                    encoded_query.push_str(&format!("{:02X}", b));
                }
            }
        }
        format!(
            "https://api.github.com/search/code?q={}+filename:Cargo.toml",
            encoded_query
        )
    }

    /// Searches GitHub code results for matching `Cargo.toml` files.
    pub async fn search_cargo_tomls(&self, query: &str) -> Result<Vec<GithubItem>> {
        let url = self.build_search_url(query);
        info!("Searching GitHub: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send GitHub search request")?;

        if response.status() == 403 {
            anyhow::bail!("GitHub API rate limit exceeded or forbidden access.");
        }

        let search_response: GithubSearchResponse = response
            .json()
            .await
            .context("Failed to parse GitHub search response")?;

        Ok(search_response.items)
    }

    /// Downloads raw file content from a GitHub API content URL.
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

        let content = response
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
        let crawler = GithubCrawler::new(None);
        let query = "anyhow&sort=stars";
        let url = crawler.build_search_url(query);
        // Currently, it just interpolates, so it will contain the raw '&' and '='
        // We want them to be encoded so they are part of the 'q' parameter.
        assert!(
            url.contains("q=anyhow%26sort%3Dstars+filename:Cargo.toml")
                || url.contains("q=anyhow%26sort%3Dstars%20filename:Cargo.toml"),
            "URL should have encoded query parameters. Got: {}",
            url
        );
    }
}
