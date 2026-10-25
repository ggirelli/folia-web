//! Markdown parity (plan v5, §10 spike 1): Folia's body HTML and excerpts against the HTML
//! kramdown made for the baseline, for every Markdown document of the site.
//!
//! Both sides are normalized (`dom`), and each document is reported as *identical*, *equal
//! after normalization*, or *differs*, with the first differing node. The gates:
//!
//! - **G1** heading IDs equal, in order;
//! - **G2** `href`/`src` multisets equal, and classes, targets and titles on the same elements;
//! - **G3** text equal;
//! - **G4** at least 95% of documents equal after normalization (a decision gate);
//! - **G5** the excerpt values templates show equal (`liquid`);
//! - **G6** code text equal, and Rouge's token classes matched per character (reported).
//!
//! A failure is excused only by an accepted entry of the deviation ledger (`ledger`); one a
//! proposed entry explains is reported as pending.

pub mod dom;
mod ledger;
mod liquid;
mod report;

use std::collections::BTreeMap;
use std::path::Path;

use folia_core::content::markdown::{LinkResolver, excerpt_html, to_html};
use folia_core::content::{DocId, split_front_matter};
use folia_garden::WikiLinks;

use crate::baseline::{self, Row};
use crate::routes::{check_checkout, short};

/// What to check, and where to write what was found.
pub struct Options<'a> {
    /// The Jekyll site, at the commit the baseline was built from.
    pub site: &'a Path,
    /// A baseline made by `scripts/baseline.sh`.
    pub baseline: &'a Path,
    /// The deviation ledger.
    pub ledger: &'a Path,
    /// Only this source.
    pub only: Option<&'a str>,
    /// Where to write the full report.
    pub report: Option<&'a Path>,
    /// Where to write both sides' HTML of what differs.
    pub dump: Option<&'a Path>,
}

/// How a document's HTML compares.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Status {
    Identical,
    Equal,
    Differs,
}

/// One document's results.
struct DocReport {
    source: String,
    status: Status,
    /// Gate failures, by gate.
    failures: BTreeMap<&'static str, Vec<String>>,
    /// For a garden note without an excerpt separator, whose excerpt is its body: the
    /// `resolved excerpt` check of D-003.
    resolved_excerpt: Option<ResolvedExcerpt>,
    code: Vec<CodeReport>,
}

/// D-003's check of a garden note's excerpt. Jekyll took excerpts before resolving wiki-links;
/// Folia resolves them, as in the body. A note without a separator has its body as its excerpt,
/// so Jekyll's excerpt and body HTML differ only by that defect. A difference is the deviation
/// only when all three hold.
#[derive(Clone, Copy)]
struct ResolvedExcerpt {
    /// Resolving wiki-links changes Folia's excerpt: it has some, outside code.
    has_wiki_links: bool,
    /// Jekyll's excerpt values differ from those of Jekyll's body: the defect shows.
    jekyll_unresolved: bool,
    /// Folia's excerpt values equal those of Jekyll's body, where the plugin had resolved them.
    matches_body: bool,
}

impl ResolvedExcerpt {
    fn holds(self) -> bool {
        self.has_wiki_links && self.jekyll_unresolved && self.matches_body
    }
}

/// One code block's results.
struct CodeReport {
    /// The language kramdown gave it, if it is a Rouge block.
    lang: Option<String>,
    text_equal: bool,
    /// Characters, not whitespace, with the same token class on both sides, and how many there are.
    agreeing: usize,
    counted: usize,
    /// Characters by (Rouge's class, Folia's) where they differ.
    confusions: BTreeMap<(String, String), usize>,
}

