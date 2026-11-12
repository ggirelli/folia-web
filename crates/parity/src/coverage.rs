//! The coverage report (work item A2.1, plan v12, §5.4): every row of the baseline's manifest gets
//! exactly one class, and a row that cannot be given one fails the run.
//!
//! - **Built and checked:** Folia writes the output, at least one applicable check compares it,
//!   and every applicable check passes.
//! - **Not built yet:** Folia does not write the output; it remains in scope.
//! - **Waived:** Folia writes the output, but an explicitly owner-approved entry waives its
//!   comparison.
//! - **Removed:** Folia does not write the output because it has been intentionally removed from
//!   the site, with explicit owner approval.
//! - **Intentional:** Folia writes the output and it differs from Jekyll's on purpose; an
//!   owner-approved entry says so. The difference stays visible: it is not parity.
//! - **Known, excluded:** Folia writes the output and it differs from Jekyll's; the difference is
//!   recorded (`status = "recorded"`, a rationale) and approved by no one. It fails the run.
//! - **Defect:** Folia writes the output and it fails a check no entry covers, or it neither
//!   writes nor plans an output Jekyll writes: an unresolved defect.
//! - **Missing evidence:** Folia writes the output, but no check compares its contents.
//!
//! An output Folia writes that is no manifest row is a problem, unless an owner-approved
//! `folia-only` entry names it: it has no Jekyll baseline, so nothing compares it, and the report
//! lists it apart from the rows.
//!
//! A run is a *clean build* when no row is unclassified, a defect or missing evidence and nothing
//! else is a problem; it has *exact parity* only when every row is built and checked with no
//! ledger-accepted difference, and Folia writes nothing a baseline lacks. Zero failed assertions
//! is a clean build at most; documented differences are not identical output.
//!
//! Enforced as: the route check ([`ROUTE`]) compares no contents, so *built and checked* needs
//! another verdict too (page checks, whole-document comparison, assertions, bytes, note graph),
//! and every verdict passing; *not built yet* needs Folia's route table to plan the output;
//! *waived* needs the output written, its bytes recorded by Folia's output manifest, and its route
//! check passing, no other; *removed* needs the output neither written nor planned.
//!
//! A waiver or a removal is a ledger entry with `check = "coverage"` and `class = "waived"` or
//! `"removed"`, listing its outputs by explicit path, each a manifest row. Only an owner approves
//! one: an `accepted` entry needs its date, a `decision` and an `accepted_by` equal to one of the
//! ledger's `owners`, besides its `rationale` (plan v12, §5.2). A `proposed` entry classifies
//! nothing: its rows are *pending*, and fail; a `withdrawn` one is ignored. The tool checks the
//! recorded approval only: that the owner gave it is for review to confirm. Nothing is waived or removed by default: a row
//! Folia neither writes nor plans, and that no accepted entry removes, is unclassified, and fails.
//!
//! Every problem fails the run: an unclassified row, an entry that is not well formed or not
//! approved, a row two entries classify, an entry listing what is no manifest row, a manifest
//! listing an output twice, an output Folia writes that is no manifest row.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::baseline::Row;

/// The `check` of a coverage entry.
pub const CHECK: &str = "coverage";

/// The route check (`parity pages`): an output Folia writes is one it planned, at Jekyll's URL and
/// output file. It compares no contents, so it alone never makes a row *built and checked*; it is
/// the one check a waived row still needs.
pub const ROUTE: &str = "route";

/// What an accepted coverage entry decides for its outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decided {
    Waived,
    Removed,
    Intentional,
    Known,
    FoliaOnly,
}

/// An entry's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Proposed,
    Accepted,
    /// Only for a `known` entry: a difference noted, not approved.
    Recorded,
    Withdrawn,
}

/// A coverage entry as written. Every field the schema knows is listed (the descriptive ones,
/// `title`, `change`, `withdrawn` and `resolution`, are free text for the reader); any other is
/// refused.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(
    dead_code,
    reason = "the descriptive fields are read so that the schema knows them, not used"
)]
struct Raw {
    #[serde(default)]
    id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    check: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    change: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    accepted: Option<String>,
    #[serde(default)]
    accepted_by: Option<String>,
    #[serde(default)]
    decision: Option<String>,
    #[serde(default)]
    withdrawn: Option<String>,
    #[serde(default)]
    resolution: Option<String>,
}

/// Who approved an entry, as the report shows it beside each row the entry classifies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Approval {
    pub entry: String,
    pub accepted: String,
    pub accepted_by: String,
    pub decision: String,
}

