//! - Naked URLs: `http://...` or `https://...` optionally between < and >,
//! - Inline links: `[...](...)`,
//! - Reference-style links: `[...][...]`,
//! - Images,
//! - Reference definitions: `[...]: http://...`,
//! - Code spans and triple quoted text between ``` and ```.
//! - Hidden sections: <div class="hidden"> </div>
//! Data structures representing Markdown elements in the AST.
//!
//! TODO Reviewed ??-??-2026.

use super::DirectiveData;

/// Data for an autolink (e.g., `<https://example.com>` or `https://example.com`).
#[derive(Debug, PartialEq, Eq)]
pub struct AutolinkData<'a> {
    /// The URL of the autolink.
    pub url: &'a str,
}

/// Data for an inline link (e.g., `[text](url "title")`).
#[derive(Debug, PartialEq, Eq)]
pub struct InlineLinkData<'a> {
    /// The visible text of the link.
    pub text: &'a str,
    /// The destination URL.
    pub url: &'a str,
    /// An optional title for the link.
    pub title: Option<&'a str>,
}

/// Data for a reference-style link such as `[text][label]`.
#[derive(Debug, PartialEq, Eq)]
pub struct ReferenceStyleLinkData<'a> {
    /// Visible link text.
    pub text: &'a str,
    /// Reference definition label.
    pub label: &'a str,
}

/// Data for an inline image such as `![alt](url "title")`.
#[derive(Debug, PartialEq, Eq)]
pub struct InlineImageData<'a> {
    /// Image alt text.
    pub image_description: &'a str,
    /// Image destination URL.
    pub url: &'a str,
    /// Optional image title.
    pub title: Option<&'a str>,
}

/// Data for a reference-style image such as `![alt][label]`.
#[derive(Debug, PartialEq, Eq)]
pub struct ReferenceStyleImageData<'a> {
    /// Image alt text.
    pub image_description: &'a str,
    /// Reference definition label.
    pub label: &'a str,
}

/// Data for a reference definition such as `[label]: https://example.com`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceDefinitionData<'a> {
    /// Label defined by the reference definition.
    pub label: &'a str,
    /// URL assigned to the label.
    pub url: &'a str,
    /// Optional reference-definition title.
    pub title: Option<&'a str>,
}

/// Data for a wikilink (e.g., `[[target | display]]`).
#[derive(Debug, PartialEq, Eq)]
pub struct WikiLinkData<'a> {
    /// The target page or anchor.
    pub target: &'a str,
    /// An optional display text.
    pub display: Option<&'a str>,
    /// Alphanumeric characters that appear immediately after the closing brackets.
    pub immediately_after: Option<&'a str>,
}

/// Data for an inline code span.
#[derive(Debug, PartialEq, Eq)]
pub struct CodeSpanData<'a> {
    /// Literal code-span contents without the surrounding backticks.
    pub content: &'a str,
}

/// Data for a fenced code block.
#[derive(Debug, PartialEq, Eq)]
pub struct FencedCodeBlockData<'a> {
    /// Optional language fence info string.
    pub language: Option<&'a str>,
    /// Stored as a collection of line slices to support zero-copy nested multi-line blocks.
    pub lines: Vec<&'a str>,
}

/// Data for a paragraph element.
#[derive(Debug, PartialEq, Eq)]
pub struct ParagraphData<'a> {
    /// Child inline and nested elements contained in the paragraph.
    pub elements: Vec<Element<'a>>,
}

/// Raw contents of a hidden HTML `div` block.
#[derive(Debug, PartialEq, Eq)]
pub struct HiddenHtmlDivData<'a> {
    /// Inner contents between the opening and closing `<div>` tags.
    pub content: &'a str,
}

/// Raw lines that make up an HTML comment.
#[derive(Debug, PartialEq, Eq)]
pub struct HtmlCommentData<'a> {
    /// Comment contents split by lines, without the surrounding markers.
    pub lines: Vec<&'a str>,
}

/// Data for a blockquote container.
#[derive(Debug, PartialEq, Eq)]
pub struct BlockquoteData<'a> {
    /// Child elements contained by the blockquote.
    pub elements: Vec<Element<'a>>,
}

/// Data for an ordered or unordered list.
#[derive(Debug, PartialEq, Eq)]
pub struct ListData<'a> {
    /// Whether the list was parsed as ordered.
    pub ordered: bool,
    /// The list items; every element should be a `ListItem`.
    pub items: Vec<Element<'a>>,
}

/// Data for a list item.
#[derive(Debug, PartialEq, Eq)]
pub struct ListItemData<'a> {
    /// The item content; can contain paragraphs, nested lists, etc.
    pub elements: Vec<Element<'a>>,
}

/// Data for a Markdown heading.
#[derive(Debug, PartialEq, Eq)]
pub struct HeadingData<'a> {
    /// Heading depth from 1 to 6.
    pub level: u8,
    /// Optional text content parsed from the heading line.
    pub content: Option<&'a str>,
    /// Optional explicit anchor id, such as `{#section}`.
    pub id: Option<&'a str>,
}

/// Plain text that did not match a more specific inline element parser.
#[derive(Debug, PartialEq, Eq)]
pub struct TextData<'a> {
    /// The text.
    pub content: &'a str,
}

/// Data for a GitHub-Flavored Markdown task list item.
#[derive(Debug, PartialEq, Eq)]
pub struct TaskItemData<'a> {
    /// Whether the task item is marked complete.
    pub completed: bool,
    /// Task item text content.
    pub content: &'a str,
}

