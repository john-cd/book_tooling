use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::warn;

/// A concurrent HTTP link verifier with built-in caching.
#[derive(Clone)]
pub struct LinkVerifier {
    client: Client,
    /// Stores previously checked URLs and whether they were valid (true = 2xx status)
    cache: Arc<Mutex<HashMap<String, bool>>>,
}

impl LinkVerifier {
    /// Creates a new `LinkVerifier` with reasonable defaults.
    pub fn new() -> Self {
        // Configuring a client with a timeout and user-agent is highly recommended
        // because some servers block blank requests, and broken links could hang forever.
        let client = Client::builder()
            .user_agent("book_tooling_link_checker/1.0")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Verifies a batch of unique URLs concurrently, utilizing the cache.
    pub async fn verify_links<'a>(&self, links: &HashSet<&'a str>) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        let mut to_verify = Vec::new();

        // 1. Check the cache first to avoid redundant tasks
        {
            let cache = self.cache.lock().await;
            for &link in links {
                let url = link.to_string();
                if let Some(&is_valid) = cache.get(&url) {
                    results.insert(url, is_valid);
                } else {
                    to_verify.push(url);
                }
            }
        }

        // 2. Concurrently verify uncached links
        let mut set = JoinSet::new();

        for url in to_verify {
            let client = self.client.clone(); // Client is an Arc internally, cheap to clone
            let cache = self.cache.clone();

            set.spawn(async move {
                let mut is_valid = false;
                let mut retries = 0;
                let max_retries = 3;

                while retries < max_retries {
                    match client.get(&url).send().await {
                        Ok(res) => {
                            let status = res.status();
                            if status.is_success() {
                                is_valid = true;
                                break;
                            } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
                                // Extract standard retry-after header if present
                                let retry_after = res.headers().get("retry-after")
                                    .and_then(|v| v.to_str().ok()).unwrap_or("unknown");

                                if url.contains("github.com") || url.contains("api.github.com") {
                                    // GitHub specific rate limit headers
                                    let limit = res.headers().get("x-ratelimit-limit")
                                        .and_then(|v| v.to_str().ok()).unwrap_or("?");
                                    let remaining = res.headers().get("x-ratelimit-remaining")
                                        .and_then(|v| v.to_str().ok()).unwrap_or("?");
                                    let reset = res.headers().get("x-ratelimit-reset")
                                        .and_then(|v| v.to_str().ok()).unwrap_or("?");

                                    warn!(
                                        "GitHub Rate Limit Exceeded for {}. Limit: {}, Remaining: {}, Reset Epoch: {}, Retry-After: {}s",
                                        url, limit, remaining, reset, retry_after
                                    );
                                } else {
                                    warn!("Rate Limit Exceeded (429) for {}. Retry-After: {}s", url, retry_after);
                                }

                                if let Ok(seconds) = retry_after.parse::<u64>() {
                                    warn!("Sleeping for {} seconds before retrying {}", seconds, url);
                                    tokio::time::sleep(Duration::from_secs(seconds)).await;
                                    retries += 1;
                                    continue;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                // Update the global cache
                cache.lock().await.insert(url.clone(), is_valid);

                (url, is_valid)
            });
        }

        // 3. Await and collect all network results
        while let Some(res) = set.join_next().await {
            if let Ok((url, is_valid)) = res {
                results.insert(url, is_valid);
            }
        }

        results
    }
}
