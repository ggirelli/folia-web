//! Raw HTML read the way kramdown reads it, where that differs from `CommonMark`:
//!
//! - **Content models.** Markdown inside an inline element is parsed only for the elements
//!   kramdown gives a span content model (`a`, `em`, `small`, `span`, …). Inside the others,
//!   such as `u`, the source stays as written: no emphasis, no smart quotes.
//! - **Self-closing tags.** `<u class="x" />` is an empty element, `<u class="x"></u>`, not an
//!   open tag (HTML ignores the `/` on elements that are not void).
//! - **Paragraphs.** A line starting with an inline element, such as `<a …>`, starts a
//!   paragraph, where `CommonMark` starts an HTML block. An inline element left open is closed at
//!   the end of its paragraph.

use std::sync::LazyLock;

use regex::Regex;

use super::tree::{Attrs, Kind, Node};

/// Elements whose content kramdown parses as Markdown (`HTML_CONTENT_MODEL_SPAN`).
const SPAN_CONTENT: &[&str] = &[
    "a", "abbr", "acronym", "b", "bdo", "big", "button", "cite", "caption", "del", "dfn", "dt",
    "em", "h1", "h2", "h3", "h4", "h5", "h6", "i", "ins", "label", "legend", "optgroup", "p", "q",
    "rb", "rbc", "rp", "rt", "rtc", "ruby", "select", "small", "span", "strong", "sub", "sup",
    "th", "tt",
];

/// Elements kramdown reads as inline (`HTML_SPAN_ELEMENTS`).
const SPAN_ELEMENTS: &[&str] = &[
    "a", "abbr", "acronym", "b", "big", "bdo", "br", "button", "cite", "code", "del", "dfn", "em",
    "i", "img", "input", "ins", "kbd", "label", "mark", "option", "q", "rb", "rbc", "rp", "rt",
    "rtc", "ruby", "samp", "select", "small", "span", "strong", "sub", "sup", "time", "tt", "u",
    "var",
];

/// Elements without content.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

static TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A<(/?)([A-Za-z][A-Za-z0-9-]*)((?:\s[^>]*?)?)\s*(/?)>\z")
        .expect("the expression is valid")
});
static BLOCK_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A[ ]{0,3}<([A-Za-z][A-Za-z0-9-]*)").expect("the expression is valid")
});

/// A tag of inline HTML.
struct Tag<'a> {
    closing: bool,
    name: String,
    attributes: &'a str,
    self_closing: bool,
}

fn tag(html: &str) -> Option<Tag<'_>> {
    let caps = TAG.captures(html)?;
    Some(Tag {
        closing: !caps[1].is_empty(),
        name: caps[2].to_ascii_lowercase(),
        attributes: caps.get(3).map_or("", |m| m.as_str()),
        self_closing: !caps[4].is_empty(),
    })
}

/// Applies kramdown's reading of raw HTML to the tree.
pub fn apply(node: &mut Node, src: &str) {
    for child in &mut node.children {
        if let Kind::HtmlBlock(html) = &child.kind
            && starts_inline(html)
        {
            let html = html.trim_end().to_owned();
            child.kind = Kind::Paragraph;
            child.children = vec![Node::leaf(Kind::InlineHtml(html), child.span.clone())];
        }
    }
    close_empty_elements(&mut node.children);
    keep_raw_content(&mut node.children, src);
    if matches!(
        node.kind,
        Kind::Paragraph | Kind::Heading(_) | Kind::TableCell
    ) {
        let open = unclosed(&node.children);
        let end = node
            .children
            .last()
            .map_or(node.span.end, |child| child.span.end);
        push_closing_tags(&mut node.children, open, end);
    }
    for child in &mut node.children {
        apply(child, src);
    }
}

fn starts_inline(html: &str) -> bool {
    BLOCK_START
        .captures(html)
        .is_some_and(|caps| SPAN_ELEMENTS.contains(&caps[1].to_ascii_lowercase().as_str()))
}

fn close_empty_elements(children: &mut [Node]) {
    for child in children {
        let Kind::InlineHtml(html) = &child.kind else {
            continue;
        };
        let Some(tag) = tag(html) else {
            continue;
        };
        if tag.self_closing && !VOID.contains(&tag.name.as_str()) {
            let attributes = tag.attributes.trim_end();
            child.kind = Kind::InlineHtml(format!("<{0}{attributes}></{0}>", tag.name));
        }
    }
}

