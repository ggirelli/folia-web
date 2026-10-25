//! Code blocks, written in the markup kramdown gives Rouge's output, highlighted with syntect.
//!
//! syntect names tokens with `TextMate` scopes; the site's stylesheet (`_sass/_highlight.scss`) styles
//! Rouge's Pygments short classes (`k`, `nf`, `s2`, …). Each token is given the Pygments class of
//! the first rule below that its scopes match, so the stylesheet keeps working. The rules follow
//! what Rouge emits for the languages the site uses.
//!
//! The grammars are bat's (`two-face`): syntect's own set has no TOML.

use std::sync::LazyLock;

use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use super::render::escape;
use super::tree::Attrs;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// Writes a code block in language `lang`, whose opening line is indented by `indent`.
pub fn block(out: &mut String, lang: Option<&str>, code: &str, attrs: &Attrs, indent: usize) {
    let Some(lang) = lang else {
        out.push_str("<pre><code>");
        escape(out, code);
        out.push_str("</code></pre>\n");
        return;
    };
    let mut classes = format!("language-{lang} highlighter-rouge");
    if let Some((_, extra)) = attrs.0.iter().find(|(key, _)| key == "class") {
        classes.push(' ');
        classes.push_str(extra);
    }
    out.push_str("<div class=\"");
    escape(out, &classes);
    out.push_str("\"><div class=\"highlight\"><pre class=\"highlight\"><code>");
    match syntax(lang).and_then(|syntax| highlight(code, lang, syntax)) {
        Some(html) => out.push_str(&html),
        None => escape(out, code),
    }
    out.push_str("</code></pre></div>");
    out.extend(std::iter::repeat_n(' ', indent));
    out.push_str("</div>\n");
}

fn syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let token = match lang.to_ascii_lowercase().as_str() {
        "text" | "plaintext" | "txt" => return None,
        "shell" | "sh" | "console" => "bash".to_owned(),
        other => other.to_owned(),
    };
    SYNTAXES.find_syntax_by_token(&token)
}

/// The code as HTML with a `span` per token, or `None` if syntect cannot parse it.
fn highlight(code: &str, lang: &str, syntax: &SyntaxReference) -> Option<String> {
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut tokens: Vec<(Option<&'static str>, String)> = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ops = state.parse_line(line, &SYNTAXES).ok()?;
        let mut at = 0;
        for (index, op) in ops {
            push_token(&mut tokens, lang, &stack, &line[at..index]);
            at = index;
            stack.apply(&op).ok()?;
        }
        push_token(&mut tokens, lang, &stack, &line[at..]);
    }
    let mut html = String::new();
    for (class, text) in tokens {
        match class {
            Some(class) => {
                html.push_str("<span class=\"");
                html.push_str(class);
                html.push_str("\">");
                escape(&mut html, &text);
                html.push_str("</span>");
            }
            None => escape(&mut html, &text),
        }
    }
    Some(html)
}

/// Adds `text`, one run of syntect's, with the class its scopes give; whitespace inside a
/// classified run, as in a comment or a string, belongs to the token. Text no rule classifies
/// is split into names, punctuation and whitespace, which Rouge leaves unmarked between tokens.
/// A token joins the previous one when their classes are the same.
fn push_token(
    tokens: &mut Vec<(Option<&'static str>, String)>,
    lang: &str,
    stack: &ScopeStack,
    text: &str,
) {
    let scopes: Vec<String> = stack
        .as_slice()
        .iter()
        .map(|scope| scope.build_string())
        .collect();
    let lang = &embedded_language(lang, &scopes);
    if let Some(class) = scoped_class(lang, &scopes).filter(|_| !text.trim().is_empty()) {
        push(tokens, Some(called_class(lang, class, text)), text);
        return;
    }
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let kind = char_kind(first);
        let len = rest.find(|c| char_kind(c) != kind).unwrap_or(rest.len());
        let piece = &rest[..len];
        let class = (kind != CharKind::Space)
            .then(|| default_class(&lang.to_ascii_lowercase(), piece))
            .flatten();
        push(tokens, class, piece);
        rest = &rest[len..];
    }
}

/// The language Rouge lexes the text in: HTML hands the content of `<script>` to its JavaScript
/// lexer.
fn embedded_language(lang: &str, scopes: &[String]) -> String {
    let lang = lang.to_ascii_lowercase();
    if lang == "html" && scopes.iter().any(|scope| scope.starts_with("source.js")) {
        return "javascript".to_owned();
    }
    lang
}

