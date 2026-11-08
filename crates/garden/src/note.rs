//! Notes' pages (plan v9, §12.2 step 3): each note's entry, with its body, dates and backlinks,
//! the page [`NOTE_TEMPLATE`] renders, and the note graph.
//!
//! Links are resolved across every note of the garden, as the site's Jekyll wiki-link
//! plugin did, so a note's backlinks and the graph are whole even
//! when only some notes' pages are built: those of notes not in the slice, and of notes dated
//! after the build, which Jekyll did not write but its plugin still saw. Every wiki-link counts,
//! repeats included: a note linking twice to another is two of its backlinks and two edges of
//! the graph, as in Jekyll.
//!
//! The template shows the note as `page`:
//!
//! | Key | Value |
//! |-|-|
//! | `url` | the note's URL (`/dg/coding/rust`) |
//! | `title` | its title |
//! | `layout` | `garden_post` |
//! | `date` | its date (`2025-11-08T12:20:00+01:00`) |
//! | `modified` | its last modification (plan v7, §7.3), in the same form |
//! | `description` | its excerpt without markup, cut to 160 characters, as HTML text (D-003) |
//! | `backlinks` | `{title, url, excerpt}` of each link to it, by the linking note's order |
//! | `garden_map` | whether to show the garden map (front matter `hide_garden_map` off) |
//! | `toc`, `cover` | as a page's, at their defaults: the frame reads them |
//! | `hide_website_junk` | as a page's, with the category menu hidden, as the site's defaults hid it for notes |
//!
//! A description keeps its wiki-links as written (`[[Rust|rustaceans]]`), as Jekyll took
//! excerpts before its plugin resolved them, and so does a line kramdown read as a table row
//! because of one: `- [[Title|text]]` is described as the cells `[[Title` and `text]]` (owner
//! decision of 2026-11-07); a backlink's `excerpt` is the linking note's first
//! paragraph as written.
//!
//! So far the template has only what the slice's notes use. A note it could not show (another
//! front matter key, no date, or no title) is reported as not built yet.

use std::collections::HashMap;

use folia_core::content::markdown::{
    LinkResolver, description, excerpt_html, to_html, wiki_link_targets,
};
use folia_core::content::{
    Diagnostic, LastCommits, SiteClock, SourceDoc, cover, hide_website_junk, modified,
};
use folia_core::publish::{Artifact, EntrySummary, FeedItem, ModeOutput, PageSpec};
use serde::Serialize;
use serde_json::{Value, json};

use super::{GRAPH_URL, Note, WikiLinks, file_route, read_notes};

/// The template of a note's page.
pub const NOTE_TEMPLATE: &str = "garden_post.html";

/// The front matter keys the note template can show.
const KEYS: [&str; 3] = ["title", "date", "hide_garden_map"];

/// How long a graph label may be before it is cut, and how much of it is kept, in characters.
const LABEL_LENGTH: usize = 25;
const LABEL_KEPT: usize = 22;

/// A link to a note, from the note that makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Backlink {
    title: String,
    url: String,
    /// The linking note's first paragraph, as written.
    excerpt: String,
}

/// Builds the note graph, from every note of the garden among `docs`, and the pages of the notes
/// whose source is in `only` and that are due at the build: not dated after it, by their front
/// matter or their file name, as [`super::routes`] has it. `root` is the garden root. A note's
/// `modified` time comes from its last commit (`commits`), then its date; times are shown in the
/// time zone of `clock`. A built note's broken wiki-link is a warning. The feeds name `author`
/// (the configuration's garden author) as every note's, and show a note's summary with its
/// wiki-links resolved, as its body (D-003).
///
/// # Errors
///
/// Returns the first kind of problem found, each of its kind: what [`super::routes`] reports;
/// else a note of the garden without a title; else, for the notes to build, front matter the
/// template cannot show yet (another key, such as `last_modified_at`, or a
/// `hide_garden_map` that is not `true` or `false`) or no date.
pub fn build(
    docs: &[SourceDoc],
    root: &str,
    only: &[&str],
    author: Option<&str>,
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let notes = read_notes(docs, root, clock)?;
    let untitled: Vec<Diagnostic> = notes
        .iter()
        .filter(|note| note.title.is_none())
        .map(|note| Diagnostic::in_doc(&note.id, "a note without a title is not built yet"))
        .collect();
    if !untitled.is_empty() {
        return Err(untitled);
    }
    let sources: Vec<&SourceDoc> = notes
        .iter()
        .filter_map(|note| docs.iter().find(|doc| doc.id == note.id))
        .collect();
    let links = WikiLinks::new(
        notes
            .iter()
            .map(|note| (title(note), &note.id, note.route.url())),
    );
    let backlinks = backlinks(&notes, &sources, &links);

    let mut output = ModeOutput::default();
    output.artifacts.push(graph(&notes, &backlinks));
    let mut problems = Vec::new();
    for ((note, doc), backlinks) in notes.iter().zip(&sources).zip(&backlinks) {
        if !only.contains(&note.id.as_str()) || !note.is_due(clock) {
            continue;
        }
        match build_note(doc, note, backlinks, &links, commits, clock) {
            Ok((entry, page)) => {
                output.warnings.extend(broken_links(doc, &links));
                let published = entry.published.unwrap_or_else(|| clock.now());
                output.feed.push(FeedItem {
                    collection: "digital_garden".to_owned(),
                    url: entry.route.url().to_owned(),
                    id: FeedItem::jekyll_id(entry.route.url(), doc),
                    title: title(note).to_owned(),
                    published,
                    updated: entry.modified.as_ref().map_or(published, |m| m.at),
                    content_html: entry.body_html.clone(),
                    excerpt_html: excerpt_html(&doc.body, Some(&links)),
                    author: author.map(str::to_owned),
                    categories: Vec::new(),
                    tags: Vec::new(),
                    lang: None,
                    search_text: None,
                });
                output.entries.push(entry);
                output.pages.push(page);
            }
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok(output)
    } else {
        Err(problems)
    }
}

/// Every note's title; [`build`] has checked each has one.
fn title(note: &Note) -> &str {
    note.title.as_deref().unwrap_or_default()
}

/// The backlinks of each note, in the order of `notes`: one for each wiki-link to it, by the
/// order of the notes that make them. `sources` are the notes' documents, in the same order.
fn backlinks(notes: &[Note], sources: &[&SourceDoc], links: &WikiLinks) -> Vec<Vec<Backlink>> {
    let by_url: HashMap<&str, usize> = notes
        .iter()
        .enumerate()
        .map(|(i, note)| (note.route.url(), i))
        .collect();
    let mut backlinks = vec![Vec::new(); notes.len()];
    for (note, doc) in notes.iter().zip(sources) {
        for target in wiki_link_targets(&doc.body) {
            let Some(i) = links
                .resolve(target)
                .and_then(|url| by_url.get(url.as_str()).copied())
            else {
                continue;
            };
            backlinks[i].push(Backlink {
                title: title(note).to_owned(),
                url: note.route.url().to_owned(),
                excerpt: doc.body.split("\n\n").next().unwrap_or_default().to_owned(),
            });
        }
    }
    backlinks
}

/// The note graph, as the plugin wrote it: a node for each note, labelled with its title cut to
/// 25 characters, and an edge for each backlink, in compact JSON with the plugin's key order.
fn graph(notes: &[Note], backlinks: &[Vec<Backlink>]) -> Artifact {
    let graph = Graph {
        nodes: notes
            .iter()
            .map(|note| GraphNode {
                id: note.route.url(),
                path: note.route.url(),
                label: label(title(note)),
            })
            .collect(),
        edges: notes
            .iter()
            .zip(backlinks)
            .flat_map(|(note, backlinks)| {
                backlinks.iter().map(|backlink| GraphEdge {
                    source: &backlink.url,
                    target: note.route.url(),
                })
            })
            .collect(),
    };
    Artifact {
        route: file_route(GRAPH_URL),
        body: serde_json::to_string(&graph).expect("the graph is plain data"),
    }
}

/// The note graph's JSON, its keys in the plugin's order.
#[derive(Serialize)]
struct Graph<'a> {
    nodes: Vec<GraphNode<'a>>,
    edges: Vec<GraphEdge<'a>>,
}

