//! Uses `book_tool` as a library: unused reference definitions and external link checking.
//!
//! The example builds a tiny book in a temporary directory, lists the unused definitions that
//! [`find_unused_refdefs_in_scopes`] proves, and checks one address with [`AsyncLinkChecker`]. The
//! link check needs network access; run it with `cargo run -p book_tool --example library_usage`.
//!
//! TODO Reviewed ??-??-2026

use anyhow::Result;
use book_tool::links::AsyncLinkChecker;
use book_tool::refdefs::find_unused_refdefs_in_scopes;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let book = tempfile::tempdir()?;
    std::fs::write(
        book.path().join("chapter.md"),
        "{{#include refs.incl.md}}\n\nSee [the Rust site][rust].\n",
    )?;
    std::fs::write(
        book.path().join("refs.incl.md"),
        "[rust]: https://www.rust-lang.org\n[forgotten]: https://example.com/forgotten\n",
    )?;

    let analysis = find_unused_refdefs_in_scopes([book.path()])?;
    for (file, labels) in &analysis.unused_by_file {
        let mut labels: Vec<_> = labels.iter().map(String::as_str).collect();
        labels.sort_unstable();
        println!("{} has unused definitions: {labels:?}", file.display());
    }

    let checker = AsyncLinkChecker::new()?;
    let url = Url::parse("https://www.rust-lang.org")?;
    let outcome = checker.check_url(url.clone()).await;
    println!(
        "{url}: success = {} ({})",
        outcome.success,
        outcome.describe()
    );
    Ok(())
}
