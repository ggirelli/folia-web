//! The hard document assertions of plan v12, §5.1 (Q15): each a failure of its own, checked
//! whatever the DOM comparison and the ledger say. Pages are read with the whole-document rules
//! ([`dom::DOCUMENT`]); text values are compared with HTML whitespace collapsed and trimmed
//! ([`dom::collapse_and_trim`], the ledger kind's operation): a no-break space is not whitespace.
//!
//! | Assertion | Applies to | Requires |
//! |-|-|-|
//! | `robots` | a page Jekyll gives a `meta[name=robots]` | Folia's robots metas, names and contents compared case-insensitively, equal Jekyll's |
//! | `canonical` | every page | one `link[rel=canonical]`, equal to Jekyll's, absolute on the site's URL |
//! | `title` | every page | the `<title>` text equal to Jekyll's |
//! | `description` | a page either side gives a `meta[name=description]` | both have it, with equal `content` |
//! | `lang` | every page | `html@lang` equal to Jekyll's |
//! | `rel-me` | `/` (`index.html`) | the `href`s of every `rel="me"` element, as a multiset, equal to Jekyll's |
//! | `h-card` | `/` | the root `h-card` (an `h-card` with a `u-url u-uid`), its `u-url` and `p-name` equal to Jekyll's |
//! | `h-entry` | a page Jekyll gives an `h-entry` | as many `h-entry`s, each with Jekyll's `p-name`, `dt-published` and `u-url` values and an `e-content` where Jekyll's has them |
//! | `stub` | a redirect stub | its target (`meta[http-equiv=refresh]` content) and canonical equal to Jekyll's |
//!
//! The document-level assertions are for whole pages, not fragments; a stub is checked by
//! `stub` only (and is compared byte for byte besides).
//!
//! The files are read as bytes ([`check_bytes`]): a side that is absent or not UTF-8 is one failed
//! `read` outcome naming the file and the side, so no assertion is ever skipped unseen.
//!
//! A property of an `h-entry` or of the root `h-card` is looked for in that item only, as the
//! microformats2 parsing rules say: the search does not descend into a nested item (an element
//! with a root class name, `h-card` for instance), so a nested author's `p-name` or `u-url` cannot
//! stand for the entry's.

use serde::Serialize;

use crate::markdown::dom::{self, Node};

/// What kind of output a file is, which decides the assertions that apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// A page; `root` for the site's root page, `/`.
    Page { root: bool },
    /// A redirect stub.
    Stub,
}

/// One assertion's result for one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub file: String,
    pub assertion: &'static str,
    pub passed: bool,
    /// What was found, and on failure what was expected.
    pub detail: String,
}

/// Checks the files of the output `file` as [`check`] does, from their bytes: when a side is
/// absent (`None`) or not valid UTF-8, the result is one failed `read` outcome that says which,
/// and no assertion is checked.
pub fn check_bytes(
    file: &str,
    output: Output,
    jekyll: Option<&[u8]>,
    folia: Option<&[u8]>,
    site_url: &str,
) -> Vec<Outcome> {
    match (as_text("Jekyll", jekyll), as_text("Folia", folia)) {
        (Ok(jekyll), Ok(folia)) => check(file, output, jekyll, folia, site_url),
        (jekyll, folia) => {
            let problems: Vec<String> = [jekyll.err(), folia.err()].into_iter().flatten().collect();
            vec![Outcome {
                file: file.to_owned(),
                assertion: "read",
                passed: false,
                detail: format!(
                    "{}; no assertion was checked on this file",
                    problems.join("; ")
                ),
            }]
        }
    }
}

/// `bytes` as text, or why it is not: absent, or not valid UTF-8.
fn as_text<'a>(side: &str, bytes: Option<&'a [u8]>) -> Result<&'a str, String> {
    let bytes = bytes.ok_or_else(|| format!("{side}'s file is absent or cannot be read"))?;
    std::str::from_utf8(bytes).map_err(|e| {
        format!(
            "{side}'s file is not valid UTF-8 (at byte {})",
            e.valid_up_to()
        )
    })
}

