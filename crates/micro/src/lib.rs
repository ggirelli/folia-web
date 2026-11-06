//! The microblog mode: short entries whose kind is the directory they sit in.
//!
//! Routes, as the site has published them under Jekyll:
//!
//! | Output | URL |
//! |-|-|
//! | an entry in `notes/`, `likes/`, `bookmarks/` or `rsvps/` | `/micro/:name`, `/likes/:name`, `/bookmarks/:name`, `/rsvps/:name` (`:name` slugified, so `_` becomes `-`) |
//! | any other entry | `/micro/:path` (its path below the micro root, unchanged) |
//! | the micro feed | `/micro/feed.xml` |
//! | a feed per kind | `/feed/micro/:kind.xml` |
//!
//! A kind applies to Markdown files directly in its directory, as Jekyll's front matter
//! defaults (`_micro/notes/*.md`) did.

use folia_core::content::{
    Diagnostic, Route, SiteClock, SlugMode, SourceDoc, dated_file_name, slugify,
};
use folia_core::publish::PlannedOutput;

/// The kinds of entry: the directory each lives in, and the URL prefix of its entries.
const KINDS: [(&str, &str); 4] = [
    ("notes", "/micro/"),
    ("likes", "/likes/"),
    ("bookmarks", "/bookmarks/"),
    ("rsvps", "/rsvps/"),
];

/// The outputs of the microblog: one page per published entry, and the feeds. Entries that are
/// unpublished, or dated after the build by their file name, are left out.
///
/// `root` is the micro root, such as `_micro`.
///
/// # Errors
///
/// Returns every problem found: a dated file name with a date that does not exist, or front
/// matter that cannot be read.
pub fn routes(
    docs: &[SourceDoc],
    root: &str,
    clock: &SiteClock,
) -> Result<Vec<PlannedOutput>, Vec<Diagnostic>> {
    let mut outputs = Vec::new();
    let mut problems = Vec::new();
    for doc in docs {
        match entry_output(doc, root, clock) {
            Ok(Some(output)) => outputs.push(output),
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    outputs.push(PlannedOutput::generated(
        file_route("/micro/feed.xml"),
        "micro feed",
    ));
    for (kind, _) in KINDS {
        outputs.push(PlannedOutput::generated(
            file_route(&format!("/feed/micro/{kind}.xml")),
            "micro kind feed",
        ));
    }
    Ok(outputs)
}

fn entry_output(
    doc: &SourceDoc,
    root: &str,
    clock: &SiteClock,
) -> Result<Option<PlannedOutput>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    if let Some((date, _)) = dated_file_name(doc.id.file_name()).map_err(|e| problem(&e))?
        && clock.is_future(clock.midnight(date).map_err(|e| problem(&e))?)
    {
        return Ok(None);
    }
    if !doc.front.published() {
        return Ok(None);
    }

    let below = doc
        .id
        .below(root)
        .ok_or_else(|| problem(&format!("is not below the micro root {root}")))?;
    let url = match doc.front.string("permalink").map_err(|e| problem(&e))? {
        Some(permalink) => permalink,
        None => match kind_prefix(below, doc.id.extension()) {
            Some(prefix) => format!(
                "{prefix}{}",
                slugify(doc.id.stem(), SlugMode::Default, false)
            ),
            None => format!(
                "/micro/{}",
                below[..below.len() - doc.id.extension().len()].trim_end_matches('.')
            ),
        },
    };
    let route = Route::html(&url).map_err(|e| problem(&e))?;
    Ok(Some(PlannedOutput {
        redirect_from: doc.redirect_from()?,
        ..PlannedOutput::authored(route, doc.id.clone())
    }))
}

/// The URL prefix of an entry's kind, for a Markdown file directly in a kind's directory.
fn kind_prefix(below: &str, extension: &str) -> Option<&'static str> {
    let (dir, _) = below.split_once('/')?;
    if below[dir.len() + 1..].contains('/') || extension != ".md" {
        return None;
    }
    KINDS
        .iter()
        .find(|(kind, _)| *kind == dir)
        .map(|(_, prefix)| *prefix)
}

fn file_route(url: &str) -> Route {
    Route::file(url).expect("the microblog's own routes are valid")
}

#[cfg(test)]
mod tests {
    use folia_core::content::{DocId, FrontMatter, Mode};

    use super::*;

    fn clock() -> SiteClock {
        SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
    }

    fn entry(path: &str, front: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new(path),
            mode: Mode::Micro,
            front: FrontMatter::parse(front).unwrap(),
            body: String::new(),
        }
    }

    fn first_url(doc: SourceDoc) -> String {
        routes(&[doc], "_micro", &clock()).unwrap()[0]
            .route
            .url()
            .to_owned()
    }

    #[test]
    fn a_kind_names_the_url_and_underscores_become_hyphens() {
        let url = first_url(entry(
            "_micro/bookmarks/2025-06-16-sample_bookmark_entry.md",
            "",
        ));
        assert_eq!(url, "/bookmarks/2025-06-16-sample-bookmark-entry");
    }

    #[test]
    fn notes_live_under_micro() {
        assert_eq!(
            first_url(entry("_micro/notes/2026-04-08-sample-note.md", "")),
            "/micro/2026-04-08-sample-note"
        );
    }

    #[test]
    fn an_entry_outside_the_kinds_keeps_its_path() {
        assert_eq!(
            first_url(entry("_micro/notes/deep/x_y.md", "")),
            "/micro/notes/deep/x_y"
        );
        assert_eq!(
            first_url(entry("_micro/about_micro.md", "")),
            "/micro/about_micro"
        );
    }

    #[test]
    fn future_entries_are_left_out_and_feeds_remain() {
        let outputs = routes(
            &[entry("_micro/notes/2026-10-26-soon.md", "")],
            "_micro",
            &clock(),
        )
        .unwrap();
        assert!(outputs.iter().all(|o| o.source.is_none()));
        assert_eq!(outputs.len(), 5);
    }
}
