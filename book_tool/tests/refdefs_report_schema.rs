//! Strict structural validation of the JSON report written by `book_tool refdefs-audit --report`.
//!
//! TODO Reviewed ??-??-2026.

#![expect(
    clippy::expect_used,
    reason = "Helpers in this test file fail loudly when test setup breaks."
)]

use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

/// Field names every definition record must have, and no others.
const DEFINITION_FIELDS: [&str; 11] = [
    "path",
    "line",
    "column",
    "label",
    "normalized_label",
    "included_by",
    "uses",
    "uses_outside_scoped_roots",
    "ambiguous",
    "protected",
    "proven_unused",
];

/// Field names every use record must have, and no others.
const USE_FIELDS: [&str; 5] = ["path", "line", "column", "label", "kind"];

/// Asserts that `value` is an object with exactly the `expected` keys.
fn assert_exact_keys(value: &Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .expect("every report record must be an object");
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    assert_eq!(actual, wanted, "unexpected keys in {context}");
}

/// Checks that a JSON value is a positive integer. Uses `context` to identify the field on failure.
fn assert_positive_integer(value: &Value, context: &str) {
    assert!(
        value.as_u64().is_some_and(|number| number >= 1),
        "{context} must be a positive integer: {value}"
    );
}

/// Checks a reference-use record's keys, source position, label, and use kind. Uses `context` to
/// identify the record on failure.
fn assert_valid_use(record: &Value, context: &str) {
    assert_exact_keys(record, &USE_FIELDS, context);
    assert!(record["path"].is_string(), "{context}.path");
    assert_positive_integer(&record["line"], &format!("{context}.line"));
    assert_positive_integer(&record["column"], &format!("{context}.column"));
    assert!(record["label"].is_string(), "{context}.label");
    assert!(
        matches!(record["kind"].as_str(), Some("link" | "image")),
        "{context}.kind must be \"link\" or \"image\": {}",
        record["kind"]
    );
}

/// Checks the definition record at `index`, including its normalized label, include provenance, and
/// nested uses.
fn assert_valid_definition(record: &Value, index: usize) {
    let context = format!("definition[{index}]");
    assert_exact_keys(record, &DEFINITION_FIELDS, &context);
    assert!(record["path"].is_string(), "{context}.path");
    assert_positive_integer(&record["line"], &format!("{context}.line"));
    assert_positive_integer(&record["column"], &format!("{context}.column"));
    let label = record["label"].as_str().expect("label is a string");
    let normalized = record["normalized_label"]
        .as_str()
        .expect("normalized_label is a string");
    assert_eq!(
        normalized,
        label
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
        "{context}.normalized_label must be the folded label"
    );
    let included_by = record["included_by"].as_array().expect("array field");
    assert!(
        included_by.iter().all(Value::is_string),
        "{context}.included_by"
    );
    for field in ["uses", "uses_outside_scoped_roots"] {
        let entries = record[field].as_array().expect("array field");
        for (position, usage) in entries.iter().enumerate() {
            assert_valid_use(usage, &format!("{context}.{field}[{position}]"));
        }
    }
    for field in ["ambiguous", "protected", "proven_unused"] {
        assert!(record[field].is_boolean(), "{context}.{field}");
    }
    if record["proven_unused"] == Value::Bool(true) {
        assert!(
            record["uses"].as_array().is_some_and(Vec::is_empty),
            "{context}: a proven-unused definition cannot have uses"
        );
        assert_eq!(record["ambiguous"], Value::Bool(false), "{context}");
    }
}

/// The report is a JSON array of records with exactly the documented fields and value types.
#[test]
fn refdefs_audit_report_matches_the_strict_schema() {
    let temp = tempdir().expect("create temporary book directory");
    let book = temp.path().join("book");
    let other = temp.path().join("other");
    fs::create_dir_all(&book).expect("create book directory");
    fs::create_dir_all(&other).expect("create supplementary directory");
    fs::write(
        book.join("chapter.md"),
        "{{#include refs.incl.md}}\n\n[Used] [also][Spaced  Label] ![img][pic]\n",
    )
    .expect("write chapter");
    fs::write(
        book.join("refs.incl.md"),
        concat!(
            "[used]: https://example.com/used\n",
            "[spaced label]: https://example.com/spaced\n",
            "[pic]: https://example.com/pic.png\n",
            "[dup]: https://example.com/dup1\n",
            "[DUP]: https://example.com/dup2\n",
            "[unused]: https://example.com/unused\n",
            "[elsewhere]: https://example.com/elsewhere\n",
        ),
    )
    .expect("write definitions");
    fs::write(other.join("notes.md"), "See [elsewhere].\n").expect("write supplementary");
    let report = temp.path().join("report.json");

    let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .arg("refdefs-audit")
        .arg(&book)
        .arg("--report")
        .arg(&report)
        .arg("--also-scan-uses-in")
        .arg(&other)
        .output()
        .expect("run refdefs-audit");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let text = fs::read_to_string(&report).expect("read report");
    assert!(text.ends_with('\n'), "report must end with a newline");
    let value: Value = serde_json::from_str(&text).expect("report is valid JSON");
    let records = value.as_array().expect("report is a JSON array");
    assert_eq!(records.len(), 7, "{text}");
    for (index, record) in records.iter().enumerate() {
        assert_valid_definition(record, index);
    }

    let by_label = |label: &str| -> &Value {
        records
            .iter()
            .find(|record| record["label"] == label)
            .expect("report has a record for the label")
    };
    assert_eq!(by_label("unused")["proven_unused"], Value::Bool(true));
    assert_eq!(by_label("used")["proven_unused"], Value::Bool(false));
    assert_eq!(by_label("dup")["ambiguous"], Value::Bool(true));
    assert_eq!(
        by_label("elsewhere")["uses_outside_scoped_roots"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

/// Running the audit twice over unchanged input produces byte-identical reports.
#[test]
fn refdefs_audit_report_is_deterministic() {
    let temp = tempdir().expect("create temporary book directory");
    fs::write(
        temp.path().join("chapter.md"),
        "{{#include refs.incl.md}}\n[a] [b] [c]\n",
    )
    .expect("write chapter");
    fs::write(
        temp.path().join("refs.incl.md"),
        "[c]: https://c.example\n[a]: https://a.example\n[b]: https://b.example\n[z]: https://z.example\n",
    )
    .expect("write definitions");
    let mut reports = Vec::new();
    for name in ["one.json", "two.json"] {
        let report = temp.path().join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
            .arg("refdefs-audit")
            .arg(temp.path())
            .arg("--report")
            .arg(&report)
            .output()
            .expect("run refdefs-audit");
        assert!(output.status.success());
        reports.push(fs::read_to_string(report).expect("read report"));
    }
    assert_eq!(reports[0], reports[1]);
}

/// An empty book still produces a valid, empty JSON array.
#[test]
fn refdefs_audit_report_for_a_book_without_definitions_is_an_empty_array() {
    let temp = tempdir().expect("create temporary book directory");
    fs::write(temp.path().join("chapter.md"), "No references here.\n").expect("write chapter");
    let report = temp.path().join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .arg("refdefs-audit")
        .arg(temp.path())
        .arg("--report")
        .arg(&report)
        .output()
        .expect("run refdefs-audit");
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(report).expect("read report"), "[]\n");
}