/// Checks Folia's `folia` document against Jekyll's `jekyll` for the output `file`, with the
/// site's URL `site_url`.
pub fn check(
    file: &str,
    output: Output,
    jekyll: &str,
    folia: &str,
    site_url: &str,
) -> Vec<Outcome> {
    let want = dom::normalize_document_with(jekyll, &dom::DOCUMENT).1;
    let got = dom::normalize_document_with(folia, &dom::DOCUMENT).1;
    let mut out = Vec::new();
    let mut result = |assertion: &'static str, passed: bool, detail: String| {
        out.push(Outcome {
            file: file.to_owned(),
            assertion,
            passed,
            detail,
        });
    };
    if output == Output::Stub {
        let (passed, detail) = stub_assertion(&want, &got);
        result("stub", passed, detail);
        return out;
    }

    let (want_robots, got_robots) = (robots(&want), robots(&got));
    if !want_robots.is_empty() {
        let lower = |list: &[String]| -> Vec<String> {
            list.iter()
                .map(|value| value.to_ascii_lowercase())
                .collect()
        };
        result(
            "robots",
            lower(&want_robots) == lower(&got_robots),
            format!("Jekyll {want_robots:?}, Folia {got_robots:?}"),
        );
    }

    let (want_canonical, got_canonical) = (canonicals(&want), canonicals(&got));
    let absolute = |href: &str| href == site_url || href.starts_with(&format!("{site_url}/"));
    let passed = want_canonical.len() == 1
        && got_canonical == want_canonical
        && got_canonical.iter().all(|href| absolute(href));
    result(
        "canonical",
        passed,
        format!("Jekyll {want_canonical:?}, Folia {got_canonical:?}, on {site_url}"),
    );

    let (want_title, got_title) = (title(&want), title(&got));
    result(
        "title",
        want_title.is_some() && want_title == got_title,
        format!("Jekyll {want_title:?}, Folia {got_title:?}"),
    );

    let (want_description, got_description) = (description(&want), description(&got));
    if !want_description.is_empty() || !got_description.is_empty() {
        result(
            "description",
            want_description == got_description,
            format!("Jekyll {want_description:?}, Folia {got_description:?}"),
        );
    }

    let (want_lang, got_lang) = (lang(&want), lang(&got));
    result(
        "lang",
        want_lang == got_lang,
        format!("Jekyll {want_lang:?}, Folia {got_lang:?}"),
    );

    if output == (Output::Page { root: true }) {
        let (mut want_me, mut got_me) = (rel_me(&want), rel_me(&got));
        want_me.sort();
        got_me.sort();
        result(
            "rel-me",
            !want_me.is_empty() && want_me == got_me,
            format!("Jekyll {want_me:?}, Folia {got_me:?}"),
        );
        let (want_card, got_card) = (root_card(&want), root_card(&got));
        result(
            "h-card",
            want_card.is_some() && want_card == got_card,
            format!("Jekyll {want_card:?}, Folia {got_card:?}"),
        );
    }

    if let Some((passed, detail)) = entry_assertion(&want, &got) {
        result("h-entry", passed, detail);
    }
    out
}

/// The `stub` assertion: whether it holds, and what was found.
fn stub_assertion(want: &[Node], got: &[Node]) -> (bool, String) {
    let (want_target, got_target) = (refresh(want), refresh(got));
    let (want_canonical, got_canonical) = (canonicals(want), canonicals(got));
    let passed = want_target.is_some()
        && want_target == got_target
        && !want_canonical.is_empty()
        && want_canonical == got_canonical;
    (
        passed,
        format!(
            "target: Jekyll {want_target:?}, Folia {got_target:?}; canonical: Jekyll {want_canonical:?}, Folia {got_canonical:?}"
        ),
    )
}

/// The `h-entry` assertion: `None` when Jekyll's page has no entry, else whether it holds and
/// what was found.
fn entry_assertion(want: &[Node], got: &[Node]) -> Option<(bool, String)> {
    let want_entries = entries(want);
    if want_entries.is_empty() {
        return None;
    }
    let got_entries = entries(got);
    let mut problems = Vec::new();
    if want_entries.len() != got_entries.len() {
        problems.push(format!(
            "Jekyll has {} h-entry, Folia {}",
            want_entries.len(),
            got_entries.len()
        ));
    }
    for (i, (want, got)) in want_entries.iter().zip(&got_entries).enumerate() {
        for (property, want_value) in want {
            let got_value = got
                .iter()
                .find(|(name, _)| name == property)
                .map(|(_, v)| v);
            if got_value != Some(want_value) {
                problems.push(format!(
                    "h-entry {}: {property}: Jekyll {want_value:?}, Folia {got_value:?}",
                    i + 1
                ));
            }
        }
    }
    let detail = if problems.is_empty() {
        format!("{} h-entry, with Jekyll's properties", want_entries.len())
    } else {
        problems.join("; ")
    };
    Some((problems.is_empty(), detail))
}

