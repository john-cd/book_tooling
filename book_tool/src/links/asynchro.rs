use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;
use tokio::task::JoinSet;
use tracing::debug;
use tracing::warn;
use url::Url;

#[derive(Debug, Clone)]
pub struct UrlCheck {
    pub success: bool,
    pub status: Option<reqwest::StatusCode>,
    pub error: Option<String>,
}

#[derive(Clone)]
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
