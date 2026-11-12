//! The whole-document DOM comparison of plan v12, §5.1: every HTML document is parsed and
//! normalized by the §5.1 rules ([`dom::DOCUMENT`]), the two trees are compared node by node, and
//! **every** difference is reported, each by its node path and kind. Nothing is suppressed: a
//! report is judged ([`Report::judge`]) by the DOM entries of the deviations ledger ([`ledger`],
//! §5.2) and the hard assertions ([`assertions`]), which give each difference a verdict and decide
//! whether the comparison passes, but remove nothing from the report. `parity dom` reports only.
//! `dom-report.json` keeps both values of every difference in full; only the Markdown summary
//! shortens them ([`VALUE_LENGTH`]).
//!
//! **Folia's files are identified by its output manifest** (a kept build's `sha256.txt`): the hash
//! it records for a page is the page's identity, which a DOM entry's `actual_sha256` is checked
//! against, and a judged report fails if a page compared is not in the manifest or its bytes have
//! another hash (plan v12, §5.2). The baseline has no output manifest: its files are identified by
//! the hash of their bytes.
//!
//! **Paths** start at the document's `html` element. Each step is `tag[n]`, the n-th child
//! element with that tag (1-based), or `#text[n]` / `#comment[n]`, the n-th text or comment child;
//! an attribute ends the path as `@name`; the doctype is `#doctype`. A node present on one side
//! only has the index it has on that side. Example: `html[1]/head[1]/meta[3]@content`.
//!
//! **Children are aligned**, so that one missing element does not shift every later one: the
//! alignment matches as many children of the same kind (an element of the same tag, a text, a
//! comment) as possible, and among those, as many equal ones as possible. Within each gap between
//! matched children, unmatched children pair up in order: two elements as *tag differs*, two
//! texts as *text differs*, two comments as *comment differs*; the rest are missing or extra.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::markdown::dom::{self, Doctype, Node};

pub mod assertions;
pub mod ledger;

/// The kinds of difference, a fixed vocabulary (plan v12, §5.1). Jekyll's document is the
/// *expected* side and Folia's the *actual* one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// An element only the expected document has.
    ElementMissing,
    /// An element only the actual document has.
    ElementExtra,
    /// Two elements at the same place with different tags.
    TagDiffers,
    /// An attribute only the expected element has.
    AttributeMissing,
    /// An attribute only the actual element has.
    AttributeExtra,
    /// An attribute with different values.
    AttributeValueDiffers,
    /// A text node that differs, or that only one side has.
    TextDiffers,
    /// A comment that differs, or that only one side has.
    CommentDiffers,
    /// Different doctypes, or a doctype on one side only.
    DoctypeDiffers,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::ElementMissing => "element-missing",
            Kind::ElementExtra => "element-extra",
            Kind::TagDiffers => "tag-differs",
            Kind::AttributeMissing => "attribute-missing",
            Kind::AttributeExtra => "attribute-extra",
            Kind::AttributeValueDiffers => "attribute-value-differs",
            Kind::TextDiffers => "text-differs",
            Kind::CommentDiffers => "comment-differs",
            Kind::DoctypeDiffers => "doctype-differs",
        }
    }
}

/// One difference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Difference {
    pub path: String,
    pub kind: Kind,
    /// The expected side's value, in full, or `None` if it has none: an element's start tag, an
    /// attribute's value, a text or comment. A text or comment only the actual side has is the
    /// empty string here (§5.1), never `None`; a normalized text or comment is never empty.
    pub expected: Option<String>,
    /// The actual side's value, in full, as `expected` is.
    pub actual: Option<String>,
    /// For an attribute value: whether the two are equal once runs of HTML whitespace are
    /// collapsed and the ends trimmed ([`dom::collapse_and_trim`]; plan v12, §5.1's named head
    /// difference). Informative only: the difference is reported all the same.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_after_collapse_and_trim: Option<bool>,
    /// How the ledger judged it, once a ledger is applied ([`Report::judge`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<ledger::Verdict>,
}

