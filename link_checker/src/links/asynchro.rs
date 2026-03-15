use std::time::Duration;

use anyhow::Result;
use reqwest::ClientBuilder;
use url::Url;

/// Results of a batch of URL checks.
pub type UrlChecks = Vec<UrlCheck>;

/// Result of a single URL check.
#[allow(dead_code)]
pub struct UrlCheck {
    pub url: Url,
    pub success: bool,
    pub status: reqwest::StatusCode,
    pub final_url: Url,
}

/// Query using a HEAD request (Client::head) and then inspect the response code
/// to determine success.
pub async fn check_web_url(url: Url) -> Result<UrlCheck> {
    let timeout = Duration::new(10, 0);
    let client = ClientBuilder::new()
        .user_agent("book_tooling/0.1.0")
        .timeout(timeout)
        .build()?;

    let response: reqwest::Response = client.head(url.clone()).send().await?;

    Ok(UrlCheck {
        url,
        success: response.status().is_success(),
        status: response.status(),
        final_url: response.url().clone(),
    })
}

/// Check a list of urls asynchronously.
#[allow(dead_code)]
pub async fn check_web_urls(urls: Vec<Url>) -> Result<UrlChecks> {
    let mut results = Vec::with_capacity(urls.len());
    for url in urls {
        results.push(check_web_url(url).await?);
    }
    Ok(results)
}

/// Launch a collection of URL checks.
#[allow(dead_code)]
pub async fn validate_all(urls: Vec<Url>) -> Result<UrlChecks> {
    check_web_urls(urls).await
}
