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

use folia_core::content::{Diagnostic, Route, SiteClock, SourceDoc};
use folia_core::publish::PlannedOutput;

/// The outputs of the garden: one page per published note, the feed and the note graph. Notes
/// that are unpublished, or whose front matter `date` is after the build, are left out.
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
    let mut outputs = Vec::new();
    let mut problems = Vec::new();
    for doc in docs {
        match note_output(doc, root, clock) {
            Ok(Some(output)) => outputs.push(output),
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    outputs.push(PlannedOutput::generated(
        file_route("/dg/feed.xml"),
        "garden feed",
    ));
    outputs.push(PlannedOutput::generated(
        file_route("/assets/json/notes_graph.json"),
        "note graph",
    ));
    Ok(outputs)
}

fn note_output(
    doc: &SourceDoc,
    root: &str,
    clock: &SiteClock,
) -> Result<Option<PlannedOutput>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    if let Some(date) = doc.front.string("date").map_err(|e| problem(&e))?
        && clock.is_future(clock.parse(&date).map_err(|e| problem(&e))?)
    {
        return Ok(None);
    }
    if !doc.front.published() {
        return Ok(None);
    }

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
    let route = Route::html(&url).map_err(|e| problem(&e))?;
    Ok(Some(PlannedOutput {
        redirect_from: doc.redirect_from()?,
        ..PlannedOutput::authored(route, doc.id.clone())
    }))
}

fn file_route(url: &str) -> Route {
    Route::file(url).expect("the garden's own routes are valid")
}

#[cfg(test)]
mod tests {
    use folia_core::content::{DocId, FrontMatter, Mode};

    use super::*;

    fn clock() -> SiteClock {
        SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
    }

    fn note(path: &str, front: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new(path),
            mode: Mode::Garden,
            front: FrontMatter::parse(front).unwrap(),
            body: String::new(),
        }
    }

    #[test]
    fn a_note_keeps_its_path_case_and_underscores() {
        let outputs = routes(
            &[note("_digital_garden/watch/topic_note.md", "")],
            "_digital_garden",
            &clock(),
        )
        .unwrap();
        assert_eq!(outputs[0].route.url(), "/dg/tracking/chat_control");
    }

    #[test]
    fn an_index_note_keeps_index_in_its_url() {
        let outputs = routes(
            &[note("_digital_garden/topic/index.md", "")],
            "_digital_garden",
            &clock(),
        )
        .unwrap();
        assert_eq!(outputs[0].route.url(), "/dg/coding/index");
        assert_eq!(outputs[0].route.output_path(), "dg/coding/index.html");
    }

    #[test]
    fn a_note_dated_after_the_build_is_left_out() {
        let docs = [
            note("_digital_garden/a.md", "date: 2026-10-25 08:00"),
            note("_digital_garden/b.md", "date: 2026-10-25 12:00"),
        ];
        let outputs = routes(&docs, "_digital_garden", &clock()).unwrap();
        let notes: Vec<_> = outputs
            .iter()
            .filter_map(|o| o.source.as_ref())
            .map(DocId::as_str)
            .collect();
        assert_eq!(notes, ["_digital_garden/a.md"]);
    }

    #[test]
    fn a_date_in_another_form_is_reported() {
        let problems = routes(
            &[note("_digital_garden/a.md", "date: April 2026")],
            "_digital_garden",
            &clock(),
        )
        .unwrap_err();
        assert_eq!(problems.len(), 1);
    }
}