/// How long a value may be, in characters, before the Markdown summary shortens it; the JSON
/// report keeps every value in full.
pub const VALUE_LENGTH: usize = 160;

/// Compares two whole HTML documents by the §5.1 rules, and returns every difference, in
/// document order.
pub fn compare_documents(expected: &str, actual: &str) -> Vec<Difference> {
    let (expected_doctype, expected_nodes) = dom::normalize_document_with(expected, &dom::DOCUMENT);
    let (actual_doctype, actual_nodes) = dom::normalize_document_with(actual, &dom::DOCUMENT);
    let mut out = Vec::new();
    if expected_doctype != actual_doctype {
        out.push(Difference {
            path: "#doctype".to_owned(),
            kind: Kind::DoctypeDiffers,
            expected: expected_doctype.as_ref().map(describe_doctype),
            actual: actual_doctype.as_ref().map(describe_doctype),
            equal_after_collapse_and_trim: None,
            ledger: None,
        });
    }
    compare_children(&expected_nodes, &actual_nodes, "", &mut out);
    out
}

fn describe_doctype(doctype: &Doctype) -> String {
    format!(
        "<!DOCTYPE {} \"{}\" \"{}\">",
        doctype.name, doctype.public_id, doctype.system_id
    )
}

/// What alignment matches: children of the same sort.
fn same_sort(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Element { name: x, .. }, Node::Element { name: y, .. }) => x == y,
        (Node::Text(_), Node::Text(_)) | (Node::Comment(_), Node::Comment(_)) => true,
        _ => false,
    }
}

