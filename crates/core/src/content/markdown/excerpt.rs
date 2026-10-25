//! Excerpts, as Jekyll takes them from a document's Markdown.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

/// The marker that ends an excerpt (the site's `excerpt_separator`).
const SEPARATOR: &str = "<!--more-->";

/// A link reference definition, as Jekyll finds them: `[label]` and the rest of the line.
static DEFINITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^ {0,3}(\[[^\]]+\])(:.+)$").expect("the expression is valid")
});

/// The Markdown of `body`'s excerpt: the text before `<!--more-->`, or all of it without one.
/// The reference definitions that follow the marker are kept when the excerpt uses their label,
/// so its links still resolve.
///
/// It is Markdown, with wiki-links as written; [`super::excerpt_html`] renders it.
#[must_use]
pub fn excerpt(body: &str) -> Cow<'_, str> {
    let Some((head, tail)) = body.split_once(SEPARATOR) else {
        return Cow::Borrowed(body);
    };
    if tail.is_empty() {
        return Cow::Borrowed(head);
    }
    let definitions: Vec<String> = DEFINITION
        .captures_iter(tail)
        .filter(|caps| head.contains(&caps[1]))
        .map(|caps| format!("{}{}", &caps[1], &caps[2]))
        .collect();
    if definitions.is_empty() {
        return Cow::Borrowed(head);
    }
    Cow::Owned(format!("{head}\n\n{}", definitions.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_marker_the_excerpt_is_the_whole_body() {
        assert_eq!(excerpt("One.\n\nTwo.\n"), "One.\n\nTwo.\n");
    }

    #[test]
    fn the_excerpt_keeps_the_definitions_it_uses() {
        let body = "See [this] and [that].\n<!--more-->\nMore.\n\n[this]: /a\n[other]: /b\n";
        assert_eq!(excerpt(body), "See [this] and [that].\n\n\n[this]: /a");
    }
}
