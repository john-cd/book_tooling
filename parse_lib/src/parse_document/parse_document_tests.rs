//! Unit tests for this module.
//!
//! TODO Reviewed ??-??-2026.

use super::*;
use crate::ast::DirectiveData;

/// A heading, a paragraph with an inline link and a reference definition parse into several
/// top-level elements.
#[test]
fn test_parser() {
    let text_input = "# Title

Hello [link](url)
[label]: url
";
    let doc = parse_document("test.md", None, text_input).expect("input should parse");
    // Since parse_plain_text is currently consuming character by character,
    // we'll have a lot of Text elements.
    assert!(doc.elements.len() >= 5);
}

/// Counts the block-level tables of a parsed document.
fn count_tables(document: &crate::Document<'_>) -> usize {
    document
        .elements
        .iter()
        .filter(|element| matches!(element, Element::Table(_)))
        .count()
}
/// Concatenates the plain-text children of `elements`.
fn plain_text(elements: &[Element<'_>]) -> String {
    let mut combined = String::new();
    for element in elements {
        let Element::Text(text) = element else {
            continue;
        };
        combined.push_str(text.content);
    }
    combined
}

/// Collects the inner text of every top-level emphasis element.
fn emphasis_texts(markdown: &str) -> Vec<String> {
    let document = parse_document("test.md", None, markdown).expect("input should parse");
    let mut texts = Vec::new();
    for element in &document.elements {
        let Element::Emphasis(emphasis) = element else {
            continue;
        };
        texts.push(plain_text(&emphasis.elements));
    }
    texts
}

/// An intraword underscore in the preamble must not swallow the table that follows
/// (john-cd/book_tooling#47).
#[test]
fn intraword_underscore_before_a_table_does_not_hide_the_table() {
    let markdown = concat!(
        "# File System\n\n",
        "Use [`fs_extra`][c~fs_extra~docs]{{hi:fs_extra}} for more.\n\n",
        "| Topic | Crates |\n",
        "|---|---|\n",
        "| Operations | [`fs_extra`][c~fs_extra~docs]{{hi:fs_extra}} copies trees. |\n",
    );
    let document = parse_document("test.md", None, markdown).expect("input should parse");
    assert_eq!(count_tables(&document), 1);
}

/// Unbalanced delimiters in link destinations, reference definitions and heading attributes
/// stay literal text (john-cd/book_tooling#48).
#[test]
fn unbalanced_delimiters_degrade_to_text_instead_of_failing() {
    for markdown in [
        "[](( ",
        "[](<",
        "[x](foo(bar)",
        "[x]: u(",
        "[x]: <u",
        "# Heading {#id",
        "![alt](<img.png",
        "text [a](b(c) more text\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
    ] {
        let document = parse_document("test.md", None, markdown)
            .unwrap_or_else(|error| panic!("{markdown:?} should parse: {error}"));
        let mut renderer =
            crate::markdown_renderer::MarkdownRenderer::new(|directive| directive.to_string());
        crate::visit::Visitor::visit_document(&mut renderer, &document);
        assert_eq!(renderer.markdown, markdown, "round trip of {markdown:?}");
    }
}

/// Snake case words are plain text.
#[test]
fn snake_case_words_are_plain_text() {
    assert!(emphasis_texts("snake_case_word and fs_extra\n").is_empty());
}

/// Underscore emphasis still requires word boundaries.
#[test]
fn underscore_emphasis_still_requires_word_boundaries() {
    assert_eq!(emphasis_texts("an _emphasized_ word"), vec!["emphasized"]);
    assert_eq!(emphasis_texts("an *emphasized* word"), vec!["emphasized"]);
    assert!(emphasis_texts("2 * 3 * 4").is_empty());
    assert!(emphasis_texts("_ not emphasis _").is_empty());
}

/// Emphasis and strong do not span blank lines.
#[test]
fn emphasis_and_strong_do_not_span_blank_lines() {
    assert!(emphasis_texts("*first\n\nsecond*").is_empty());
    let document =
        parse_document("test.md", None, "**first\n\nsecond**").expect("input should parse");
    assert!(
        !document
            .elements
            .iter()
            .any(|element| matches!(element, Element::Strong(_)))
    );
}

/// A `- [x]` list line parses as a completed task item.
#[test]
fn test_parse_task_item() {
    let input = "- [x] task 1";
    let mut input_ref = input;
    let result = parse_task_item
        .parse_next(&mut input_ref)
        .expect("input should parse");
    if let Element::TaskItem(task) = result {
        assert!(task.completed);
        assert_eq!(task.content, "task 1");
    } else {
        panic!("Expected TaskItem");
    }
}

/// A `- [ ]` list line parses as an unchecked task item.
#[test]
fn test_parse_task_item_incomplete() {
    let input = "- [ ] todo item";
    let mut input_ref = input;
    let result = parse_task_item
        .parse_next(&mut input_ref)
        .expect("input should parse");
    if let Element::TaskItem(task) = result {
        assert!(!task.completed);
        assert_eq!(task.content, "todo item");
    } else {
        panic!("Expected TaskItem");
    }
}

/// A marker other than space or `x` inside the brackets is not a task item.
#[test]
fn test_parse_task_item_invalid() {
    let input = "- [y] invalid";
    let mut input_ref = input;
    parse_task_item
        .parse_next(&mut input_ref)
        .expect_err("input should be rejected");
}

/// A pipe table with a delimiter row yields its header cells and one data row.
#[test]
fn test_parse_table() {
    let input = "| h1 | h2 |\n|---|---|\n| r1c1 | r1c2 |";
    let mut input_ref = input;
    let result = parse_table
        .parse_next(&mut input_ref)
        .expect("input should parse");
    if let Element::Table(table) = result {
        assert_eq!(table.header, vec!["h1", "h2"]);
        assert_eq!(table.rows[0], vec!["r1c1", "r1c2"]);
    } else {
        panic!("Expected Table");
    }
}

/// Leading and trailing pipes are optional: a table written without them parses the same.
#[test]
fn test_parse_table_no_outer_pipes() {
    let input = "h1 | h2\n---|---\nr1c1 | r1c2";
    let mut input_ref = input;
    let result = parse_table
        .parse_next(&mut input_ref)
        .expect("input should parse");
    if let Element::Table(table) = result {
        assert_eq!(table.header, vec!["h1", "h2"]);
        assert_eq!(table.rows[0], vec!["r1c1", "r1c2"]);
    } else {
        panic!("Expected Table");
    }
}

/// Every row after the delimiter row becomes a data row, in order.
#[test]
fn test_parse_table_multiple_rows() {
    let input = "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |";
    let mut input_ref = input;
    let result = parse_table
        .parse_next(&mut input_ref)
        .expect("input should parse");
    if let Element::Table(table) = result {
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1], vec!["3", "4"]);
    } else {
        panic!("Expected Table");
    }
}