/// The pairs `(i, j)` of `expected[i]` and `actual[j]` matched: a longest common subsequence by
/// sort, and among those, one with the most equal pairs. Ties prefer the earliest children.
fn align(expected: &[Node], actual: &[Node]) -> Vec<(usize, usize)> {
    let (n, m) = (expected.len(), actual.len());
    // score[i][j]: the best (matches, equal) over expected[i..] and actual[j..].
    let mut score = vec![vec![(0usize, 0usize); m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let mut best = score[i + 1][j].max(score[i][j + 1]);
            if same_sort(&expected[i], &actual[j]) {
                let (matches, equal) = score[i + 1][j + 1];
                let taken = (matches + 1, equal + usize::from(expected[i] == actual[j]));
                best = best.max(taken);
            }
            score[i][j] = best;
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut pairs = Vec::new();
    while i < n && j < m {
        if same_sort(&expected[i], &actual[j]) {
            let (matches, equal) = score[i + 1][j + 1];
            let taken = (matches + 1, equal + usize::from(expected[i] == actual[j]));
            if taken == score[i][j] {
                pairs.push((i, j));
                i += 1;
                j += 1;
                continue;
            }
        }
        if score[i + 1][j] >= score[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// The step naming each child in its list: `tag[n]`, `#text[n]` or `#comment[n]`.
fn steps(nodes: &[Node]) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    nodes
        .iter()
        .map(|node| {
            let label = match node {
                Node::Element { name, .. } => name.clone(),
                Node::Text(_) => "#text".to_owned(),
                Node::Comment(_) => "#comment".to_owned(),
            };
            let count = counts.entry(label.clone()).or_default();
            *count += 1;
            format!("{label}[{count}]")
        })
        .collect()
}

fn join(path: &str, step: &str) -> String {
    if path.is_empty() {
        step.to_owned()
    } else {
        format!("{path}/{step}")
    }
}

fn compare_children(expected: &[Node], actual: &[Node], path: &str, out: &mut Vec<Difference>) {
    let (expected_steps, actual_steps) = (steps(expected), steps(actual));
    let pairs = align(expected, actual);
    let (mut i, mut j) = (0, 0);
    for &(pi, pj) in pairs
        .iter()
        .chain(std::iter::once(&(expected.len(), actual.len())))
    {
        gap(
            &expected[i..pi],
            &actual[j..pj],
            &expected_steps[i..pi],
            &actual_steps[j..pj],
            path,
            out,
        );
        if pi < expected.len() && pj < actual.len() {
            compare_nodes(
                &expected[pi],
                &actual[pj],
                &join(path, &expected_steps[pi]),
                out,
            );
        }
        (i, j) = (pi + 1, pj + 1);
    }
}

/// The unmatched children between two matched ones: paired in order where they are of the same
/// kind of node, the rest missing or extra.
fn gap(
    expected: &[Node],
    actual: &[Node],
    expected_steps: &[String],
    actual_steps: &[String],
    path: &str,
    out: &mut Vec<Difference>,
) {
    let mut actual_used = vec![false; actual.len()];
    for (e, node) in expected.iter().enumerate() {
        let partner = (0..actual.len()).find(|&a| !actual_used[a] && same_class(node, &actual[a]));
        let here = join(path, &expected_steps[e]);
        match partner {
            Some(a) => {
                actual_used[a] = true;
                let kind = match node {
                    Node::Element { .. } => Kind::TagDiffers,
                    Node::Text(_) => Kind::TextDiffers,
                    Node::Comment(_) => Kind::CommentDiffers,
                };
                out.push(difference(
                    here,
                    kind,
                    Some(describe(node)),
                    Some(describe(&actual[a])),
                ));
            }
            None => out.push(only_one_side(node, here, true)),
        }
    }
    for (a, node) in actual.iter().enumerate() {
        if !actual_used[a] {
            out.push(only_one_side(node, join(path, &actual_steps[a]), false));
        }
    }
}

/// Elements pair with elements, texts with texts, comments with comments.
fn same_class(a: &Node, b: &Node) -> bool {
    matches!(
        (a, b),
        (Node::Element { .. }, Node::Element { .. })
            | (Node::Text(_), Node::Text(_))
            | (Node::Comment(_), Node::Comment(_))
    )
}

/// A node only one side has: an element is missing or extra, and has no value on the other side;
/// a text or comment differs from the empty string (plan v12, §5.1).
fn only_one_side(node: &Node, path: String, expected_side: bool) -> Difference {
    let (kind, other) = match (node, expected_side) {
        (Node::Element { .. }, true) => (Kind::ElementMissing, None),
        (Node::Element { .. }, false) => (Kind::ElementExtra, None),
        (Node::Text(_), _) => (Kind::TextDiffers, Some(String::new())),
        (Node::Comment(_), _) => (Kind::CommentDiffers, Some(String::new())),
    };
    let value = Some(describe(node));
    if expected_side {
        difference(path, kind, value, other)
    } else {
        difference(path, kind, other, value)
    }
}

fn compare_nodes(expected: &Node, actual: &Node, path: &str, out: &mut Vec<Difference>) {
    match (expected, actual) {
        (
            Node::Element {
                attrs: expected_attrs,
                children: expected_children,
                ..
            },
            Node::Element {
                attrs: actual_attrs,
                children: actual_children,
                ..
            },
        ) => {
            for (name, value) in expected_attrs {
                let here = format!("{path}@{name}");
                match actual_attrs.get(name) {
                    None => out.push(difference(
                        here,
                        Kind::AttributeMissing,
                        Some(value.clone()),
                        None,
                    )),
                    Some(other) if other != value => out.push(Difference {
                        equal_after_collapse_and_trim: Some(
                            dom::collapse_and_trim(value) == dom::collapse_and_trim(other),
                        ),
                        ..difference(
                            here,
                            Kind::AttributeValueDiffers,
                            Some(value.clone()),
                            Some(other.clone()),
                        )
                    }),
                    Some(_) => {}
                }
            }
            for (name, value) in actual_attrs {
                if !expected_attrs.contains_key(name) {
                    out.push(difference(
                        format!("{path}@{name}"),
                        Kind::AttributeExtra,
                        None,
                        Some(value.clone()),
                    ));
                }
            }
            compare_children(expected_children, actual_children, path, out);
        }
        (Node::Text(a), Node::Text(b)) if a != b => out.push(difference(
            path.to_owned(),
            Kind::TextDiffers,
            Some(a.clone()),
            Some(b.clone()),
        )),
        (Node::Comment(a), Node::Comment(b)) if a != b => out.push(difference(
            path.to_owned(),
            Kind::CommentDiffers,
            Some(a.clone()),
            Some(b.clone()),
        )),
        _ => {}
    }
}

fn difference(
    path: String,
    kind: Kind,
    expected: Option<String>,
    actual: Option<String>,
) -> Difference {
    Difference {
        path,
        kind,
        expected,
        actual,
        equal_after_collapse_and_trim: None,
        ledger: None,
    }
}

/// A node as the report shows it, in full: an element's start tag with its attributes, the text,
/// or the comment.
fn describe(node: &Node) -> String {
    match node {
        Node::Element { name, attrs, .. } => {
            let mut out = format!("<{name}");
            for (key, value) in attrs {
                let _ = write!(out, " {key}=\"{value}\"");
            }
            out.push('>');
            out
        }
        Node::Text(text) => text.clone(),
        Node::Comment(text) => format!("<!--{text}-->"),
    }
}

/// `text` cut to [`VALUE_LENGTH`] characters, with `…` if it was longer: for the Markdown summary.
fn shorten(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(VALUE_LENGTH).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

/// Whether a file is compared, or found on one side only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// On both sides, and compared.
    Compared,
    /// Only the expected tree has it.
    Missing,
    /// Only the actual tree has it.
    Extra,
}

/// One HTML file of the comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileReport {
    /// Its path below the trees' roots, with `/` separators.
    pub path: String,
    pub status: Status,
    /// The SHA-256 of the expected side's file, or `None` if it has none.
    pub expected_sha256: Option<String>,
    /// The SHA-256 of the actual side's file's bytes, or `None` if it has none.
    pub actual_sha256: Option<String>,
    /// The SHA-256 the actual side's output manifest records for the file, when that side has a
    /// manifest ([`Side::manifest`]) and lists it: the identity a DOM ledger entry's
    /// `actual_sha256` is checked against (plan v12, §5.2). A judged report fails when it is not
    /// [`FileReport::actual_sha256`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_manifest_sha256: Option<String>,
    pub differences: Vec<Difference>,
}

/// Which HTML files of two trees to compare.
pub enum Scope<'a> {
    /// Every `.html` file either tree has.
    All,
    /// These paths only, such as the pages a build wrote.
    Only(&'a [String]),
}

/// Compares the HTML files of two output trees, matched by path, sorted by path.
///
/// # Errors
///
/// Fails when a tree or file cannot be read.
pub fn compare_trees(
    expected: &Path,
    actual: &Path,
    scope: &Scope,
) -> Result<Vec<FileReport>, String> {
    let paths: Vec<String> = match scope {
        Scope::All => {
            let mut all = html_files(expected)?;
            all.extend(html_files(actual)?);
            all.sort();
            all.dedup();
            all
        }
        Scope::Only(paths) => {
            let mut paths = paths.to_vec();
            paths.sort();
            paths.dedup();
            paths
        }
    };
    let read = |root: &Path, path: &str| -> Result<Option<Vec<u8>>, String> {
        let file = root.join(path);
        if !file.is_file() {
            return Ok(None);
        }
        std::fs::read(&file)
            .map(Some)
            .map_err(|e| format!("{}: {e}", file.display()))
    };
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    paths
        .into_iter()
        .map(|path| {
            let (a, b) = (read(expected, &path)?, read(actual, &path)?);
            let (status, differences) = match (&a, &b) {
                (Some(a), Some(b)) => (Status::Compared, compare_documents(&text(a), &text(b))),
                (None, Some(_)) => (Status::Extra, Vec::new()),
                // A listed file neither side has is missing from what was expected.
                (Some(_) | None, None) => (Status::Missing, Vec::new()),
            };
            Ok(FileReport {
                path,
                status,
                expected_sha256: a.as_deref().map(|bytes| hex(&Sha256::digest(bytes))),
                actual_sha256: b.as_deref().map(|bytes| hex(&Sha256::digest(bytes))),
                actual_manifest_sha256: None,
                differences,
            })
        })
        .collect()
}

fn html_files(root: &Path) -> Result<Vec<String>, String> {
    Ok(crate::builds::manifest_paths(root)?
        .into_iter()
        .filter(|path| {
            Path::new(path).extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")
            })
        })
        .collect())
}

/// What a side of the comparison is: its record and manifest, by their hashes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Side {
    /// `jekyll` or `folia`, or what the caller names it.
    pub label: String,
    /// The record of its build (`build.json` for a kept build, `BASELINE.json` for a baseline),
    /// by name and SHA-256; `None` when there is none.
    pub record: Option<crate::builds::Hashed>,
    /// The SHA-256 of its manifest: of a kept build's `sha256.txt`, or of one computed in that
    /// format for any other tree.
    pub manifest_sha256: String,
    /// A kept build's manifest, `sha256.txt`, as path to the SHA-256 it records; `None` for any
    /// other tree (a baseline has none). Not written in the report, which has each page's entry.
    #[serde(skip)]
    pub manifest: Option<BTreeMap<String, String>>,
}

