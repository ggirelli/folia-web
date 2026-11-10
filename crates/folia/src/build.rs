//! The build pipeline (plan v6, §11).
//!
//! 1. Plan every output of the site, and check the route table (`site::plan`).
//! 2. Build the documents: pages with the pages mode, posts with the blog mode, notes, with
//!    the note graph, with the garden mode, and micro entries with the microblog mode, dated with their last commits. A configured slice
//!    ([`Slice`], plan v9, §12.2) names the documents to build, and each must build. Without one,
//!    every document is built, except those Folia cannot build yet: each is skipped, with the
//!    reason, in [`Built::skipped`]. Only documents Folia does not support are skipped; a
//!    document that is supported but fails (a template, an author) fails the build.
//! 3. Assemble the site index templates show: the site's identity and authors, the build's year,
//!    and the blog's categories.
//! 4. Check every page's template, then render each page. A page whose template is missing or
//!    fails, or whose `author` is not one of the configuration's, fails the build, whatever the
//!    configuration: it is never skipped, and every such problem is reported.
//! 5. Add a redirect stub for each `redirect_from` URL of a page rendered, the files the modes
//!    generated (the note graph), the sitemap (module `sitemap`), the feeds the modes planned
//!    when the configuration has `[feeds]` (module `feed`), and copy every static file.
//! 6. Write it all to the destination (`output`).
//!
//! Every other planned output is not built yet; [`Built::not_built`] counts them. Nothing is
//! written unless every step succeeds; warnings, such as a broken wiki-link, do not stop the
//! build, and are returned in [`Built::warnings`].

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use folia_core::content::{
    Diagnostic, DocId, LastCommits, Mode, Preview, Route, SiteClock, SourceDoc,
};
use folia_core::publish::{EntrySummary, FeedItem, ModeOutput, PageSpec};
use jiff::Timestamp;
use serde_json::json;

use crate::config::Config;
#[cfg(doc)]
use crate::config::Slice;
use crate::feed::{self, Feed};
use crate::output::{self, Content, Writer};
use crate::redirects::redirects_json;
use crate::render::Renderer;
use crate::sass::Sass;
use crate::search;
use crate::site::{Site, plan_for, redirect_stub};
use crate::sitemap;

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
    /// What the build added to the normal one; not the normal build when set, and not for
    /// publishing.
    pub preview: Preview,
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
    build_as(root, config, dest, now, Writer::Build, Preview::default())
}

/// The sentence that says a build with `preview` is not for publishing, led by a space so it can
/// end a summary line; `None` for the normal build.
#[must_use]
pub fn preview_notice(preview: Preview) -> Option<String> {
    preview
        .label()
        .map(|label| format!(" PREVIEW BUILD ({label}): not for publishing."))
}

