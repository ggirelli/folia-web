//! The build pipeline (plan v6, §11).
//!
//! 1. Plan every output of the site, and check the route table (`site::plan`).
//! 2. Build the documents: pages with the pages mode, posts with the blog mode, and notes, with
//!    the note graph, with the garden mode, dated with their last commits. A configured slice
//!    ([`Slice`], plan v9, §12.2) names the documents to build, and each must build. Without one,
//!    every document is built, except those Folia cannot build yet: each is skipped, with the
//!    reason, in [`Built::skipped`].
//! 3. Assemble the site index templates show: the site's identity and authors, the build's year,
//!    and the blog's categories.
//! 4. Check every page's template, then render each page. Without a slice, a page whose template
//!    is missing or fails, or whose `author` is not one of the configuration's, is skipped too.
//! 5. Add a redirect stub for each `redirect_from` URL of a page rendered, the files the modes
//!    generated (the note graph), and copy every static file.
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
use folia_core::publish::{EntrySummary, ModeOutput, PageSpec};
use jiff::Timestamp;
use serde_json::json;

use crate::config::Config;
#[cfg(doc)]
use crate::config::Slice;
use crate::output::{self, Content};
use crate::redirects::redirects_json;
use crate::render::Renderer;
use crate::sass::Sass;
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
    /// Without a slice, the documents Folia cannot build yet, each with why; they are not built.
    pub skipped: Vec<Diagnostic>,
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
    let strict = config.slice.is_some();
    let mut built = Built::default();
    let pages = build_documents(root, config, &site, &clock, &mut built.skipped)?;
    let index = site_index(config, &site, &clock)?;

    let renderer = Renderer::new(&root.join(&config.templates_dir));
    let renderable = check_templates(&renderer, &pages.pages, strict, &mut built.skipped)?;
    let renderable = check_authors(renderable, config, strict, &mut built.skipped)?;
    let mut outputs = BTreeMap::new();
    let mut problems = Vec::new();
    let mut shown = BTreeSet::new();
    for page in renderable {
        let content = pages
            .entries
            .iter()
            .find(|entry| Some(&entry.doc) == page.source.as_ref())
            .map_or("", |entry| entry.body_html.as_str());
        match renderer.render(page, content, &index) {
            Ok(html) => {
                built.pages.push(page.route.clone());
                shown.extend(page.source.clone());
                let html = Content::Bytes(html.into_bytes());
                add(&mut outputs, page.route.output_path(), html)
                    .unwrap_or_else(|clash| problems.push(clash));
            }
            Err(problem) if strict => problems.push(problem),
            Err(problem) => built.skipped.push(skipped_page(page, problem)),
        }
    }
    for entry in pages.entries.iter().filter(|e| shown.contains(&e.doc)) {
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
    if !strict {
        for (route, path, text) in stylesheets(root, config, &site)? {
            built.artifacts.push(route);
            add(&mut outputs, path, Content::Bytes(text.into_bytes()))
                .unwrap_or_else(|clash| problems.push(clash));
        }
        // Site-wide files (`site::SITE_FILES`), each planned unless a file of the site takes
        // its URL.
        for planned in site.routes.outputs() {
            if planned.generator == Some("redirects") {
                let json = redirects_json(&config.site.url, &site);
                add(
                    &mut outputs,
                    planned.route.output_path(),
                    Content::Bytes(json.into_bytes()),
                )
                .unwrap_or_else(|clash| problems.push(clash));
                built.artifacts.push(planned.route.clone());
            }
        }
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
    built.entries = pages
        .entries
        .into_iter()
        .filter(|e| shown.contains(&e.doc))
        .collect();
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

/// The pages, posts and notes to build, built by their modes, with the git dates of their
/// sources: the slice's, or, without one, every document, less those a mode reports it cannot
/// build yet, which go to `skipped`.
fn build_documents(
    root: &Path,
    config: &Config,
    site: &Site,
    clock: &SiteClock,
    skipped: &mut Vec<Diagnostic>,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let ids = |mode| -> BTreeSet<String> {
        documents(site, mode)
            .iter()
            .map(|doc| doc.id.to_string())
            .collect()
    };
    let named = |names: &[String]| names.iter().cloned().collect::<BTreeSet<_>>();
    let (mut pages, mut posts, mut notes) = if let Some(slice) = &config.slice {
        (
            named(&slice.pages),
            named(&slice.posts),
            named(&slice.notes),
        )
    } else {
        let mut pages = ids(Mode::Pages);
        pages.retain(|id| !is_sass(id));
        (pages, ids(Mode::Blog), ids(Mode::Garden))
    };
    let dated: Vec<_> = [Mode::Pages, Mode::Blog, Mode::Garden]
        .into_iter()
        .flat_map(|mode| documents(site, mode))
        .map(|doc| &doc.id)
        .filter(|id| {
            [&pages, &posts, &notes]
                .iter()
                .any(|s| s.contains(id.as_str()))
        })
        .collect();
    let commits = LastCommits::read(root, dated).map_err(|e| vec![Diagnostic::site(e)])?;

    let strict = config.slice.is_some();
    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    let only = |set: &BTreeSet<String>| -> Vec<String> { set.iter().cloned().collect() };
    let results = [
        leniently(strict, &mut pages, skipped, |pages| {
            let docs: Vec<SourceDoc> = documents(site, Mode::Pages)
                .iter()
                .filter(|doc| pages.contains(doc.id.as_str()))
                .cloned()
                .collect();
            folia_pages::build(&docs, &commits, clock)
        }),
        leniently(strict, &mut posts, skipped, |posts| {
            let only = only(posts);
            let only: Vec<&str> = only.iter().map(String::as_str).collect();
            folia_blog::build(documents(site, Mode::Blog), &only, &commits, clock)
        }),
        leniently(strict, &mut notes, skipped, |notes| {
            let only = only(notes);
            let only: Vec<&str> = only.iter().map(String::as_str).collect();
            let docs = documents(site, Mode::Garden);
            folia_garden::build(docs, &config.roots.garden, &only, &commits, clock)
        }),
    ];
    for built in results {
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

/// Runs a mode's `build` over the documents `selected`. Unless `strict`, when every problem it
/// reports is one of those documents', they are moved to `skipped` and the rest built again, until
/// the mode builds; a problem of no document selected (of the mode as a whole, or of a document
/// it reads but was not asked to build) fails the build.
fn leniently(
    strict: bool,
    selected: &mut BTreeSet<String>,
    skipped: &mut Vec<Diagnostic>,
    build: impl Fn(&BTreeSet<String>) -> Result<ModeOutput, Vec<Diagnostic>>,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    loop {
        match build(selected) {
            Err(problems) if !strict && skippable(&problems, selected) => {
                for problem in &problems {
                    if let Some(id) = &problem.source {
                        selected.remove(id.as_str());
                    }
                }
                skipped.extend(problems);
            }
            result => return result,
        }
    }
}

/// Whether every problem, at least one, is of a document in `selected`.
fn skippable(problems: &[Diagnostic], selected: &BTreeSet<String>) -> bool {
    !problems.is_empty()
        && problems.iter().all(|problem| {
            problem
                .source
                .as_ref()
                .is_some_and(|id| selected.contains(id.as_str()))
        })
}

/// The pages whose template exists and compiles. Unless `strict`, the others are skipped, each
/// with its template's problem; when `strict`, any fails the build.
fn check_templates<'a>(
    renderer: &Renderer,
    pages: &'a [PageSpec],
    strict: bool,
    skipped: &mut Vec<Diagnostic>,
) -> Result<Vec<&'a PageSpec>, Vec<Diagnostic>> {
    if strict {
        renderer.check(pages).map_err(|e| vec![e])?;
        return Ok(pages.iter().collect());
    }
    let mut by_template: BTreeMap<&str, Vec<&PageSpec>> = BTreeMap::new();
    for page in pages {
        by_template.entry(&page.template).or_default().push(page);
    }
    let mut renderable = Vec::new();
    for (_, users) in by_template {
        let owned: Vec<PageSpec> = users.iter().map(|page| (*page).clone()).collect();
        match renderer.check(&owned) {
            Ok(()) => renderable.extend(users),
            Err(problem) => {
                skipped.extend(users.iter().map(|page| skipped_page(page, problem.clone())));
            }
        }
    }
    renderable.sort_by_key(|page| page.route.output_path());
    Ok(renderable)
}

/// The pages whose `author`, if they have one, is one of the configuration's. Unless `strict`,
/// the others are skipped, each saying so; when `strict`, any fails the build.
fn check_authors<'a>(
    pages: Vec<&'a PageSpec>,
    config: &Config,
    strict: bool,
    skipped: &mut Vec<Diagnostic>,
) -> Result<Vec<&'a PageSpec>, Vec<Diagnostic>> {
    let (known, unknown): (Vec<_>, Vec<_>) = pages
        .into_iter()
        .map(|page| (page, unknown_author(page, config)))
        .partition(|(_, problem)| problem.is_none());
    let problems = unknown.into_iter().filter_map(|(_, problem)| problem);
    if strict {
        let problems: Vec<_> = problems.collect();
        if !problems.is_empty() {
            return Err(problems);
        }
    } else {
        skipped.extend(problems);
    }
    Ok(known.into_iter().map(|(page, _)| page).collect())
}