impl Side {
    /// Identifies the tree `root`: a kept build's `site/` (its `build.json` and `sha256.txt`
    /// beside it), a baseline's `site/` (`BASELINE.json` beside it), or any other directory.
    ///
    /// # Errors
    ///
    /// Fails when a file cannot be read.
    pub fn of(label: &str, root: &Path) -> Result<Self, String> {
        let parent = root.parent().map_or_default(Path::to_owned);
        let is_site = root.file_name().is_some_and(|name| name == "site");
        let record = ["build.json", "BASELINE.json"]
            .iter()
            .map(|name| parent.join(name))
            .find(|file| is_site && file.is_file())
            .map(|file| hash_file(&file))
            .transpose()?;
        let kept_manifest = parent.join("sha256.txt");
        let (manifest_sha256, manifest) = if is_site && kept_manifest.is_file() {
            (
                hash_file(&kept_manifest)?.sha256,
                Some(crate::builds::read_manifest(&kept_manifest)?),
            )
        } else {
            let manifest = crate::builds::manifest(root)?;
            let text = crate::builds::manifest_text(&manifest);
            (hex(&Sha256::digest(text.as_bytes())), None)
        };
        Ok(Side {
            label: label.to_owned(),
            record,
            manifest_sha256,
            manifest,
        })
    }
}

