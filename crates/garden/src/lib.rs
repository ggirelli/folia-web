//! The digital garden mode: notes connected by wiki-links.
//!
//! Routes, as the site has published them under Jekyll:
//!
//! | Output | URL |
//! |-|-|
//! | a note | `/dg/:path` (its path below the garden root, case and `_` kept) |
//! | the garden feed | `/dg/feed.xml` |
//! | the note graph | `/assets/json/notes_graph.json` |
//!
//! An `index.md` keeps `index` in its URL (`/dg/coding/index`), and is written to
//! `dg/coding/index.html`, where the host also serves it as `/dg/coding/`.
//!
//! Wiki-links between notes are resolved by [`WikiLinks`]. [`build`] builds notes' pages, with
//! their backlinks, and the note graph (module `note`).

mod note;
mod wiki_links;

pub use note::{NOTE_TEMPLATE, build};
pub use wiki_links::WikiLinks;

use folia_core::content::{
    Diagnostic, DocId, Route, SiteClock, SourceDoc, dated_file_name, file_title,
};
use folia_core::publish::PlannedOutput;
use jiff::Timestamp;

/// The URL of the note graph.
const GRAPH_URL: &str = "/assets/json/notes_graph.json";

/// A note of the garden, with what the garden's outputs need to know about it.
struct Note {
    id: DocId,
    /// Its front matter title, or else the one Jekyll derives from its file name.
    title: String,
    route: Route,
    redirect_from: Vec<Route>,
    /// Its date: its front matter `date`, or else the date its file name starts with, at
    /// midnight.
    date: Option<Timestamp>,
}

impl Note {
    /// Whether the note's page is written at this build: as Jekyll held back a document dated
    /// after the build, by its front matter or its file name, a note whose date is after `clock`
    /// has no page yet. It is still a note of the garden, linked to and linking (`read_notes`).
    fn is_due(&self, clock: &SiteClock) -> bool {
        !self.date.is_some_and(|date| clock.is_future(date))
    }
}

/// The outputs of the garden: one page per note due at this build, the feed and the note graph.
/// A note is due unless it is unpublished or dated after the build, by its front matter `date`
/// or else by the date its file name starts with.
///
/// `root` is the garden root, such as `_digital_garden`.
///
/// # Errors
///
/// Returns every problem found: a date in a form Folia does not read, or front matter that
/// cannot be read.
pub fn routes(
    docs: &[SourceDoc],
    root: &str,
    clock: &SiteClock,
) -> Result<Vec<PlannedOutput>, Vec<Diagnostic>> {
    let mut outputs: Vec<PlannedOutput> = read_notes(docs, root, clock)?
        .into_iter()
        .filter(|note| note.is_due(clock))
        .map(|note| PlannedOutput {
            redirect_from: note.redirect_from,
            ..PlannedOutput::authored(note.route, note.id)
        })
        .collect();
    outputs.push(PlannedOutput::generated(
        file_route("/dg/feed.xml"),
        "garden feed",
    ));
    outputs.push(PlannedOutput::generated(
        file_route(GRAPH_URL),
        "note graph",
    ));
    Ok(outputs)
}

/// The notes of the garden among `docs`: every note but those with `published: false`, dated
/// after the build or not, as Jekyll's collection kept them for the site's plugin. They are what
/// wiki-links resolve to, what backlinks come from, and the graph's nodes.
///
/// They are in Jekyll's order for dated notes: by date, then by source path, compared byte by
/// byte. A note with neither a front matter `date` nor a dated file name is ordered as if dated
/// at the build, which puts it after every other note but one dated later: Folia's own choice.
/// Jekyll sorted its collection before it dated such a note, comparing it with the others by
/// path only, so where it went depended on the order Jekyll read the files; Folia does not
/// reproduce that. No note of the reference site is undated.
fn read_notes(
    docs: &[SourceDoc],
    root: &str,
    clock: &SiteClock,
) -> Result<Vec<Note>, Vec<Diagnostic>> {
    let mut notes = Vec::new();
    let mut problems = Vec::new();
    for doc in docs {
        match read_note(doc, root, clock) {
            Ok(Some(note)) => notes.push(note),
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    notes.sort_by(|a, b| {
        let order = |note: &Note| note.date.unwrap_or(clock.now());
        (order(a), &a.id).cmp(&(order(b), &b.id))
    });
    Ok(notes)
}

/// Reads a note, or `None` when its front matter says `published: false`.
fn read_note(doc: &SourceDoc, root: &str, clock: &SiteClock) -> Result<Option<Note>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let front_date = match doc.front.string("date").map_err(|e| problem(&e))? {
        Some(date) => Some(clock.parse(&date).map_err(|e| problem(&e))?),
        None => None,
    };
    if !doc.front.published() {
        return Ok(None);
    }
    let date = match front_date {
        Some(date) => Some(date),
        None => match dated_file_name(doc.id.file_name()).map_err(|e| problem(&e))? {
            Some((date, _)) => Some(clock.midnight(date).map_err(|e| problem(&e))?),
            None => None,
        },
    };

    let below = doc
        .id
        .below(root)
        .ok_or_else(|| problem(&format!("is not below the garden root {root}")))?;
    let url = match doc.front.string("permalink").map_err(|e| problem(&e))? {
        Some(permalink) => permalink,
        None => format!(
            "/dg/{}",
            below[..below.len() - doc.id.extension().len()].trim_end_matches('.')
        ),
    };
    Ok(Some(Note {
        id: doc.id.clone(),
        title: doc
            .front
            .string("title")
            .map_err(|e| problem(&e))?
            .unwrap_or_else(|| file_title(doc.id.file_name())),
        route: Route::html(&url).map_err(|e| problem(&e))?,
        redirect_from: doc.redirect_from()?,
        date,
    }))
}

fn file_route(url: &str) -> Route {
    Route::file(url).expect("the garden's own routes are valid")
}

#[cfg(test)]
mod tests;