/// A coverage entry, read. `class` is `None`, and `problems` says why, when it names no class
/// this report knows.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub status: Option<Status>,
    pub class: Option<Decided>,
    pub outputs: Vec<String>,
    /// Set when the entry is accepted and well formed, its approval by an owner.
    pub approval: Option<Approval>,
    /// What is wrong with the entry; an entry with problems classifies nothing.
    pub problems: Vec<String>,
}

impl Entry {
    /// Whether it classifies (accepted) or claims to classify (proposed) its outputs.
    fn active(&self) -> bool {
        self.status != Some(Status::Withdrawn)
    }
}

/// The coverage entries of a ledger, and its owners.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    /// Who may accept a coverage entry: the ledger's top-level `owners`.
    pub owners: Vec<String>,
    pub entries: Vec<Entry>,
    /// What is wrong with the ledger's `owners`.
    pub problems: Vec<String>,
}

impl Ledger {
    /// The outputs whose comparison an accepted, well formed entry waives.
    pub fn waived(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .filter(|e| e.class == Some(Decided::Waived) && e.approval.is_some())
            .flat_map(|e| e.outputs.iter().cloned())
            .collect()
    }
}

/// Reads the coverage entries and owners of the ledger at `path`.
///
/// # Errors
///
/// Fails when the file cannot be read or is not TOML. Problems with an entry or with `owners` are
/// not errors: they are reported, and fail the run.
pub fn load(path: &Path) -> Result<Ledger, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Result<Ledger, String> {
    let file: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut ledger = Ledger::default();
    match file.get("owners") {
        None => {}
        Some(toml::Value::Array(owners)) => {
            for owner in owners {
                match owner.as_str() {
                    Some(owner) if !owner.trim().is_empty() => {
                        ledger.owners.push(owner.to_owned());
                    }
                    _ => ledger
                        .problems
                        .push(format!("owners: {owner} is not a name that is not blank")),
                }
            }
        }
        Some(other) => ledger
            .problems
            .push(format!("owners: {other} is not a list of names")),
    }
    let entries = match file.get("deviation") {
        Some(toml::Value::Array(entries)) => entries.as_slice(),
        _ => &[],
    };
    for (index, value) in entries.iter().enumerate() {
        if value.get("check").and_then(toml::Value::as_str) != Some(CHECK) {
            continue;
        }
        ledger.entries.push(entry(value, index, &ledger.owners));
    }
    Ok(ledger)
}

