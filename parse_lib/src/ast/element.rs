//! - Naked URLs: `http://...` or `https://...` optionally between < and >,
//! - Inline links: `[...](...)`,
//! - Reference-style links: `[...][...]`,
//! - Images,
//! - Reference definitions: `[...]: http://...`,
//! - Code spans and triple quoted text between ``` and ```.
//! - Hidden sections: <div class="hidden"> </div>
//!
//! Data structures representing Markdown elements in the AST.
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

#[derive(Debug, PartialEq, Eq)]
pub struct ReferenceStyleLinkData<'a> {
    pub text: &'a str,
    pub label: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InlineImageData<'a> {
    pub image_description: &'a str,
    pub url: &'a str,
    pub title: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReferenceStyleImageData<'a> {
    pub image_description: &'a str,
    pub label: &'a str,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceDefinitionData<'a> {
    pub label: &'a str,
    pub url: &'a str,
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

#[derive(Debug, PartialEq, Eq)]
pub struct CodeSpanData<'a> {
    pub content: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FencedCodeBlockData<'a> {
    pub language: Option<&'a str>,
    /// Stored as a collection of line slices to support zero-copy nested multi-line blocks.
    pub lines: Vec<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParagraphData<'a> {
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HiddenHtmlDivData<'a> {
    pub content: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HtmlCommentData<'a> {
    pub lines: Vec<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockquoteData<'a> {
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListData<'a> {
    pub ordered: bool,
    pub items: Vec<Element<'a>>, // Elements here should strictly be `ListItem`s
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListItemData<'a> {
    pub elements: Vec<Element<'a>>, // Can contain paragraphs, nested lists, etc.
}

#[derive(Debug, PartialEq, Eq)]
pub struct HeadingData<'a> {
    pub level: u8,
    pub content: Option<&'a str>,
    pub id: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TextData<'a> {
    pub content: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskItemData<'a> {
    pub completed: bool,
    pub content: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TableData<'a> {
    pub header: Vec<&'a str>,
    pub rows: Vec<Vec<&'a str>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StrongData<'a> {
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EmphasisData<'a> {
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StrikethroughData<'a> {
    pub elements: Vec<Element<'a>>,
}

/// Represents a single Markdown element in the AST.
#[derive(Debug, PartialEq, Eq)]
pub enum Element<'a> {
    /// A standalone URL: `http://...` or `https://...`
    /// optionally between < and > (i.e., an autolink).
    /// By design, more permissive than the CommonMark spec:
    /// <https://spec.commonmark.org/0.31.2/#autolinks>
    Autolink(AutolinkData<'a>),

    /// A Markdown-style inline link: `[text](url)`.
    /// <https://spec.commonmark.org/0.31.2/#inline-link>
    InlineLink(InlineLinkData<'a>),

    /// Reference-style link: `[...][...]`.
    /// <https://spec.commonmark.org/0.31.2/#reference-link>
    ReferenceStyleLink(ReferenceStyleLinkData<'a>),

    /// Images: `![foo](/url "title")`.
    /// <https://spec.commonmark.org/0.31.2/#images>
    InlineImage(InlineImageData<'a>),

    /// Reference-style image: `![foo][bar]`.
    ReferenceStyleImage(ReferenceStyleImageData<'a>),

    /// Reference definition: `[...]: http://... "title"`.
    /// <https://spec.commonmark.org/0.31.2/#link-reference-definitions>
    ReferenceDefinition(ReferenceDefinitionData<'a>),

    /// Wikilink: `[[target_page]]` or `[[target_page | display]]`, with or without spaces.
    /// <https://en.wikipedia.org/wiki/Help:Link#Wikilinks_(internal_links)>
    WikiLink(WikiLinkData<'a>),

    /// CodeSpan enclosed between ` and `.
    /// <https://spec.commonmark.org/0.31.2/#code-spans>
    CodeSpan(CodeSpanData<'a>),

    /// Fenced Code Block enclosed in e.g. triple backticks (```).
    /// <https://spec.commonmark.org/0.31.2/#fenced-code-blocks>
    FencedCodeBlock(FencedCodeBlockData<'a>),

    /// An HTML div block, storing its raw content between `<div class="hidden"> </div>`.
    /// <https://rust-lang.github.io/mdBook/format/mdbook.html#html-classes-provided-by-mdbook>
    HiddenHtmlDiv(HiddenHtmlDivData<'a>),

    /// An HTML comment: `<!-- ... -->`.
    HtmlComment(HtmlCommentData<'a>),

    /// A Markdown ATX heading: `# Some Text {#an-anchor}`.
    /// <https://spec.commonmark.org/0.31.2/#atx-headings>
    /// <https://rust-lang.github.io/mdBook/format/markdown.html#heading-attributes>
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