fn hash_file(file: &Path) -> Result<crate::builds::Hashed, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(crate::builds::Hashed {
        path: file
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned()),
        sha256: hex(&Sha256::digest(&bytes)),
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// A comparison's report, written as `dom-report.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The report's format version: 3 since values are kept in full, a text or comment one side
    /// lacks is empty there, Folia's pages carry their manifest hash and every stale entry fails
    /// (2: DOM entries bind both files of an output).
    pub format: u32,
    /// `report-only` when no ledger is applied; `judged` when it is ([`Report::judge`]).
    pub mode: &'static str,
    pub expected: Side,
    pub actual: Side,
    pub files: Vec<FileReport>,
    pub totals: Totals,
    /// The ledger as applied, when it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<ledger::Applied>,
    /// The hard assertions' results, when judged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<assertions::Outcome>,
    /// When judged, each page compared whose bytes are not what Folia's output manifest records
    /// for it, or that the manifest does not list, one line each.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub manifest_mismatches: Vec<String>,
    /// Whether the comparison passes, when judged: every difference accepted by the ledger, no
    /// entry stale, no ledger problem, no file missing or extra, no manifest mismatch, every
    /// assertion holding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passed: Option<bool>,
}

/// Counts over a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Totals {
    pub files: usize,
    pub compared: usize,
    pub missing: usize,
    pub extra: usize,
    pub differing: usize,
    pub differences: usize,
    /// Differences by kind, by the kind's name.
    pub by_kind: BTreeMap<&'static str, usize>,
}

