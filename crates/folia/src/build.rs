//! The build pipeline (plan v6, §11), so far for the vertical slice (plan v9, §12.2).
//!
//! 1. Plan every output of the site, and check the route table (`site::plan`).
//! 2. Build the slice's pages ([`Slice::pages`]) with the pages mode, its posts
//!    ([`Slice::posts`]) with the blog mode, and its notes ([`Slice::notes`]), with the note graph,
//!    with the garden mode, dated with their last commits. The site's configuration names them.
//! 3. Assemble the site index templates show: the site's identity and authors, the build's year,
//!    and the blog's categories.
//! 4. Check every page's template, then render each page.
//! 5. Add a redirect stub for each of their `redirect_from` URLs, the files the modes generated
//!    (the note graph), and copy every static file.
//! 6. Write it all to the destination (`output`).
//!
//! Every other planned output is not built yet; [`Built::not_built`] counts them. Nothing is
//! written unless every step succeeds; warnings, such as a broken wiki-link, do not stop the
//! build, and are returned in [`Built::warnings`].

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use folia_core::content::{Diagnostic, LastCommits, Mode, Route, SiteClock, SourceDoc};
use folia_core::publish::{EntrySummary, ModeOutput};
use jiff::Timestamp;
use serde_json::json;

use crate::config::Config;
#[cfg(doc)]
use crate::config::Slice;
use crate::output::{self, Content};
use crate::render::Renderer;
use crate::site::{Site, plan, redirect_stub};

/// What a build wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// Pages rendered with a template.
    pub pages: Vec<Route>,
    /// Redirect stubs.
    pub stubs: Vec<Route>,
    /// Files the modes generated, such as the note graph.
    pub artifacts: Vec<Route>,
    /// Static files copied.
    pub statics: usize,
    /// Planned outputs not built yet.
    pub not_built: usize,
    /// The entries of the documents built, with their dates.
    pub entries: Vec<EntrySummary>,
    /// Problems that did not stop the build.
    pub warnings: Vec<Diagnostic>,
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
    let pages = build_documents(root, config, &site, &clock)?;
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
                let html = Content::Bytes(html.into_bytes());
                add(&mut outputs, page.route.output_path(), html)
                    .unwrap_or_else(|clash| problems.push(clash));
            }
            Err(problem) => problems.push(problem),
        }
    }
    for entry in &pages.entries {
        let to = format!("{}{}", config.site.url, entry.route.url());
        for from in &entry.redirect_from {
            built.stubs.push(from.clone());
            let stub = Content::Bytes(redirect_stub(&to).into_bytes());
            add(&mut outputs, from.output_path(), stub)
                .unwrap_or_else(|clash| problems.push(clash));
        }
    }
    for artifact in pages.artifacts {
        let body = Content::Bytes(artifact.body.into_bytes());
        add(&mut outputs, artifact.route.output_path(), body)
            .unwrap_or_else(|clash| problems.push(clash));
        built.artifacts.push(artifact.route);
    }
    for id in &site.discovery.statics {
        let copy = Content::Copy(root.join(id.as_str()));
        add(&mut outputs, id.to_string(), copy).unwrap_or_else(|clash| problems.push(clash));
    }
    if !problems.is_empty() {
        return Err(problems);
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
    built.entries = pages.entries;
    built.warnings = pages.warnings;
    output::write(dest, root, &outputs).map_err(|e| vec![e])?;
    Ok(built)
}

/// Adds the output `content` at `path`, unless another output is already there. The route table
/// keeps outputs apart (`site::plan`); this keeps a clash it missed from overwriting one.
fn add(
    outputs: &mut BTreeMap<String, Content>,
    path: String,
    content: Content,
) -> Result<(), Diagnostic> {
    match outputs.entry(path) {
        Entry::Vacant(free) => {
            free.insert(content);
            Ok(())
        }
        Entry::Occupied(taken) => Err(Diagnostic::site(format!("{} is built twice", taken.key()))),
    }
}

/// The slice's pages, posts and notes, built by their modes, with the git dates of their
/// sources.
fn build_documents(
    root: &Path,
    config: &Config,
    site: &Site,
    clock: &SiteClock,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let slice = &config.slice;
    let only_posts: Vec<&str> = slice.posts.iter().map(String::as_str).collect();
    let only_notes: Vec<&str> = slice.notes.iter().map(String::as_str).collect();
    let pages: Vec<SourceDoc> = documents(site, Mode::Pages)
        .iter()
        .filter(|doc| slice.pages.iter().any(|page| page == doc.id.as_str()))
        .cloned()
        .collect();
    let posts = documents(site, Mode::Blog);
    let notes = documents(site, Mode::Garden);
    let dated = pages
        .iter()
        .map(|doc| &doc.id)
        .chain(
            posts
                .iter()
                .map(|doc| &doc.id)
                .filter(|id| only_posts.contains(&id.as_str())),
        )
        .chain(
            notes
                .iter()
                .map(|doc| &doc.id)
                .filter(|id| only_notes.contains(&id.as_str())),
        );
    let commits = LastCommits::read(root, dated).map_err(|e| vec![Diagnostic::site(e)])?;

    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    for built in [
        folia_pages::build(&pages, &commits, clock),
        folia_blog::build(posts, &only_posts, &commits, clock),
        folia_garden::build(notes, &config.roots.garden, &only_notes, &commits, clock),
    ] {
        match built {
            Ok(built) => {
                output.entries.extend(built.entries);
                output.pages.extend(built.pages);
                output.artifacts.extend(built.artifacts);
                output.warnings.extend(built.warnings);
            }
            Err(more) => problems.extend(more),
        }
    }
    if problems.is_empty() {
        Ok(output)
    } else {
        Err(problems)
    }
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
    let authors: serde_json::Map<String, serde_json::Value> = config
        .authors
        .iter()
        .map(|(key, author)| {
            let author = json!({
                "name": author.name,
                "display_name": author.display_name,
                "avatar": author.avatar,
                "web": author.web,
                "description": author.description,
            });
            (key.clone(), author)
        })
        .collect();
    Ok(json!({
        "url": config.site.url,
        "title": config.site.title,
        "description": config.site.description,
        "year": clock.year(),
        "categories": categories,
        "authors": authors,
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
