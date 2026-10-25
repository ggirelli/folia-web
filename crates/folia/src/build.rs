//! The build pipeline (plan v6, §11), so far for the first vertical slice (§12.2 step 1).
//!
//! 1. Plan every output of the site, and check the route table (`site::plan`).
//! 2. Build the slice's pages ([`SLICE_PAGES`]) with the pages mode, dated by their last commit.
//! 3. Assemble the site index templates show: the site's identity, the build's year, and the
//!    blog's categories.
//! 4. Check every page's template, then render each page.
//! 5. Add a redirect stub for each of their `redirect_from` URLs, and copy every static file.
//! 6. Write it all to the destination (`output`).
//!
//! Every other planned output is not built yet; [`Built::not_built`] counts them. Nothing is
//! written unless every step succeeds.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use folia_core::content::{Diagnostic, LastCommits, Mode, Route, SiteClock, SourceDoc};
use folia_core::publish::ModeOutput;
use jiff::Timestamp;
use serde_json::json;

use crate::config::Config;
use crate::output::{self, Content};
use crate::render::Renderer;
use crate::site::{Site, plan, redirect_stub};

/// The pages the first vertical slice builds.
pub const SLICE_PAGES: [&str; 1] = ["pages/profile.md"];

/// What a build wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// Pages rendered with a template.
    pub pages: Vec<Route>,
    /// Redirect stubs.
    pub stubs: Vec<Route>,
    /// Static files copied.
    pub statics: usize,
    /// Planned outputs not built yet.
    pub not_built: usize,
}

/// Builds the site at `root` into `dest`, as of the build time `now`.
///
/// # Errors
///
/// Returns every problem found while planning, building or rendering, or the one that stopped
/// the writing; the destination is then left as it was.
pub fn build(
    root: &Path,
    config: &Config,
    dest: &Path,
    now: Timestamp,
) -> Result<Built, Vec<Diagnostic>> {
    let site = plan(root, config, now)?;
    let clock = SiteClock::new(&config.time_zone, now).map_err(|e| vec![Diagnostic::site(e)])?;
    let pages = build_pages(root, &site, &clock)?;
    let index = site_index(config, &site, &clock)?;

    let renderer = Renderer::new(&root.join(&config.templates_dir));
    renderer.check(&pages.pages).map_err(|e| vec![e])?;
    let mut outputs = BTreeMap::new();
    let mut built = Built::default();
    let mut problems = Vec::new();
    for page in &pages.pages {
        let content = pages
            .entries
            .iter()
            .find(|entry| Some(&entry.doc) == page.source.as_ref())
            .map_or("", |entry| entry.body_html.as_str());
        match renderer.render(page, content, &index) {
            Ok(html) => {
                built.pages.push(page.route.clone());
                outputs.insert(page.route.output_path(), Content::Bytes(html.into_bytes()));
            }
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    for entry in &pages.entries {
        let to = format!("{}{}", config.site.url, entry.route.url());
        for from in &entry.redirect_from {
            built.stubs.push(from.clone());
            outputs.insert(
                from.output_path(),
                Content::Bytes(redirect_stub(&to).into_bytes()),
            );
        }
    }
    for id in &site.discovery.statics {
        outputs.insert(id.to_string(), Content::Copy(root.join(id.as_str())));
    }
    built.statics = site.discovery.statics.len();

    let planned: BTreeSet<String> = site
        .routes
        .outputs()
        .iter()
        .map(|o| o.route.output_path())
        .collect();
    let unplanned: Vec<Diagnostic> = outputs
        .keys()
        .filter(|path| !planned.contains(*path))
        .map(|path| Diagnostic::site(format!("{path} is built but not planned")))
        .collect();
    if !unplanned.is_empty() {
        return Err(unplanned);
    }
    built.not_built = planned.len() - outputs.len();
    output::write(dest, root, &outputs).map_err(|e| vec![e])?;
    Ok(built)
}

/// The slice's pages, built by the pages mode.
fn build_pages(root: &Path, site: &Site, clock: &SiteClock) -> Result<ModeOutput, Vec<Diagnostic>> {
    let docs: Vec<SourceDoc> = documents(site, Mode::Pages)
        .iter()
        .filter(|doc| SLICE_PAGES.contains(&doc.id.as_str()))
        .cloned()
        .collect();
    let commits = LastCommits::read(root, docs.iter().map(|doc| &doc.id))
        .map_err(|e| vec![Diagnostic::site(e)])?;
    folia_pages::build(&docs, &commits, clock)
}

/// What every template sees as `site` (plan v6, §6.3).
fn site_index(
    config: &Config,
    site: &Site,
    clock: &SiteClock,
) -> Result<serde_json::Value, Vec<Diagnostic>> {
    let categories: Vec<_> = folia_blog::category_counts(documents(site, Mode::Blog), clock)?
        .into_iter()
        .map(|(name, count)| json!({"name": name, "anchor": anchor(&name), "count": count}))
        .collect();
    Ok(json!({
        "url": config.site.url,
        "title": config.site.title,
        "description": config.site.description,
        "year": clock.year(),
        "categories": categories,
    }))
}

fn documents(site: &Site, mode: Mode) -> &[SourceDoc] {
    site.documents.get(&mode).map_or(&[], Vec::as_slice)
}

/// A category's fragment on the categories page, as the site's templates made it with Liquid
/// (`url_escape | strip | replace: ' ', '-'`): bytes other than letters, digits and
/// ``-._~!$&'()*+,;=:@/?`` are percent-encoded, so no whitespace survives to be stripped or
/// replaced.
fn anchor(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests;