/// Directives such as `{{#crate ..}}` inside table cells are kept intact in the cell text.
#[test]
fn test_parse_table_inline_directives_in_cells() {
    let input = concat!(
        "| Recipe | Crates | Categories |\n",
        "|--------|--------|------------|\n",
        "| Sort values | {{#crate std}} | {{#categories algorithms=Algorithms}} |"
    );
    let mut input_ref = input;
    let result = parse_table
        .parse_next(&mut input_ref)
        .expect("input should parse");
    let Element::Table(table) = result else {
        panic!("Expected Table");
    };

    assert!(matches!(
        table.row_elements[0][1].as_slice(),
        [Element::CustomDirective(DirectiveData::CrateBlock {
            crate_name: "std",
            ..
        })]
    ));
    assert!(matches!(
        table.row_elements[0][2].as_slice(),
        [Element::CustomDirective(
            DirectiveData::CategoryBlock { .. }
        )]
    ));
}

/// Keeps a directive-bearing table recognizable after a filesystem chapter preamble.
#[test]
fn filesystem_preamble_preserves_directives_in_following_table() {
    let input = concat!(
        "# File System\n\n",
        "[![cat~filesystem][cat~filesystem~badge]][cat~filesystem]\n\n",
        "This chapter deals with files{{hi:Files}} and file systems.{{hi:File systems}}\n\n",
        "For common tasks, [`fs_extra`][c~fs_extra~docs]↗{{hi:fs_extra}} and ",
        "[`std::path`][c~std::path~docs]↗{{hi:std::path}} are sufficient.\n\n",
        "| Topic | Rust Crates or Modules |\n",
        "|---|---|\n",
        "| File I/O | {{#crate std}} provides file operations. |\n",
        "| Paths | [`Path`][c~std::path::Path~docs] and `PathBuf`. |\n",
    );

    let document = parse_document("filesystem/index.md", None, input)
        .expect("parse filesystem preamble and table");
    let table = document
        .elements
        .iter()
        .find_map(|element| {
            if let Element::Table(table) = element {
                Some(table)
            } else {
                None
            }
        })
        .expect("find table following filesystem preamble");

    assert!(table.row_elements.first().is_some_and(|row| {
        row.get(1).is_some_and(|cell| {
            cell.iter().any(|element| {
                matches!(
                    element,
                    Element::CustomDirective(DirectiveData::CrateBlock {
                        crate_name: "std",
                        ..
                    })
                )
            })
        })
    }));
}

