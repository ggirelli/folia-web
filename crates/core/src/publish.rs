//! The contracts a mode hands to the site builder (plan v3, §6.2).
//!
//! So far only what route parity needs (spike 2): the outputs a mode plans, and where each one
//! comes from. Body HTML, feeds and template data join it later.

use crate::content::{DocId, Route};

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
