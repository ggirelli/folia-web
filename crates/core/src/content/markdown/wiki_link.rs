//! Wiki-links: `[[Target]]` and `[[Target|text]]`.
//!
//! They are replaced in the source, before parsing, by the HTML the site's Jekyll plugin
//! (`_plugins/wiki_links.rb`) wrote: a link with the class `internal-link`, or a
//! `broken-link` span when nothing matches. The text is left as written, so Markdown and
//! typography in it apply as they did. Code is skipped the way the plugin's regular expression
//! skips it: fenced blocks and code spans, found without parsing.

use std::sync::LazyLock;

use regex::{Captures, Regex};

/// Finds the URL a wiki-link points to. Which document a target names is the garden's business;
/// Markdown only writes the link.
pub trait LinkResolver {
    /// The URL of the document `target` names, or `None` for a broken link. `target` is as
    /// written, before the `|`.
    fn resolve(&self, target: &str) -> Option<String>;
}

/// Code, which is kept as it is, or a wiki-link: the plugin's expression.
static WIKI_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(?:```.*?```|`[^`]+`|\{%.*?%\})|(\[\[([^\]|]+)(?:\|([^\]]+))?\]\])")
        .expect("the expression is valid")
});

/// Replaces every wiki-link in `src` with its HTML.
pub fn replace(src: &str, links: &dyn LinkResolver) -> String {
    WIKI_LINK
        .replace_all(src, |caps: &Captures| {
            let Some(target) = caps.get(2) else {
                return caps[0].to_owned();
            };
            let text = caps.get(3).unwrap_or(target).as_str();
            match links.resolve(target.as_str()) {
                Some(url) => format!("<a href=\"{url}\" class=\"internal-link\">{text}</a>"),
                None => format!("<span class=\"broken-link\">{text}</span>"),
            }
        })
        .into_owned()
}
