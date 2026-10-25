//! kramdown inline attribute lists (IALs), the subset the site uses.
//!
//! - **Span:** `{: .class key="value"}` right after a link, image, emphasis or code span sets
//!   that element's attributes. After text it is left as text, as kramdown does.
//! - **Block:** `{: .class}` on a line of its own. With no blank line before it, it applies to
//!   the block before; otherwise to the block after. It also ends a paragraph.
//!
//! pulldown-cmark reads both as text, so they are found in the parsed tree: a block IAL is a
//! paragraph line made only of an attribute list.

use std::sync::LazyLock;

use regex::Regex;

use super::tree::{Attrs, Kind, Node};

/// One attribute of a list: `key="value"` (or single quotes), or a run of `.class` and `#id`.
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:\A|\s)(?:(\w[\w-]*)=(?:"((?:\\\}|\\"|[^}"])*?)"|'((?:\\\}|\\'|[^}'])*?)')|((?:#[A-Za-z][\w:-]*|\.[^\s.#]+)+))(?:\s|\z)"#,
    )
    .expect("the expression is valid")
});
static ID_OR_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#([A-Za-z][\w:-]*)|\.([^\s.#]+)").expect("the expression is valid")
});
/// A whole attribute list, as a block IAL line.
static BLOCK_IAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A\{:([^:/](?:\\\}|[^}])*)\}[ \t]*\z").expect("the expression is valid")
});
/// An attribute list at the start of text, as a span IAL.
static SPAN_IAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A\{:((?:\\\}|[^}])+)\}").expect("the expression is valid"));

/// Reads the attributes of an attribute list's content (between `{:` and `}`).
fn parse(list: &str) -> Attrs {
    let mut attrs = Attrs::default();
    // Attributes are separated by one space, which each match may consume on both sides.
    let mut at = 0;
    while let Some(caps) = ATTRIBUTE.captures_at(list, at) {
        let whole = caps.get(0).expect("a match has a whole");
        if let Some(key) = caps.get(1) {
            let value = caps.get(2).or(caps.get(3)).map_or("", |v| v.as_str());
            let unescaped = value.replace("\\}", "}").replace("\\\"", "\"");
            attrs.set(key.as_str(), &unescaped.replace("\\'", "'"));
        } else if let Some(names) = caps.get(4) {
            for name in ID_OR_CLASS.captures_iter(names.as_str()) {
                match (name.get(1), name.get(2)) {
                    (Some(id), _) => attrs.set("id", id.as_str()),
                    (_, Some(class)) => attrs.set("class", class.as_str()),
                    _ => {}
                }
            }
        }
        // The space that ended this match may start the next.
        at = if whole.as_str().ends_with(char::is_whitespace) {
            whole.end() - 1
        } else {
            whole.end()
        };
    }
    attrs
}

/// Applies every IAL in the tree, and removes them from its text.
pub fn apply(node: &mut Node, src: &str) {
    apply_span_ials(&mut node.children);
    apply_block_ials(node, src);
    for child in &mut node.children {
        apply(child, src);
    }
}

/// A span IAL is text starting with an attribute list, right after an element. The list may
/// go on over a line break.
fn apply_span_ials(children: &mut Vec<Node>) {
    let mut i = 1;
    while i < children.len() {
        if matches!(children[i - 1].kind, Kind::Text { .. } | Kind::SoftBreak) {
            i += 1;
            continue;
        }
        let Some((list, end)) = span_ial_at(children, i) else {
            i += 1;
            continue;
        };
        let attrs = parse(&list);
        let target = match &children[i - 1].kind {
            Kind::InlineHtml(_) => html_element_start(children, i - 1),
            _ => Some(i - 1),
        };
        if let Some(target) = target {
            children[target].attrs.extend(attrs);
        }
        remove_text(children, i, end);
        i += 1;
    }
}

/// The attribute list starting the text at child `i`, and how many bytes of text it takes.
fn span_ial_at(children: &[Node], i: usize) -> Option<(String, usize)> {
    if !matches!(&children[i].kind, Kind::Text { text, plain: true } if text.starts_with("{:")) {
        return None;
    }
    let mut text = String::new();
    for child in &children[i..] {
        match &child.kind {
            Kind::Text { text: more, .. } => text.push_str(more),
            Kind::SoftBreak => text.push('\n'),
            _ => break,
        }
        if let Some(caps) = SPAN_IAL.captures(&text) {
            return Some((caps[1].to_owned(), caps[0].len()));
        }
    }
    None
}

/// Removes `len` bytes of text from the start of child `i` on, counting a line break as one.
fn remove_text(children: &mut Vec<Node>, i: usize, mut len: usize) {
    while len > 0 && i < children.len() {
        let taken = match &mut children[i].kind {
            Kind::Text { text, .. } if text.len() > len => {
                text.drain(..len);
                children[i].span.start += len;
                return;
            }
            Kind::Text { text, .. } => text.len(),
            _ => 1,
        };
        children.remove(i);
        len -= taken;
    }
}

