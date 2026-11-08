//! The site's Atom feeds, as the `jekyll-feed` plugin (0.17) wrote them: its template, minified as
//! the plugin minifies it (whitespace after `>` and `}` removed), with the values its Liquid
//! filters gave. Serialized here, not by a template (plan v12, §3.8).
//!
//! The modes plan the feeds' routes; each feed is written when the configuration has `[feeds]`
//! ([`FeedsConfig`]). What a feed lists, and its title, follow from what generates it
//! ([`Feed::of`]):
//!
//! | Generator | Route | Items | Title |
//! |-|-|-|-|
//! | `blog feed` | `/blog/feed.xml` | the posts | the site's |
//! | `tag feed` | `/blog/tag/:tag.xml` | the posts with the tag | the site's |
//! | `micro feed` | `/micro/feed.xml` | the micro entries | the site's, then ` \| Micro` |
//! | `micro kind feed` | `/feed/micro/:kind.xml` | the entries of the kind | the micro feed's, then ` \| ` and the kind, capitalized |
//! | `garden feed` | `/dg/feed.xml` | the notes | the site's, then ` \| Digital_garden` |
//!
//! Each lists its newest `limit` items (10), newest first, those of the same date by URL,
//! descending. An item's title is smartened, its markup stripped and its whitespace collapsed;
//! its summary is its excerpt's text, whitespace collapsed (always there, as Jekyll's excerpt is
//! never empty).
//!
//! The feed's own `<updated>` is the latest `<updated>` of the items it lists, or of every item
//! of the site when it lists none, never the build time Jekyll wrote (plan v9, §7.3).

use std::fmt::Write;

use folia_core::content::SiteClock;
use folia_core::content::markdown::{strip_html, to_html};
use folia_core::publish::FeedItem;
use jiff::Timestamp;
use serde::Deserialize;

#[cfg(test)]
mod tests;

/// The site's feeds.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeedsConfig {
    /// The language the feeds declare (`xml:lang`), if any.
    #[serde(default)]
    pub lang: Option<String>,
    /// What the feeds say generated them.
    pub generator: Generator,
    /// How many items a feed lists.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// What a feed says generated it: `<generator uri="…" version="…">name</generator>`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    /// Its name (`Jekyll`).
    pub name: String,
    /// Its home page.
    pub uri: String,
    /// Its version.
    pub version: String,
}

/// The site's identity, as the feeds show it.
pub struct Site<'a> {
    /// The URL the site is published at, without a trailing `/`.
    pub url: &'a str,
    /// Its name.
    pub title: &'a str,
    /// Its tagline.
    pub description: &'a str,
}

/// What a feed lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    /// The collection of its items: `posts`, `micro` or `digital_garden`.
    pub collection: &'static str,
    /// The tag its items have, for a tag's feed.
    pub tag: Option<String>,
    /// The category its items are in, for a micro kind's feed.
    pub category: Option<String>,
}

impl Feed {
    /// The feed that `generator` plans at `url`, or `None` when it plans no feed.
    #[must_use]
    pub fn of(generator: &str, url: &str) -> Option<Self> {
        let stem = || {
            url.rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".xml"))
                .map(str::to_owned)
        };
        let (collection, tag, category) = match generator {
            "blog feed" => ("posts", None, None),
            "tag feed" => ("posts", Some(stem()?), None),
            "micro feed" => ("micro", None, None),
            "micro kind feed" => ("micro", None, Some(stem()?)),
            "garden feed" => ("digital_garden", None, None),
            _ => return None,
        };
        Some(Self {
            collection,
            tag,
            category,
        })
    }

    /// The feed's title: the site's, then the collection's name and the category, each
    /// capitalized, as jekyll-feed names a collection's feed and a category's.
    fn title(&self, site: &Site) -> String {
        let mut title = site.title.to_owned();
        if self.collection != "posts" {
            let _ = write!(title, " | {}", capitalize(self.collection));
        }
        if let Some(category) = &self.category {
            let _ = write!(title, " | {}", capitalize(category));
        }
        title
    }

    fn lists(&self, item: &FeedItem) -> bool {
        item.collection == self.collection
            && self.tag.as_ref().is_none_or(|tag| item.tags.contains(tag))
            && self
                .category
                .as_ref()
                .is_none_or(|category| item.categories.contains(category))
    }
}

