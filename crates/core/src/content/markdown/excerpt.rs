//! Excerpts, as Jekyll takes them from a document's Markdown, and the description a page's head
//! makes of one.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

/// The marker that ends an excerpt (the site's `excerpt_separator`).
const SEPARATOR: &str = "<!--more-->";

/// A link reference definition, as Jekyll finds them: `[label]` and the rest of the line.
static DEFINITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^ {0,3}(\[[^\]]+\])(:.+)$").expect("the expression is valid")
});

/// Scripts, comments and styles, which Liquid's `strip_html` drops with their content.
static HTML_BLOCKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<script.*?</script>|<!--.*?-->|<style.*?</style>")
        .expect("the expression is valid")
});
static HTML_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<.*?>").expect("the expression is valid"));

/// The length, in characters and with its `...`, past which a description is cut.
const DESCRIPTION_LENGTH: usize = 160;

/// The description of a page whose excerpt is `excerpt_html`, as the site's head template wrote
/// it with Liquid (`strip_html | strip_newlines | truncate: 160`): without scripts, comments,
/// styles or tags, with entities as written, without line breaks, and cut to 160 characters,
/// `...` included. It is HTML text, to be shown unescaped.
#[must_use]
pub fn description(excerpt_html: &str) -> String {
    let text = HTML_TAG
        .replace_all(&HTML_BLOCKS.replace_all(excerpt_html, ""), "")
        .replace("\r\n", "")
        .replace('\n', "");
    if text.chars().count() > DESCRIPTION_LENGTH {
        let kept: String = text.chars().take(DESCRIPTION_LENGTH - 3).collect();
        kept + "..."
    } else {
        text
    }
}

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
    fn a_description_drops_markup_and_line_breaks_and_keeps_entities() {
        assert_eq!(
            description("<p>A &amp; B\n<!-- c --><em>c</em></p>\n<script>x</script>"),
            "A &amp; Bc"
        );
    }

    #[test]
    fn a_long_description_is_cut_with_its_ellipsis_within_the_length() {
        let long = "é".repeat(200);
        let cut = description(&long);
        assert_eq!(cut.chars().count(), 160);
        assert!(cut.ends_with("é..."));
        assert_eq!(description(&"a".repeat(160)), "a".repeat(160));
    }

    #[test]
    fn the_excerpt_keeps_the_definitions_it_uses() {
        let body = "See [this] and [that].\n<!--more-->\nMore.\n\n[this]: /a\n[other]: /b\n";
        assert_eq!(excerpt(body), "See [this] and [that].\n\n\n[this]: /a");
    }
}