fn walk<'a>(nodes: &'a [Node], visit: &mut dyn FnMut(&'a Node)) {
    for node in nodes {
        visit(node);
        if let Node::Element { children, .. } = node {
            walk(children, visit);
        }
    }
}

fn elements<'a>(
    nodes: &'a [Node],
    keep: &dyn Fn(&str, &std::collections::BTreeMap<String, String>) -> bool,
) -> Vec<&'a Node> {
    let mut out = Vec::new();
    walk(nodes, &mut |node| {
        if let Node::Element { name, attrs, .. } = node
            && keep(name, attrs)
        {
            out.push(node);
        }
    });
    out
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    match node {
        Node::Element { attrs, .. } => attrs.get(name).map(String::as_str),
        _ => None,
    }
}

fn tokens<'a>(attrs: &'a std::collections::BTreeMap<String, String>, name: &str) -> Vec<&'a str> {
    attrs
        .get(name)
        .map(|value| value.split_whitespace().collect())
        .unwrap_or_default()
}

fn has_token(attrs: &std::collections::BTreeMap<String, String>, name: &str, token: &str) -> bool {
    tokens(attrs, name).contains(&token)
}

fn text(node: &Node) -> String {
    match node {
        Node::Element { children, .. } => dom::collapse_and_trim(&dom::text(children)),
        Node::Text(text) => dom::collapse_and_trim(text),
        Node::Comment(_) => String::new(),
    }
}

fn robots(nodes: &[Node]) -> Vec<String> {
    elements(nodes, &|name, attrs| {
        name == "meta"
            && attrs
                .get("name")
                .is_some_and(|value| value.eq_ignore_ascii_case("robots"))
    })
    .into_iter()
    .map(|node| attr(node, "content").unwrap_or_default().to_owned())
    .collect()
}

fn canonicals(nodes: &[Node]) -> Vec<String> {
    elements(nodes, &|name, attrs| {
        name == "link"
            && tokens(attrs, "rel")
                .iter()
                .any(|t| t.eq_ignore_ascii_case("canonical"))
    })
    .into_iter()
    .map(|node| attr(node, "href").unwrap_or_default().to_owned())
    .collect()
}

fn title(nodes: &[Node]) -> Option<String> {
    elements(nodes, &|name, _| name == "title")
        .first()
        .map(|node| text(node))
}

fn description(nodes: &[Node]) -> Vec<String> {
    elements(nodes, &|name, attrs| {
        name == "meta"
            && attrs
                .get("name")
                .is_some_and(|v| v.eq_ignore_ascii_case("description"))
    })
    .into_iter()
    .map(|node| dom::collapse_and_trim(attr(node, "content").unwrap_or_default()))
    .collect()
}

fn lang(nodes: &[Node]) -> Option<String> {
    elements(nodes, &|name, _| name == "html")
        .first()
        .and_then(|node| attr(node, "lang"))
        .map(str::to_owned)
}

fn rel_me(nodes: &[Node]) -> Vec<String> {
    elements(nodes, &|_, attrs| has_token(attrs, "rel", "me"))
        .into_iter()
        .map(|node| attr(node, "href").unwrap_or_default().to_owned())
        .collect()
}

fn refresh(nodes: &[Node]) -> Option<String> {
    elements(nodes, &|name, attrs| {
        name == "meta"
            && attrs
                .get("http-equiv")
                .is_some_and(|v| v.eq_ignore_ascii_case("refresh"))
    })
    .first()
    .and_then(|node| attr(node, "content"))
    .map(str::to_owned)
}

