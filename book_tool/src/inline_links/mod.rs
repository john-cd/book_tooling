use std::path::PathBuf;

use anyhow::Result;
use parse_lib::visit::Visitor;

use crate::common::process_text_file;
use crate::common::walk::Scope;
use crate::common::walk::walk_directory_and_process_files;

pub fn execute(directories: Vec<PathBuf>) -> Result<()> {
    let scope = Scope::default();

    for dir in &directories {
        walk_directory_and_process_files(dir, &scope, |path| {
            process_text_file(
                path,
                |s| s.contains("http") || s.contains("["),
                |content| {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    match parse_lib::parse_document(&filename, Some(path), content) {
                        Ok(doc) => {
                            let mut existing_url_to_label = std::collections::HashMap::new();
                            let mut existing_labels = std::collections::HashSet::new();

                            struct RefDefCollector<'a> {
                                existing_url_to_label:
                                    &'a mut std::collections::HashMap<String, String>,
                                existing_labels: &'a mut std::collections::HashSet<String>,
                            }
                            impl<'a, 'ast> Visitor<'ast> for RefDefCollector<'a> {
                                fn visit_reference_definition(
                                    &mut self,
                                    data: &'ast parse_lib::ast::ReferenceDefinitionData<'ast>,
                                ) {
                                    self.existing_url_to_label
                                        .insert(data.url.to_string(), data.label.to_string());
                                    self.existing_labels.insert(data.label.to_string());
                                }
                            }
                            let mut collector = RefDefCollector {
                                existing_url_to_label: &mut existing_url_to_label,
                                existing_labels: &mut existing_labels,
                            };
                            collector.visit_document(&doc);

                            let existing_url_to_label = std::rc::Rc::new(existing_url_to_label);
                            let existing_labels =
                                std::rc::Rc::new(std::cell::RefCell::new(existing_labels));
                            let new_url_to_label = std::rc::Rc::new(std::cell::RefCell::new(
                                std::collections::BTreeMap::new(),
                            ));
                            let link_counter = std::rc::Rc::new(std::cell::RefCell::new(1));

                            let mut renderer =
                                parse_lib::markdown_renderer::MarkdownRenderer::new(|directive| {
                                    directive.to_string()
                                });

                            let inline_existing_url_to_label =
                                std::rc::Rc::clone(&existing_url_to_label);
                            let inline_existing_labels = std::rc::Rc::clone(&existing_labels);
                            let inline_new_url_to_label = std::rc::Rc::clone(&new_url_to_label);
                            let inline_link_counter = std::rc::Rc::clone(&link_counter);

                            renderer = renderer.with_inline_link_resolver(move |link| {
                                if link.url.contains("github.com/john-cd/rust_howto/issues") {
                                    return format!("[{}]({})", link.text, link.url);
                                }
                                let url_str = link.url.to_string();
                                if let Some(lbl) = inline_existing_url_to_label.get(&url_str) {
                                    return format!("[{}][{}]", link.text, lbl);
                                }
                                let mut new_map = inline_new_url_to_label.borrow_mut();
                                let mut labels = inline_existing_labels.borrow_mut();
                                let mut counter = inline_link_counter.borrow_mut();

                                let label = new_map
                                    .entry(url_str)
                                    .or_insert_with(|| {
                                        while labels.contains(&counter.to_string()) {
                                            *counter += 1;
                                        }
                                        let l = counter.to_string();
                                        labels.insert(l.clone());
                                        l
                                    })
                                    .clone();
                                format!("[{}][{}]", link.text, label)
                            });

                            let auto_existing_url_to_label =
                                std::rc::Rc::clone(&existing_url_to_label);
                            let auto_existing_labels = std::rc::Rc::clone(&existing_labels);
                            let auto_new_url_to_label = std::rc::Rc::clone(&new_url_to_label);
                            let auto_link_counter = std::rc::Rc::clone(&link_counter);

                            renderer = renderer.with_autolink_resolver(move |link| {
                                if link.url.contains("github.com/john-cd/rust_howto/issues") {
                                    return format!("<{}>", link.url);
                                }
                                let url_str = link.url.to_string();
                                if let Some(lbl) = auto_existing_url_to_label.get(&url_str) {
                                    return format!("[{}][{}]", link.url, lbl);
                                }
                                let mut new_map = auto_new_url_to_label.borrow_mut();
                                let mut labels = auto_existing_labels.borrow_mut();
                                let mut counter = auto_link_counter.borrow_mut();

                                let label = new_map
                                    .entry(url_str)
                                    .or_insert_with(|| {
                                        while labels.contains(&counter.to_string()) {
                                            *counter += 1;
                                        }
                                        let l = counter.to_string();
                                        labels.insert(l.clone());
                                        l
                                    })
                                    .clone();
                                format!("[{}][{}]", link.url, label)
                            });

                            renderer.visit_document(&doc);
                            let mut final_output = std::mem::take(&mut renderer.markdown);
                            drop(renderer);

                            let new_map = std::rc::Rc::try_unwrap(new_url_to_label)
                                .expect("Rc references still alive")
                                .into_inner();
                            if !new_map.is_empty() {
                                final_output.push('\n');
                                let mut sorted_refdefs: Vec<_> = new_map
                                    .into_iter()
                                    .map(|(url, label)| {
                                        (label.parse::<usize>().unwrap_or(0), label, url)
                                    })
                                    .collect();
                                sorted_refdefs.sort_by_key(|k| k.0);
                                for (_, label, url) in sorted_refdefs {
                                    final_output.push_str(&format!("[{}]: {}\n", label, url));
                                }
                            }

                            std::borrow::Cow::Owned(final_output)
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse {}: {}", path.display(), e);
                            std::borrow::Cow::Borrowed(content)
                        }
                    }
                },
            )?;
            Ok(())
        })?;
    }
    println!("Inline links converted successfully.");
    Ok(())
}