/// Builds the site as [`build`] does, for the command `writer`, which the destination's record
/// names, and with what `preview` adds to the normal build.
///
/// # Errors
///
/// As [`build`]; and when `writer` is `folia build` and the destination holds a build of the
/// other kind, normal or preview.
pub fn build_as(
    root: &Path,
    config: &Config,
    dest: &Path,
    now: Timestamp,
    writer: Writer,
    preview: Preview,
) -> Result<Built, Vec<Diagnostic>> {
    let site = plan_for(root, config, now, preview)?;
    let clock = SiteClock::new(&config.time_zone, now)
        .map_err(|e| vec![Diagnostic::site(e)])?
        .with_future(preview.future)
        .with_unpublished(preview.unpublished);
    let strict = config.slice.is_some();
    let mut built = Built {
        preview,
        ..Built::default()
    };
    let pages = build_documents(root, config, &site, &clock, &mut built.skipped)?;
    let index = site_index(config, &site, &clock)?;

    let renderer = Renderer::new(&root.join(&config.templates_dir));
    let mut problems = Vec::new();
    let renderable = check_templates(&renderer, &pages.pages, &mut problems);
    let renderable = check_authors(renderable, config, &mut problems);
    let mut outputs = BTreeMap::new();
    let mut shown = BTreeSet::new();
    let mut listings = BTreeMap::new();
    for page in renderable {
        let entry = pages
            .entries
            .iter()
            .find(|entry| Some(&entry.doc) == page.source.as_ref());
        let content = entry.map_or("", |entry| entry.body_html.as_str());
        match renderer.render(page, content, &index) {
            Ok(html) => {
                if let (true, Some(block)) = (content.is_empty(), search_block(config)) {
                    match renderer.render_block(page, content, &index, block) {
                        Ok(Some(text)) => {
                            // The templates' autoescaping writes `'` and `/` as entities, which
                            // Jekyll's Liquid wrote as they are, and search keeps entities.
                            let text = text.replace("&#x27;", "'").replace("&#x2f;", "/");
                            listings.insert(page.route.url().to_owned(), text);
                        }
                        Ok(None) => {}
                        Err(problem) => problems.push(problem),
                    }
                }
                built.pages.push(page.route.clone());
                shown.extend(page.source.clone());
                let html = Content::Bytes(html.into_bytes());
                add(&mut outputs, page.route.output_path(), html)
                    .unwrap_or_else(|clash| problems.push(clash));
            }
            Err(problem) => problems.push(skipped_page(page, problem)),
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
    for artifact in &pages.artifacts {
        let body = Content::Bytes(artifact.body.clone().into_bytes());
        add(&mut outputs, artifact.route.output_path(), body)
            .unwrap_or_else(|clash| problems.push(clash));
        built.artifacts.push(artifact.route.clone());
    }
    if !strict {
        let generated = Generated {
            root,
            config,
            site: &site,
            pages: &pages,
            listings: &listings,
            clock: &clock,
        };
        generated.write(&mut built, &mut outputs, &mut problems)?;
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
    check_planned(&outputs, &planned)?;
    built.not_built = planned.len() - outputs.len();
    built.entries = pages
        .entries
        .into_iter()
        .filter(|e| shown.contains(&e.doc))
        .collect();
    built.warnings = pages.warnings;
    output::write_as(dest, root, &outputs, writer, preview).map_err(|e| vec![e])?;
    Ok(built)
}

/// Fails with each output that was built without being planned.
fn check_planned(
    outputs: &BTreeMap<String, Content>,
    planned: &BTreeSet<String>,
) -> Result<(), Vec<Diagnostic>> {
    let unplanned: Vec<Diagnostic> = outputs
        .keys()
        .filter(|path| !planned.contains(*path))
        .map(|path| Diagnostic::site(format!("{path} is built but not planned")))
        .collect();
    if unplanned.is_empty() {
        Ok(())
    } else {
        Err(unplanned)
    }
}

/// Adds the output `content` at `path`, unless another output is already there. The route table
/// keeps outputs apart (`site::plan`); this keeps a clash it missed from overwriting one.
/// What a whole-site build generates after rendering, from what it built.
struct Generated<'a> {
    root: &'a Path,
    config: &'a Config,
    site: &'a Site,
    pages: &'a ModeOutput,
    /// The content block of each page with no entry, by URL (`[search] content-block`).
    listings: &'a BTreeMap<String, String>,
    clock: &'a SiteClock,
}

impl Generated<'_> {
    /// Adds to `outputs` the stylesheets, the serialized files (sitemap, `redirects.json`,
    /// feeds), then the search script, which reads them; a file built twice is a problem.
    fn write(
        &self,
        built: &mut Built,
        outputs: &mut BTreeMap<String, Content>,
        problems: &mut Vec<Diagnostic>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut generated = Vec::new();
        for (route, path, text) in stylesheets(self.root, self.config, self.site, self.clock)? {
            generated.push((route, path, text));
        }
        let (config, site, clock) = (self.config, self.site, self.clock);
        let urls = sitemap_urls(self.root, site, self.pages, built).map_err(|e| vec![e])?;
        for (route, text) in serialized(config, site, &self.pages.feed, &urls, built, clock) {
            generated.push((route.clone(), route.output_path(), text));
        }
        for (route, path, text) in generated {
            add(outputs, path, Content::Bytes(text.into_bytes()))
                .unwrap_or_else(|clash| problems.push(clash));
            built.artifacts.push(route);
        }
        if config.search.is_some() {
            let script = self.search_script(built, outputs);
            let (route, js) = script.map_err(|e| vec![e])?;
            add(
                outputs,
                route.output_path(),
                Content::Bytes(js.into_bytes()),
            )
            .unwrap_or_else(|clash| problems.push(clash));
            built.artifacts.push(route);
        }
        Ok(())
    }
}

impl Generated<'_> {
    /// The search script (module `search`), at its claimed source's route: the records of the pages
    /// rendered (`built`), with their content as written to `outputs` unless they are entries, of
    /// the stubs and `redirects.json`, and of the collections' entries rendered.
    fn search_script(
        &self,
        built: &Built,
        outputs: &BTreeMap<String, Content>,
    ) -> Result<(Route, String), Diagnostic> {
        let Self {
            root,
            config,
            site,
            pages,
            listings,
            clock,
        } = *self;
        let Some(search) = &config.search else {
            return Err(Diagnostic::site("no [search] is configured"));
        };
        let source = DocId::new(&search.source);
        let route = site
            .routes
            .outputs()
            .iter()
            .find(|o| o.generator.is_none() && o.source.as_ref() == Some(&source))
            .map(|o| o.route.clone())
            .ok_or_else(|| {
                Diagnostic::site(format!("search: {source} is not a page of the site"))
            })?;
        let fragment = std::fs::read_to_string(root.join(&search.fragment)).map_err(|e| {
            Diagnostic::site(format!("search: cannot read {}: {e}", search.fragment))
        })?;
        let written = |route: &Route| match outputs.get(&route.output_path()) {
            Some(Content::Bytes(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
            _ => String::new(),
        };
        let rendered: BTreeSet<&str> = built.pages.iter().map(Route::url).collect();
        let items: Vec<FeedItem> = pages
            .feed
            .iter()
            .filter(|item| rendered.contains(item.url.as_str()))
            .cloned()
            .collect();
        let in_collection: BTreeSet<&str> = items.iter().map(|item| item.url.as_str()).collect();
        let mut listed: Vec<search::Page> = built
            .pages
            .iter()
            .filter(|route| !in_collection.contains(route.url()))
            .filter_map(|route| pages.pages.iter().find(|page| &page.route == route))
            .map(|page| search::Page {
                url: page.route.url().to_owned(),
                title: page.context["title"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                content: {
                    let entry = pages
                        .entries
                        .iter()
                        .find(|entry| Some(&entry.doc) == page.source.as_ref());
                    match entry {
                        Some(entry) if !entry.body_html.is_empty() => entry.body_html.clone(),
                        // A listing has no body of its own: its template's content block, else
                        // (a claimed page, with no block named) nothing, or the page as rendered.
                        entry => listings.get(page.route.url()).cloned().unwrap_or_else(|| {
                            if entry.is_some() {
                                String::new()
                            } else {
                                written(&page.route)
                            }
                        }),
                    }
                },
            })
            .collect();
        let redirects = Route::file("/redirects.json").map_err(Diagnostic::site)?;
        let files = built.stubs.iter().chain(
            built
                .artifacts
                .iter()
                .filter(|artifact| **artifact == redirects),
        );
        listed.extend(files.map(|route| search::Page {
            url: route.url().to_owned(),
            title: String::new(),
            // A redirect stub has no text of its own; `redirects.json` is its content.
            content: if built.stubs.contains(route) {
                String::new()
            } else {
                written(route)
            },
        }));
        let js = search::script(&config.site.url, &listed, &items, &fragment, clock);
        Ok((route, js))
    }
}

/// What the sitemap lists (module `sitemap`): each page rendered (`built`) but the 404 page, by
/// URL, dated by its entry or its source's last commit, then each static file of the sitemap's
/// extensions, by path, dated by its last commit.
fn sitemap_urls(
    root: &Path,
    site: &Site,
    pages: &ModeOutput,
    built: &Built,
) -> Result<Vec<sitemap::Url>, Diagnostic> {
    let source = |route: &Route| {
        pages
            .pages
            .iter()
            .find(|page| &page.route == route)
            .and_then(|page| page.source.as_ref())
    };
    let entry = |id: &DocId| pages.entries.iter().find(|entry| &entry.doc == id);
    let statics: Vec<&DocId> = site
        .discovery
        .statics
        .iter()
        .filter(|id| sitemap::STATIC_EXTENSIONS.contains(&id.extension()))
        .filter(|id| id.file_name() != "404.html")
        .collect();
    let claimed = built
        .pages
        .iter()
        .filter_map(source)
        .filter(|id| entry(id).is_none());
    let commits = LastCommits::read(root, claimed.chain(statics.iter().copied()))
        .map_err(Diagnostic::site)?;
    let mut urls: Vec<sitemap::Url> = built
        .pages
        .iter()
        .filter(|route| route.output_path() != "404.html")
        .map(|route| sitemap::Url {
            url: route.url().to_owned(),
            lastmod: source(route).and_then(|id| match entry(id) {
                Some(entry) => entry.modified.as_ref().map(|m| m.at).or(entry.published),
                None => commits.get(id),
            }),
        })
        .collect();
    urls.sort_by(|a, b| a.url.cmp(&b.url));
    urls.extend(statics.into_iter().map(|id| sitemap::Url {
        url: format!("/{id}"),
        lastmod: commits.get(id),
    }));
    Ok(urls)
}

/// The planned outputs Folia serializes, with their text: the site-wide files
/// (`site::SITE_FILES`), each planned unless a file of the site takes its URL: the sitemap of
/// `urls` and `redirects.json`; and, with `[feeds]`, each feed the modes planned. A feed lists
/// only the items whose pages were rendered (`built`), so a page skipped is in no feed.
fn serialized(
    config: &Config,
    site: &Site,
    items: &[FeedItem],
    urls: &[sitemap::Url],
    built: &Built,
    clock: &SiteClock,
) -> Vec<(Route, String)> {
    let identity = feed::Site {
        url: &config.site.url,
        title: &config.site.title,
        description: &config.site.description,
    };
    let rendered: BTreeSet<&str> = built.pages.iter().map(Route::url).collect();
    let items: Vec<FeedItem> = items
        .iter()
        .filter(|item| rendered.contains(item.url.as_str()))
        .cloned()
        .collect();
    let combined = config
        .feeds
        .as_ref()
        .and_then(|feeds| feeds.combined.as_ref())
        .and_then(|combined| {
            let source = DocId::new(&combined.source);
            let planned = site
                .routes
                .outputs()
                .iter()
                .find(|o| o.generator.is_none() && o.source.as_ref() == Some(&source))?;
            let feeds = config.feeds.as_ref()?;
            let feed = Feed::combined(&combined.collections);
            let url = planned.route.url();
            let text = feed::xml(feeds, &identity, &feed, url, &items, clock);
            Some((planned.route.clone(), text))
        });
    let generated = site.routes.outputs().iter().filter_map(|planned| {
        let (url, generator) = (planned.route.url(), planned.generator?);
        let text = if generator == "redirects" {
            redirects_json(&config.site.url, site)
        } else if generator == "sitemap" {
            sitemap::xml(&config.site.url, urls, clock)
        } else {
            let feeds = config.feeds.as_ref()?;
            feed::xml(
                feeds,
                &identity,
                &Feed::of(generator, url)?,
                url,
                &items,
                clock,
            )
        };
        Some((planned.route.clone(), text))
    });
    generated.chain(combined).collect()
}

/// The template block search indexes a listing by, if the site names one.
fn search_block(config: &Config) -> Option<&str> {
    config.search.as_ref()?.content_block.as_deref()
}

/// The page the combined feed takes the place of, if the site has one.
fn combined_source(config: &Config) -> Option<&String> {
    config.feeds.as_ref()?.combined.as_ref().map(|c| &c.source)
}

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

/// The documents each mode builds: the slice's, or, without one, all of its documents but the
/// pages another mode claims (the blog's index, the search script, the combined feed) and the
/// stylesheets.
fn selected(
    config: &Config,
    site: &Site,
) -> (
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
) {
    let ids = |mode| -> BTreeSet<String> {
        documents(site, mode)
            .iter()
            .map(|doc| doc.id.to_string())
            .collect()
    };
    let named = |names: &[String]| names.iter().cloned().collect::<BTreeSet<_>>();
    if let Some(slice) = &config.slice {
        (
            named(&slice.pages),
            named(&slice.posts),
            named(&slice.notes),
            named(&slice.micro),
        )
    } else {
        let mut pages = ids(Mode::Pages);
        // The blog builds its claimed index's pages, and search its script.
        let claimed = [
            config.blog.index.as_ref().map(|index| &index.source),
            config.search.as_ref().map(|search| &search.source),
            combined_source(config),
        ];
        pages.retain(|id| !is_sass(id) && !claimed.contains(&Some(id)));
        (pages, ids(Mode::Blog), ids(Mode::Garden), ids(Mode::Micro))
    }
}

/// The pages, posts, notes and micro entries to build, built by their modes, with the git dates of their
/// sources: the slice's, or, without one, every document, less those a mode reports it cannot
/// build yet, which go to `skipped`.
fn build_documents(
    root: &Path,
    config: &Config,
    site: &Site,
    clock: &SiteClock,
    skipped: &mut Vec<Diagnostic>,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let (mut pages, mut posts, mut notes, mut micro) = selected(config, site);
    let dated: Vec<_> = [Mode::Pages, Mode::Blog, Mode::Garden, Mode::Micro]
        .into_iter()
        .flat_map(|mode| documents(site, mode))
        .map(|doc| &doc.id)
        .filter(|id| {
            [&pages, &posts, &notes, &micro]
                .iter()
                .any(|s| s.contains(id.as_str()))
        })
        .collect();
    let commits = LastCommits::read(root, dated).map_err(|e| vec![Diagnostic::site(e)])?;

    let strict = config.slice.is_some();
    let claimed: BTreeMap<String, String> = config
        .micro
        .listings
        .iter()
        .map(|listing| (listing.source.clone(), listing.template.clone()))
        .chain(
            config
                .blog
                .grouped()
                .map(|(page, _)| (page.source.clone(), page.template.clone())),
        )
        .collect();
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
            folia_pages::build(&docs, &commits, clock, &claimed)
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
            let author = config.garden.author.as_deref();
            folia_garden::build(docs, &config.roots.garden, &only, author, &commits, clock)
        }),
        leniently(strict, &mut micro, skipped, |micro| {
            let only = only(micro);
            let only: Vec<&str> = only.iter().map(String::as_str).collect();
            let docs = documents(site, Mode::Micro);
            let author = config.micro.author.as_deref();
            folia_micro::build(docs, &config.roots.micro, &only, author, &commits, clock)
        }),
    ];
    for built in results {
        match built {
            Ok(built) => {
                output.entries.extend(built.entries);
                output.pages.extend(built.pages);
                output.artifacts.extend(built.artifacts);
                output.warnings.extend(built.warnings);
                output.feed.extend(built.feed);
            }
            Err(more) => problems.extend(more),
        }
    }
    list_micro(config, site, clock, &mut output.pages, &mut problems);
    if config.slice.is_none() {
        list_blog(config, site, clock, &mut output.pages, &mut problems);
    }
    if problems.is_empty() {
        Ok(output)
    } else {
        Err(problems)
    }
}