/// The XML of `feed`, published at `path`, over the site's `items`.
#[must_use]
pub fn xml(
    config: &FeedsConfig,
    site: &Site,
    feed: &Feed,
    path: &str,
    items: &[FeedItem],
    clock: &SiteClock,
) -> String {
    let mut listed: Vec<&FeedItem> = items.iter().filter(|item| feed.lists(item)).collect();
    listed.sort_by(|a, b| (b.published, &b.url).cmp(&(a.published, &a.url)));
    listed.truncate(config.limit);
    let updated = latest(listed.iter().copied())
        .or_else(|| latest(items.iter()))
        .unwrap_or_else(|| clock.now());

    let url = |path: &str| format!("{}{path}", site.url);
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    out.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\" ");
    if let Some(lang) = &config.lang {
        let _ = write!(out, "xml:lang=\"{lang}\"");
    }
    out.push('>');
    let generator = &config.generator;
    let _ = write!(
        out,
        "<generator uri=\"{}\" version=\"{}\">{}</generator>",
        generator.uri, generator.version, generator.name
    );
    let _ = write!(
        out,
        "<link href=\"{}\" rel=\"self\" type=\"application/atom+xml\" />",
        url(path)
    );
    let _ = write!(
        out,
        "<link href=\"{}\" rel=\"alternate\" type=\"text/html\" ",
        url("/")
    );
    if let Some(lang) = &config.lang {
        let _ = write!(out, "hreflang=\"{lang}\" ");
    }
    out.push_str("/>");
    let _ = write!(out, "<updated>{}</updated>", clock.xmlschema(updated));
    let _ = write!(out, "<id>{}</id>", xml_escape(&url(path)));
    let _ = write!(
        out,
        "<title type=\"html\">{}</title>",
        xml_escape(&smartify(&feed.title(site)))
    );
    let _ = write!(out, "<subtitle>{}</subtitle>", xml_escape(site.description));
    for item in listed {
        entry(&mut out, item, &url, clock);
    }
    out.push_str("</feed>");
    out
}

fn latest<'a>(items: impl Iterator<Item = &'a FeedItem>) -> Option<Timestamp> {
    items.map(|item| item.updated).max()
}

fn entry(out: &mut String, item: &FeedItem, url: &dyn Fn(&str) -> String, clock: &SiteClock) {
    out.push_str("<entry");
    if let Some(lang) = &item.lang {
        let _ = write!(out, " xml:lang=\"{lang}\"");
    }
    out.push('>');
    let title = xml_escape(&normalize_whitespace(&strip_html(&smartify(&item.title))));
    let _ = write!(out, "<title type=\"html\">{title}</title>");
    let _ = write!(
        out,
        "<link href=\"{}\" rel=\"alternate\" type=\"text/html\" title=\"{title}\" />",
        url(&item.url)
    );
    let _ = write!(
        out,
        "<published>{}</published><updated>{}</updated>",
        clock.xmlschema(item.published),
        clock.xmlschema(item.updated)
    );
    let _ = write!(out, "<id>{}</id>", xml_escape(&url(&item.id)));
    let _ = write!(
        out,
        "<content type=\"html\" xml:base=\"{}\"><![CDATA[{}]]></content>",
        xml_escape(&url(&item.url)),
        item.content_html.trim()
    );
    let _ = write!(
        out,
        "<author><name>{}</name></author>",
        xml_escape(item.author.as_deref().unwrap_or_default())
    );
    for term in item.categories.iter().chain(&item.tags) {
        let _ = write!(out, "<category term=\"{}\" />", xml_escape(term));
    }
    // Jekyll's excerpt is never empty (an empty body's is a line break), so neither is the
    // summary element, though its text may be.
    let summary = normalize_whitespace(&strip_html(&item.excerpt_html));
    let _ = write!(
        out,
        "<summary type=\"html\"><![CDATA[{summary}]]></summary>"
    );
    out.push_str("</entry>");
}

/// Jekyll's `smartify`: the text with kramdown's typography (curly quotes, dashes, ellipses), as
/// HTML.
fn smartify(text: &str) -> String {
    let html = to_html(text, None);
    let html = html.trim_end();
    html.strip_prefix("<p>")
        .and_then(|inner| inner.strip_suffix("</p>"))
        .unwrap_or(html)
        .to_owned()
}

/// Liquid's `normalize_whitespace` (`gsub(/\s+/, " ").strip`): each run of ASCII whitespace one
/// space, and the ends trimmed. Ruby's `\s` is ASCII only: a no-break space is kept.
fn normalize_whitespace(text: &str) -> String {
    let ascii = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];
    text.split(ascii)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Liquid's `xml_escape`: `&`, `<`, `>`, `"` and `'` as entities.
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Ruby's `capitalize`: the first character upper case, the rest lower case.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}