fn entry(value: &toml::Value, index: usize, owners: &[String]) -> Entry {
    let id = value
        .get("id")
        .and_then(toml::Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map_or_else(
            || format!("entry {} of the ledger", index + 1),
            str::to_owned,
        );
    let raw: Raw = match value.clone().try_into() {
        Ok(raw) => raw,
        Err(e) => {
            return Entry {
                id,
                status: None,
                class: None,
                outputs: outputs_of(value),
                approval: None,
                problems: vec![format!(
                    "not a coverage entry of the known schema: {}",
                    e.to_string().trim()
                )],
            };
        }
    };
    let mut problems = Vec::new();
    let present = |value: &Option<String>| value.as_deref().is_some_and(|v| !v.trim().is_empty());
    let status = match raw.status.as_str() {
        "proposed" => Some(Status::Proposed),
        "accepted" => Some(Status::Accepted),
        "recorded" => Some(Status::Recorded),
        "withdrawn" => Some(Status::Withdrawn),
        other => {
            problems.push(format!(
                "status {other:?} is not proposed, accepted, recorded or withdrawn"
            ));
            None
        }
    };
    let class = match raw.class.as_str() {
        "waived" => Some(Decided::Waived),
        "removed" => Some(Decided::Removed),
        "intentional" => Some(Decided::Intentional),
        "known" => Some(Decided::Known),
        "folia-only" => Some(Decided::FoliaOnly),
        other => {
            problems.push(format!(
                "class {other:?} is not a coverage classification an entry can decide (waived, \
                 removed, intentional, known or folia-only)"
            ));
            None
        }
    };
    // A difference is recorded, never approved, and only a known one is: the two never mix.
    match (class, status) {
        (Some(Decided::Known), Some(Status::Accepted)) => problems.push(
            "a known difference is recorded, not accepted: use status \"recorded\"".to_owned(),
        ),
        (Some(c), Some(Status::Recorded)) if c != Decided::Known => problems.push(
            "status \"recorded\" is for a known difference; this class needs an owner's \
             approval"
                .to_owned(),
        ),
        _ => {}
    }
    if raw.outputs.is_empty() {
        problems.push("lists no output".to_owned());
    }
    let mut seen = BTreeSet::new();
    for output in &raw.outputs {
        if output.trim().is_empty() || output.contains(['*', '?', '[', ']', '{', '}']) {
            problems.push(format!("output {output:?} is not an explicit path"));
        } else if !seen.insert(output) {
            problems.push(format!("output {output} is listed twice"));
        }
    }
    if status != Some(Status::Withdrawn) && !present(&raw.rationale) {
        problems.push("has no rationale".to_owned());
    }
    let approval = if status == Some(Status::Accepted) {
        approval(&raw, &id, owners, &mut problems)
    } else {
        None
    };
    Entry {
        id,
        status,
        class,
        outputs: raw.outputs,
        approval,
        problems,
    }
}

/// The approval of the accepted entry `raw`, when an owner gave it in full: a date, a decision,
/// and `accepted_by` one of `owners`; otherwise `None`, with each reason added to `problems`.
fn approval(
    raw: &Raw,
    id: &str,
    owners: &[String],
    problems: &mut Vec<String>,
) -> Option<Approval> {
    let present = |value: &Option<String>| value.as_deref().is_some_and(|v| !v.trim().is_empty());
    let before = problems.len();
    let date = raw.accepted.as_deref().unwrap_or("");
    if !is_date(date) {
        problems.push(format!(
            "is accepted, but `accepted` {date:?} is not a date"
        ));
    }
    if !present(&raw.decision) {
        problems.push("is accepted, but names no `decision`".to_owned());
    }
    let by = raw.accepted_by.as_deref().unwrap_or("");
    if owners.is_empty() {
        problems.push("is accepted, but the ledger names no `owners`".to_owned());
    } else if !owners.iter().any(|owner| owner == by) {
        problems.push(format!(
            "is accepted by {by:?}, who is not an owner ({}): only an owner approves a waiver or \
             a removal",
            owners.join(", ")
        ));
    }
    // Any problem of the entry, found before or here, withholds the approval.
    (problems.len() == before && before == 0).then(|| Approval {
        entry: id.to_owned(),
        accepted: date.to_owned(),
        accepted_by: by.to_owned(),
        decision: raw.decision.clone().unwrap_or_default(),
    })
}

/// The outputs an entry lists, read as leniently as possible, so that an entry of an unknown
/// schema still marks its rows.
fn outputs_of(value: &toml::Value) -> Vec<String> {
    value
        .get("outputs")
        .and_then(toml::Value::as_array)
        .map_or_default(|outputs| {
            outputs
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
}

fn is_date(value: &str) -> bool {
    value.parse::<jiff::civil::Date>().is_ok() && value.len() == 10
}

/// Each check's verdict on each output it compared, by output path and check name; a check that
/// compares an output more than once passes it only when every time it does.
#[derive(Clone, Debug, Default)]
pub struct Checks(
    BTreeMap<String, BTreeMap<&'static str, bool>>,
    BTreeMap<String, usize>,
);

impl Checks {
    /// Notes that `count` differences between Jekyll's output and Folia's were accepted by the
    /// ledger: the output passes its checks, but is not identical.
    pub fn accepted(&mut self, output: &str, count: usize) {
        if count > 0 {
            *self.1.entry(output.to_owned()).or_default() += count;
        }
    }

    pub fn record(&mut self, output: &str, check: &'static str, passed: bool) {
        *self
            .0
            .entry(output.to_owned())
            .or_default()
            .entry(check)
            .or_insert(true) &= passed;
    }
}

/// A row's class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    BuiltAndChecked,
    NotBuiltYet,
    Waived,
    Removed,
    Intentional,
    KnownExcluded,
    Defect,
    MissingEvidence,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::BuiltAndChecked => "built and checked",
            Class::NotBuiltYet => "not built yet",
            Class::Waived => "waived",
            Class::Removed => "removed",
            Class::Intentional => "intentional difference (approved)",
            Class::KnownExcluded => "known difference (excluded, unapproved)",
            Class::Defect => "unresolved defect",
            Class::MissingEvidence => "missing evidence",
        }
    }
}

/// One check's verdict on a row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub check: &'static str,
    pub passed: bool,
}