impl Report {
    /// A report-only comparison of `files`; each page's manifest hash is taken from `actual`'s
    /// manifest, when it has one.
    pub fn new(expected: Side, actual: Side, mut files: Vec<FileReport>) -> Self {
        if let Some(manifest) = &actual.manifest {
            for file in &mut files {
                file.actual_manifest_sha256 = manifest.get(&file.path).cloned();
            }
        }
        let count = |status| files.iter().filter(|file| file.status == status).count();
        let mut by_kind = BTreeMap::new();
        for difference in files.iter().flat_map(|file| &file.differences) {
            *by_kind.entry(difference.kind.name()).or_default() += 1;
        }
        let totals = Totals {
            files: files.len(),
            compared: count(Status::Compared),
            missing: count(Status::Missing),
            extra: count(Status::Extra),
            differing: files
                .iter()
                .filter(|file| !file.differences.is_empty())
                .count(),
            differences: files.iter().map(|file| file.differences.len()).sum(),
            by_kind,
        };
        Report {
            format: 3,
            mode: "report-only",
            expected,
            actual,
            files,
            totals,
            ledger: None,
            assertions: Vec::new(),
            manifest_mismatches: Vec::new(),
            passed: None,
        }
    }

    /// Judges the report: applies `ledger` to every difference and records `assertions`, so that
    /// the report decides whether the comparison passes. Nothing is removed from the report: an
    /// accepted difference stays, with its verdict.
    #[must_use]
    pub fn judge(mut self, ledger: &ledger::Ledger, assertions: Vec<assertions::Outcome>) -> Self {
        let applied = ledger::apply(ledger, &self);
        for (file, verdicts) in self.files.iter_mut().zip(&applied.verdicts) {
            for (difference, verdict) in file.differences.iter_mut().zip(verdicts) {
                difference.ledger = Some(verdict.clone());
            }
        }
        self.manifest_mismatches = self.files.iter().filter_map(manifest_mismatch).collect();
        let passed = applied.passed
            && self.totals.missing == 0
            && self.totals.extra == 0
            && self.manifest_mismatches.is_empty()
            && assertions.iter().all(|outcome| outcome.passed);
        self.mode = "judged";
        self.ledger = Some(applied);
        self.assertions = assertions;
        self.passed = Some(passed);
        self
    }

