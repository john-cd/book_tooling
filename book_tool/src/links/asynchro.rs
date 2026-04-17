use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;
use tracing::debug;
use url::Url;

#[derive(Debug, Clone)]
pub struct UrlCheck {
    pub success: bool,
    pub status: Option<reqwest::StatusCode>,
    pub error: Option<String>,
}

pub struct AsyncLinkChecker {
    client: Client,
    cache: Cache<Url, UrlCheck>,
}

impl Default for AsyncLinkChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncLinkChecker {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("book-tooling-link-checker")
            .build()
            .unwrap();

        let cache = Cache::new(1000);

        Self { client, cache }
    }

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

    async fn do_check(&self, url: &Url, use_head: bool) -> UrlCheck {
        let request = if use_head {
            self.client.head(url.clone())
        } else {
            self.client.get(url.clone())
        };

        match request.send().await {
            Ok(resp) => UrlCheck {
                success: resp.status().is_success(),
                status: Some(resp.status()),
                error: None,
            },
            Err(e) => UrlCheck {
                success: false,
                status: None,
                error: Some(e.to_string()),
            },
        }
    }
}