/// For the inline HTML at child `close`, if it closes an element, the child that opens it.
fn html_element_start(children: &[Node], close: usize) -> Option<usize> {
    let Kind::InlineHtml(html) = &children[close].kind else {
        return None;
    };
    let name = html.strip_prefix("</")?.trim_end_matches('>').trim();
    let opening = format!("<{name}");
    children[..close].iter().rposition(|child| {
        matches!(&child.kind, Kind::InlineHtml(html)
            if html.starts_with(&opening)
                && html[opening.len()..].starts_with([' ', '>', '/', '\t', '\n']))
    })
}

/// Where a block IAL goes.
enum Target {
    Previous,
    Next,
}

fn apply_block_ials(node: &mut Node, src: &str) {
    let mut i = 0;
    let mut pending: Option<Attrs> = None;
    while i < node.children.len() {
        let child = &mut node.children[i];
        if !matches!(child.kind, Kind::Paragraph) {
            if let Some(attrs) = pending.take() {
                child.attrs.extend(attrs);
            }
            i += 1;
            continue;
        }
        let (leading, trailing) = take_ial_lines(child);
        let empty = child.children.is_empty();
        let before = target_of_block_ial(src, child.span.start);
        if let Some(mut attrs) = leading {
            if let Some(waiting) = pending.take() {
                let mut all = waiting;
                all.extend(attrs);
                attrs = all;
            }
            match before {
                Target::Previous if i > 0 => node.children[i - 1].attrs.extend(attrs),
                _ if empty => pending = Some(attrs),
                _ => node.children[i].attrs.extend(attrs),
            }
        } else if let Some(attrs) = pending.take() {
            node.children[i].attrs.extend(attrs);
        }
        if let Some(attrs) = trailing {
            node.children[i].attrs.extend(attrs);
        }
        if empty {
            node.children.remove(i);
        } else {
            i += 1;
        }
    }
}

/// kramdown applies a block IAL to the block before it unless a blank line separates them.
fn target_of_block_ial(src: &str, start: usize) -> Target {
    let before = &src[..start];
    let previous_line = before
        .strip_suffix('\n')
        .map(|text| text.rsplit_once('\n').map_or(text, |(_, line)| line));
    match previous_line {
        Some(line) if !line.trim().is_empty() => Target::Previous,
        _ => Target::Next,
    }
}

/// Removes the IAL lines at the start and end of a paragraph, and returns their attributes.
fn take_ial_lines(paragraph: &mut Node) -> (Option<Attrs>, Option<Attrs>) {
    let mut leading: Option<Attrs> = None;
    while let Some(attrs) = ial_line(&paragraph.children, 0) {
        let end = line_end(&paragraph.children, 0);
        paragraph
            .children
            .drain(..end.min(paragraph.children.len()));
        leading.get_or_insert_with(Attrs::default).extend(attrs);
    }
    let mut trailing: Option<Attrs> = None;
    while let Some(start) = last_line_start(&paragraph.children)
        && start > 0
        && let Some(attrs) = ial_line(&paragraph.children, start)
    {
        // Drop the line and the break before it.
        paragraph.children.truncate(start - 1);
        let mut all = attrs;
        if let Some(later) = trailing.take() {
            all.extend(later);
        }
        trailing = Some(all);
    }
    (leading, trailing)
}

/// The attributes of the line starting at child `start`, if it is only an attribute list.
fn ial_line(children: &[Node], start: usize) -> Option<Attrs> {
    let mut line = String::new();
    for child in &children[start..] {
        match &child.kind {
            Kind::Text { text, .. } => line.push_str(text),
            Kind::SoftBreak => break,
            _ => return None,
        }
    }
    BLOCK_IAL.captures(&line).map(|caps| parse(&caps[1]))
}

/// The index after the line starting at child `start`, including its line break.
fn line_end(children: &[Node], start: usize) -> usize {
    children[start..]
        .iter()
        .position(|child| matches!(child.kind, Kind::SoftBreak))
        .map_or(children.len(), |i| start + i + 1)
}

fn last_line_start(children: &[Node]) -> Option<usize> {
    if children.is_empty() {
        return None;
    }
    Some(
        children
            .iter()
            .rposition(|child| matches!(child.kind, Kind::SoftBreak))
            .map_or(0, |i| i + 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(list: &str) -> Vec<(String, String)> {
        parse(list).0
    }

    #[test]
    fn an_attribute_list_reads_classes_ids_and_values() {
        assert_eq!(
            attrs(r#" target="blank" .u-in-reply-to"#),
            [
                ("target".to_owned(), "blank".to_owned()),
                ("class".to_owned(), "u-in-reply-to".to_owned()),
            ]
        );
        assert_eq!(
            attrs(" .lh-sm .fs-0p5em"),
            [("class".to_owned(), "lh-sm fs-0p5em".to_owned())]
        );
        assert_eq!(
            attrs(r#" title="Yes... I am reading Terry Pratchett! 😉""#),
            [(
                "title".to_owned(),
                "Yes... I am reading Terry Pratchett! 😉".to_owned()
            )]
        );
    }
}
