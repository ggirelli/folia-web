//! Writes the tree as HTML, the way kramdown's HTML converter does: `<br />` and `<hr />`,
//! code spans and blocks marked for Rouge, task-list classes, and text escaped only where HTML
//! needs it (`&`, `<`, `>`, and `"` in attributes). Links are written as given, not
//! percent-encoded.

use std::fmt::Write;

use pulldown_cmark::Alignment;

use super::tree::{Attrs, Kind, Node};
use super::{highlight, html};

/// The HTML of the document `root`, parsed from `src`.
pub fn html(root: &Node, src: &str) -> String {
    let mut out = String::new();
    blocks(&mut out, &root.children, 0, src, Some(0..src.len()));
    out
}

/// Writes `nodes`, blocks of one container, at `indent`. kramdown writes a line break for each
/// run of blank lines in the source between them; in `outer`, the container's own span, also
/// before the first and after the last (list items have none there).
fn blocks(
    out: &mut String,
    nodes: &[Node],
    indent: usize,
    src: &str,
    outer: Option<std::ops::Range<usize>>,
) {
    let mut after = outer.as_ref().map(|outer| outer.start);
    for (i, node) in nodes.iter().enumerate() {
        if let Some(from) = after {
            push_blank_line(out, src, from, node.span.start, i == 0);
        }
        if let Kind::Transparent = node.kind {
            inlines(out, node, src);
            // kramdown ends the text with a line break when a block follows without a blank line.
            let end = content_end(src, node);
            if nodes
                .get(i + 1)
                .is_some_and(|next| !src[end..next.span.start].contains("\n\n"))
            {
                out.push('\n');
            }
        } else {
            block(out, node, indent, src);
        }
        after = Some(content_end(src, node));
    }
    if let Some(outer) = outer {
        let from = after.unwrap_or(outer.start);
        push_blank_line(out, src, from, outer.end, nodes.is_empty());
    }
}

/// kramdown writes one line break for the blank lines between two blocks, however many there
/// are: the only other lines a gap can hold, IALs and link definitions, leave no element between
/// them. With `line_start`, `from` starts a line; otherwise it ends some content.
fn push_blank_line(out: &mut String, src: &str, from: usize, to: usize, line_start: bool) {
    if to <= from {
        return;
    }
    let mut lines: Vec<&str> = src[from..to].split('\n').collect();
    // The last piece starts the next block's line; the first ends the previous content's.
    lines.pop();
    if !line_start && !lines.is_empty() {
        lines.remove(0);
    }
    if lines
        .iter()
        .any(|line| line.trim_start_matches([' ', '\t', '>']).trim().is_empty())
    {
        out.push('\n');
    }
}

/// Where a block's content ends: before the blank lines, and for a list the link definitions,
/// that its span may include.
fn content_end(src: &str, node: &Node) -> usize {
    match (&node.kind, node.children.last()) {
        (Kind::List(_) | Kind::Item | Kind::BlockQuote, Some(last)) => content_end(src, last),
        _ => node.span.start + src[node.span.clone()].trim_end().len(),
    }
}

