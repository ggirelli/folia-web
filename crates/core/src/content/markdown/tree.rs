//! The parsed document: pulldown-cmark's events gathered into a small owned tree, so the passes
//! that follow can look at a node's neighbours and change its attributes.

use std::ops::Range;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

/// A node of the document, with the source it came from.
#[derive(Debug)]
pub struct Node {
    pub kind: Kind,
    /// Attributes added by IALs, in the order they were given.
    pub attrs: Attrs,
    pub children: Vec<Node>,
    /// Where it is in the source.
    pub span: Range<usize>,
}

/// What a node is.
#[derive(Debug)]
pub enum Kind {
    Root,
    Paragraph,
    Heading(u8),
    BlockQuote,
    /// A fenced or indented code block, with its language for a fenced one.
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    HtmlBlock(String),
    List(Option<u64>),
    Item,
    TaskMarker(bool),
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        dest: String,
        title: String,
        /// `<https://…>`: its text is the address, not Markdown.
        auto: bool,
    },
    Image {
        dest: String,
        title: String,
    },
    /// Text. `plain` text is the source as written, so typography may change it; text from an
    /// escape or an entity is not.
    Text {
        text: String,
        plain: bool,
    },
    Code(String),
    InlineHtml(String),
    SoftBreak,
    HardBreak,
    Rule,
    /// A list item's first paragraph when kramdown writes it without `<p>`.
    Transparent,
}

/// HTML attributes, in order; `class` values accumulate.
#[derive(Debug, Default)]
pub struct Attrs(pub Vec<(String, String)>);

impl Attrs {
    /// Sets `key`; a class is appended to the classes already there.
    pub fn set(&mut self, key: &str, value: &str) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) if key == "class" => {
                v.push(' ');
                v.push_str(value);
            }
            Some((_, v)) => value.clone_into(v),
            None => self.0.push((key.to_owned(), value.to_owned())),
        }
    }

    /// Adds every attribute of `other`.
    pub fn extend(&mut self, other: Attrs) {
        for (key, value) in other.0 {
            self.set(&key, &value);
        }
    }
}

impl Node {
    /// A node without children.
    pub fn leaf(kind: Kind, span: Range<usize>) -> Self {
        Self::new(kind, span)
    }

    fn new(kind: Kind, span: Range<usize>) -> Self {
        Self {
            kind,
            attrs: Attrs::default(),
            children: Vec::new(),
            span,
        }
    }
}

/// The extensions kramdown's GFM parser has, among those the site uses. Smart punctuation is
/// kramdown's own (`typography`), and attributes are kramdown IALs (`ial`).
fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

/// Parses `src` into a tree.
pub fn parse(src: &str) -> Node {
    let mut stack = vec![Node::new(Kind::Root, 0..src.len())];
    for (event, span) in Parser::new_ext(src, options()).into_offset_iter() {
        match event {
            Event::Start(tag) => stack.push(Node::new(start_kind(tag), span)),
            Event::End(end) => {
                let mut node = stack.pop().expect("events are balanced");
                let parent = &mut stack.last_mut().expect("the root stays").children;
                match end {
                    TagEnd::CodeBlock | TagEnd::HtmlBlock => gather_raw_text(&mut node),
                    TagEnd::Item => wrap_inline_content(&mut node),
                    // kramdown's GFM strikes through `~~text~~` only; `~text~` stays text.
                    TagEnd::Strikethrough if !src[node.span.start..].starts_with("~~") => {
                        let span = node.span.clone();
                        parent.push(Node::leaf(
                            text_at(src, span.start),
                            span.start..span.start + 1,
                        ));
                        parent.append(&mut node.children);
                        parent.push(Node::leaf(
                            text_at(src, span.end - 1),
                            span.end - 1..span.end,
                        ));
                        continue;
                    }
                    _ => {}
                }
                parent.push(node);
            }
            event => {
                let leaf = leaf(event, src, span);
                stack
                    .last_mut()
                    .expect("the root stays")
                    .children
                    .push(leaf);
            }
        }
    }
    stack.pop().expect("the root stays")
}