/// Whether `class` is a microformats2 root class name, such as `h-card` or `h-entry`:
/// `h-`, an optional vendor prefix ending in `-`, then lowercase words joined by `-`. Bootstrap's
/// `h-100` is not one.
fn is_root_class(class: &str) -> bool {
    let Some(rest) = class.strip_prefix("h-") else {
        return false;
    };
    let words: Vec<&str> = rest.split('-').collect();
    let lower = |word: &str| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase());
    let vendor = |word: &str| {
        !word.is_empty()
            && word
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    };
    match words.as_slice() {
        [] => false,
        [first, others @ ..] => {
            (lower(first) && others.iter().all(|w| lower(w)))
                || (vendor(first) && !others.is_empty() && others.iter().all(|w| lower(w)))
        }
    }
}

fn is_item(node: &Node) -> bool {
    match node {
        Node::Element { attrs, .. } => tokens(attrs, "class").into_iter().any(is_root_class),
        _ => false,
    }
}

/// The first element below `item`'s children, in document order, with the class `class`, not
/// looking inside nested items (an element that is itself a nested item may still carry the
/// class, as a property of `item`).
fn property<'a>(children: &'a [Node], class: &str) -> Option<&'a Node> {
    for node in children {
        if let Node::Element {
            attrs,
            children: inner,
            ..
        } = node
        {
            if has_token(attrs, "class", class) {
                return Some(node);
            }
            if is_item(node) {
                continue;
            }
            if let Some(found) = property(inner, class) {
                return Some(found);
            }
        }
    }
    None
}

/// The first element below `children` with both classes, not looking inside nested items.
fn property_with<'a>(children: &'a [Node], class: &str, also: &str) -> Option<&'a Node> {
    for node in children {
        if let Node::Element {
            attrs,
            children: inner,
            ..
        } = node
        {
            if has_token(attrs, "class", class) && has_token(attrs, "class", also) {
                return Some(node);
            }
            if is_item(node) {
                continue;
            }
            if let Some(found) = property_with(inner, class, also) {
                return Some(found);
            }
        }
    }
    None
}

/// The root h-card: the first `h-card` with an element of class `u-url u-uid` among its own
/// properties, as its `u-url` href and its `p-name` text.
fn root_card(nodes: &[Node]) -> Option<(String, String)> {
    elements(nodes, &|_, attrs| has_token(attrs, "class", "h-card"))
        .into_iter()
        .find_map(|card| {
            let Node::Element { children, .. } = card else {
                return None;
            };
            let url = attr(property_with(children, "u-url", "u-uid")?, "href")?.to_owned();
            let name = property(children, "p-name").map(text).unwrap_or_default();
            Some((url, name))
        })
}

/// A `dt-*` value: the element's `datetime`, else that of the first `time` with one inside it,
/// else its text.
fn datetime(node: &Node) -> String {
    if let Some(value) = attr(node, "datetime") {
        return value.to_owned();
    }
    let Node::Element { children, .. } = node else {
        return String::new();
    };
    elements(children, &|name, attrs| {
        name == "time" && attrs.contains_key("datetime")
    })
    .first()
    .and_then(|time| attr(time, "datetime"))
    .map_or_else(|| text(node), str::to_owned)
}

/// Each `h-entry`, as its properties: `p-name` (text), `dt-published` ([`datetime`]),
/// `u-url` (`href`) and `e-content` (present), the first of each among its own properties
/// ([`property`]: not inside a nested item).
fn entries(nodes: &[Node]) -> Vec<Vec<(String, String)>> {
    elements(nodes, &|_, attrs| has_token(attrs, "class", "h-entry"))
        .into_iter()
        .map(|entry| {
            let Node::Element { children, .. } = entry else {
                return Vec::new();
            };
            let first = |class: &str| property(children, class);
            let mut properties = Vec::new();
            if let Some(node) = first("p-name") {
                properties.push(("p-name".to_owned(), text(node)));
            }
            if let Some(node) = first("dt-published") {
                properties.push(("dt-published".to_owned(), datetime(node)));
            }
            if let Some(node) = first("u-url") {
                properties.push((
                    "u-url".to_owned(),
                    attr(node, "href").unwrap_or_default().to_owned(),
                ));
            }
            if first("e-content").is_some() {
                properties.push(("e-content".to_owned(), "present".to_owned()));
            }
            properties
        })
        .collect()
}

#[cfg(test)]
mod tests;