/// Why `page` cannot be rendered, when its `author` is not one of the site's: its template
/// would show nothing of them.
fn unknown_author(page: &PageSpec, config: &Config) -> Option<Diagnostic> {
    let author = page.context.get("author")?.as_str()?;
    (!config.authors.contains_key(author)).then(|| {
        skipped_page(
            page,
            Diagnostic::site(format!(
                "the author {author} is not one of the configuration's authors"
            )),
        )
    })
}

/// A page that is not built, because of `problem`, named by its source.
fn skipped_page(page: &PageSpec, problem: Diagnostic) -> Diagnostic {
    match &page.source {
        Some(id) => Diagnostic::in_doc(id, problem.message),
        None => problem,
    }
}

/// Whether the source `id` is a Sass stylesheet.
fn is_sass(id: &str) -> bool {
    Path::new(id)
        .extension()
        .is_some_and(|ext| ext == "scss" || ext == "sass")
}

/// A file a build writes: its route, its output path and its text.
type Written = (Route, String, String);

/// The site's published Sass stylesheets, compiled with Dart Sass: for each, its CSS and its
/// source map. Dart Sass is looked for only when the site
/// has a stylesheet.
///
/// # Errors
///
/// Fails when Dart Sass is missing or not the version required, or cannot compile a
/// stylesheet; no other compiler is tried.
fn stylesheets(root: &Path, config: &Config, site: &Site) -> Result<Vec<Written>, Vec<Diagnostic>> {
    let sheets: Vec<(&SourceDoc, Route)> = documents(site, Mode::Pages)
        .iter()
        .filter(|doc| is_sass(doc.id.as_str()) && doc.front.published())
        .filter_map(|doc| {
            site.routes
                .outputs()
                .iter()
                .find(|o| o.generator.is_none() && o.source.as_ref() == Some(&doc.id))
                .map(|o| (doc, o.route.clone()))
        })
        .collect();
    if sheets.is_empty() {
        return Ok(Vec::new());
    }
    let sass = Sass::find().map_err(|e| vec![Diagnostic::site(e)])?;
    let mut compiled = Vec::new();
    let mut problems = Vec::new();
    for (doc, route) in sheets {
        match sass.compile(root, &config.sass, doc.id.as_str(), &doc.body) {
            Ok(sheet) => {
                let css = route.output_path();
                let map = format!("{css}.map");
                match Route::file(&format!("{}.map", route.url())) {
                    Ok(map_route) => compiled.push((map_route, map, sheet.map)),
                    Err(e) => problems.push(Diagnostic::in_doc(&doc.id, e)),
                }
                compiled.push((route, css, sheet.css));
            }
            Err(e) => problems.push(Diagnostic::in_doc(&doc.id, e)),
        }
    }
    if problems.is_empty() {
        Ok(compiled)
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
            // Every field, by its Rust name, and the covers as the author page's script reads
            // them, serialized here rather than by a template.
            let mut shown = serde_json::to_value(author).expect("an author is strings");
            shown["covers_json"] = author.covers_json().into();
            (key.clone(), shown)
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