fn start_kind(tag: Tag) -> Kind {
    match tag {
        Tag::Heading { level, .. } => Kind::Heading(level as u8),
        Tag::BlockQuote(_) => Kind::BlockQuote,
        Tag::CodeBlock(CodeBlockKind::Fenced(info)) => Kind::CodeBlock {
            lang: info.split_whitespace().next().map(str::to_owned),
            code: String::new(),
        },
        Tag::CodeBlock(CodeBlockKind::Indented) => Kind::CodeBlock {
            lang: None,
            code: String::new(),
        },
        Tag::HtmlBlock => Kind::HtmlBlock(String::new()),
        Tag::List(start) => Kind::List(start),
        Tag::Item => Kind::Item,
        Tag::Table(alignments) => Kind::Table(alignments),
        Tag::TableHead => Kind::TableHead,
        Tag::TableRow => Kind::TableRow,
        Tag::TableCell => Kind::TableCell,
        Tag::Emphasis => Kind::Emphasis,
        Tag::Strong => Kind::Strong,
        Tag::Strikethrough => Kind::Strikethrough,
        Tag::Link {
            link_type,
            dest_url,
            title,
            ..
        } => Kind::Link {
            auto: matches!(link_type, LinkType::Autolink | LinkType::Email),
            dest: dest_url.into_string(),
            title: title.into_string(),
        },
        Tag::Image {
            dest_url, title, ..
        } => Kind::Image {
            dest: dest_url.into_string(),
            title: title.into_string(),
        },
        // The parser options leave only paragraphs; the others are not enabled.
        _ => Kind::Paragraph,
    }
}

fn leaf(event: Event, src: &str, span: Range<usize>) -> Node {
    let kind = match event {
        Event::Text(text) => Kind::Text {
            plain: src.get(span.clone()) == Some(&*text),
            text: text.into_string(),
        },
        Event::Code(code) => Kind::Code(code.into_string()),
        Event::Html(html) | Event::InlineHtml(html) => Kind::InlineHtml(html.into_string()),
        Event::SoftBreak => Kind::SoftBreak,
        Event::HardBreak => Kind::HardBreak,
        Event::Rule => Kind::Rule,
        Event::TaskListMarker(checked) => Kind::TaskMarker(checked),
        // Not enabled: math and footnotes.
        _ => Kind::Text {
            text: src[span.clone()].to_owned(),
            plain: false,
        },
    };
    Node::new(kind, span)
}

fn text_at(src: &str, at: usize) -> Kind {
    Kind::Text {
        text: src[at..=at].to_owned(),
        plain: false,
    }
}

/// A tight list item holds its text directly; it becomes a paragraph, as in a loose list, so
/// every item starts the same way.
fn wrap_inline_content(item: &mut Node) {
    let inline = item
        .children
        .iter()
        .take_while(|child| is_inline(&child.kind))
        .count();
    if inline == 0 {
        return;
    }
    let content: Vec<Node> = item.children.drain(..inline).collect();
    let span = content[0].span.start..content[inline - 1].span.end;
    let mut paragraph = Node::new(Kind::Paragraph, span);
    paragraph.children = content;
    item.children.insert(0, paragraph);
}

fn is_inline(kind: &Kind) -> bool {
    !matches!(
        kind,
        Kind::Paragraph
            | Kind::Heading(_)
            | Kind::BlockQuote
            | Kind::CodeBlock { .. }
            | Kind::HtmlBlock(_)
            | Kind::List(_)
            | Kind::Table(_)
            | Kind::Rule
    )
}

/// A code or HTML block keeps its content as one string.
fn gather_raw_text(node: &mut Node) {
    let raw: String = node
        .children
        .drain(..)
        .map(|child| match child.kind {
            Kind::Text { text, .. } | Kind::InlineHtml(text) => text,
            _ => String::new(),
        })
        .collect();
    match &mut node.kind {
        Kind::CodeBlock { code, .. } => *code = raw,
        Kind::HtmlBlock(html) => *html = raw,
        _ => {}
    }
}
