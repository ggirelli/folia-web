//! HTML fragments, and whole pages, parsed with html5ever and normalized as plan v5, §10 (spike 1)
//! prescribes:
//!
//! - whitespace is collapsed outside `pre`, and dropped where blocks meet;
//! - entities are decoded (the parser does it);
//! - attributes are sorted, and so are the tokens of `class`;
//! - a `pre` is compared by its text only; highlighting is measured separately.
//!
//! Nothing else is normalized.

use std::collections::BTreeMap;
use std::fmt::Write;

use html5ever::tendril::TendrilSink;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// A node of a normalized fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element {
        name: String,
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
    },
    Text(String),
    Comment(String),
}

/// Elements whose edges whitespace does not survive.
const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "div",
    "dl",
    "dd",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "link",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "script",
    "section",
    "style",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

fn is_block(node: &Node) -> bool {
    matches!(node, Node::Element { name, .. } if BLOCKS.contains(&name.as_str()))
}

/// Parses `html` as the content of a `body`, and reads the body with `read`. The DOM must stay
/// alive meanwhile: dropping it empties every node.
fn with_body<T>(html: &str, read: impl FnOnce(&Handle) -> T) -> T {
    let page = format!("<!DOCTYPE html><html><head></head><body>{html}");
    let dom = parse_document(RcDom::default(), ParseOpts::default()).one(page);
    let body = find_element(&dom.document, "body").expect("a page has a body");
    read(&body)
}

fn find_element(handle: &Handle, wanted: &str) -> Option<Handle> {
    if let NodeData::Element { name, .. } = &handle.data
        && &*name.local == wanted
    {
        return Some(handle.clone());
    }
    handle
        .children
        .borrow()
        .iter()
        .find_map(|child| find_element(child, wanted))
}

/// The normalized `html` element of a whole page, head and body.
pub fn normalize_document(html: &str) -> Vec<Node> {
    let dom = parse_document(RcDom::default(), ParseOpts::default()).one(html);
    let root = find_element(&dom.document, "html").expect("a parsed page has an html element");
    tidy(convert(&root).into_iter().collect(), true)
}

/// The normalized nodes of `html`.
pub fn normalize(html: &str) -> Vec<Node> {
    let nodes = with_body(html, |body| {
        body.children.borrow().iter().filter_map(convert).collect()
    });
    tidy(nodes, true)
}

fn convert(handle: &Handle) -> Option<Node> {
    match &handle.data {
        NodeData::Text { contents } => Some(Node::Text(contents.borrow().to_string())),
        NodeData::Comment { contents } => Some(Node::Comment(contents.to_string())),
        NodeData::Element { name, attrs, .. } => {
            let name = name.local.to_string();
            let attrs = attrs
                .borrow()
                .iter()
                .map(|attr| {
                    let key = attr.name.local.to_string();
                    let value = if key == "class" {
                        let mut classes: Vec<&str> = attr.value.split_whitespace().collect();
                        classes.sort_unstable();
                        classes.join(" ")
                    } else {
                        attr.value.to_string()
                    };
                    (key, value)
                })
                .collect();
            let children = if name == "pre" {
                vec![Node::Text(text_of(handle))]
            } else {
                handle
                    .children
                    .borrow()
                    .iter()
                    .filter_map(convert)
                    .collect()
            };
            Some(Node::Element {
                name,
                attrs,
                children,
            })
        }
        _ => None,
    }
}

/// All the text below `handle`, as written.
pub fn text_of(handle: &Handle) -> String {
    let mut out = String::new();
    gather_text(handle, &mut out);
    out
}

fn gather_text(handle: &Handle, out: &mut String) {
    if let NodeData::Text { contents } = &handle.data {
        out.push_str(&contents.borrow());
    }
    for child in handle.children.borrow().iter() {
        gather_text(child, out);
    }
}

