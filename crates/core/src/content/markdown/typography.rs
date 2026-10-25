//! Smart typography as kramdown does it: `---` → —, `--` → –, `...` → …, and curly quotes.
//!
//! kramdown decides each quote with an ordered list of regular expressions, matched where its
//! scanner stands: on the quote, or on the character before it when that character has not
//! been taken by something else (an earlier quote, a dash, an escape, an element). The
//! expressions look ahead into the raw source, markup included. This module replays that scan
//! on the tree's plain text, which is the source as written, so it sees the same characters.
//!
//! kramdown's `<<` and `>>` guillemets are not supported; the site has none.

use super::tree::{Kind, Node};

/// Applies smart typography to the text of `root`, except code, raw HTML, image descriptions and
/// autolinks, which kramdown leaves alone.
pub fn apply(root: &mut Node, src: &str) {
    let bound = content_end(root);
    walk(root, src, bound);
}

fn walk(node: &mut Node, src: &str, bound: usize) {
    // Where the last text ended, if its last character is untaken.
    let mut free_end: Option<usize> = None;
    for child in &mut node.children {
        match &mut child.kind {
            Kind::Text { text, plain: true } => {
                let start = child.span.start;
                let has_previous = free_end == Some(start);
                let (smartened, last_free) =
                    smarten(src, start..child.span.end, has_previous, bound);
                *text = smartened;
                free_end = last_free.then_some(child.span.end);
                continue;
            }
            Kind::SoftBreak => {
                // The line break and the next line's indentation are text to kramdown.
                free_end = Some(next_line_text_start(src, child.span.end));
                continue;
            }
            Kind::Code(_)
            | Kind::CodeBlock { .. }
            | Kind::HtmlBlock(_)
            | Kind::Image { .. }
            | Kind::Link { auto: true, .. } => {}
            Kind::Emphasis | Kind::Strong | Kind::Strikethrough => walk(child, src, bound),
            _ => {
                let bound = content_end(child);
                walk(child, src, bound);
            }
        }
        free_end = None;
    }
}

/// Where the text kramdown scans for a block or link ends: after its last child.
fn content_end(node: &Node) -> usize {
    node.children
        .last()
        .map_or(node.span.end, |child| child.span.end)
}

fn next_line_text_start(src: &str, from: usize) -> usize {
    from + src[from..].len() - src[from..].trim_start_matches([' ', '\t']).len()
}

/// The smart version of the plain text at `text` in `src`, and whether its last character is
/// untaken. `has_previous` says whether the character before it is untaken text; the
/// expressions may look up to `bound`.
fn smarten(
    src: &str,
    text: std::ops::Range<usize>,
    has_previous: bool,
    bound: usize,
) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut at = text.start;
    // The untaken character before `at`, which a scan may start on (never a backslash).
    let mut free = if has_previous {
        src[..at].chars().next_back().filter(|&c| c != '\\')
    } else {
        None
    };
    while at < text.end {
        let rest = &src[at..text.end];
        let c = rest.chars().next().expect("not at the end");
        if let Some((symbol, len)) = typographic_symbol(rest) {
            out.push_str(symbol);
            at += len;
            free = None;
        } else if is_quote(c) {
            // Only at the start of the text can a free character precede an unscanned quote.
            let (quotes, len) = quote_at(src, at, free, bound);
            out.push_str(&quotes);
            at += len;
            free = None;
        } else {
            out.push(c);
            at += c.len_utf8();
            free = (c != '\\').then_some(c);
        }
    }
    (out, free.is_some())
}

fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}

fn typographic_symbol(text: &str) -> Option<(&'static str, usize)> {
    [("---", "—"), ("--", "–"), ("...", "…")]
        .into_iter()
        .find(|(from, _)| text.starts_with(from))
        .map(|(from, to)| (to, from.len()))
}

/// Quote kinds, before they are made single or double by the quote they replace.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

fn curly(quote: char, side: Side) -> char {
    match (quote, side) {
        ('"', Side::Left) => '“',
        ('"', Side::Right) => '”',
        (_, Side::Left) => '‘',
        (_, Side::Right) => '’',
    }
}