/// Data for a GitHub-Flavored Markdown table.
#[derive(Debug, PartialEq, Eq)]
pub struct TableData<'a> {
    /// Header row values.
    pub header: Vec<&'a str>,
    /// Data rows in display order.
    pub rows: Vec<Vec<&'a str>>,
    /// Parsed inline elements for each header cell, used by visitors that transform inline syntax.
    pub header_elements: Vec<Vec<Element<'a>>>,
    /// Parsed inline elements for each data cell, used by visitors that transform inline syntax.
    pub row_elements: Vec<Vec<Vec<Element<'a>>>>,
}

impl<'a> TableData<'a> {
    /// Creates a table from raw cells without parsed inline elements.
    #[must_use]
    pub fn new(header: Vec<&'a str>, rows: Vec<Vec<&'a str>>) -> Self {
        let header_elements = header.iter().map(|_| Vec::new()).collect();
        let row_elements = rows
            .iter()
            .map(|row| row.iter().map(|_| Vec::new()).collect())
            .collect();

        Self {
            header,
            rows,
            header_elements,
            row_elements,
        }
    }
}

/// Data for a strong-emphasis inline container.
#[derive(Debug, PartialEq, Eq)]
pub struct StrongData<'a> {
    /// The inline elements inside the strong emphasis.
    pub elements: Vec<Element<'a>>,
}

/// Data for an emphasis inline container.
#[derive(Debug, PartialEq, Eq)]
pub struct EmphasisData<'a> {
    /// The inline elements inside the emphasis.
    pub elements: Vec<Element<'a>>,
}

/// Data for a strikethrough inline container.
#[derive(Debug, PartialEq, Eq)]
pub struct StrikethroughData<'a> {
    /// The inline elements inside the strikethrough.
    pub elements: Vec<Element<'a>>,
}

/// A structural element in a parsed Markdown document.
#[derive(Debug, PartialEq, Eq)]
pub enum Element<'a> {
    /// A standalone URL: `http://...` or `https://...`
    /// optionally between < and > (i.e., an autolink).
    /// By design, more permissive than the `CommonMark` spec:
    /// See <https://spec.commonmark.org/0.31.2/#autolinks>.
    Autolink(AutolinkData<'a>),

    /// A Markdown-style inline link: `[text](url)`.
    /// See <https://spec.commonmark.org/0.31.2/#inline-link>.
    InlineLink(InlineLinkData<'a>),

    /// Reference-style link: `[...][...]`.
    /// See <https://spec.commonmark.org/0.31.2/#reference-link>.
    ReferenceStyleLink(ReferenceStyleLinkData<'a>),

    /// Images: `![foo](/url "title")`.
    /// See <https://spec.commonmark.org/0.31.2/#images>.
    InlineImage(InlineImageData<'a>),

    /// Reference-style image: `![foo][bar]`.
    ReferenceStyleImage(ReferenceStyleImageData<'a>),

    /// Reference definition: `[...]: http://... "title"`.
    /// See <https://spec.commonmark.org/0.31.2/#link-reference-definitions>.
    ReferenceDefinition(ReferenceDefinitionData<'a>),

    /// Wikilink: `[[target_page]]` or `[[target_page | display]]`, with or without spaces.
    /// See <https://en.wikipedia.org/wiki/Help:Link#Wikilinks_(internal_links)>.
    WikiLink(WikiLinkData<'a>),

    /// A code span enclosed between single backticks.
    /// See <https://spec.commonmark.org/0.31.2/#code-spans>.
    CodeSpan(CodeSpanData<'a>),

    /// A fenced code block enclosed in, for example, triple backticks.
    /// See <https://spec.commonmark.org/0.31.2/#fenced-code-blocks>.
    FencedCodeBlock(FencedCodeBlockData<'a>),

    /// An HTML div block, storing its raw content between `<div class="hidden"> </div>`.
    /// See <https://rust-lang.github.io/mdBook/format/mdbook.html#html-classes-provided-by-mdbook>.
    HiddenHtmlDiv(HiddenHtmlDivData<'a>),

    /// An HTML comment: `<!-- ... -->`.
    HtmlComment(HtmlCommentData<'a>),

    /// A Markdown ATX heading: `# Some Text {#an-anchor}`.
    /// <https://spec.commonmark.org/0.31.2/#atx-headings>
    /// See <https://rust-lang.github.io/mdBook/format/markdown.html#heading-attributes>.
    Heading(HeadingData<'a>),

    /// Plain text that doesn't match other elements.
    Text(TextData<'a>),

    /// A standard paragraph acting as a container for consecutive lines of text.
    Paragraph(ParagraphData<'a>),

    /// A blockquote containing recursively parsed elements.
    Blockquote(BlockquoteData<'a>),

    /// A standard Markdown list (ordered or unordered).
    List(ListData<'a>),

    /// An item inside a Markdown list.
    ListItem(ListItemData<'a>),

    /// Strong text (bold), usually enclosed in ** or __.
    Strong(StrongData<'a>),

    /// Emphasized text (italic), usually enclosed in * or _.
    Emphasis(EmphasisData<'a>),

    /// Strikethrough text, usually enclosed in ~~.
    Strikethrough(StrikethroughData<'a>),

    /// Custom directive, e.g., `{{#crate my_crate}}`.
    CustomDirective(DirectiveData<'a>),

    /// GFM Task List Item: `- [ ] content` or `- [x] content`.
    TaskItem(TaskItemData<'a>),

    /// GFM Table.
    Table(TableData<'a>),

    /// Horizontal Rule.
    HorizontalRule,
}