/// Collapses whitespace in `nodes`, the children of a block if `in_block`, and drops it at the
/// edges of blocks.
fn tidy(nodes: Vec<Node>, in_block: bool) -> Vec<Node> {
    let mut merged: Vec<Node> = Vec::new();
    for node in nodes {
        let node = match node {
            Node::Text(text) => Node::Text(collapse(&text)),
            Node::Element {
                name,
                attrs,
                children,
            } if name != "pre" => {
                let block = BLOCKS.contains(&name.as_str());
                Node::Element {
                    children: tidy(children, block),
                    name,
                    attrs,
                }
            }
            other => other,
        };
        if let (Some(Node::Text(last)), Node::Text(text)) = (merged.last_mut(), &node) {
            let joined = collapse(&format!("{last}{text}"));
            *last = joined;
        } else {
            merged.push(node);
        }
    }
    let len = merged.len();
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let Node::Text(text) = &merged[i] else {
            out.push(merged[i].clone());
            continue;
        };
        let at_start = if i == 0 {
            in_block
        } else {
            is_block(&merged[i - 1])
        };
        let at_end = if i + 1 == len {
            in_block
        } else {
            is_block(&merged[i + 1])
        };
        let mut text = text.as_str();
        if at_start {
            text = text.trim_start_matches(' ');
        }
        if at_end {
            text = text.trim_end_matches(' ');
        }
        if !text.is_empty() {
            out.push(Node::Text(text.to_owned()));
        }
    }
    out
}

/// Runs of HTML whitespace become one space.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c') {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// Where two normalized fragments first differ, as a path and what differs; `None` if equal.
pub fn first_difference(jekyll: &[Node], folia: &[Node]) -> Option<String> {
    diff_children(jekyll, folia, "")
}

fn diff_children(jekyll: &[Node], folia: &[Node], path: &str) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..jekyll.len().max(folia.len()) {
        let (Some(left), Some(right)) = (jekyll.get(i), folia.get(i)) else {
            let (side, node) = if i < jekyll.len() {
                ("Jekyll", &jekyll[i])
            } else {
                ("Folia", &folia[i])
            };
            return Some(format!("{path}/: only {side} has {}", describe(node)));
        };
        let label = label(left);
        let count = counts.entry(label.clone()).or_default();
        *count += 1;
        let here = format!("{path}/{label}[{count}]");
        match (left, right) {
            (
                Node::Element {
                    name: left_name,
                    attrs: left_attrs,
                    children: left_children,
                },
                Node::Element {
                    name: right_name,
                    attrs: right_attrs,
                    children: right_children,
                },
            ) if left_name == right_name => {
                if left_attrs != right_attrs {
                    return Some(format!(
                        "{here}: attributes {left_attrs:?} / {right_attrs:?}"
                    ));
                }
                if let Some(found) = diff_children(left_children, right_children, &here) {
                    return Some(found);
                }
            }
            _ if left == right => {}
            _ => {
                return Some(format!("{here}: {} / {}", describe(left), describe(right)));
            }
        }
    }
    None
}

fn label(node: &Node) -> String {
    match node {
        Node::Element { name, .. } => name.clone(),
        Node::Text(_) => "text()".to_owned(),
        Node::Comment(_) => "comment()".to_owned(),
    }
}

fn describe(node: &Node) -> String {
    match node {
        Node::Element { name, attrs, .. } => format!("<{name} {attrs:?}>"),
        Node::Text(text) => format!("text {:?}", shorten(text)),
        Node::Comment(text) => format!("comment {:?}", shorten(text)),
    }
}

fn shorten(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(80).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

/// The IDs of the headings, in order.
pub fn heading_ids(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    walk(nodes, &mut |node| {
        if let Node::Element { name, attrs, .. } = node
            && matches!(name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        {
            out.push(attrs.get("id").cloned().unwrap_or_default());
        }
    });
    out
}

/// Every `href` and `src`, sorted, so two lists compare as multisets.
pub fn link_targets(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    walk(nodes, &mut |node| {
        if let Node::Element { name, attrs, .. } = node {
            for key in ["href", "src"] {
                if let Some(value) = attrs.get(key) {
                    out.push(format!("{name} {key}={value}"));
                }
            }
        }
    });
    out.sort();
    out
}

/// The elements carrying a class, target or title, in order, with what they link to: an IAL's
/// attributes must sit on the same element.
pub fn placed_attributes(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    walk(nodes, &mut |node| {
        if let Node::Element { name, attrs, .. } = node {
            let shown: Vec<String> = ["class", "target", "title"]
                .iter()
                .filter_map(|key| attrs.get(*key).map(|v| format!("{key}={v}")))
                .collect();
            if !shown.is_empty() {
                let target = attrs.get("href").or(attrs.get("src"));
                let mut line = format!("<{name}> {}", shown.join(" "));
                if let Some(target) = target {
                    let _ = write!(line, " -> {target}");
                }
                out.push(line);
            }
        }
    });
    out
}

/// The text of the fragment, with one space where blocks meet and whitespace collapsed.
pub fn text(nodes: &[Node]) -> String {
    let mut out = String::new();
    gather(nodes, &mut out);
    collapse(&out).trim().to_owned()
}

fn gather(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Element { children, .. } => {
                let block = is_block(node);
                if block {
                    out.push(' ');
                }
                gather(children, out);
                if block {
                    out.push(' ');
                }
            }
            Node::Comment(_) => {}
        }
    }
}