/// Adds the blog's listing pages to `pages`: its index's, when the configuration claims one, and
/// its category archives, when it names their template; and gives its claimed categories and
/// tags pages, among `pages`, their posts by category or tag as `groups`.
fn list_blog(
    config: &Config,
    site: &Site,
    clock: &SiteClock,
    pages: &mut Vec<PageSpec>,
    problems: &mut Vec<Diagnostic>,
) {
    let index = match &config.blog.index {
        Some(claim) => {
            let found = documents(site, Mode::Pages)
                .iter()
                .find(|doc| doc.id.as_str() == claim.source);
            let Some(doc) = found else {
                problems.push(Diagnostic::site(format!(
                    "the blog's index {} is not one of the site's pages",
                    claim.source
                )));
                return;
            };
            Some((doc, claim.template.as_str()))
        }
        None => None,
    };
    let archive = config.blog.archive_template.as_deref();
    match folia_blog::listings(documents(site, Mode::Blog), index, archive, clock) {
        Ok(more) => pages.extend(more),
        Err(more) => problems.extend(more),
    }
    for (claim, by) in config.blog.grouped() {
        let Some(page) = pages.iter_mut().find(|page| {
            page.source
                .as_ref()
                .is_some_and(|id| id.as_str() == claim.source)
        }) else {
            continue;
        };
        match folia_blog::groups(documents(site, Mode::Blog), by, clock) {
            Ok(groups) => page.context["groups"] = groups,
            Err(more) => problems.extend(more),
        }
    }
}

