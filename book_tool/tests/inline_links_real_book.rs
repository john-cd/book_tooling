use std::fs;
use std::process::Command;

use tempfile::tempdir;

/// Verifies inline-link conversion on a copied include-heavy Rust How-To chapter excerpt.
#[test]
fn inline_links_cli_preserves_titles_images_native_helpers_and_issue_links() {
    let temp = tempdir().expect("create temporary book directory");
    let chapter = temp.path().join("constants_and_statics.md");
    let references = temp.path().join("refs.incl.md");
    let source = concat!(
        "<!-- Copied syntax excerpt from bk/src/language/constants_and_statics.md -->\n",
        "# Constants and Statics\n\n",
        "{{#include constants_and_statics.incl.md}}\n\n",
        "[Book source][1]\n",
        "[Existing collision][3]\n",
        "[Book source again](https://doc.rust-lang.org/book/)\n",
        "[Rust standard library](https://doc.rust-lang.org/std/ \"Rust std\")\n",
        "[Rust by Example](https://doc.rust-lang.org/rust-by-example/)\n",
        "![Rust logo](https://www.rust-lang.org/logos/rust-logo-256x256.png \"Rust logo\")\n",
        "[Issue](https://github.com/john-cd/rust_howto/issues/1432)\n",
        "Inline code: `https://example.com/code`.\n",
        "```rust\nlet url = \"https://example.com/untouched\";\n```\n",
        "{{#include refs.incl.md}}\n\n",
        "[1]: https://doc.rust-lang.org/book/\n",
        "[3]: https://example.com/already-used\n"
    );
    fs::write(&chapter, source).expect("write copied chapter excerpt");
    fs::write(
        &references,
        "[book]: https://doc.rust-lang.org/book/\n[shared]: https://example.com/shared\n",
    )
    .expect("write shared reference include");

    let first = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args([
            "inline-links",
            temp.path().to_str().expect("UTF-8 temp path"),
        ])
        .output()
        .expect("run inline-links CLI");
    assert!(
        first.status.success(),
        "inline-links stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let expected = concat!(
        "<!-- Copied syntax excerpt from bk/src/language/constants_and_statics.md -->\n",
        "# Constants and Statics\n\n",
        "{{#include constants_and_statics.incl.md}}\n\n",
        "[Book source][1]\n",
        "[Existing collision][3]\n",
        "[Book source again][1]\n",
        "[Rust standard library](https://doc.rust-lang.org/std/ \"Rust std\")\n",
        "[Rust by Example][2]\n",
        "![Rust logo](https://www.rust-lang.org/logos/rust-logo-256x256.png \"Rust logo\")\n",
        "[Issue](https://github.com/john-cd/rust_howto/issues/1432)\n",
        "Inline code: `https://example.com/code`.\n",
        "```rust\nlet url = \"https://example.com/untouched\";\n```\n",
        "{{#include refs.incl.md}}\n\n",
        "[1]: https://doc.rust-lang.org/book/\n",
        "[3]: https://example.com/already-used\n",
        "\n[2]: https://doc.rust-lang.org/rust-by-example/\n"
    );
    assert_eq!(
        fs::read_to_string(&chapter).expect("read converted chapter"),
        expected
    );
    assert_eq!(
        fs::read_to_string(&references).expect("read shared references"),
        "[book]: https://doc.rust-lang.org/book/\n[shared]: https://example.com/shared\n"
    );

    let second = Command::new(env!("CARGO_BIN_EXE_book_tool"))
        .args([
            "inline-links",
            temp.path().to_str().expect("UTF-8 temp path"),
        ])
        .output()
        .expect("run inline-links CLI a second time");
    assert!(
        second.status.success(),
        "second inline-links stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(
        fs::read_to_string(chapter).expect("read second-run chapter"),
        expected
    );
}