/// Replaces what lies between the tags of an element without a span content model by its
/// source: text that typography leaves alone, and tags. Without a closing tag the element takes
/// the rest of the paragraph, as kramdown auto-closes it.
fn keep_raw_content(children: &mut Vec<Node>, src: &str) {
    let mut i = 0;
    while i < children.len() {
        let opening = match &children[i].kind {
            Kind::InlineHtml(html) => tag(html).filter(|tag| {
                !tag.closing
                    && !tag.self_closing
                    && !VOID.contains(&tag.name.as_str())
                    && !SPAN_CONTENT.contains(&tag.name.as_str())
            }),
            _ => None,
        };
        let Some(opening) = opening else {
            i += 1;
            continue;
        };
        let close = children[i + 1..]
            .iter()
            .position(|child| {
                matches!(&child.kind, Kind::InlineHtml(html)
                    if tag(html).is_some_and(|t| t.closing && t.name == opening.name))
            })
            .map(|offset| i + 1 + offset);
        let (end, span_end) = match close {
            Some(close) => (close, children[close].span.start),
            None => (children.len(), children.last().map_or(0, |c| c.span.end)),
        };
        let mut raw = raw_nodes(src, children[i].span.end..span_end);
        if close.is_none() {
            close_open_tags(&mut raw, &opening.name, span_end);
        }
        let count = raw.len();
        children.splice(i + 1..end, raw);
        i += count + 2;
    }
}

/// Appends closing tags for the elements left open in `nodes`, then for `outer`, as kramdown
/// closes them at the end of the paragraph.
fn close_open_tags(nodes: &mut Vec<Node>, outer: &str, at: usize) {
    let mut open = vec![outer.to_owned()];
    open.extend(unclosed(nodes));
    push_closing_tags(nodes, open, at);
}

/// The elements `nodes` open and do not close, outermost first.
fn unclosed(nodes: &[Node]) -> Vec<String> {
    let mut open: Vec<String> = Vec::new();
    for node in nodes {
        let Kind::InlineHtml(html) = &node.kind else {
            continue;
        };
        match tag(html) {
            Some(t) if t.closing => {
                if let Some(i) = open.iter().rposition(|name| *name == t.name) {
                    open.truncate(i);
                }
            }
            Some(t) if !t.self_closing && !VOID.contains(&t.name.as_str()) => open.push(t.name),
            _ => {}
        }
    }
    open
}

fn push_closing_tags(nodes: &mut Vec<Node>, open: Vec<String>, at: usize) {
    for name in open.into_iter().rev() {
        nodes.push(Node::leaf(Kind::InlineHtml(format!("</{name}>")), at..at));
    }
}

/// The source in `span` as raw text and tags.
fn raw_nodes(src: &str, span: std::ops::Range<usize>) -> Vec<Node> {
    static ANY_TAG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"</?[A-Za-z][A-Za-z0-9-]*(?:\s[^>]*)?/?>").expect("the expression is valid")
    });
    let mut nodes = Vec::new();
    let mut at = span.start;
    let text = |from: usize, to: usize| {
        Node::leaf(
            Kind::Text {
                text: src[from..to].to_owned(),
                plain: false,
            },
            from..to,
        )
    };
    for found in ANY_TAG.find_iter(&src[span.clone()]) {
        let (start, end) = (span.start + found.start(), span.start + found.end());
        if start > at {
            nodes.push(text(at, start));
        }
        nodes.push(Node::leaf(
            Kind::InlineHtml(found.as_str().to_owned()),
            start..end,
        ));
        at = end;
    }
    if span.end > at {
        nodes.push(text(at, span.end));
    }
    nodes
}

/// The opening tag `html` with `attrs` added, as an IAL after the element adds them: classes
/// join the element's own.
pub fn with_attributes(html: &str, attrs: &Attrs) -> String {
    let mut tag = html.to_owned();
    for (key, value) in &attrs.0 {
        let value = value.replace('"', "&quot;");
        let existing = format!(" {key}=\"");
        if let Some(at) = tag.find(&existing) {
            let start = at + existing.len();
            let end = start + tag[start..].find('"').unwrap_or(0);
            if key == "class" {
                tag.insert_str(end, &format!(" {value}"));
            } else {
                tag.replace_range(start..end, &value);
            }
        } else {
            let end = tag
                .trim_end_matches('>')
                .trim_end_matches('/')
                .trim_end()
                .len();
            tag.insert_str(end, &format!(" {key}=\"{value}\""));
        }
    }
    tag
}
