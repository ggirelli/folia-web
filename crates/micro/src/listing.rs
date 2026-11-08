//! The micro listings: the entries of some kinds, newest first, as a listing page shows them.
//!
//! Which pages list which kinds is the site's configuration; the pages' frame is the pages
//! mode's, from the claimed source's front matter, and the builder puts the two together. Each
//! item is `{url, title, kind, layout, date, excerpt, text, summary}`, with its kind's data
//! (`like`, `bookmark`, `rsvp` and `event`, as the entry's page has them):
//!
//! | Key | Value |
//! |-|-|
//! | `kind`, `layout` | its kind (`notes`) and its kind's layout (`page`) |
//! | `date` | its file name's date, at midnight in the site's time zone |
//! | `excerpt` | its excerpt as HTML, as Jekyll's `item.excerpt` |
//! | `text` | its body's HTML without markup (Liquid's `strip_html`), line breaks kept |
//! | `summary` | `text`'s first 25 words, with `...` when it has more (`truncatewords: 25`) |
//!
//! The order is Jekyll's `sort: "date" | reverse` over the collection: newest first, and entries
//! of the same date by source path, descending.

use folia_core::content::markdown::{excerpt_html, strip_html, to_html, truncate_words};
use folia_core::content::{Diagnostic, SiteClock, SourceDoc};
use jiff::Timestamp;
use serde_json::{Value, json};

use super::entry::{KINDS, event, link, rsvp};
use super::{entry_output, kind_of};

/// The words of an item's `summary`.
const SUMMARY_WORDS: usize = 25;

/// The published entries of `kinds` among `docs` (below `root`), newest first, as listing items.
///
/// # Errors
///
/// Returns every problem found: a kind that is not one of the four, and an entry whose route,
/// title or kind data cannot be read.
pub fn listing(
    docs: &[SourceDoc],
    root: &str,
    kinds: &[String],
    clock: &SiteClock,
) -> Result<Value, Vec<Diagnostic>> {
    let unknown: Vec<Diagnostic> = kinds
        .iter()
        .filter(|kind| !KINDS.iter().any(|(name, _, _)| name == kind))
        .map(|kind| Diagnostic::site(format!("a micro listing lists the unknown kind {kind}")))
        .collect();
    if !unknown.is_empty() {
        return Err(unknown);
    }
    let mut items: Vec<(Timestamp, &str, Value)> = Vec::new();
    let mut problems = Vec::new();
    for doc in docs {
        match item(doc, root, kinds, clock) {
            Ok(Some((date, value))) => items.push((date, doc.id.as_str(), value)),
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    items.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    Ok(items.into_iter().map(|(_, _, value)| value).collect())
}

fn item(
    doc: &SourceDoc,
    root: &str,
    kinds: &[String],
    clock: &SiteClock,
) -> Result<Option<(Timestamp, Value)>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let Some(kind) = kind_of(doc, root).filter(|kind| kinds.iter().any(|k| k == kind)) else {
        return Ok(None);
    };
    let Some(planned) = entry_output(doc, root, clock)? else {
        return Ok(None);
    };
    let (_, layout, own) = KINDS
        .iter()
        .find(|(name, _, _)| *name == kind)
        .copied()
        .expect("kind_of gives one of the kinds");
    let title = doc
        .front
        .string("title")
        .map_err(|e| problem(&e))?
        .unwrap_or_else(|| folia_core::content::file_title(doc.id.file_name()));
    let (date, _) = folia_core::content::dated_file_name(doc.id.file_name())
        .map_err(|e| problem(&e))?
        .ok_or_else(|| problem(&"an entry without a date in its file name is not built yet"))?;
    let published = clock.midnight(date).map_err(|e| problem(&e))?;
    let text = strip_html(&to_html(&doc.body, None));
    let mut value = json!({
        "url": planned.route.url(),
        "title": title,
        "kind": kind,
        "layout": layout,
        "date": clock.xmlschema(published),
        "excerpt": excerpt_html(&doc.body, None),
        "summary": truncate_words(&text, SUMMARY_WORDS),
        "text": text,
    });
    for key in own {
        value[*key] = match *key {
            "like" | "bookmark" => link(doc, key),
            "rsvp" => rsvp(doc),
            _ => event(doc, clock),
        }
        .map_err(|message| problem(&message))?;
    }
    Ok(Some((published, value)))
}

#[cfg(test)]
mod tests;
