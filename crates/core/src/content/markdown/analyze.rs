//! Facts about a rendered document that its page shows besides its body (plan v7, §5.3): its
//! headings, and the table of contents built from them.
//!
//! The site's templates built the TOC with an include using jekyll-toc 1.1.0,
//! which splits the rendered HTML on `<h`. [`toc`] gives the same tree from the headings:
//!
//! - only headings from `min_level` to `max_level` count, and none with the class `no_toc`;
//! - the first heading that counts sets the top level;
//! - a heading deeper than the one before it starts one nested list under that one, whatever the
//!   levels between them; one at the same level follows it; one `k` levels shallower closes `k`
//!   lists, counted in levels, not in the lists actually open.
//!
//! Where that would close more lists than are open (a heading shallower than the first one),
//! jekyll-toc writes broken HTML; Folia closes the lists that are open. No document of the site
//! has such a heading, nor skips a level.

use super::render;
use super::tree::{Kind, Node};

/// A heading of a document, as its rendered HTML has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Its level, 1 to 6.
    pub level: u8,
    /// Its ID, which may be empty.
    pub id: String,
    /// Its content as HTML, without the `h` element.
    pub html: String,
    /// Its classes, from an IAL.
    pub classes: Vec<String>,
}

/// An entry of a table of contents: a heading, and the entries nested under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocEntry {
    /// The heading's ID, the entry's link target.
    pub id: String,
    /// The heading's content as HTML.
    pub html: String,
    /// The headings under it.
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    /// The entry as templates see it: `{"id", "html", "children"}`.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "html": self.html,
            "children": self.children.iter().map(Self::to_json).collect::<Vec<_>>(),
        })
    }
}

/// The headings of the document `root`, parsed from `src` and with its IDs assigned, in order.
pub(super) fn headings(root: &Node, src: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    gather(root, src, &mut out);
    out
}

fn gather(node: &Node, src: &str, out: &mut Vec<Heading>) {
    if let Kind::Heading(level) = node.kind {
        let attr = |key: &str| {
            node.attrs
                .0
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        out.push(Heading {
            level,
            id: attr("id").unwrap_or_default(),
            html: render::inline_html(node, src),
            classes: attr("class")
                .as_deref()
                .into_iter()
                .flat_map(str::split_whitespace)
                .map(str::to_owned)
                .collect(),
        });
        return;
    }
    for child in &node.children {
        gather(child, src, out);
    }
}

/// The table of contents of `headings`, from `min_level` to `max_level`.
#[must_use]
pub fn toc(headings: &[Heading], min_level: u8, max_level: u8) -> Vec<TocEntry> {
    // The open entries, outermost first.
    let mut open: Vec<TocEntry> = Vec::new();
    let mut top = Vec::new();
    let mut last_level = None;
    for heading in headings
        .iter()
        .filter(|h| (min_level..=max_level).contains(&h.level))
        .filter(|h| !h.classes.iter().any(|c| c == "no_toc"))
    {
        // The entries this heading ends: none under a shallower one, the one before at the
        // same level, and one more per level it rises.
        let ending = match last_level {
            Some(last) if heading.level <= last => usize::from(last - heading.level) + 1,
            _ => 0,
        };
        for _ in 0..ending.min(open.len()) {
            close(&mut open, &mut top);
        }
        open.push(TocEntry {
            id: heading.id.clone(),
            html: heading.html.clone(),
            children: Vec::new(),
        });
        last_level = Some(heading.level);
    }
    while !open.is_empty() {
        close(&mut open, &mut top);
    }
    top
}

/// Closes the innermost open entry, adding it to the one around it or to the top level.
fn close(open: &mut Vec<TocEntry>, top: &mut Vec<TocEntry>) {
    if let Some(entry) = open.pop() {
        match open.last_mut() {
            Some(parent) => parent.children.push(entry),
            None => top.push(entry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(src: &str) -> Vec<Heading> {
        super::super::analyze(src, None)
    }

    fn heading(level: u8, id: &str) -> Heading {
        Heading {
            level,
            id: id.to_owned(),
            html: id.to_owned(),
            classes: Vec::new(),
        }
    }

    fn ids(entries: &[TocEntry]) -> String {
        entries
            .iter()
            .map(|e| {
                if e.children.is_empty() {
                    e.id.clone()
                } else {
                    format!("{}({})", e.id, ids(&e.children))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn headings_have_their_level_id_html_and_classes() {
        let headings = analyze("### Webmentions: a *not so new* one\n\n#### Skip\n{: .no_toc}\n");
        assert_eq!(
            headings,
            [
                Heading {
                    level: 3,
                    id: "webmentions-a-not-so-new-one".into(),
                    html: "Webmentions: a <em>not so new</em> one".into(),
                    classes: vec![],
                },
                Heading {
                    level: 4,
                    id: "skip".into(),
                    html: "Skip".into(),
                    classes: vec!["no_toc".into()],
                },
            ]
        );
    }

    #[test]
    fn a_deeper_heading_nests_under_the_one_before_it() {
        let headings = [
            heading(3, "a"),
            heading(3, "b"),
            heading(4, "c"),
            heading(4, "d"),
            heading(3, "e"),
        ];
        assert_eq!(ids(&toc(&headings, 2, 4)), "a b(c d) e");
    }

    #[test]
    fn levels_outside_the_range_and_no_toc_headings_are_left_out() {
        let mut skipped = heading(2, "x");
        skipped.classes.push("no_toc".into());
        let headings = [
            heading(1, "title"),
            heading(2, "a"),
            skipped,
            heading(5, "deep"),
            heading(2, "b"),
        ];
        assert_eq!(ids(&toc(&headings, 2, 4)), "a b");
    }

    #[test]
    fn a_skipped_level_nests_one_list_and_rising_closes_one_per_level() {
        let headings = [heading(2, "a"), heading(4, "b"), heading(3, "c")];
        assert_eq!(ids(&toc(&headings, 2, 4)), "a(b) c");
    }

    #[test]
    fn a_heading_above_the_first_closes_only_the_open_lists() {
        let headings = [heading(3, "a"), heading(4, "b"), heading(2, "c")];
        assert_eq!(ids(&toc(&headings, 2, 4)), "a(b) c");
    }
}