/// One manifest row, classified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RowReport {
    pub output: String,
    pub url: String,
    /// The manifest's `kind` (`page`, `document`, `static`).
    pub kind: String,
    /// `None` when the row cannot be classified: `problems` says why, and the run fails.
    pub class: Option<Class>,
    /// For a waived or removed row, the entry and its approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    /// The entry that classifies it, when one does (a known difference has no approval).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// The checks that compared it.
    pub checks: Vec<Verdict>,
    /// How many differences from Jekyll's output the ledger accepted (it is built and checked,
    /// but not identical).
    #[serde(skip_serializing_if = "is_zero")]
    pub accepted_differences: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub problems: Vec<String>,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes a reference"
)]
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// How many rows have each class.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub rows: usize,
    pub built_and_checked: usize,
    pub not_built_yet: usize,
    pub waived: usize,
    pub removed: usize,
    pub intentional: usize,
    pub known_excluded: usize,
    pub defects: usize,
    pub missing_evidence: usize,
    /// No class: its entry is invalid, pending or doubled.
    pub unclassified: usize,
    /// Of `built_and_checked`, the rows with differences the ledger accepted.
    pub built_with_accepted_differences: usize,
}

/// The coverage report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    pub format: u32,
    /// The ledger's `owners`, the only names an approval may give: recorded so that a change to
    /// them shows in every report.
    pub owners: Vec<String>,
    pub counts: Counts,
    /// What is wrong beyond a row: the ledger's entries and owners, the manifest, Folia's output.
    pub problems: Vec<String>,
    /// Every manifest row, once, in the manifest's order of output paths.
    pub rows: Vec<RowReport>,
    /// Outputs Folia writes that Jekyll never wrote, named by an approved `folia-only` entry.
    pub folia_only: Vec<FoliaOnly>,
    /// No unclassified row, defect or missing evidence, and no problem: every output is
    /// accounted for. Says nothing of parity.
    pub clean_build: bool,
    /// Every row built and checked with no ledger-accepted difference, and nothing Folia-only.
    pub exact_parity: bool,
    /// A clean build with no known difference left unapproved.
    pub passed: bool,
}

/// An output with no Jekyll baseline, and the approval of its being written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FoliaOnly {
    pub output: String,
    pub approval: Approval,
}

/// What Folia did with the site: what it plans and what it wrote.
pub struct Folia<'a> {
    pub planned: &'a BTreeSet<String>,
    pub written: &'a BTreeSet<String>,
    /// Of the outputs written, those whose bytes Folia's output manifest records, with their
    /// hash (computed for the waived ones, which no other check reads).
    pub accounted: &'a BTreeSet<String>,
    /// Folia's own count of planned outputs it did not write.
    pub not_built: usize,
}

/// Checks a `folia-only` entry: each output is no manifest row, is written and approved, and is
/// named by no other such entry. Those that pass go into `named`.
fn check_folia_only<'a>(
    entry: &'a Entry,
    manifest: &BTreeSet<&str>,
    folia: &Folia,
    named: &mut BTreeMap<&'a str, &'a Entry>,
    problems: &mut Vec<String>,
) {
    for output in &entry.outputs {
        if manifest.contains(output.as_str()) {
            problems.push(format!(
                "{}: {output} is a manifest row, so Jekyll writes it: it is not Folia-only",
                entry.id
            ));
        } else if !folia.written.contains(output) {
            problems.push(format!(
                "{}: {output} is not written, so the Folia-only entry is stale",
                entry.id
            ));
        } else if entry.approval.is_none() {
            problems.push(format!(
                "{}: {output} is not approved by an owner",
                entry.id
            ));
        } else if let Some(other) = named.insert(output.as_str(), entry) {
            problems.push(format!(
                "{output} is named by two Folia-only entries: {}, {}",
                other.id, entry.id
            ));
        }
    }
}

/// How many rows have each class.
fn count(report_rows: &[RowReport]) -> Counts {
    let mut counts = Counts {
        rows: report_rows.len(),
        ..Counts::default()
    };
    for row in report_rows {
        *match row.class {
            Some(Class::BuiltAndChecked) => &mut counts.built_and_checked,
            Some(Class::NotBuiltYet) => &mut counts.not_built_yet,
            Some(Class::Waived) => &mut counts.waived,
            Some(Class::Removed) => &mut counts.removed,
            Some(Class::Intentional) => &mut counts.intentional,
            Some(Class::KnownExcluded) => &mut counts.known_excluded,
            Some(Class::Defect) => &mut counts.defects,
            Some(Class::MissingEvidence) => &mut counts.missing_evidence,
            None => &mut counts.unclassified,
        } += 1;
        if row.class == Some(Class::BuiltAndChecked) && row.accepted_differences > 0 {
            counts.built_with_accepted_differences += 1;
        }
    }
    counts
}

