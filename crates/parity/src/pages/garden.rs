//! What the garden adds to page parity (plan v9, §12.2 step 3):
//!
//! - **The note graph:** Folia's `assets/json/notes_graph.json` has Jekyll's nodes and edges, as
//!   multisets: an edge is counted once per link, so a repeated one must be repeated. Whether the
//!   file is also byte-identical is shown, not checked. A graph Jekyll wrote and Folia did not,
//!   or the other way round, fails.
//! - **Broken links:** each document built has one warning per broken link Jekyll rendered on its
//!   page (`span.broken-link`), and no warning concerns a document that was not built.

use std::collections::BTreeMap;

use folia::build::Built;
use serde_json::Value;

use super::{elements, has_class, report};
use crate::markdown::dom;

/// The note graph's output file.
pub const GRAPH: &str = "assets/json/notes_graph.json";

/// Checks Folia's note graph against Jekyll's, either `None` when that side wrote none. A graph
/// one side wrote and the other did not fails; when neither wrote one there is nothing to compare.
pub fn check_graph_file(jekyll: Option<&[u8]>, folia: Option<&[u8]>) -> bool {
    match (jekyll, folia) {
        (Some(jekyll), Some(folia)) => check_graph(jekyll, folia),
        (None, None) => report(&format!("{GRAPH}: neither Jekyll nor Folia wrote it"), &[]),
        (Some(_), None) => report(
            &format!("{GRAPH}: Folia wrote the graph Jekyll wrote"),
            &["Folia wrote no such file".to_owned()],
        ),
        (None, Some(_)) => report(
            &format!("{GRAPH}: Folia wrote the graph Jekyll wrote"),
            &["Jekyll wrote no such file".to_owned()],
        ),
    }
}

/// Checks Folia's note graph (`folia`) against Jekyll's (`jekyll`).
fn check_graph(jekyll: &[u8], folia: &[u8]) -> bool {
    let mut differing = Vec::new();
    match (members(jekyll), members(folia)) {
        (Ok((want_nodes, want_edges)), Ok((got_nodes, got_edges))) => {
            for (what, want, got) in [
                ("nodes", want_nodes, got_nodes),
                ("edges", want_edges, got_edges),
            ] {
                differing.extend(multiset_difference(what, &want, &got));
            }
        }
        (Err(problem), _) => differing.push(format!("Jekyll's graph: {problem}")),
        (_, Err(problem)) => differing.push(format!("Folia's graph: {problem}")),
    }
    let passed = report(
        &format!("{GRAPH}: the nodes and edges are Jekyll's"),
        &differing,
    );
    if passed {
        println!(
            "  the file is {}byte-identical to Jekyll's.",
            if jekyll == folia { "" } else { "not " }
        );
    }
    passed
}

/// A graph's nodes and edges, each written as sorted JSON.
fn members(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>), String> {
    let graph: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let list = |key: &str| -> Result<Vec<String>, String> {
        let items = graph[key]
            .as_array()
            .ok_or_else(|| format!("no {key} list"))?;
        let mut items: Vec<String> = items.iter().map(Value::to_string).collect();
        items.sort();
        Ok(items)
    };
    Ok((list("nodes")?, list("edges")?))
}

/// What one sorted list has more of than the other, member by member.
fn multiset_difference(what: &str, want: &[String], got: &[String]) -> Vec<String> {
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for item in want {
        counts.entry(item).or_default().0 += 1;
    }
    for item in got {
        counts.entry(item).or_default().1 += 1;
    }
    counts
        .into_iter()
        .filter(|(_, (jekyll, folia))| jekyll != folia)
        .map(|(item, (jekyll, folia))| {
            format!("{what}: {item}: Jekyll has {jekyll}, Folia {folia}")
        })
        .collect()
}