/// Rouge's Python lexer names any call `nf`, except a capitalized one, which it takes for a class.
fn called_class(lang: &str, class: &'static str, text: &str) -> &'static str {
    let capitalized = text.trim_start().starts_with(char::is_uppercase);
    if lang == "python" && class == "nf" && capitalized {
        "nc"
    } else {
        class
    }
}

fn push(tokens: &mut Vec<(Option<&'static str>, String)>, class: Option<&'static str>, text: &str) {
    if text.is_empty() {
        return;
    }
    match tokens.last_mut() {
        Some((last, joined)) if *last == class => joined.push_str(text),
        _ => tokens.push((class, text.to_owned())),
    }
}

#[derive(PartialEq, Eq)]
enum CharKind {
    Space,
    Word,
    Other,
}

fn char_kind(c: char) -> CharKind {
    if c.is_whitespace() {
        CharKind::Space
    } else if c.is_alphanumeric() || c == '_' {
        CharKind::Word
    } else {
        CharKind::Other
    }
}

/// The Pygments class the scopes (outermost first) give in language `lang`, if a rule matches.
fn scoped_class(lang: &str, scopes: &[String]) -> Option<&'static str> {
    let has = |prefix: &str| scopes.iter().any(|scope| scope.starts_with(prefix));
    let innermost = scopes.last().map_or("", String::as_str);
    let lang = lang.to_ascii_lowercase();
    if let Some((_, _, class)) = LANGUAGE_RULES
        .iter()
        .find(|(language, prefix, _)| *language == lang && has(prefix))
    {
        return *class;
    }
    if has("comment") {
        return Some(match lang.as_str() {
            _ if has("comment.line.double-slash") => "c1",
            "python" if !has("comment.block") => "c1",
            _ if has("comment.block") && lang != "bash" && lang != "html" => "cm",
            _ => "c",
        });
    }
    if has("constant.character.escape") {
        return Some("se");
    }
    if has("string") && !has("meta.interpolation") {
        let delimiter = innermost.starts_with("punctuation.definition.string");
        return Some(string_class(&lang, scopes, delimiter));
    }
    RULES
        .iter()
        .find(|(prefix, _)| scopes.iter().rev().any(|scope| scope.starts_with(prefix)))
        .map(|(_, class)| *class)
}

/// Rules for one language, checked before all others: where Rouge's lexer differs from the
/// general mapping. `None` leaves the text unmarked.
const LANGUAGE_RULES: &[(&str, &str, Option<&str>)] = &[
    ("python", "comment.block.documentation", Some("s")),
    ("python", "string.quoted.docstring", Some("s")),
    ("python", "constant.language", Some("bp")),
    ("python", "variable.language", Some("n")),
    ("python", "variable.function", Some("nf")),
    ("python", "keyword.declaration", Some("k")),
    ("python", "keyword.control.loop.for.in", Some("ow")),
    (
        "python",
        "punctuation.separator.annotation.return",
        Some("o"),
    ),
    ("r", "support.function", Some("n")),
    ("r", "variable.function", Some("n")),
    ("r", "constant.numeric", Some("m")),
    ("javascript", "meta.object-literal.key", Some("na")),
    ("html", "punctuation.definition.tag", Some("nt")),
    ("html", "punctuation.separator.key-value", Some("na")),
    ("bash", "variable.function", None),
    ("bash", "entity.name.function", None),
    ("bash", "keyword.operator.logical.pipe", None),
    ("bash", "variable.parameter.option", Some("nt")),
    ("bash", "variable.other.readwrite.assignment", Some("nv")),
    ("toml", "entity.name.tag", Some("n")),
    ("toml", "punctuation.definition.key-value", Some("o")),
    ("toml", "punctuation.definition.table", Some("k")),
];

fn string_class(lang: &str, scopes: &[String], delimiter: bool) -> &'static str {
    let double = scopes.iter().any(|s| s.starts_with("string.quoted.double"));
    match lang {
        "python" if delimiter => "sh",
        "javascript" | "js" if delimiter => "dl",
        "javascript" | "js" => "s2",
        "bash" | "r" if double => "s2",
        "bash" | "r" => "s1",
        _ => "s",
    }
}

/// Scope prefixes and the class they give, most specific first; the innermost matching scope
/// wins within a rule's order.
const RULES: &[(&str, &str)] = &[
    ("keyword.control.import", "kn"),
    ("keyword.operator.logical.python", "ow"),
    ("keyword.operator.word", "ow"),
    ("keyword.operator", "o"),
    ("storage.type.function.js", "kd"),
    ("storage.type.js", "kd"),
    ("keyword.declaration", "kd"),
    ("keyword", "k"),
    ("storage.type", "k"),
    ("storage.modifier", "k"),
    ("entity.name.function.decorator", "nd"),
    ("meta.annotation", "nd"),
    ("punctuation.definition.annotation", "nd"),
    ("variable.language", "bp"),
    ("support.function.builtin", "nb"),
    ("support.function", "nb"),
    ("support.type", "nb"),
    ("entity.name.function", "nf"),
    ("variable.function", "nf"),
    ("entity.name.class", "nc"),
    ("entity.name.type.class", "nc"),
    ("support.class", "nc"),
    ("entity.name.tag", "nt"),
    ("entity.other.attribute-name", "na"),
    ("meta.mapping.key", "na"),
    ("variable.other.readwrite.shell", "nv"),
    ("variable.other.normal.shell", "nv"),
    ("constant.numeric.integer", "mi"),
    ("constant.numeric.float", "mf"),
    ("constant.numeric", "m"),
    ("constant.language", "kc"),
    ("punctuation", "p"),
];

/// Rouge's class for text no rule matched: names in most languages, nothing in shell and markup.
fn default_class(lang: &str, text: &str) -> Option<&'static str> {
    let word = text.chars().all(|c| c.is_alphanumeric() || c == '_');
    match lang {
        "javascript" | "js" if word => Some("nx"),
        "python" | "r" | "rust" | "toml" if word => Some("n"),
        "python" | "r" | "rust" | "toml" | "javascript" | "js" => Some("p"),
        _ => None,
    }
}