/// Gives each micro listing page built, among `pages`, the entries it lists as `items`.
fn list_micro(
    config: &Config,
    site: &Site,
    clock: &SiteClock,
    pages: &mut [PageSpec],
    problems: &mut Vec<Diagnostic>,
) {
    for listing in &config.micro.listings {
        let Some(page) = pages.iter_mut().find(|page| {
            page.source
                .as_ref()
                .is_some_and(|id| id.as_str() == listing.source)
        }) else {
            continue;
        };
        let docs = documents(site, Mode::Micro);
        match folia_micro::listing(docs, &config.roots.micro, &listing.kinds, clock) {
            Ok(items) => page.context["items"] = items,
            Err(more) => problems.extend(more),
        }
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

/// The pages whose template exists and compiles. Each template that does not is one problem in
/// `problems`, naming it and the pages that use it: a page is never skipped for its template.
fn check_templates<'a>(
    renderer: &Renderer,
    pages: &'a [PageSpec],
    problems: &mut Vec<Diagnostic>,
) -> Vec<&'a PageSpec> {
    let mut by_template: BTreeMap<&str, Vec<&PageSpec>> = BTreeMap::new();
    for page in pages {
        by_template.entry(&page.template).or_default().push(page);
    }
    let mut renderable = Vec::new();
    for users in by_template.into_values() {
        let owned: Vec<PageSpec> = users.iter().map(|page| (*page).clone()).collect();
        match renderer.check(&owned) {
            Ok(()) => renderable.extend(users),
            // The problem names the template and the pages that use it.
            Err(problem) => problems.push(problem),
        }
    }
    renderable.sort_by_key(|page| page.route.output_path());
    renderable
}

/// The pages whose `author`, if they have one, is one of the configuration's; the others are
/// problems in `problems`.
fn check_authors<'a>(
    pages: Vec<&'a PageSpec>,
    config: &Config,
    problems: &mut Vec<Diagnostic>,
) -> Vec<&'a PageSpec> {
    let mut known = Vec::new();
    for page in pages {
        match unknown_author(page, config) {
            Some(problem) => problems.push(problem),
            None => known.push(page),
        }
    }
    known
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
fn stylesheets(
    root: &Path,
    config: &Config,
    site: &Site,
    clock: &SiteClock,
) -> Result<Vec<Written>, Vec<Diagnostic>> {
    let sheets: Vec<(&SourceDoc, Route)> = documents(site, Mode::Pages)
        .iter()
        .filter(|doc| {
            is_sass(doc.id.as_str()) && (doc.front.published() || clock.unpublished_allowed())
        })
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
