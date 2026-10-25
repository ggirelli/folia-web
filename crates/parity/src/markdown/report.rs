//! The Markdown parity report: each gate's verdict, the highlighting agreement, and every
//! document that differs or fails a gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use super::ledger::Entry;
use super::{CodeReport, DocReport, ResolvedExcerpt, Status};

/// The hard gates, and G4, the decision gate.
const GATES: [&str; 7] = [
    "G1 heading IDs",
    "G2 link targets",
    "G2 attributes",
    "G3 text",
    "G4 structure",
    "G5 excerpts",
    "G6 code text",
];

/// What explains a gate failure.
enum Explained<'a> {
    No,
    By(&'a Entry),
}

fn explanation<'a>(doc: &DocReport, gate: &str, ledger: &'a [Entry]) -> Explained<'a> {
    ledger
        .iter()
        .find(|entry| {
            entry.active()
                && entry.sources.contains(&doc.source)
                && entry.markdown_gates.iter().any(|g| g == gate)
                && match entry.markdown_check.as_deref() {
                    Some("resolved excerpt") => {
                        doc.resolved_excerpt.is_some_and(ResolvedExcerpt::holds)
                    }
                    Some(_) => false,
                    None => true,
                }
        })
        .map_or(Explained::No, Explained::By)
}

/// The sources an active entry lists that this run checked but whose differences it does not
/// explain: an entry covers exactly the documents that show it.
fn unused_sources<'a>(reports: &[DocReport], ledger: &'a [Entry]) -> Vec<(&'a str, &'a str)> {
    let mut unused = Vec::new();
    for entry in ledger
        .iter()
        .filter(|e| e.active() && !e.markdown_gates.is_empty())
    {
        for source in &entry.sources {
            let Some(doc) = reports.iter().find(|r| &r.source == source) else {
                continue;
            };
            let shown = doc.failures.keys().any(|gate| {
                matches!(explanation(doc, gate, ledger), Explained::By(e) if e.id == entry.id)
            });
            if !shown {
                unused.push((entry.id.as_str(), source.as_str()));
            }
        }
    }
    unused
}

/// The report as Markdown, and whether every gate passed with only accepted deviations.
pub fn render(reports: &[DocReport], ledger: &[Entry], commit: &str) -> (String, bool) {
    let mut out = String::new();
    let count = |status| reports.iter().filter(|r| r.status == status).count();
    let _ = writeln!(out, "# Markdown parity against {commit}\n");
    let _ = writeln!(
        out,
        "{} documents: {} identical, {} equal after normalization, {} differ.\n",
        reports.len(),
        count(Status::Identical),
        count(Status::Equal),
        count(Status::Differs)
    );
    let _ = writeln!(
        out,
        "| Gate | Failing | Unexplained | Explained by the ledger | Result |\n|-|-:|-:|-|-|"
    );
    let mut passed = true;
    for gate in GATES {
        let failing: Vec<&DocReport> = reports
            .iter()
            .filter(|r| r.failures.contains_key(gate))
            .collect();
        let mut unexplained = 0;
        let mut pending = BTreeSet::new();
        let mut accepted = BTreeSet::new();
        for doc in &failing {
            match explanation(doc, gate, ledger) {
                Explained::No => unexplained += 1,
                Explained::By(entry) if entry.accepted() => {
                    accepted.insert(entry.id.as_str());
                }
                Explained::By(entry) => {
                    pending.insert(entry.id.as_str());
                }
            }
        }
        let mut result = if unexplained > 0 {
            "FAIL".to_owned()
        } else if pending.is_empty() {
            "pass".to_owned()
        } else {
            let verb = if pending.len() == 1 { "is" } else { "are" };
            format!("pass if {} {verb} accepted", join(&pending))
        };
        if gate == "G4 structure" {
            let share = percent(reports.len() - failing.len(), reports.len());
            result = format!("{share} equal; {result}");
        }
        passed &= unexplained == 0 && pending.is_empty();
        let explained: Vec<String> = accepted
            .iter()
            .map(|id| format!("{id} (accepted)"))
            .chain(pending.iter().map(|id| format!("{id} (proposed)")))
            .collect();
        let _ = writeln!(
            out,
            "| {gate} | {} | {unexplained} | {} | {result} |",
            failing.len(),
            explained.join(", ")
        );
    }
    let structure_ok = reports
        .iter()
        .filter(|r| r.status != Status::Differs)
        .count()
        * 100
        >= reports.len() * 95;
    passed &= structure_ok;

    let unused = unused_sources(reports, ledger);
    passed &= unused.is_empty();
    for (id, source) in &unused {
        let _ = writeln!(
            out,
            "\nFAIL: {id} lists `{source}`, which shows no difference it explains."
        );
    }

    let checked: Vec<ResolvedExcerpt> = reports.iter().filter_map(|r| r.resolved_excerpt).collect();
    let count = |f: fn(&ResolvedExcerpt) -> bool| checked.iter().filter(|c| f(c)).count();
    let _ = writeln!(
        out,
        "\nGarden excerpts (D-003), for the {} notes whose excerpt is their body: {} have \
         wiki-links; Jekyll's excerpt shows something other than its body for {}; Folia's, with \
         wiki-links resolved, shows what Jekyll's body shows for {}.",
        checked.len(),
        count(|c| c.has_wiki_links),
        count(|c| c.jekyll_unresolved),
        count(|c| c.matches_body),
    );

    highlighting(&mut out, reports);
    documents(&mut out, reports, ledger);
    (out, passed)
}

