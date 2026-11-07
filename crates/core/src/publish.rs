//! The contracts a mode hands to the site builder (plan v6, §6.2).
//!
//! - [`PlannedOutput`]: the outputs a mode plans, and where each one comes from (spike 2).
//! - [`ModeOutput`]: what a mode builds, so far with only what the first vertical slice uses
//!   (plan v9, §12.2): its entries, with their dates, their pages, the files generated from them
//!   ([`Artifact`], such as the garden's note graph), and warnings. Feeds join it with the step
//!   that needs them.

use jiff::Timestamp;

use crate::content::{Diagnostic, DocId, Modified, Route};

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