/// Calls `visit` on every node, depth first, in document order.
pub fn walk(nodes: &[Node], visit: &mut impl FnMut(&Node)) {
    for node in nodes {
        visit(node);
        if let Node::Element { children, .. } = node {
            walk(children, visit);
        }
    }
}

/// A code block: its language (from kramdown's `language-x` wrapper), its text, and the token
/// class of each character (empty outside a token).
pub struct CodeBlock {
    pub lang: Option<String>,
    pub text: String,
    pub classes: Vec<String>,
}

/// The code blocks (`pre` elements) of `html`, in order.
pub fn code_blocks(html: &str) -> Vec<CodeBlock> {
    let mut out = Vec::new();
    with_body(html, |body| find_pre(body, None, &mut out));
    out
}

fn find_pre(handle: &Handle, lang: Option<&str>, out: &mut Vec<CodeBlock>) {
    let mut lang = lang.map(str::to_owned);
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        if &*name.local == "pre" {
            let mut block = CodeBlock {
                lang,
                text: String::new(),
                classes: Vec::new(),
            };
            classify(handle, "", &mut block);
            out.push(block);
            return;
        }
        let classes = attrs
            .borrow()
            .iter()
            .find(|a| &*a.name.local == "class")
            .map(|a| a.value.to_string())
            .unwrap_or_default();
        if let Some(found) = classes
            .split_whitespace()
            .find_map(|class| class.strip_prefix("language-"))
            .filter(|_| classes.contains("highlighter-rouge"))
        {
            lang = Some(found.to_owned());
        }
    }
    for child in handle.children.borrow().iter() {
        find_pre(child, lang.as_deref(), out);
    }
}

fn classify(handle: &Handle, class: &str, block: &mut CodeBlock) {
    match &handle.data {
        NodeData::Text { contents } => {
            for c in contents.borrow().chars() {
                block.text.push(c);
                block.classes.push(class.to_owned());
            }
        }
        NodeData::Element { name, attrs, .. } => {
            let own = (&*name.local == "span")
                .then(|| {
                    attrs
                        .borrow()
                        .iter()
                        .find(|a| &*a.name.local == "class")
                        .map(|a| a.value.to_string())
                })
                .flatten();
            let class = own.as_deref().unwrap_or(class);
            for child in handle.children.borrow().iter() {
                classify(child, class, block);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_between_blocks_and_entities_do_not_count() {
        let kramdown = "<ul>\n  <li>It&rsquo;s <em>a</em> <code>x</code></li>\n</ul>\n\n<p>b</p>\n";
        let folia = "<ul>\n<li>It’s <em>a</em> <code>x</code></li>\n</ul>\n<p>b</p>\n";
        assert_eq!(
            first_difference(&normalize(kramdown), &normalize(folia)),
            None
        );
    }

    #[test]
    fn whitespace_inside_a_line_counts() {
        let a = normalize("<p><em>a</em> <em>b</em></p>");
        let b = normalize("<p><em>a</em><em>b</em></p>");
        assert!(first_difference(&a, &b).is_some());
    }

    #[test]
    fn a_fragment_keeps_its_nodes() {
        assert_eq!(heading_ids(&normalize("<link><h2 id=\"a\">A</h2>")), ["a"]);
        assert_eq!(code_blocks("<pre><code>x</code></pre>").len(), 1);
    }

    #[test]
    fn classes_and_attributes_compare_sorted() {
        let a = normalize(r#"<a href="/x" class="b a" target="blank">x</a>"#);
        let b = normalize(r#"<a target="blank" class="a b" href="/x">x</a>"#);
        assert_eq!(first_difference(&a, &b), None);
    }
}