fn join(ids: &BTreeSet<&str>) -> String {
    ids.iter().copied().collect::<Vec<_>>().join(" and ")
}

fn percent(part: usize, whole: usize) -> String {
    #[allow(clippy::cast_precision_loss, reason = "at most a few thousand items")]
    let share = 100.0 * part as f64 / whole.max(1) as f64;
    format!("{share:.1}%")
}

/// G6: code text, and token classes against Rouge's, by language.
fn highlighting(out: &mut String, reports: &[DocReport]) {
    let blocks: Vec<&CodeReport> = reports.iter().flat_map(|r| &r.code).collect();
    let _ = writeln!(
        out,
        "\nG6 highlighting: {} code blocks, {} with equal text. Token classes agree with Rouge's \
         on these non-whitespace characters:\n",
        blocks.len(),
        blocks.iter().filter(|b| b.text_equal).count()
    );
    let mut by_lang: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for block in &blocks {
        let lang = block
            .lang
            .clone()
            .unwrap_or_else(|| "(raw HTML pre)".to_owned());
        let entry = by_lang.entry(lang).or_default();
        entry.0 += 1;
        entry.1 += block.agreeing;
        entry.2 += block.counted;
    }
    let _ = writeln!(
        out,
        "| Language | Blocks | Characters | Agreement |\n|-|-:|-:|-:|"
    );
    let (mut agreeing, mut counted) = (0, 0);
    for (lang, (n, agree, count)) in &by_lang {
        let _ = writeln!(
            out,
            "| {lang} | {n} | {count} | {} |",
            percent(*agree, *count)
        );
        if !matches!(lang.as_str(), "text" | "plaintext" | "(raw HTML pre)") {
            agreeing += agree;
            counted += count;
        }
    }
    let _ = writeln!(
        out,
        "| **lexed by Rouge** | | {counted} | **{}** |\n",
        percent(agreeing, counted)
    );

    let mut confusions: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for block in &blocks {
        let lang = block.lang.clone().unwrap_or_default();
        for ((rouge, folia), n) in &block.confusions {
            *confusions
                .entry((lang.clone(), rouge.clone(), folia.clone()))
                .or_default() += n;
        }
    }
    let mut confusions: Vec<_> = confusions.into_iter().collect();
    confusions.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let _ = writeln!(
        out,
        "Largest disagreements (characters; an empty class is unmarked text):\n\n\
         | Language | Rouge | Folia | Characters |\n|-|-|-|-:|"
    );
    for ((lang, rouge, folia), n) in confusions.iter().take(20) {
        let _ = writeln!(out, "| {lang} | `{rouge}` | `{folia}` | {n} |");
    }
}

/// Every document that differs or fails a gate, with what explains each failure.
fn documents(out: &mut String, reports: &[DocReport], ledger: &[Entry]) {
    let _ = writeln!(out, "\n## Documents\n");
    for doc in reports
        .iter()
        .filter(|r| r.status == Status::Differs || !r.failures.is_empty())
    {
        let _ = writeln!(out, "### `{}` ({:?})\n", doc.source, doc.status);
        for (gate, problems) in &doc.failures {
            let why = match explanation(doc, gate, ledger) {
                Explained::No => String::new(),
                Explained::By(entry) => format!(" ({}, {})", entry.id, entry.status),
            };
            for problem in problems {
                let _ = writeln!(out, "- {gate}{why}: `{}`", shorten(problem));
            }
        }
        if let Some(check) = doc.resolved_excerpt
            && doc.failures.contains_key("G5 excerpts")
        {
            let _ = writeln!(
                out,
                "- D-003 check: wiki-links in the excerpt: {}; Jekyll's excerpt differs from its \
                 body: {}; Folia's excerpt shows what Jekyll's body shows: {}",
                check.has_wiki_links, check.jekyll_unresolved, check.matches_body
            );
        }
        out.push('\n');
    }
}

fn shorten(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(400).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out.replace('`', "'")
}
