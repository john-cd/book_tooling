use std::time::Duration;
use anyhow::Result;
use reqwest::{ClientBuilder, redirect::Policy};
use url::Url;
use futures_util::stream::{self, StreamExt};
use moka::future::Cache;

/// Results of a batch of URL checks.
pub type UrlChecks = Vec<UrlCheck>;

/// Result of a single URL check.
#[derive(Clone, Debug)]
pub struct UrlCheck {
    pub url: Url,
    pub success: bool,
    pub status: reqwest::StatusCode,
}

pub struct LinkValidator {
    client: reqwest::Client,
    cache: Cache<Url, UrlCheck>,
}

impl LinkValidator {
    pub fn new(follow_redirects: bool) -> Self {
        let timeout = Duration::new(10, 0);
        let redirect_policy = if follow_redirects {
            Policy::default()
        } else {
            Policy::none()
        };

        let client = ClientBuilder::new()
            .user_agent("book_tooling/0.1.0")
            .timeout(timeout)
            .redirect(redirect_policy)
            .build()
            .unwrap();

        let cache = Cache::new(1000);

        Self { client, cache }
    }

    pub async fn check_web_url(&self, url: Url) -> Result<UrlCheck> {
        if let Some(check) = self.cache.get(&url).await {
            return Ok(check);
        }

        let response = self.client.head(url.clone()).send().await?;
        let check = UrlCheck {
            url: url.clone(),
            success: response.status().is_success(),
            status: response.status(),
        };

        self.cache.insert(url, check.clone()).await;
        Ok(check)
    }

    pub async fn validate_all(&self, urls: Vec<Url>) -> UrlChecks {
        let mut stream = stream::iter(urls)
            .map(|url| async move {
                self.check_web_url(url).await
            })
            .buffer_unordered(10);

        let mut results = Vec::new();
        while let Some(res) = stream.next().await {
            if let Ok(check) = res {
                results.push(check);
            }
        }
        results
    }
}

impl Default for LinkValidator {
    fn default() -> Self {
        Self::new(true)
    }
}
