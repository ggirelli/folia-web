//! The contracts a mode hands to the site builder (plan v6, §6.2).
//!
//! - [`PlannedOutput`]: the outputs a mode plans, and where each one comes from (spike 2).
//! - [`ModeOutput`]: what a mode builds, so far with only what the first vertical slice uses
//!   (plan v9, §12.2): its entries, with their dates, their pages, the files generated from them
//!   ([`Artifact`], such as the garden's note graph), warnings, and what the site's feeds show
//!   of each entry ([`FeedItem`]).

use jiff::Timestamp;

use crate::content::{Diagnostic, DocId, Modified, Route, SourceDoc, dated_file_name};

/// An output a mode, or the site builder, plans to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedOutput {
    /// Where it is published.
    pub route: Route,
    /// The source document it renders, if any.
    pub source: Option<DocId>,
    /// What generates it, for outputs that are not one document's page (`"pagination"`,
    /// `"tag feed"`, …).
    pub generator: Option<&'static str>,
    /// Old URLs that redirect to it.
    pub redirect_from: Vec<Route>,
}

impl PlannedOutput {
    /// The page of an authored document.
    #[must_use]
    pub fn authored(route: Route, source: DocId) -> Self {
        Self {
            route,
            source: Some(source),
            generator: None,
            redirect_from: Vec::new(),
        }
    }

    /// An output generated from other content.
    #[must_use]
    pub fn generated(route: Route, generator: &'static str) -> Self {
        Self {
            route,
            source: None,
            generator: Some(generator),
            redirect_from: Vec::new(),
        }
    }
}

/// What a mode builds from its documents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeOutput {
    /// One per published document, with its prepared body.
    pub entries: Vec<EntrySummary>,
    /// The pages to render: the entries' own pages, and pages generated from them.
    pub pages: Vec<PageSpec>,
    /// Files generated from the entries, written as they are.
    pub artifacts: Vec<Artifact>,
    /// Problems that do not stop the build, such as a broken wiki-link.
    pub warnings: Vec<Diagnostic>,
    /// What the site's feeds can show of each entry built.
    pub feed: Vec<FeedItem>,
}

/// An entry as a feed shows it, in the terms of Jekyll's documents, which the site's feeds read:
/// the site builder chooses the feeds an item is in, and serializes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedItem {
    /// Its collection: `posts`, `micro` or `digital_garden`.
    pub collection: String,
    /// Its URL (`/blog/2021/08/08/webmentions`).
    pub url: String,
    /// Its ID, Jekyll's ([`FeedItem::jekyll_id`]).
    pub id: String,
    /// Its title, as written.
    pub title: String,
    /// Its date.
    pub published: Timestamp,
    /// Its last modification, or its date.
    pub updated: Timestamp,
    /// Its body's HTML.
    pub content_html: String,
    /// Its excerpt's HTML, of which the feed's summary is made.
    pub excerpt_html: String,
    /// Its author, as written (a key of the site's authors), if any.
    pub author: Option<String>,
    /// Its categories: the singular front matter `category` alone when set, as the feed reads it.
    pub categories: Vec<String>,
    /// Its tags.
    pub tags: Vec<String>,
    /// Its front matter `lang`, if any.
    pub lang: Option<String>,
}

impl FeedItem {
    /// Jekyll's ID of `doc`, published at `url`: the URL's directory joined with its slug, its
    /// front matter `slug` or its file name without extension and without a leading date
    /// (`/likes/2026-07-06-game-ownership` from `2026-07-06-game-ownership.md` is
    /// `/likes/game-ownership`).
    #[must_use]
    pub fn jekyll_id(url: &str, doc: &SourceDoc) -> String {
        let dir = match url.trim_end_matches('/').rfind('/') {
            Some(0) | None => "",
            Some(i) => &url[..i],
        };
        let slug = doc
            .front
            .string("slug")
            .ok()
            .flatten()
            .or_else(|| {
                dated_file_name(doc.id.file_name())
                    .ok()
                    .flatten()
                    .map(|(_, slug)| slug)
            })
            .unwrap_or_else(|| doc.id.stem().to_owned());
        format!("{dir}/{slug}")
    }
}

/// A file a mode generates, not rendered with a template, such as the garden's note graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// Where it is published.
    pub route: Route,
    /// Its text, written as it is: the mode serializes it, so that it decides, for example,
    /// the order of a JSON object's keys.
    pub body: String,
}

/// A published document, as the site builder sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrySummary {
    /// The document.
    pub doc: DocId,
    /// Where its page is published.
    pub route: Route,
    /// When it was published, for a dated document.
    pub published: Option<Timestamp>,
    /// When it was last modified, and how Folia knows (plan v7, §7.3).
    pub modified: Option<Modified>,
    /// Its body as HTML, with links resolved.
    pub body_html: String,
    /// Old URLs that redirect to its page.
    pub redirect_from: Vec<Route>,
}

/// The template of a page without a layout: the page is written as its content alone, as Jekyll
/// writes such a page. The renderer provides it; no site template can take the name.
pub const NO_LAYOUT_TEMPLATE: &str = "folia:no-layout";

/// A page to render with a template.
#[derive(Clone, Debug, PartialEq)]
pub struct PageSpec {
    /// Where it is published.
    pub route: Route,
    /// The template, a path relative to the site's template directory (`page.html`).
    pub template: String,
    /// The entry whose page this is, whose body the template shows as `content`; `None` for a
    /// generated page.
    pub source: Option<DocId>,
    /// What the template shows of the page, as `page`.
    pub context: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{FrontMatter, Mode};

    fn doc(path: &str, front: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new(path),
            mode: Mode::Micro,
            front: FrontMatter::parse(front).unwrap(),
            body: String::new(),
        }
    }

    #[test]
    fn jekyll_s_id_is_the_url_s_directory_and_the_slug() {
        let id = |url: &str, path: &str, front: &str| FeedItem::jekyll_id(url, &doc(path, front));
        assert_eq!(
            id(
                "/likes/2026-07-06-game-ownership",
                "_micro/likes/2026-07-06-game-ownership.md",
                ""
            ),
            "/likes/game-ownership"
        );
        assert_eq!(
            id(
                "/blog/2021/08/08/hello",
                "_posts/2021-08-08-hello.md",
                "slug: other"
            ),
            "/blog/2021/08/08/other"
        );
        assert_eq!(
            id("/dg/coding/rust", "_digital_garden/coding/rust.md", ""),
            "/dg/coding/rust"
        );
        assert_eq!(
            id("/dg/coding/", "_digital_garden/coding/index.md", ""),
            "/dg/index"
        );
        assert_eq!(id("/about", "pages/about.md", ""), "/about");
    }
}