fn pad(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

fn block(out: &mut String, node: &Node, indent: usize, src: &str) {
    match &node.kind {
        Kind::Paragraph => simple_block(out, "p", node, indent, src),
        Kind::Heading(level) => simple_block(out, &format!("h{level}"), node, indent, src),
        Kind::BlockQuote => {
            pad(out, indent);
            open(out, "blockquote", &node.attrs);
            out.push('\n');
            blocks(
                out,
                &node.children,
                indent + 2,
                src,
                Some(node.span.clone()),
            );
            pad(out, indent);
            out.push_str("</blockquote>\n");
        }
        Kind::CodeBlock { lang, code } => {
            pad(out, indent);
            highlight::block(out, lang.as_deref(), code, &node.attrs, indent);
        }
        Kind::HtmlBlock(html) => out.push_str(html),
        Kind::List(start) => list(out, *start, node, indent, src),
        Kind::Table(alignments) => table(out, alignments, node, indent, src),
        Kind::Rule => {
            pad(out, indent);
            out.push_str("<hr />\n");
        }
        // Text outside a paragraph: kramdown has none, but write it rather than lose it.
        _ => inline(out, node, src),
    }
}

fn simple_block(out: &mut String, tag: &str, node: &Node, indent: usize, src: &str) {
    pad(out, indent);
    open(out, tag, &node.attrs);
    inlines(out, node, src);
    let _ = writeln!(out, "</{tag}>");
}

/// kramdown marks a list holding a task item, and its items from the first task on.
fn list(out: &mut String, start: Option<u64>, node: &Node, indent: usize, src: &str) {
    let tag = if start.is_some() { "ol" } else { "ul" };
    let mut attrs = Attrs::default();
    let first_task = node.children.iter().position(is_task);
    if first_task.is_some() {
        attrs.set("class", "task-list");
    }
    attrs.extend_from(&node.attrs);
    pad(out, indent);
    open(out, tag, &attrs);
    out.push('\n');
    for (i, item) in node.children.iter().enumerate() {
        let mut attrs = Attrs::default();
        if first_task.is_some_and(|first| i >= first) {
            attrs.set("class", "task-list-item");
        }
        attrs.extend_from(&item.attrs);
        list_item(out, item, &attrs, indent + 2, src);
    }
    pad(out, indent);
    let _ = writeln!(out, "</{tag}>");
}

/// kramdown's `<li>`: a first paragraph without `<p>` stays on the `<li>` line.
fn list_item(out: &mut String, item: &Node, attrs: &Attrs, indent: usize, src: &str) {
    pad(out, indent);
    open(out, "li", attrs);
    let mut content = String::new();
    blocks(&mut content, &item.children, indent + 2, src, None);
    let transparent = item
        .children
        .first()
        .is_none_or(|first| matches!(first.kind, Kind::Transparent));
    if transparent {
        out.push_str(&content);
        if content.ends_with('\n') {
            pad(out, indent);
        }
    } else {
        out.push('\n');
        out.push_str(&content);
        pad(out, indent);
    }
    out.push_str("</li>\n");
}

fn is_task(item: &Node) -> bool {
    let first = item.children.first();
    let first = match first.map(|node| &node.kind) {
        Some(Kind::Paragraph | Kind::Transparent) => first.and_then(|p| p.children.first()),
        _ => first,
    };
    first.is_some_and(|node| matches!(node.kind, Kind::TaskMarker(_)))
}

fn table(out: &mut String, alignments: &[Alignment], node: &Node, indent: usize, src: &str) {
    pad(out, indent);
    open(out, "table", &node.attrs);
    out.push('\n');
    let (head, body): (Vec<&Node>, Vec<&Node>) = node
        .children
        .iter()
        .partition(|part| matches!(part.kind, Kind::TableHead));
    for (section, cell, rows) in [("thead", "th", head), ("tbody", "td", body)] {
        if rows.is_empty() {
            continue;
        }
        pad(out, indent + 2);
        let _ = writeln!(out, "<{section}>");
        for row in rows {
            pad(out, indent + 4);
            out.push_str("<tr>\n");
            for (i, content) in row.children.iter().enumerate() {
                let mut attrs = Attrs::default();
                match alignments.get(i) {
                    Some(Alignment::Left) => attrs.set("style", "text-align: left"),
                    Some(Alignment::Center) => attrs.set("style", "text-align: center"),
                    Some(Alignment::Right) => attrs.set("style", "text-align: right"),
                    _ => {}
                }
                pad(out, indent + 6);
                open(out, cell, &attrs);
                inlines(out, content, src);
                let _ = writeln!(out, "</{cell}>");
            }
            pad(out, indent + 4);
            out.push_str("</tr>\n");
        }
        pad(out, indent + 2);
        let _ = writeln!(out, "</{section}>");
    }
    pad(out, indent);
    out.push_str("</table>\n");
}

fn inlines(out: &mut String, node: &Node, src: &str) {
    for child in &node.children {
        inline(out, child, src);
    }
}

fn inline(out: &mut String, node: &Node, src: &str) {
    match &node.kind {
        Kind::TaskMarker(checked) => {
            out.push_str(
                r#"<input type="checkbox" class="task-list-item-checkbox" disabled="disabled""#,
            );
            if *checked {
                out.push_str(r#" checked="checked""#);
            }
            out.push_str(" />");
        }
        Kind::Emphasis => element(out, "em", node, src),
        Kind::Strong => element(out, "strong", node, src),
        Kind::Strikethrough => element(out, "del", node, src),
        Kind::Link { dest, title, .. } => {
            let mut attrs = Attrs(vec![("href".to_owned(), dest.clone())]);
            if !title.is_empty() {
                attrs.set("title", title);
            }
            attrs.extend_from(&node.attrs);
            open(out, "a", &attrs);
            inlines(out, node, src);
            out.push_str("</a>");
        }
        Kind::Image { dest, title } => {
            let mut attrs = Attrs(vec![
                ("src".to_owned(), dest.clone()),
                ("alt".to_owned(), image_description(src, node)),
            ]);
            if !title.is_empty() {
                attrs.set("title", title);
            }
            attrs.extend_from(&node.attrs);
            out.push_str("<img");
            attributes(out, &attrs);
            out.push_str(" />");
        }
        Kind::Text { text, .. } => escape(out, text),
        Kind::Code(code) => {
            let mut attrs = Attrs(vec![(
                "class".to_owned(),
                "language-plaintext highlighter-rouge".to_owned(),
            )]);
            attrs.extend_from(&node.attrs);
            open(out, "code", &attrs);
            escape(out, code);
            out.push_str("</code>");
        }
        Kind::InlineHtml(html) => out.push_str(&html::with_attributes(html, &node.attrs)),
        Kind::SoftBreak => out.push('\n'),
        Kind::HardBreak => out.push_str("<br />\n"),
        // Blocks inside inline content: pulldown-cmark never nests them so.
        _ => block(out, node, 0, src),
    }
}

fn element(out: &mut String, tag: &str, node: &Node, src: &str) {
    open(out, tag, &node.attrs);
    inlines(out, node, src);
    let _ = write!(out, "</{tag}>");
}

/// kramdown takes an image's description as written, with escapes removed.
fn image_description(src: &str, node: &Node) -> String {
    let raw = &src[node.span.clone()];
    let inner = raw.strip_prefix("![").unwrap_or(raw);
    let mut depth = 0;
    let mut end = inner.len();
    let mut escaped = false;
    for (i, c) in inner.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' if depth == 0 => {
                end = i;
                break;
            }
            ']' => depth -= 1,
            _ => {}
        }
    }
    let mut description = String::new();
    let mut chars = inner[..end].chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
            continue;
        }
        description.push(c);
    }
    description
}

fn open(out: &mut String, tag: &str, attrs: &Attrs) {
    out.push('<');
    out.push_str(tag);
    attributes(out, attrs);
    out.push('>');
}

fn attributes(out: &mut String, attrs: &Attrs) {
    for (key, value) in &attrs.0 {
        let _ = write!(out, " {key}=\"");
        escape_attribute(out, value);
        out.push('"');
    }
}

/// Escapes text for HTML.
pub fn escape(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

fn escape_attribute(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '"' => out.push_str("&quot;"),
            c => escape(out, c.encode_utf8(&mut [0; 4])),
        }
    }
}

impl Attrs {
    /// Adds a copy of every attribute of `other`.
    fn extend_from(&mut self, other: &Attrs) {
        for (key, value) in &other.0 {
            self.set(key, value);
        }
    }
}