/// An unordered list inside a blockquote becomes a list element nested in the blockquote.
#[test]
fn test_simple_list_in_blockquote() {
    let input = "> - one
> - two
";
    let doc = parse_document("test.md", None, input).expect("input should parse");

    assert_eq!(
        doc.elements.len(),
        1,
        "Document should have one root element"
    );
    let blockquote = doc.elements.first().expect("element should exist");

    // Check that the root element is a Blockquote
    if let Element::Blockquote(blockquote_data) = blockquote {
        assert_eq!(
            blockquote_data.elements.len(),
            1,
            "Blockquote should contain one element"
        );
        let list = blockquote_data
            .elements
            .first()
            .expect("element should exist");

        // Check that the blockquote contains a List
        if let Element::List(list_data) = list {
            assert!(!list_data.ordered, "List should be unordered");
            assert_eq!(list_data.items.len(), 2, "List should have two items");
        } else {
            panic!("Expected List inside Blockquote, found {list:?}");
        }
    } else {
        panic!("Expected Blockquote element, found {blockquote:?}");
    }
}

/// A blockquote holding a paragraph and a nested list parses into the matching element tree.
#[test]
fn test_nested_list_in_blockquote() {
    let input = "> A paragraph.
>
> - list item 1
>   - nested item 1.1
> - list item 2
";
    let doc = parse_document("test.md", None, input).expect("input should parse");

    assert_eq!(doc.elements.len(), 1);
    let blockquote = doc.elements.first().expect("element should exist");

    // Root element should be a Blockquote
    if let Element::Blockquote(blockquote_data) = blockquote {
        // Blockquote should contain a Paragraph and a List
        assert_eq!(blockquote_data.elements.len(), 2);

        // Check the List element
        let list = &blockquote_data.elements[1];
        if let Element::List(list_data) = list {
            assert!(!list_data.ordered);
            assert_eq!(list_data.items.len(), 2);

            // -- Check first list item and its nested list --
            let item1 = list_data.items.first().expect("element should exist");
            if let Element::ListItem(item1_data) = item1 {
                // Item 1 should contain a Paragraph ("list item 1") and a nested List
                assert_eq!(item1_data.elements.len(), 2);

                let Some(Element::List(nested_list_data)) = item1_data.elements.get(1) else {
                    panic!("Expected nested List in first list item");
                };
                assert_eq!(
                    nested_list_data.items.len(),
                    1,
                    "Nested list should have one item"
                );
            } else {
                panic!("Expected ListItem");
            }

            // -- Check second list item --
            let item2 = &list_data.items[1];
            if let Element::ListItem(item2_data) = item2 {
                // Item 2 should just contain a single Paragraph
                assert_eq!(item2_data.elements.len(), 1);
            } else {
                panic!("Expected ListItem");
            }
        } else {
            panic!("Expected List as the second element in the Blockquote");
        }
    } else {
        panic!("Expected Blockquote element");
    }
}

/// A fenced code block indented under a list item stays part of that item and does not end the
/// list.
#[test]
fn test_list_with_fenced_code_block() {
    let input = "- item 1\n  ```rust\n  let x = 1;\n  ```\n- item 2\n";
    let doc = parse_document("test.md", None, input).expect("input should parse");

    assert_eq!(doc.elements.len(), 1);
    let list = doc.elements.first().expect("element should exist");

    if let Element::List(list_data) = list {
        assert_eq!(list_data.items.len(), 2);
        if let Element::ListItem(item1) = &list_data.items[0] {
            assert_eq!(item1.elements.len(), 2); // Paragraph, then FencedCodeBlock
            assert!(matches!(item1.elements[0], Element::Paragraph(_)));
            assert!(matches!(item1.elements[1], Element::FencedCodeBlock(_)));
        } else {
            panic!("Expected ListItem");
        }
    } else {
        panic!("Expected List");
    }
}

