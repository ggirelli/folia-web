//! Markdown: the site's dialect, rendered as Jekyll renders it today with kramdown (GFM input)
//! and Rouge (plan v5, §5.3 and §10.1a).
//!
//! The source is parsed with pulldown-cmark into a small tree, which a few passes adjust before
//! it is written as HTML:
//!
//! 1. `wiki_link`: garden wiki-links become links or broken-link spans, before parsing, as the
//!    site's Ruby plugin did.
//! 2. `html`: raw HTML read as kramdown reads it.
//! 3. `ial`: kramdown inline attribute lists, the subset the site uses: `{: .class key="value"}`
//!    after a link, image or emphasis, and on its own line for a block.
//! 4. `lists`: which list items' first paragraphs get `<p>`.
//! 5. `typography`: kramdown's smart quotes, dashes and ellipses.
//! 6. `heading`: kramdown's GFM heading IDs.
//! 7. `render`: HTML as kramdown writes it, with code highlighted by `highlight`.
//!
//! [`excerpt`] finds the part of a document that listings and feeds show, and [`excerpt_html`]
//! renders it the way the body is rendered. [`code_ranges`] says where the source is code.

mod excerpt;
mod heading;
mod highlight;
mod html;
mod ial;
mod lists;
mod render;
mod tree;
mod typography;
mod wiki_link;

use std::ops::Range;

pub use excerpt::excerpt;
pub use wiki_link::LinkResolver;

use tree::{Kind, Node};

/// Renders `src` as HTML. Wiki-links are resolved with `links`; without one they stay as written.
#[must_use]
pub fn to_html(src: &str, links: Option<&dyn LinkResolver>) -> String {
    let mut src = match links {
        Some(links) => wiki_link::replace(src, links),
        None => src.to_owned(),
    };
    with_line_break(&mut src);
    let mut root = tree::parse(&src);
    html::apply(&mut root, &src);
    ial::apply(&mut root, &src);
    lists::mark_transparent(&mut root, &src);
    typography::apply(&mut root, &src);
    heading::assign_ids(&mut root);
    render::html(&root, &src)
}

/// Where `src` is code, as Folia renders it: code blocks, fenced or indented, with their fences,
/// and code spans, with their backticks. A code span inside an element kramdown keeps as written,
/// such as `u`, is text (`html`), so it is not code.
#[must_use]
pub fn code_ranges(src: &str) -> Vec<Range<usize>> {
    let mut parsed = src.to_owned();
    with_line_break(&mut parsed);
    let mut root = tree::parse(&parsed);
    html::apply(&mut root, &parsed);
    let mut ranges = Vec::new();
    gather_code(&root, &mut ranges);
    // Within `src`, without the line break added for parsing.
    for range in &mut ranges {
        range.end = range.end.min(src.len());
    }
    ranges
}

/// kramdown reads the source as ending with a line break.
fn with_line_break(src: &mut String) {
    if !src.ends_with('\n') {
        src.push('\n');
    }
}

fn gather_code(node: &Node, ranges: &mut Vec<Range<usize>>) {
    match node.kind {
        Kind::CodeBlock { .. } | Kind::Code(_) => ranges.push(node.span.clone()),
        _ => node
            .children
            .iter()
            .for_each(|child| gather_code(child, ranges)),
    }
}

/// Renders the excerpt of `body` as HTML, resolving wiki-links with `links` as the body does.
///
/// Jekyll took excerpts before its plugin resolved wiki-links, so its garden excerpts showed the
/// raw `[[…]]`, and kramdown read a line of `[[Title|text]]` as a table row. Folia resolves them
/// on purpose (deviation D-003, plan v5 §10).
#[must_use]
pub fn excerpt_html(body: &str, links: Option<&dyn LinkResolver>) -> String {
    to_html(&excerpt(body), links)
}

#[cfg(test)]
mod tests;