/// Checks that each built document has a warning per broken link on Jekyll's page of it, and
/// that every warning concerns a built document. `jekyll_page` reads Jekyll's page at an
/// output path.
pub fn check_warnings(built: &Built, jekyll_page: &dyn Fn(&str) -> Option<String>) -> bool {
    let mut differing = Vec::new();
    for entry in &built.entries {
        let warnings = built
            .warnings
            .iter()
            .filter(|warning| warning.source.as_ref() == Some(&entry.doc))
            .count();
        let path = entry.route.output_path();
        let Some(page) = jekyll_page(&path) else {
            continue;
        };
        let broken = elements(&dom::normalize_document(&page))
            .into_iter()
            .filter(|node| has_class(node, "broken-link"))
            .count();
        if broken != warnings {
            differing.push(format!(
                "{}: Jekyll's page has {broken} broken links, Folia gave {warnings} warnings",
                entry.doc
            ));
        }
    }
    for warning in &built.warnings {
        let built_doc = built
            .entries
            .iter()
            .any(|entry| warning.source.as_ref() == Some(&entry.doc));
        if !built_doc {
            differing.push(format!("a warning about no document built: {warning}"));
        }
    }
    report(
        &format!(
            "one warning per broken link of the pages built ({} warnings)",
            built.warnings.len()
        ),
        &differing,
    )
}

#[cfg(test)]
mod tests {
    use folia_core::content::{Diagnostic, DocId, Route};
    use folia_core::publish::EntrySummary;

    use super::*;

    fn built(warnings: usize) -> Built {
        let doc = DocId::new("_digital_garden/a.md");
        Built {
            entries: vec![EntrySummary {
                doc: doc.clone(),
                route: Route::html("/dg/a").unwrap(),
                published: None,
                modified: None,
                body_html: String::new(),
                redirect_from: Vec::new(),
            }],
            warnings: (0..warnings)
                .map(|_| Diagnostic::in_doc(&doc, "broken"))
                .collect(),
            ..Built::default()
        }
    }

    #[test]
    fn each_broken_link_jekyll_rendered_needs_a_warning() {
        let page = |_: &str| Some(r#"<p><span class="broken-link">x</span></p>"#.to_owned());
        assert!(check_warnings(&built(1), &page));
        assert!(!check_warnings(&built(0), &page));
        assert!(!check_warnings(&built(2), &page));
        let mut stray = built(1);
        stray.warnings.push(Diagnostic::site("about nothing built"));
        assert!(!check_warnings(&stray, &page));
    }

    const GRAPH_JSON: &str = r#"{"nodes":[{"id":"/dg/a","path":"/dg/a","label":"A"},{"id":"/dg/b","path":"/dg/b","label":"B"}],"edges":[{"source":"/dg/b","target":"/dg/a"},{"source":"/dg/b","target":"/dg/a"}]}"#;

    #[test]
    fn a_graph_in_another_order_or_with_other_key_order_is_the_same() {
        let reordered = r#"{"edges":[{"target":"/dg/a","source":"/dg/b"},{"source":"/dg/b","target":"/dg/a"}],"nodes":[{"id":"/dg/b","path":"/dg/b","label":"B"},{"id":"/dg/a","path":"/dg/a","label":"A"}]}"#;
        assert!(check_graph(GRAPH_JSON.as_bytes(), reordered.as_bytes()));
    }

    #[test]
    fn a_missing_repeated_edge_or_a_changed_label_differs() {
        let one_edge = GRAPH_JSON.replace(r#",{"source":"/dg/b","target":"/dg/a"}]"#, "]");
        assert!(!check_graph(GRAPH_JSON.as_bytes(), one_edge.as_bytes()));
        let label = GRAPH_JSON.replace(r#""label":"B""#, r#""label":"C""#);
        assert!(!check_graph(GRAPH_JSON.as_bytes(), label.as_bytes()));
        assert!(!check_graph(GRAPH_JSON.as_bytes(), b"[]"));
    }

    #[test]
    fn a_graph_only_one_side_wrote_differs_and_none_is_no_difference() {
        let graph = Some(GRAPH_JSON.as_bytes());
        assert!(!check_graph_file(graph, None));
        assert!(!check_graph_file(None, graph));
        assert!(check_graph_file(None, None));
        assert!(check_graph_file(graph, graph));
    }
}
