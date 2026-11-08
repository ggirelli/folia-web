//! The site's search index: its search script, the `documents` array the site's Liquid wrote,
//! followed by the script's code, read as it is from a fragment ([`SearchConfig`]). The array is
//! serialized here, not by a template (plan v12, §3.5 and §3.8; Q20: one file).
//!
//! Each record is `{"id", "url", "title", "body"}`, with ids counted from 0, in the script's
//! order:
//!
//! 1. The site's pages, by URL: every page rendered but the collections' entries, each redirect
//!    stub and `/redirects.json`; those whose URL contains `.xml`, `assets`, `category` or `tag`
//!    are left out, as the script left them (Q3e). A page's body is its content's text: an
//!    entry's body, else the rendered page, stub or file (Q3f).
//! 2. The RSVPs, likes and bookmarks, then the garden's notes, then the posts, each oldest first
//!    (same dates by URL). The script meant to list the micro kinds and never did (Q3c); micro
//!    notes have no loop. Their body is their date (`2021/08/20`), ` - ` and their content's
//!    text, or an RSVP's own text.
//!
//! A body's text is the script's filter chain: a space after each `.`, `: ` for the end of an
//! `h2` to `h4`, a space for the end of a paragraph, the markup stripped, line breaks removed,
//! two spaces made one and `"` a space. Its `markdownify` over HTML is left out (Q3q). Strings are
//! written as JSON strings, so a title with a quote no longer breaks the script (Q3q).

use std::fmt::Write;

use folia_core::content::SiteClock;
use folia_core::content::markdown::strip_html;
use folia_core::publish::FeedItem;
use serde::Deserialize;

#[cfg(test)]
mod tests;

/// The site's search script, which the builder writes.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchConfig {
    /// The script's source, which search claims, relative to the site root
    /// (`assets/js/lunrsearchengine.js`).
    pub source: String,
    /// The script's code after the `documents` array, relative to the site root: written as it is.
    pub fragment: String,
}

/// A page of the site, as search lists it.
pub struct Page {
    /// Its URL, from the site root.
    pub url: String,
    /// Its title, or empty.
    pub title: String,
    /// The HTML (or text) of its content.
    pub content: String,
}

/// What a page's URL contains when search leaves it out.
const LEFT_OUT: [&str; 4] = [".xml", "assets", "category", "tag"];

/// The collections' entries search lists, in its order, each as its feed item's collection and,
/// for micro, its kind.
const ENTRIES: [(&str, Option<&str>); 5] = [
    ("micro", Some("rsvps")),
    ("micro", Some("likes")),
    ("micro", Some("bookmarks")),
    ("digital_garden", None),
    ("posts", None),
];

/// The search script: the records of `pages` and `items`, then `fragment` as it is.
#[must_use]
pub fn script(
    site_url: &str,
    pages: &[Page],
    items: &[FeedItem],
    fragment: &str,
    clock: &SiteClock,
) -> String {
    let mut records: Vec<(String, &str, String)> = Vec::new();
    let mut pages: Vec<&Page> = pages
        .iter()
        .filter(|page| !LEFT_OUT.iter().any(|word| page.url.contains(word)))
        .collect();
    pages.sort_by(|a, b| a.url.cmp(&b.url));
    for page in pages {
        records.push((page.url.clone(), &page.title, text(&page.content)));
    }
    for (collection, kind) in ENTRIES {
        let mut entries: Vec<&FeedItem> = items
            .iter()
            .filter(|item| item.collection == collection)
            .filter(|item| kind.is_none_or(|kind| item.categories.iter().any(|c| c == kind)))
            .collect();
        entries.sort_by(|a, b| (a.published, &a.url).cmp(&(b.published, &b.url)));
        for item in entries {
            let date = clock.xmlschema(item.published)[..10].replace('-', "/");
            let body = item
                .search_text
                .clone()
                .unwrap_or_else(|| text(&item.content_html));
            records.push((item.url.clone(), &item.title, format!("{date} - {body}")));
        }
    }

    let mut out = String::from("var documents = [\n");
    for (id, (url, title, body)) in records.iter().enumerate() {
        if id > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{\"id\": {id}, \"url\": {}, \"title\": {}, \"body\": {}}}",
            json(&format!("{site_url}{url}")),
            json(title),
            json(body)
        );
    }
    out.push_str("\n];\n");
    out.push_str(fragment);
    out
}

/// The script's filter chain over `html`.
fn text(html: &str) -> String {
    let marked = html
        .replace('.', ". ")
        .replace("</h2>", ": ")
        .replace("</h3>", ": ")
        .replace("</h4>", ": ")
        .replace("</p>", " ");
    strip_html(&marked)
        .replace("\r\n", "")
        .replace('\n', "")
        .replace("  ", " ")
        .replace('"', " ")
}

fn json(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}
