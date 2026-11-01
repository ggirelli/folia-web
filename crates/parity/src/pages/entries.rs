//! What the slice's documents carry besides their pages (plan v7, §12.2 step 2):
//!
//! - **Dates:** an entry's publication and last modification equal the baseline's meta dump,
//!   value and origin (`date`, `last_modified_at`, `last_modified_at_from`). A front matter date
//!   without a time is midnight in the site's time zone.
//! - **Table of contents:** the page's `div.side-summary` equals Jekyll's after normalization.
//! - **Highlighting:** the page's code blocks have the language, text and token classes of the
//!   entry's body, as spike 1's highlighter gives them; how far they agree with Rouge's is shown,
//!   not checked (plan v7, §14 Q12).

use std::path::Path;

use folia_core::content::SiteClock;
use folia_core::publish::EntrySummary;
use serde::Deserialize;

use super::{find, has_class, report};
use crate::baseline::Row;
use crate::markdown::dom;

/// What the dump records of a document's dates.
#[derive(Deserialize)]
struct Meta {
    date: Option<String>,
    last_modified_at: Option<String>,
    last_modified_at_from: Option<String>,
}

/// Checks each entry's dates against the dump in `dump`, showing times with `clock`.
pub fn check_dates(entries: &[EntrySummary], rows: &[Row], dump: &Path, clock: &SiteClock) -> bool {
    let mut passed = true;
    for entry in entries {
        let mut differing = Vec::new();
        match read_meta(entry, rows, dump) {
            Ok(meta) => {
                let published = entry.published.map(|at| clock.xmlschema(at));
                if published != meta.date {
                    differing.push(format!("date: Jekyll {:?}, Folia {published:?}", meta.date));
                }
                let modified = entry
                    .modified
                    .map(|m| (clock.xmlschema(m.at), m.from.as_str().to_owned()));
                let jekyll = meta.last_modified_at.zip(meta.last_modified_at_from);
                let same = match (&jekyll, &modified) {
                    (Some((want, want_from)), Some((got, got_from))) => {
                        want_from == got_from && same_time(want, got)
                    }
                    (None, None) => true,
                    _ => false,
                };
                if !same {
                    differing.push(format!(
                        "last modified (value, origin): Jekyll {jekyll:?}, Folia {modified:?}"
                    ));
                }
            }
            Err(problem) => differing.push(problem),
        }
        passed &= report(
            &format!("{}: dates equal the baseline's dump", entry.doc),
            &differing,
        );
    }
    passed
}

/// Whether a dumped time equals one Folia wrote as `xmlschema`. The dump writes a front matter
/// date as it was given (`2021-08-24`), which Jekyll reads as midnight.
fn same_time(dumped: &str, xmlschema: &str) -> bool {
    if dumped.len() == 10 {
        xmlschema.starts_with(dumped) && xmlschema[10..].starts_with("T00:00:00")
    } else {
        dumped == xmlschema
    }
}

fn read_meta(entry: &EntrySummary, rows: &[Row], dump: &Path) -> Result<Meta, String> {
    let path = rows
        .iter()
        .find(|row| row.source.as_deref() == Some(entry.doc.as_str()))
        .and_then(|row| row.meta.as_deref())
        .ok_or_else(|| format!("the dump has no metadata for {}", entry.doc))?;
    let text = std::fs::read_to_string(dump.join(path))
        .map_err(|e| format!("{}: {e}", dump.join(path).display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

/// Checks the table of contents of page `path`, when either page has one. Returns whether it
/// passed, and whether there was one.
pub fn check_toc(path: &str, jekyll: &[dom::Node], folia: &[dom::Node]) -> (bool, bool) {
    let toc = |nodes: &[dom::Node]| find(nodes, &|node| has_class(node, "side-summary")).cloned();
    let difference = match (toc(jekyll), toc(folia)) {
        (None, None) => return (true, false),
        (Some(want), Some(got)) => dom::first_difference(&[want], &[got]),
        (None, Some(_)) => Some("Jekyll's page has no table of contents".to_owned()),
        (Some(_), None) => Some("Folia's page has no table of contents".to_owned()),
    };
    let differing: Vec<String> = difference.into_iter().collect();
    let passed = report(
        &format!("{path}: table of contents equal after normalization"),
        &differing,
    );
    (passed, true)
}

/// Checks that the code blocks of page `path` (`folia`) are the entry's, and shows how far their
/// token classes agree with Rouge's (`jekyll`).
pub fn check_code(path: &str, entry: &EntrySummary, jekyll: &str, folia: &str) -> bool {
    let (body, page, rouge) = (
        dom::code_blocks(&entry.body_html),
        dom::code_blocks(folia),
        dom::code_blocks(jekyll),
    );
    if body.is_empty() && page.is_empty() {
        return true;
    }
    let mut differing = Vec::new();
    if body.len() == page.len() {
        for (i, (want, got)) in body.iter().zip(&page).enumerate() {
            if (&want.lang, &want.text, &want.classes) != (&got.lang, &got.text, &got.classes) {
                differing.push(format!("block {}: not the body's", i + 1));
            }
        }
    } else {
        differing.push(format!(
            "the body has {} code blocks, the page {}",
            body.len(),
            page.len()
        ));
    }
    let (mut agreeing, mut counted) = (0, 0);
    for (ours, theirs) in page.iter().zip(&rouge).filter(|(a, b)| a.text == b.text) {
        for ((c, a), b) in ours.text.chars().zip(&ours.classes).zip(&theirs.classes) {
            if !c.is_whitespace() {
                counted += 1;
                agreeing += usize::from(a == b);
            }
        }
    }
    let passed = report(
        &format!(
            "{path}: {} highlighted blocks have spike 1's classes",
            page.len()
        ),
        &differing,
    );
    if counted > 0 {
        let share = 100.0 * f64::from(u32::try_from(agreeing).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(counted).unwrap_or(u32::MAX));
        println!(
            "  their token classes agree with Rouge's on {agreeing} of {counted} non-whitespace characters ({share:.1}%)."
        );
    }
    passed
}

#[cfg(test)]
mod tests {
    use folia_core::content::{DocId, Route};

    use super::*;

    fn entry(body_html: &str) -> EntrySummary {
        EntrySummary {
            doc: DocId::new("_posts/2021/2021-08-08-a.md"),
            route: Route::html("/blog/2021/08/08/a").unwrap(),
            published: None,
            modified: None,
            body_html: body_html.to_owned(),
            redirect_from: Vec::new(),
        }
    }

    const BLOCK: &str = r#"<div class="language-html highlighter-rouge"><pre><code><span class="nt">&lt;a&gt;</span></code></pre></div>"#;

    #[test]
    fn code_blocks_must_be_the_bodys() {
        let page = format!("<html><body><p>x</p>{BLOCK}</body></html>");
        assert!(check_code("a.html", &entry(BLOCK), &page, &page));
        let changed = page.replace(r#"class="nt""#, r#"class="na""#);
        assert!(!check_code("a.html", &entry(BLOCK), &page, &changed));
        assert!(!check_code("a.html", &entry(""), &page, &page));
    }

    #[test]
    fn a_dumped_date_is_midnight() {
        assert!(same_time("2021-08-24", "2021-08-24T00:00:00+02:00"));
        assert!(!same_time("2021-08-24", "2021-08-24T10:00:00+02:00"));
        assert!(!same_time("2021-08-25", "2021-08-24T00:00:00+02:00"));
        assert!(same_time(
            "2026-01-04T12:51:21+01:00",
            "2026-01-04T12:51:21+01:00"
        ));
    }
}
