use anyhow::anyhow;
use book_tool::directives::Config;
use book_tool::directives::process_markdown_with_example_resolver;

/// Verifies chapter-relative example paths and anchors reach the caller-owned resolver unchanged.
#[test]
fn example_resolver_receives_the_original_chapter_relative_path_and_anchor() {
    let source =
        "{{#example ../../crates/language/examples/constants_and_statics/constants.rs:example}}\n";
    let expected_name = "../../crates/language/examples/constants_and_statics/constants.rs:example";
    let mut resolved_name = None;

    let rendered = process_markdown_with_example_resolver(source, &Config::default(), |name| {
        resolved_name = Some(name.to_owned());
        Ok(format!("{{{{#include {name}}}}}"))
    })
    .expect("resolve chapter-relative Rust example");

    assert_eq!(resolved_name.as_deref(), Some(expected_name));
    assert_eq!(rendered, format!("{{{{#include {expected_name}}}}}\n"));
    assert_eq!(
        process_markdown_with_example_resolver(&rendered, &Config::default(), |_| {
            Err(anyhow!(
                "native include must not invoke the example resolver"
            ))
        })
        .expect("native include output is idempotent"),
        rendered
    );
}

/// Verifies an adapter's ambiguous-example error is returned with the original directive name.
#[test]
fn example_resolver_reports_ambiguous_names_with_context() {
    let source = "{{#example snippets/shared.rs:example}}";
    let error = process_markdown_with_example_resolver(source, &Config::default(), |name| {
        Err(anyhow!("ambiguous example path: {name}"))
    })
    .expect_err("ambiguous examples must not produce a successful render");

    assert!(format!("{error:#}").contains("ambiguous example path: snippets/shared.rs:example"));
}

/// Verifies disabling example processing preserves source and does not call the resolver.
#[test]
fn disabled_example_processing_preserves_the_directive_without_resolution() {
    let source = "{{#example snippets/read_file.rs:example}}\n";
    let mut resolver_called = false;
    let config = Config {
        process_example_directives: false,
        ..Config::default()
    };

    let rendered = process_markdown_with_example_resolver(source, &config, |_| {
        resolver_called = true;
        Ok("unexpected replacement".to_owned())
    })
    .expect("disabled example directives remain unexpanded");

    assert_eq!(rendered, source);
    assert!(!resolver_called);
}