    /// The report as pretty JSON, ending in a newline.
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a report is plain data") + "\n"
    }

    /// A short Markdown summary: the sides, the totals, and per file its differences, at most
    /// `limit` each (the JSON report has them all).
    pub fn summary(&self, limit: usize) -> String {
        let mut out = match self.passed {
            None => String::from("# Whole-document DOM comparison (report only)\n\n"),
            Some(true) => String::from("# Whole-document DOM comparison: passes\n\n"),
            Some(false) => String::from("# Whole-document DOM comparison: FAILS\n\n"),
        };
        for side in [&self.expected, &self.actual] {
            let record = side.record.as_ref().map_or("none".to_owned(), |record| {
                format!("`{}` `{}`", record.path, record.sha256)
            });
            let _ = writeln!(
                out,
                "- {}: record {record}; manifest `{}`",
                side.label, side.manifest_sha256
            );
        }
        let totals = &self.totals;
        let _ = writeln!(
            out,
            "\n{} files: {} compared, {} missing, {} extra; {} differences in {} files.\n",
            totals.files,
            totals.compared,
            totals.missing,
            totals.extra,
            totals.differences,
            totals.differing
        );
        for (kind, count) in &totals.by_kind {
            let _ = writeln!(out, "- {kind}: {count}");
        }
        for file in &self.files {
            let _ = writeln!(
                out,
                "\n## `{}`: {} differences ({:?})\n",
                file.path,
                file.differences.len(),
                file.status
            );
            for difference in file.differences.iter().take(limit) {
                let _ = writeln!(
                    out,
                    "- `{}` {}: {} → {}{}{}",
                    difference.path,
                    difference.kind.name(),
                    quote(difference.expected.as_deref()),
                    quote(difference.actual.as_deref()),
                    match difference.equal_after_collapse_and_trim {
                        Some(true) => " (equal after collapse and trim)",
                        _ => "",
                    },
                    verdict_note(difference.ledger.as_ref())
                );
            }
            if file.differences.len() > limit {
                let _ = writeln!(
                    out,
                    "- … and {} more in `dom-report.json`",
                    file.differences.len() - limit
                );
            }
        }
        self.judgement(&mut out);
        out
    }

    /// The summary's ledger and assertion sections, once judged.
    fn judgement(&self, out: &mut String) {
        if let Some(applied) = &self.ledger {
            let _ = writeln!(
                *out,
                "\n## Ledger `{}` (`{}`)\n",
                applied.path, applied.sha256
            );
            for entry in &applied.entries {
                let _ = writeln!(
                    *out,
                    "- {} ({:?}): explains {} differences{}; follow-up {}",
                    entry.id,
                    entry.status,
                    entry.explained.len(),
                    if entry.stale { ", STALE" } else { "" },
                    entry.follow_up_status
                );
                if entry.expected_build != ledger::Binding::AsReviewed {
                    let _ = writeln!(
                        *out,
                        "  - the baseline's record is {}",
                        ledger::binding_name(entry.expected_build)
                    );
                }
                for output in &entry.outputs {
                    let _ = writeln!(
                        *out,
                        "  - `{}`: the baseline's file {}, Folia's {}",
                        output.path,
                        ledger::binding_name(output.expected),
                        ledger::binding_name(output.actual)
                    );
                }
                for item in &entry.unused {
                    let _ = writeln!(
                        *out,
                        "  - unused on `{}`: `{}` {}",
                        item.output, item.path, item.kind
                    );
                }
            }
            for problem in &applied.problems {
                let _ = writeln!(*out, "- problem: {problem}");
            }
            if !applied.final_acceptance.is_empty() {
                let _ = writeln!(
                    *out,
                    "\nFinal acceptance (F13) would refuse, though this run does not fail on it:\n"
                );
                for line in &applied.final_acceptance {
                    let _ = writeln!(*out, "- {line}");
                }
            }
        }
        for mismatch in &self.manifest_mismatches {
            let _ = writeln!(*out, "- FAILS: Folia's output manifest: {mismatch}");
        }
        if !self.assertions.is_empty() {
            let failed: Vec<_> = self.assertions.iter().filter(|o| !o.passed).collect();
            let _ = writeln!(
                *out,
                "\n## Assertions: {} checked, {} failed",
                self.assertions.len(),
                failed.len()
            );
            if !failed.is_empty() {
                out.push('\n');
            }
            for outcome in failed {
                let _ = writeln!(
                    *out,
                    "- FAILS: `{}` {}: {}",
                    outcome.file, outcome.assertion, outcome.detail
                );
            }
        }
    }

    /// Writes `dom-report.json` and `dom-report.md` into `dir`, never over existing files.
    ///
    /// # Errors
    ///
    /// Fails when either file exists or cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
        let json = dir.join("dom-report.json");
        let summary = dir.join("dom-report.md");
        for file in [&json, &summary] {
            if file.symlink_metadata().is_ok() {
                return Err(format!(
                    "{} already exists, and is never overwritten",
                    file.display()
                ));
            }
        }
        std::fs::write(&json, self.json()).map_err(|e| format!("{}: {e}", json.display()))?;
        std::fs::write(&summary, self.summary(SUMMARY_LIMIT))
            .map_err(|e| format!("{}: {e}", summary.display()))?;
        Ok((json, summary))
    }
}