/// Classifies every row of `rows`.
pub fn classify(rows: &[Row], folia: &Folia, checks: &Checks, ledger: &Ledger) -> Report {
    let mut problems: Vec<String> = ledger.problems.clone();
    let manifest: BTreeSet<&str> = rows.iter().map(|row| row.output.as_str()).collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        if !seen.insert(row.output.as_str()) {
            problems.push(format!("the manifest lists {} twice", row.output));
        }
    }
    // Which active entries name each output.
    let mut naming: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();
    let mut folia_only: BTreeMap<&str, &Entry> = BTreeMap::new();
    for entry in ledger.entries.iter().filter(|e| e.active()) {
        for problem in &entry.problems {
            problems.push(format!("{}: {problem}", entry.id));
        }
        if entry.class == Some(Decided::FoliaOnly) {
            check_folia_only(entry, &manifest, folia, &mut folia_only, &mut problems);
            continue;
        }
        for output in &entry.outputs {
            if manifest.contains(output.as_str()) {
                naming.entry(output.as_str()).or_default().push(entry);
            } else {
                problems.push(format!(
                    "{}: {output} is no manifest row: only an output Jekyll writes can be waived \
                     or removed",
                    entry.id
                ));
            }
        }
    }
    for entry in ledger.entries.iter().filter(|e| !e.active()) {
        for problem in &entry.problems {
            problems.push(format!("{}: {problem}", entry.id));
        }
    }
    let mut folia_only_report = Vec::new();
    for output in folia
        .written
        .difference(&manifest.iter().map(|o| (*o).to_owned()).collect())
    {
        match folia_only
            .get(output.as_str())
            .and_then(|e| e.approval.clone())
        {
            Some(approval) => folia_only_report.push(FoliaOnly {
                output: output.clone(),
                approval,
            }),
            None => problems.push(format!("Folia writes {output}, which is no manifest row")),
        }
    }
    let unwritten = folia.planned.difference(folia.written).count();
    if unwritten != folia.not_built {
        problems.push(format!(
            "Folia counts {} planned outputs not built, but {unwritten} planned outputs are not \
             written",
            folia.not_built
        ));
    }

    let mut report_rows: Vec<RowReport> = rows
        .iter()
        .map(|row| classify_row(row, folia, checks, naming.get(row.output.as_str())))
        .collect();
    report_rows.sort_by(|a, b| a.output.cmp(&b.output));
    let counts = count(&report_rows);
    let clean_build =
        problems.is_empty() && counts.unclassified + counts.defects + counts.missing_evidence == 0;
    let passed = clean_build && counts.known_excluded == 0;
    let exact_parity = passed
        && counts.built_and_checked == counts.rows
        && counts.built_with_accepted_differences == 0
        && folia_only_report.is_empty();
    Report {
        format: 2,
        owners: ledger.owners.clone(),
        counts,
        problems,
        rows: report_rows,
        folia_only: folia_only_report,
        clean_build,
        exact_parity,
        passed,
    }
}

