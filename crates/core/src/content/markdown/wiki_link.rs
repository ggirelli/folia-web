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

/// The target of every wiki-link in `src`, as written, in order; links in code are not counted.
pub fn targets(src: &str) -> Vec<&str> {
    WIKI_LINK
        .captures_iter(src)
        .filter_map(|caps| caps.get(2))
        .map(|target| target.as_str())
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every target is broken, so `replace` shows which links it counted.
    struct Nowhere;

    impl LinkResolver for Nowhere {
        fn resolve(&self, _: &str) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_wiki_link_in_a_fenced_block_is_not_a_target() {
        let src =
            "```rust\nlet a = [[x]];\n// [[In code]]\n```\n\n[[Out]] and ```[[Inline fence]]```\n";
        assert_eq!(targets(src), ["Out"]);
        assert_eq!(
            replace(src, &Nowhere),
            "```rust\nlet a = [[x]];\n// [[In code]]\n```\n\n\
             <span class=\"broken-link\">Out</span> and ```[[Inline fence]]```\n"
        );
    }

    #[test]
    fn a_wiki_link_in_a_liquid_tag_or_a_code_span_is_not_a_target() {
        let src = "{% include note.html title=\"[[Tag]]\" %} `[[Span]]`\n{% raw %}[[Raw]]{% endraw %} [[A|text]] [[ B ]]\n";
        // Between `{% raw %}` and `{% endraw %}` is outside both tags, so it counts, as in the
        // plugin, whose expression skips each tag, not what it encloses.
        assert_eq!(targets(src), ["Raw", "A", " B "]);
    }

    #[test]
    fn a_wiki_link_in_a_liquid_output_is_a_target() {
        // The plugin skipped `{% %}` tags only, not `{{ }}`.
        assert_eq!(targets("{{ \"[[Out]]\" }}"), ["Out"]);
    }

    #[test]
    fn a_target_is_as_written_before_the_bar_and_repeats_count() {
        assert_eq!(
            targets("[[Rust|rustaceans]], [[rust]], [[Rust|again]]"),
            ["Rust", "rust", "Rust"]
        );
    }
}