/// Why a page's bytes are not the file Folia's output manifest records, if they are not: the page is
/// not listed, or listed with another hash. A page Folia did not write has nothing to check.
fn manifest_mismatch(file: &FileReport) -> Option<String> {
    let bytes = file.actual_sha256.as_deref()?;
    match file.actual_manifest_sha256.as_deref() {
        Some(recorded) if recorded == bytes => None,
        Some(recorded) => Some(format!(
            "{}: the manifest records {recorded}, the file's bytes hash to {bytes}",
            file.path
        )),
        None => Some(format!("{}: the manifest does not list it", file.path)),
    }
}

fn verdict_note(verdict: Option<&ledger::Verdict>) -> String {
    match verdict {
        None => String::new(),
        Some(ledger::Verdict::Accepted(id)) => format!(" [accepted: {id}]"),
        Some(ledger::Verdict::Pending(id)) => format!(" [PENDING: {id} is not accepted]"),
        Some(ledger::Verdict::Unexplained) => " [UNEXPLAINED]".to_owned(),
        Some(ledger::Verdict::Overlap(ids)) => format!(" [OVERLAP: {}]", ids.join(", ")),
        Some(ledger::Verdict::Stale(ids)) => format!(
            " [STALE: {} not reviewed against these files]",
            ids.join(", ")
        ),
    }
}

/// How many differences per file the summary lists.
pub const SUMMARY_LIMIT: usize = 25;

/// A value in the summary, on one line and shortened ([`VALUE_LENGTH`]): backticks become `'`, and
/// line breaks `\n` and `\r` (the JSON report keeps the exact values, in full).
fn quote(value: Option<&str>) -> String {
    value.map_or("(none)".to_owned(), |value| {
        let value = shorten(value)
            .replace('`', "'")
            .replace('\r', "\\r")
            .replace('\n', "\\n");
        format!("`{value}`")
    })
}

/// `parity dom`: compares two files or two trees, prints the summary, and writes the report into
/// `report_dir` (created if need be) when given. Returns whether they are equal.
///
/// # Errors
///
/// Fails when an input cannot be read, the inputs are a file and a directory, or the report
/// cannot be written.
pub fn run(expected: &Path, actual: &Path, report_dir: Option<&Path>) -> Result<bool, String> {
    let files = match (expected.is_dir(), actual.is_dir()) {
        (true, true) => compare_trees(expected, actual, &Scope::All)?,
        (false, false) => {
            let read = |file: &Path| {
                std::fs::read(file)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .map_err(|e| format!("{}: {e}", file.display()))
            };
            let (a, b) = (read(expected)?, read(actual)?);
            vec![FileReport {
                path: actual
                    .file_name()
                    .map_or_default(|name| name.to_string_lossy().into_owned()),
                status: Status::Compared,
                expected_sha256: Some(hex(&Sha256::digest(a.as_bytes()))),
                actual_sha256: Some(hex(&Sha256::digest(b.as_bytes()))),
                actual_manifest_sha256: None,
                differences: compare_documents(&a, &b),
            }]
        }
        _ => return Err("compare two files or two directories, not one of each".to_owned()),
    };
    let side = |label: &str, path: &Path| {
        if path.is_dir() {
            Side::of(label, path)
        } else {
            let file = hash_file(path)?;
            Ok(Side {
                label: label.to_owned(),
                record: None,
                manifest_sha256: file.sha256,
                manifest: None,
            })
        }
    };
    let report = Report::new(side("expected", expected)?, side("actual", actual)?, files);
    print!("{}", report.summary(SUMMARY_LIMIT));
    if let Some(dir) = report_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        report.write(dir)?;
    }
    let totals = &report.totals;
    Ok(totals.differences == 0 && totals.missing == 0 && totals.extra == 0)
}

#[cfg(test)]
mod tests;