/// kramdown's rules for the quote at `at`, scanned from `before` (the untaken character before
/// it) if there is one. Returns what replaces the quotes it takes, without `before`, and how many
/// bytes they span from `at`.
fn quote_at(src: &str, at: usize, before: Option<char>, bound: usize) -> (String, usize) {
    let ahead = &src[at..bound.max(at)];
    let mut chars = ahead.chars();
    let quote = chars.next().expect("a quote is here");
    let n1 = chars.next();
    let n2 = chars.next();
    let n3 = chars.next();
    let one = |side| (curly(quote, side).to_string(), 1);

    let Some(before) = before else {
        // [_*]{1,2}\S after the quote: it opens emphasised text.
        if n1.is_some_and(|c| c == '_' || c == '*') && n2.is_some_and(|c| !is_space(c)) {
            return one(Side::Left);
        }
        // Punctuation not followed by `..` nor by a word boundary: it closes.
        if let Some(p) = n1.filter(|&c| is_punctuation(c))
            && !(n2 == Some('.') && n3 == Some('.'))
            && is_word(p) == n2.is_some_and(is_word)
        {
            return one(Side::Right);
        }
        if let Some(pair) = double_set(quote, n1, n2) {
            return pair;
        }
        if quote == '\'' && is_decade(n1, n2, n3) {
            return ("’".to_owned(), 1);
        }
        // A quote before a second one is SQ_CLOSE for it, and stays straight.
        if n1.is_some_and(is_quote) {
            return (
                format!("{quote}{}", curly(n1.expect("checked"), Side::Right)),
                2,
            );
        }
        if n1.is_none_or(|c| is_space(c) || c == '\n')
            || (n1 == Some('s') && !n2.is_some_and(is_word))
        {
            return one(Side::Right);
        }
        return one(Side::Left);
    };

    // The scan stands on `before`; only the rules that may start with a character apply.
    if is_space(before) {
        if let Some(pair) = double_set(quote, n1, n2) {
            return pair;
        }
        if quote == '\'' && is_decade(n1, n2, n3) {
            return ("’".to_owned(), 1);
        }
        if n1.is_some_and(is_ascii_word) {
            return one(Side::Left);
        }
    }
    if !is_close_exception(before) {
        return one(Side::Right);
    }
    one(Side::Left)
}

/// `"'` or `'"` before a word: both open.
fn double_set(quote: char, n1: Option<char>, n2: Option<char>) -> Option<(String, usize)> {
    let other = n1.filter(|&c| is_quote(c) && c != quote)?;
    if !n2.is_some_and(is_ascii_word) {
        return None;
    }
    Some((
        format!("{}{}", curly(quote, Side::Left), curly(other, Side::Left)),
        2,
    ))
}

/// `'80s`
fn is_decade(n1: Option<char>, n2: Option<char>, n3: Option<char>) -> bool {
    n1.is_some_and(|c| c.is_ascii_digit())
        && n2.is_some_and(|c| c.is_ascii_digit())
        && n3 == Some('s')
}

/// Characters after which a quote does not close (`SQ_CLOSE` excludes them).
fn is_close_exception(c: char) -> bool {
    matches!(c, ' ' | '\\' | '\t' | '\r' | '\n' | '[' | '{' | '(' | '-')
}

/// Ruby's `\s`.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// Ruby's `\w`, which is ASCII.
fn is_ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// A word character for Ruby's `\b`, which is Unicode.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// kramdown's `SQ_PUNCT`.
fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
}

#[cfg(test)]
mod tests {
    use super::super::tree;
    use super::*;

    fn smart(src: &str) -> String {
        let mut root = tree::parse(src);
        apply(&mut root, src);
        let mut out = String::new();
        collect(&root, &mut out);
        out
    }

    fn collect(node: &Node, out: &mut String) {
        match &node.kind {
            Kind::Text { text, .. } => out.push_str(text),
            Kind::SoftBreak => out.push('\n'),
            _ => node.children.iter().for_each(|c| collect(c, out)),
        }
    }

    #[test]
    fn quotes_curl_by_their_neighbours() {
        assert_eq!(smart("that's \"it\""), "that’s “it”");
        assert_eq!(smart("'quoted' words"), "‘quoted’ words");
        assert_eq!(smart("Cos'è l'autismo?"), "Cos’è l’autismo?");
        assert_eq!(smart("the '80s"), "the ’80s");
    }

    #[test]
    fn dashes_and_ellipses_become_symbols() {
        assert_eq!(smart("a -- b --- c..."), "a – b — c…");
    }

    #[test]
    fn a_quote_after_an_element_has_no_character_before_it() {
        // kramdown's scanner never sees the `*` the emphasis took.
        assert_eq!(smart("*word*'s end"), "word’s end");
    }
}