/// Checks Folia's Markdown against the baseline, prints a summary, and writes the full report
/// and the differing HTML if asked. Returns whether every gate passed, with no failure left
/// unexplained or explained only by a proposed deviation.
///
/// # Errors
///
/// Fails when the baseline, the ledger or a source cannot be read, or the site checkout is not
/// commit the baseline was built from.
pub fn run(options: &Options) -> Result<bool, String> {
    let baseline = baseline::load(options.baseline)?;
    check_checkout(options.site, &baseline.commit, baseline.diff.as_deref())?;
    let deviations = ledger::load(options.ledger)?;
    let dump = options.baseline.join("dump");
    let read = |path: &str| {
        std::fs::read_to_string(dump.join(path)).map_err(|e| format!("dump/{path}: {e}"))
    };

    let documents: Vec<&Row> = baseline
        .rows
        .iter()
        .filter(|row| row.written && row.content.is_some())
        .filter(|row| row.source.as_deref().is_some_and(is_markdown))
        .collect();
    let links = wiki_links(&documents, &read)?;

    let mut reports = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in documents {
        let source = row.source.as_deref().expect("filtered on it");
        if !seen.insert(source) || options.only.is_some_and(|only| only != source) {
            continue;
        }
        let text = std::fs::read_to_string(options.site.join(source))
            .map_err(|e| format!("{source}: {e}"))?;
        let body = split_front_matter(&text).map_or(text.as_str(), |(_, body)| body);
        let garden = row.collection.as_deref() == Some("digital_garden");
        let resolver: Option<&dyn LinkResolver> = garden.then_some(&links);
        let kramdown = read(row.content.as_deref().expect("filtered on it"))?;
        let folia = to_html(body, resolver);
        let mut doc = compare(source, &kramdown, &folia);
        if let Some(dir) = options.dump
            && doc.status == Status::Differs
        {
            write_pair(dir, source, &kramdown, &folia)?;
        }
        if let Some(path) = &row.excerpt {
            let jekyll = read(path)?;
            // Folia renders an excerpt with the body's wiki-links resolver (D-003).
            let folia = excerpt_html(body, resolver);
            let micro = row.collection.as_deref() == Some("micro");
            let problems = compare_excerpts(&jekyll, &folia, micro);
            if !problems.is_empty() {
                if let Some(dir) = options.dump {
                    write_pair(dir, &format!("{source}.excerpt"), &jekyll, &folia)?;
                }
                doc.failures.insert("G5 excerpts", problems);
            }
            if garden && !body.contains("<!--more-->") {
                doc.resolved_excerpt = Some(ResolvedExcerpt {
                    has_wiki_links: excerpt_html(body, None) != folia,
                    jekyll_unresolved: excerpt_values(&jekyll) != excerpt_values(&kramdown),
                    matches_body: compare_excerpts(&kramdown, &folia, micro).is_empty(),
                });
            }
        }
        reports.push(doc);
    }

    let commit = short(&baseline.commit).to_owned();
    let (text, passed) = report::render(&reports, &deviations, &commit);
    if let Some(path) = options.report {
        std::fs::write(path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let summary = text.split("\n## Documents").next().unwrap_or(&text);
    print!("{summary}");
    Ok(passed)
}

fn write_pair(dir: &Path, source: &str, kramdown: &str, folia: &str) -> Result<(), String> {
    let base = dir.join(source);
    let parent = base.parent().expect("a source has a directory");
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    for (side, html) in [("kramdown", kramdown), ("folia", folia)] {
        let path = dir.join(format!("{source}.{side}.html"));
        std::fs::write(&path, html).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn is_markdown(source: &str) -> bool {
    Path::new(source)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

/// The garden's notes as the Jekyll plugin indexed them: Jekyll's titles, in manifest order.
fn wiki_links(
    documents: &[&Row],
    read: &dyn Fn(&str) -> Result<String, String>,
) -> Result<WikiLinks, String> {
    let mut notes = Vec::new();
    for row in documents
        .iter()
        .filter(|row| row.collection.as_deref() == Some("digital_garden"))
    {
        let meta = row
            .meta
            .as_deref()
            .ok_or("a garden note has no meta dump")?;
        let meta: serde_json::Value =
            serde_json::from_str(&read(meta)?).map_err(|e| format!("{meta}: {e}"))?;
        let title = meta["title"].as_str().unwrap_or_default().to_owned();
        let id = DocId::new(row.source.clone().expect("documents have sources"));
        notes.push((title, id, row.url.clone()));
    }
    Ok(WikiLinks::new(notes.iter().map(|(title, id, url)| {
        (title.as_str(), id, url.as_str())
    })))
}

fn compare(source: &str, kramdown: &str, folia: &str) -> DocReport {
    let (a, b) = (dom::normalize(kramdown), dom::normalize(folia));
    let first_difference = dom::first_difference(&a, &b);
    let status = if kramdown == folia {
        Status::Identical
    } else if first_difference.is_none() {
        Status::Equal
    } else {
        Status::Differs
    };
    let mut failures = BTreeMap::new();
    let mut gate = |name, jekyll: Vec<String>, ours: Vec<String>| {
        if jekyll != ours {
            failures.insert(name, sequence_difference(&jekyll, &ours));
        }
    };
    gate("G1 heading IDs", dom::heading_ids(&a), dom::heading_ids(&b));
    gate(
        "G2 link targets",
        dom::link_targets(&a),
        dom::link_targets(&b),
    );
    gate(
        "G2 attributes",
        dom::placed_attributes(&a),
        dom::placed_attributes(&b),
    );
    gate("G3 text", vec![dom::text(&a)], vec![dom::text(&b)]);
    if let Some(difference) = &first_difference {
        failures.insert("G4 structure", vec![difference.clone()]);
    }
    let code = compare_code(kramdown, folia, &mut failures);
    DocReport {
        source: source.to_owned(),
        status,
        failures,
        resolved_excerpt: None,
        code,
    }
}

fn compare_code(
    kramdown: &str,
    folia: &str,
    failures: &mut BTreeMap<&'static str, Vec<String>>,
) -> Vec<CodeReport> {
    let (a, b) = (dom::code_blocks(kramdown), dom::code_blocks(folia));
    if a.len() != b.len() {
        failures.insert(
            "G6 code text",
            vec![format!(
                "{} code blocks in Jekyll, {} in Folia",
                a.len(),
                b.len()
            )],
        );
    }
    let mut reports = Vec::new();
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        let text_equal = x.text == y.text;
        if !text_equal {
            failures.entry("G6 code text").or_default().push(format!(
                "block {}: {:?} / {:?}",
                i + 1,
                x.text,
                y.text
            ));
        }
        let (mut agreeing, mut counted) = (0, 0);
        let mut confusions = BTreeMap::new();
        if text_equal {
            for ((c, j), f) in x.text.chars().zip(&x.classes).zip(&y.classes) {
                if !c.is_whitespace() {
                    counted += 1;
                    if j == f {
                        agreeing += 1;
                    } else {
                        *confusions.entry((j.clone(), f.clone())).or_default() += 1;
                    }
                }
            }
        }
        reports.push(CodeReport {
            lang: x.lang.clone(),
            text_equal,
            agreeing,
            counted,
            confusions,
        });
    }
    reports
}

/// The excerpt values the templates show, on both sides: three filtered texts (the page
/// description, the listing cards, the feed summary), and for a micro entry the HTML `/micro`
/// shows as it is.
fn compare_excerpts(kramdown: &str, folia: &str, micro: bool) -> Vec<String> {
    let mut problems = Vec::new();
    for (name, view) in VIEWS {
        let (a, b) = (view(kramdown), view(folia));
        if a != b {
            problems.push(format!("{name}: {a:?} / {b:?}"));
        }
    }
    if micro
        && let Some(difference) =
            dom::first_difference(&dom::normalize(kramdown), &dom::normalize(folia))
    {
        problems.push(format!("raw excerpt: {difference}"));
    }
    problems
}

/// The three filtered values of an excerpt.
fn excerpt_values(html: &str) -> Vec<String> {
    VIEWS.iter().map(|(_, view)| view(html)).collect()
}

/// A template's name for a value, and how it computes it from the excerpt.
type View = (&'static str, fn(&str) -> String);

/// The filtered excerpt values the templates show: the page description, the listing cards and
/// the feed summary.
const VIEWS: [View; 3] = [
    ("strip_html | strip_newlines | truncate: 160", |x| {
        liquid::truncate(&liquid::strip_newlines(&liquid::strip_html(x)), 160)
    }),
    ("strip_html | truncatewords: 30", |x| {
        liquid::truncatewords(&liquid::strip_html(x), 30)
    }),
    ("strip_html | normalize_whitespace", |x| {
        liquid::normalize_whitespace(&liquid::strip_html(x))
    }),
];

/// The first place two sequences differ.
fn sequence_difference(jekyll: &[String], folia: &[String]) -> Vec<String> {
    let i = jekyll
        .iter()
        .zip(folia)
        .position(|(a, b)| a != b)
        .unwrap_or(jekyll.len().min(folia.len()));
    vec![format!(
        "item {} of {} / {}: {:?} / {:?}",
        i + 1,
        jekyll.len(),
        folia.len(),
        jekyll.get(i),
        folia.get(i)
    )]
}
