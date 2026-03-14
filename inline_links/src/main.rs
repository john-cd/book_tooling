//! Convert any naked URLs and inline links in Markdown files within a folder into reference-style links.
//!
//! - Convert inline links e.g. [...](http(s)://...) into reference-style links: [...][...] [...]: http://...
//! - Convert http(s)://... naked links into reference-style links.
//! - Skip URLs between ``` and ``` and within hidden sections: <div class="hidden">...</div>
//! - Ignore URLs within reference-style link labels: [https://...][...]
//! - Do not convert links to GitHub issues e.g. https://github.com/john-cd/rust_howto/issues links.
//! - Move refdefs into a central file (and sort them).

mod cli;
mod converter;

use std::borrow::Cow;
use std::path::Path;

use clap::Parser;
use converter::LinkConverter;
use parse_lib::parse_document;

fn main() -> anyhow::Result<()> {
    // Install a global tracing subscriber that listens for events
    // and filters based on the value of the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();

    // Process command-line arguments to retrieve the directory to process:
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

/// Convert inline and naked links to reference-style links.
fn convert_links(filepath: &Path) -> anyhow::Result<()> {
    core_lib::process_text_file(
        filepath,
        |s: &str| s.contains("http") || s.contains("["),
        |content: &str| {
            let filename = filepath.file_name().unwrap_or_default().to_string_lossy();
            let doc = match parse_document(&filename, Some(filepath), content) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to parse {}: {}", filepath.display(), e);
                    return Cow::Borrowed(content);
                }
            };

            let mut converter = LinkConverter::new();
            if let Err(e) = doc.accept(&mut converter) {
                eprintln!("Failed to transform {}: {}", filepath.display(), e);
                return Cow::Borrowed(content);
            }

            let mut final_output = converter.output;
            if !converter.refdefs.is_empty() {
                final_output.push_str("\n\n");
                // Sort refdefs by label for consistency.
                let sorted_refdefs: std::collections::BTreeMap<_, _> = converter.refdefs;
                for (label, url) in sorted_refdefs {
                    final_output.push_str(&format!("[{}]: {}\n", label, url));
                }
            }

            Cow::Owned(final_output)
        },
    )?;
    Ok(())
}
