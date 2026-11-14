//! Collects the reference definitions of a document.
//!
//! TODO Reviewed ??-??-2026.

use super::Visitable as _;
use crate::ast::Documents;
use crate::ast::ReferenceDefinitionData;

/// A visitor that collects reference definitions.
struct RefDefVisitor<'v>(RefDefManager<'v>);

impl RefDefVisitor<'_> {
    /// Creates a visitor with an empty collection.
    fn new() -> Self {
        Self(RefDefManager::new())
    }
}

impl<'a> super::Visitor<'a> for RefDefVisitor<'a> {
    /// Gathers the definition for later lookup.
    fn visit_reference_definition(&mut self, refdef: &'a ReferenceDefinitionData<'a>) {
        self.0.refdefs.push(refdef);
    }
}

/// Manages a collection of Markdown reference definitions.
///
/// It provides methods to identify duplicate labels, duplicate URLs,
/// and to obtain a deduplicated list of definitions.
pub struct RefDefManager<'v> {
    /// The collection of reference definitions.
    pub refdefs: Vec<&'v ReferenceDefinitionData<'v>>,
}

use multimap::MultiMap;

impl RefDefManager<'_> {
    /// Creates an empty manager.
    fn new() -> Self {
        Self {
            refdefs: Vec::new(),
        }
    }

    /// Returns the sorted, case-sensitive labels that occur more than once.
    #[must_use]
    pub fn dupe_labels(&self) -> Vec<&str> {
        // Could use `itertools` as well.
        let mm: MultiMap<&str, &ReferenceDefinitionData<'_>> =
            self.refdefs.iter().map(|&rd| (rd.label, rd)).collect();
        let mut labels: Vec<&str> = mm.keys().filter(|&&key| mm.is_vec(key)).copied().collect();
        labels.sort_unstable();
        labels
    }

    /// Returns the sorted destination strings that occur more than once.
    #[must_use]
    pub fn dupe_urls(&self) -> Vec<&str> {
        let mm: MultiMap<&str, &ReferenceDefinitionData<'_>> =
            self.refdefs.iter().map(|&rd| (rd.url, rd)).collect();
        let mut urls: Vec<&str> = mm.keys().filter(|&&key| mm.is_vec(key)).copied().collect();
        urls.sort_unstable();
        urls
    }

    /// Returns a sorted copy of the collected definitions with duplicates removed.
    #[must_use]
    pub fn get_sorted_dedupe_refdefs(&self) -> Vec<&ReferenceDefinitionData<'_>> {
        let mut refdefs = self.refdefs.clone();
        refdefs.sort();
        // If the vector is sorted, this removes all duplicates:
        refdefs.dedup();
        refdefs
    }
}

/// Collects reference definitions from a set of documents and returns a manager over them.
#[must_use]
pub fn refdefs<'d, 'a>(docs: &'a Documents<'d>) -> RefDefManager<'d>
where
    'a: 'd,
{
    let mut vis = RefDefVisitor::new();
    docs.accept(&mut vis);
    vis.0
}

/// Tests for duplicate reference labels and destinations.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    /// Reports duplicate labels and urls independently.
    #[test]
    fn reports_duplicate_labels_and_urls_independently() {
        let doc = parse_document(
            "test.md",
            None,
            "[alpha]: https://one.example\n[alpha]: https://two.example\n[beta]: https://one.example\n",
        )
        .expect("parse test markdown");
        let docs = Documents::new(vec![doc]);
        let manager = refdefs(&docs);

        assert_eq!(manager.dupe_labels(), vec!["alpha"]);
        assert_eq!(manager.dupe_urls(), vec!["https://one.example"]);
    }

    /// Get sorted dedupe refdefs removes only exact duplicates.
    #[test]
    fn get_sorted_dedupe_refdefs_removes_only_exact_duplicates() {
        let doc = parse_document(
            "test.md",
            None,
            "[alpha]: https://one.example\n[alpha]: https://one.example\n[alpha]: https://two.example\n[beta]: https://one.example\n",
        )
        .expect("parse test markdown");
        let docs = Documents::new(vec![doc]);
        let manager = refdefs(&docs);

        let deduped: Vec<(&str, &str)> = manager
            .get_sorted_dedupe_refdefs()
            .into_iter()
            .map(|refdef| (refdef.label, refdef.url))
            .collect();

        assert_eq!(
            deduped,
            vec![
                ("alpha", "https://one.example"),
                ("alpha", "https://two.example"),
                ("beta", "https://one.example"),
            ]
        );
    }
}
