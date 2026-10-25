//! The Liquid 4.0.4 and Jekyll 4.4.1 filters the site's templates apply to excerpts, so gate G5
//! compares the values the templates actually show.

use std::sync::LazyLock;

use regex::Regex;

static BLOCKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<script.*?</script>|<!--.*?-->|<style.*?</style>")
        .expect("the expression is valid")
});
static TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<.*?>").expect("the expression is valid"));

/// Ruby's `\s`.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// `strip_html`: drops scripts, comments, styles and tags; entities stay as written.
pub fn strip_html(input: &str) -> String {
    TAGS.replace_all(&BLOCKS.replace_all(input, ""), "")
        .into_owned()
}

/// `strip_newlines`
pub fn strip_newlines(input: &str) -> String {
    input.replace("\r\n", "").replace('\n', "")
}

/// `truncate: length`, counting characters, with `...` within the length.
pub fn truncate(input: &str, length: usize) -> String {
    if input.chars().count() > length {
        let kept: String = input.chars().take(length.saturating_sub(3)).collect();
        kept + "..."
    } else {
        input.to_owned()
    }
}

/// `truncatewords: words`. Liquid 4 appends `...` once there are `words` words, even with no
/// more after them.
pub fn truncatewords(input: &str, words: usize) -> String {
    let wordlist: Vec<&str> = input.split(is_space).filter(|w| !w.is_empty()).collect();
    let last = words.saturating_sub(1);
    if wordlist.len() > last {
        wordlist[..=last].join(" ") + "..."
    } else {
        input.to_owned()
    }
}

/// Jekyll's `normalize_whitespace`.
pub fn normalize_whitespace(input: &str) -> String {
    input
        .split(is_space)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_keeps_entities() {
        assert_eq!(strip_html("<p>a &amp; <!-- c -->b</p>"), "a &amp; b");
    }

    #[test]
    fn truncate_counts_the_ellipsis() {
        assert_eq!(truncate("abcdefgh", 5), "ab...");
        assert_eq!(truncate("abcde", 5), "abcde");
    }

    #[test]
    fn truncatewords_marks_exactly_enough_words() {
        assert_eq!(truncatewords("a b c", 3), "a b c...");
        assert_eq!(truncatewords("a  b\nc d", 3), "a b c...");
        assert_eq!(truncatewords("a b", 3), "a b");
    }
}