/// Lines indented by four spaces form a code block that ends at the first blank line followed
/// by unindented text.
#[test]
fn test_parse_indented_code_block() {
    let input = "    let x = 1;\n    let y = 2;\n\nParagraph";
    let mut input_ref = input;
    let result = parse_indented_code_block
        .parse_next(&mut input_ref)
        .expect("input should parse");

    if let Element::FencedCodeBlock(data) = result {
        assert_eq!(data.language, None);
        assert_eq!(data.lines, vec!["let x = 1;", "let y = 2;"]);
    } else {
        panic!("Expected FencedCodeBlock");
    }
    assert_eq!(input_ref, "\nParagraph");
}

/// `**bold *italic***` parses as strong text containing emphasized text.
#[test]
fn test_parse_strong_and_emphasis() {
    let input = "**bold *italic***";
    let mut input_ref = input;
    let result = parse_inline_element
        .parse_next(&mut input_ref)
        .expect("input should parse");

    if let Element::Strong(strong) = result {
        assert!(strong.elements.len() >= 2);
        // Due to how fallback_char works, "bold " might be multiple Text elements
        let mut combined_text = String::new();
        let mut next_idx = 0;
        for (i, el) in strong.elements.iter().enumerate() {
            if let Element::Text(text) = el {
                combined_text.push_str(text.content);
            } else {
                next_idx = i;
                break;
            }
        }
        assert_eq!(combined_text, "bold ");
        assert!(matches!(strong.elements[next_idx], Element::Emphasis(_)));
    } else {
        panic!("Expected Strong element");
    }
}

/// `~~text~~` parses as strikethrough.
#[test]
fn test_parse_strikethrough() {
    let input = "~~deleted~~";
    let mut input_ref = input;
    let result = parse_inline_element
        .parse_next(&mut input_ref)
        .expect("input should parse");

    if let Element::Strikethrough(strikethrough) = result {
        assert_eq!(strikethrough.elements.len(), 1);
        assert!(matches!(strikethrough.elements[0], Element::Text(_)));
    } else {
        panic!("Expected Strikethrough element");
    }
}

/// A `---` line, even with trailing space, is a horizontal rule and is fully consumed.
#[test]
fn test_parse_horizontal_rule() {
    let mut input = "--- \n";
    assert_eq!(
        parse_horizontal_rule.parse_next(&mut input),
        Ok(Element::HorizontalRule)
    );
    assert_eq!(input, "");

    let mut rule_input = "  * * * \nnext line";
    assert_eq!(
        parse_horizontal_rule.parse_next(&mut rule_input),
        Ok(Element::HorizontalRule)
    );
    assert_eq!(rule_input, "next line");
}

/// A multi-line HTML comment keeps each line separately, without the comment delimiters.
#[test]
fn test_parse_html_comment() {
    let mut input = "<!-- hidden \n text -->\n";
    let result = parse_html_comment
        .parse_next(&mut input)
        .expect("input should parse");
    if let Element::HtmlComment(comment) = result {
        assert_eq!(comment.lines, vec![" hidden ", " text "]);
    } else {
        panic!("Expected HtmlComment");
    }
    assert_eq!(input, "\n");
}

/// An indented multi-line HTML comment inside a list item is recognised and sits between the
/// item paragraphs.
#[test]
fn test_parse_html_comment_from_lines() {
    let input_doc = "- item 1\n  <!-- multi\n  line\n  comment -->\n  Paragraph";
    let doc = parse_document("test.md", None, input_doc).expect("input should parse");

    let list = doc.elements.first().expect("element should exist");
    if let Element::List(list_data) = list {
        if let Element::ListItem(item) = &list_data.items[0] {
            assert_eq!(item.elements.len(), 3); // Para, Comment, Para
            assert!(matches!(item.elements[1], Element::HtmlComment(_)));
        } else {
            panic!("expected list item");
        }
    } else {
        panic!("expected list");
    }
}
