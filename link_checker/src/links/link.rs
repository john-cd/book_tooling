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
