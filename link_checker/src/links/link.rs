use tracing::{error, info};
use url::Url;
use parse_lib::Element;

pub struct Link {
    pub url: Url,
}

/// Check links in the provided content.
pub fn check_links(content: &str, _check_external_links: bool) -> Vec<Link> {
    let mut links = Vec::new();

    if let Ok(document) = parse_lib::parse_document("content", None, content) {
        for element in document.elements {
            match element {
                Element::InlineLink(data) => {
                    if let Ok(url) = Url::parse(data.url) {
                        links.push(Link { url });
                    }
                }
                Element::Autolink(data) => {
                    if let Ok(url) = Url::parse(data.url) {
                        links.push(Link { url });
                    }
                }
                _ => {}
            }
        }
    }

    links
}

pub async fn validate_link(link: &Link) -> bool {
    info!("Validating link: {}", link.url);
    match crate::links::asynchro::check_web_url(link.url.clone()).await {
        Ok(check) => {
            if check.success {
                info!("Link OK: {}", link.url);
                true
            } else {
                error!("Link FAILED ({}): {}", check.status, link.url);
                false
            }
        }
        Err(e) => {
            error!("Error checking link {}: {}", link.url, e);
            false
        }
    }
}
