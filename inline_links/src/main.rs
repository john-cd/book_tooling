mod cli;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;

use clap::Parser;
use parse_lib::{Document, Element, reconstruct_markdown, ReferenceStyleLinkData};
use url::Url;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = cli::Args::parse();
    let scope = core_lib::Scope::default();
    for directory in &args.directories {
        let dir = directory.as_path().canonicalize()?;
        println!("Processing {}", dir.display());
        core_lib::walk_directory_and_process_files(&dir, &scope, convert_links)?;
    }
    println!("DONE");
    Ok(())
}

fn convert_links(filepath: &Path) -> anyhow::Result<()> {
    core_lib::process_text_file(
        filepath,
        |s: &str| s.contains("http://") || s.contains("https://"),
        |content: &str| {
            if let Ok(document) = parse_lib::parse_document("file", Some(filepath), content) {
                let (new_elements, refdefs) = transform_document(document);
                if refdefs.is_empty() {
                    return Cow::Borrowed(content);
                }

                let mut final_content = reconstruct_markdown(&new_elements);
                if !final_content.ends_with("\n\n") {
                    if final_content.ends_with('\n') {
                        final_content.push('\n');
                    } else {
                        final_content.push_str("\n\n");
                    }
                }

                // Sort refdefs by label
                let mut sorted_labels: Vec<_> = refdefs.keys().collect();
                sorted_labels.sort();

                for label in sorted_labels {
                    final_content.push_str(&format!(
                        "[{}]: {}\n",
                        label,
                        refdefs.get(label).unwrap()
                    ));
                }

                Cow::Owned(final_content)
            } else {
                Cow::Borrowed(content)
            }
        },
    )?;
    Ok(())
}

fn transform_document(doc: Document) -> (Vec<Element>, BTreeMap<String, String>) {
    let mut refdefs = BTreeMap::new();
    let mut new_elements = Vec::new();
    let mut used_labels = HashSet::new();

    // First pass to collect existing reference labels to avoid collisions
    for element in &doc.elements {
        if let Element::ReferenceDefinition(data) = element {
            used_labels.insert(data.label.to_string());
        }
    }

    for element in doc.elements {
        match element {
            Element::InlineLink(data) => {
                let label = generate_label(data.url, &used_labels);
                used_labels.insert(label.clone());
                refdefs.insert(label.clone(), data.url.to_string());
                new_elements.push(Element::ReferenceStyleLink(ReferenceStyleLinkData {
                    text: data.text,
                    label: Box::leak(label.into_boxed_str()),
                }));
            }
            Element::Autolink(data) => {
                let label = generate_label(data.url, &used_labels);
                used_labels.insert(label.clone());
                refdefs.insert(label.clone(), data.url.to_string());
                new_elements.push(Element::ReferenceStyleLink(ReferenceStyleLinkData {
                    text: data.url,
                    label: Box::leak(label.into_boxed_str()),
                }));
            }
            Element::ReferenceDefinition(data) => {
                // Keep existing refdefs but only if they are not being replaced.
                // Actually, we keep them all for simplicity here.
                new_elements.push(Element::ReferenceDefinition(data));
            }
            _ => {
                new_elements.push(element);
            }
        }
    }

    (new_elements, refdefs)
}

fn generate_label(url_str: &str, used: &HashSet<String>) -> String {
    if let Ok(url) = Url::parse(url_str) {
        let mut label = if let Some(host) = url.host_str() {
            host.to_string()
        } else {
            "link".to_string()
        };

        label = label.replace('.', "-");

        if used.contains(&label) {
            let mut i = 1;
            while used.contains(&format!("{}-{}", label, i)) {
                i += 1;
            }
            label = format!("{}-{}", label, i);
        }
        label
    } else {
        "link".to_string()
    }
}