#[derive(Serialize)]
struct GraphNode<'a> {
    id: &'a str,
    path: &'a str,
    label: String,
}

#[derive(Serialize)]
struct GraphEdge<'a> {
    source: &'a str,
    target: &'a str,
}

/// A title of more than 25 characters is cut to 22, followed by `...`.
fn label(title: &str) -> String {
    if title.chars().count() > LABEL_LENGTH {
        title.chars().take(LABEL_KEPT).collect::<String>() + "..."
    } else {
        title.to_owned()
    }
}

/// The parts of the site's frame a note hides: the category menu, as the site's defaults hid it
/// for every note, and nothing else, since a note's own `hide_website_junk` is not built yet.
fn frame(doc: &SourceDoc) -> Value {
    let mut parts = hide_website_junk(doc);
    parts["cat_jumbotron"] = Value::Bool(true);
    parts
}

/// A warning for each wiki-link of `doc` that names no note.
fn broken_links(doc: &SourceDoc, links: &WikiLinks) -> Vec<Diagnostic> {
    wiki_link_targets(&doc.body)
        .into_iter()
        .filter(|target| links.resolve(target).is_none())
        .map(|target| {
            Diagnostic::in_doc(
                &doc.id,
                format!("the wiki-link [[{target}]] names no note, so it is shown as broken"),
            )
        })
        .collect()
}

fn build_note(
    doc: &SourceDoc,
    note: &Note,
    backlinks: &[Backlink],
    links: &WikiLinks,
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<(EntrySummary, PageSpec), Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    if let Some(key) = doc.front.keys().find(|key| !KEYS.contains(key)) {
        return Err(problem(&format!(
            "the front matter key {key} is not built yet"
        )));
    }
    let date = note
        .date
        .ok_or_else(|| problem(&"a note without a date is not built yet"))?;
    let garden_map = match doc.front.json("hide_garden_map") {
        None => true,
        Some(Value::Bool(hide)) => !hide,
        Some(_) => return Err(problem(&"hide_garden_map must be true or false")),
    };
    let modified = modified(doc, commits, Some(date), clock)?;

    let context = json!({
        "url": note.route.url(),
        "title": title(note),
        "layout": "garden_post",
        "date": clock.xmlschema(date),
        "modified": modified.map(|m| clock.xmlschema(m.at)),
        "description": description(&excerpt_html(&doc.body, None)),
        "backlinks": backlinks
            .iter()
            .map(|b| json!({"title": b.title, "url": b.url, "excerpt": b.excerpt}))
            .collect::<Vec<_>>(),
        "garden_map": garden_map,
        "toc": null,
        "cover": cover(doc)?.to_json(),
        "hide_website_junk": frame(doc),
    });
    let page = PageSpec {
        route: note.route.clone(),
        template: NOTE_TEMPLATE.to_owned(),
        source: Some(doc.id.clone()),
        context,
    };
    let entry = EntrySummary {
        doc: doc.id.clone(),
        route: note.route.clone(),
        published: Some(date),
        modified,
        body_html: to_html(&doc.body, Some(links)),
        redirect_from: note.redirect_from.clone(),
    };
    Ok((entry, page))
}