fn classify_row(
    row: &Row,
    folia: &Folia,
    checks: &Checks,
    entries: Option<&Vec<&Entry>>,
) -> RowReport {
    let written = folia.written.contains(&row.output);
    let verdicts: Vec<Verdict> = checks.0.get(&row.output).map_or_default(|checks| {
        checks
            .iter()
            .map(|(check, passed)| Verdict {
                check,
                passed: *passed,
            })
            .collect()
    });
    let mut problems = Vec::new();
    let mut approval = None;
    let mut entry_id = None;
    let class = match entries.map_or_default(Vec::as_slice) {
        [] if written => {
            let failed: Vec<&str> = verdicts
                .iter()
                .filter(|v| !v.passed)
                .map(|v| v.check)
                .collect();
            if !verdicts.iter().any(|v| v.check != ROUTE) {
                problems.push(
                    "Folia writes it, but no check compares its contents (a route check is no \
                     comparison)"
                        .to_owned(),
                );
                Some(Class::MissingEvidence)
            } else if !failed.is_empty() {
                problems.push(format!(
                    "Folia writes it, but it fails its checks, and no entry covers it: {}",
                    failed.join(", ")
                ));
                Some(Class::Defect)
            } else {
                Some(Class::BuiltAndChecked)
            }
        }
        [] if folia.planned.contains(&row.output) => Some(Class::NotBuiltYet),
        [] => {
            problems.push(
                "Folia neither writes nor plans it, and no accepted entry removes it".to_owned(),
            );
            Some(Class::Defect)
        }
        [entry] => {
            let route = verdicts.iter().find(|v| v.check == ROUTE).map(|v| v.passed);
            let found = Found {
                written,
                planned: folia.planned.contains(&row.output),
                accounted: folia.accounted.contains(&row.output),
                route,
            };
            let failed = verdicts.iter().any(|v| !v.passed);
            let decided = decided_class(entry, &found, failed, &mut problems);
            if decided.is_some() {
                approval.clone_from(&entry.approval);
                entry_id = Some(entry.id.clone());
            }
            decided
        }
        several => {
            let ids: Vec<&str> = several.iter().map(|e| e.id.as_str()).collect();
            problems.push(format!(
                "classified by more than one entry: {}",
                ids.join(", ")
            ));
            None
        }
    };
    RowReport {
        output: row.output.clone(),
        url: row.url.clone(),
        kind: row.kind.clone(),
        class,
        approval,
        entry: entry_id,
        checks: verdicts,
        accepted_differences: if class == Some(Class::BuiltAndChecked) {
            checks.1.get(&row.output).copied().unwrap_or(0)
        } else {
            0
        },
        problems,
    }
}

/// What a run found of one row's output, as an entry's class requires it.
struct Found {
    written: bool,
    planned: bool,
    /// Whether Folia's output manifest records its bytes.
    accounted: bool,
    /// The route check's verdict, if it ran.
    route: Option<bool>,
}

/// The class one entry gives a row, or why it gives none. Only an accepted entry an owner
/// approved, with no problem, gives one: a waiver to an output Folia writes, whose bytes its
/// manifest records and whose route check passes; a removal to an output Folia neither writes
/// nor plans, so that a removal cannot stand for an output that is merely not built yet.
fn decided_class(
    entry: &Entry,
    found: &Found,
    failed: bool,
    problems: &mut Vec<String>,
) -> Option<Class> {
    let id = &entry.id;
    if !entry.problems.is_empty() || entry.class.is_none() {
        problems.push(format!("{id} classifies it, but is not valid"));
        return None;
    }
    if entry.status == Some(Status::Proposed) {
        problems.push(format!(
            "pending: {id} is proposed, not approved by an owner"
        ));
        return None;
    }
    if entry.approval.is_none() && entry.class != Some(Decided::Known) {
        problems.push(format!("{id} is not approved by an owner"));
        return None;
    }
    match entry.class {
        Some(Decided::Waived) => {
            if !found.written {
                problems.push(format!(
                    "{id} waives its comparison, but Folia does not write it: a waiver never \
                     excuses a missing output"
                ));
            } else if !found.accounted {
                problems.push(format!(
                    "{id} waives its comparison, but Folia's output manifest does not record its \
                     bytes"
                ));
            } else if found.route != Some(true) {
                problems.push(format!(
                    "{id} waives its comparison, but its route check {}",
                    if found.route.is_some() {
                        "fails"
                    } else {
                        "did not run"
                    }
                ));
            } else {
                return Some(Class::Waived);
            }
            None
        }
        Some(Decided::Intentional | Decided::Known) => {
            let kind = if entry.class == Some(Decided::Known) {
                "records a known difference"
            } else {
                "approves an intentional difference"
            };
            if !found.written {
                problems.push(format!("{id} {kind}, but Folia does not write it"));
            } else if !failed {
                problems.push(format!(
                    "{id} {kind}, but every check on it passes: the entry is stale"
                ));
            } else if entry.class == Some(Decided::Known) {
                return Some(Class::KnownExcluded);
            } else {
                return Some(Class::Intentional);
            }
            None
        }
        Some(Decided::FoliaOnly) => {
            problems.push(format!(
                "{id} is a Folia-only entry, but this is a manifest row: Jekyll writes it"
            ));
            None
        }
        Some(Decided::Removed) => {
            if found.written {
                problems.push(format!("{id} removes it, but Folia writes it"));
            } else if found.planned {
                problems.push(format!(
                    "{id} removes it, but Folia still plans it: an output Folia plans and does \
                     not write is not built yet, not removed"
                ));
            } else {
                return Some(Class::Removed);
            }
            None
        }
        None => None,
    }
}

mod render;

#[cfg(test)]
mod tests;
