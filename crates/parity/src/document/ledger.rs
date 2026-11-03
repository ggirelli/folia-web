//! The deviations ledger as the whole-document DOM comparison reads it (plan v12, §5.2): the
//! `[[deviation]]` entries of `parity/deviations.toml` whose `check` is `dom`. Other entries are
//! the Markdown check's (`markdown::ledger`) and are not read here, except that every entry needs
//! an `id` that is not blank and that no other entry has.
//!
//! A DOM entry names exactly what it explains, and the two files it was reviewed against:
//!
//! - `expected_build`: the SHA-256 of the expected side's record, the baseline's `BASELINE.json`;
//! - `outputs`: each output file it applies to, with `expected_sha256`, the SHA-256 of the
//!   baseline's (Jekyll's) file reviewed, and `actual_sha256`, that of the Folia file reviewed.
//!   `expected_sha256` is checked against the hash of the baseline file's bytes (a baseline has
//!   no output manifest), `actual_sha256` against the hash Folia's output manifest records for
//!   the page, which the judged report checks against the page's bytes in turn
//!   ([`super::FileReport`]); a different file on either side is not covered, so a changed page
//!   or baseline needs a new review. Each output is bound by its own files only: a change to
//!   another output leaves the entry as it was;
//! - `match`: the differences, each a node path (§5.1) and a kind. The kind is one of the
//!   comparator's, which matches a difference of that kind at that path whatever its values, or
//!   the ledger-only `attribute-value-equal-after-collapse-and-trim`, which matches an
//!   `attribute-value-differs` only when the two values are equal once runs of HTML whitespace
//!   are collapsed and the ends trimmed (§5.1's named head difference; a no-break space is not
//!   whitespace).
//!
//! An entry *covers* a difference when its output is listed and one of its `match` items holds;
//! it *explains* it when, besides, the expected record and both files are the ones reviewed. Then:
//!
//! - only an `accepted` entry explains a difference; a `proposed` one leaves it *pending*, which
//!   fails; a `withdrawn` one is ignored;
//! - a difference that two or more entries (accepted or proposed) explain fails, as an overlap;
//! - a difference that an entry covers but cannot explain, because the record or a file is not
//!   the one reviewed, is *stale* for that entry, and fails;
//! - an accepted or proposed entry is valid only when the record is the one reviewed, every
//!   output it lists was compared with both its files the ones reviewed, and every `match` item
//!   explains a difference on every output it lists; one that is not is *stale*, and fails the
//!   run, with each reason (plan v12, §5.2, as decided on 2026-11-03);
//! - an entry that is not well formed fails (its problems are [`Ledger::problems`]): a field the
//!   schema does not know, in the entry, an output or a `match` item, is a problem, never ignored.
//!
//! An `open` follow-up does not fail a run. What final acceptance (plan v12, F13) would refuse is
//! listed by [`Applied::final_acceptance`]: a proposed entry, an open follow-up, each reason an
//! entry is stale, and any problem.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{Difference, FileReport, Kind, Report};

/// The ledger-only kind of §5.1's named head difference.
pub const EQUAL_AFTER_COLLAPSE_AND_TRIM: &str = "attribute-value-equal-after-collapse-and-trim";

/// The ledger file: `[[deviation]]` tables and the `owners` who may accept an entry (read by the
/// coverage check, `crate::coverage`), nothing else. Each entry is read as a table first, so that
/// the entries of other checks, whose fields differ, are left to their check.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    deviation: Vec<toml::Table>,
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "read so that the schema knows it; the coverage check uses it"
    )]
    owners: Option<toml::Value>,
}

/// A DOM entry as written; [`Entry`] is it checked. Every field the schema knows is listed: the
/// descriptive ones (`title`, `change`, `rationale`, `boundary`, `evidence`, `withdrawn`,
/// `resolution`) are free text, kept in the file for the reader; any other field is refused.
#[derive(Clone, Debug, Default, Deserialize)]
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
    check: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    change: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    boundary: Option<String>,
    #[serde(default)]
    evidence: Option<String>,
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
    #[serde(default)]
    follow_up: Option<String>,
    #[serde(default)]
    follow_up_status: Option<String>,
    #[serde(default)]
    expected_build: Option<String>,
    #[serde(default)]
    outputs: Vec<RawOutput>,
    #[serde(default, rename = "match")]
    matches: Vec<RawMatch>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutput {
    #[serde(default)]
    path: String,
    #[serde(default)]
    expected_sha256: String,
    #[serde(default)]
    actual_sha256: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMatch {
    #[serde(default)]
    path: String,
    #[serde(default)]
    kind: String,
}

/// An entry's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Proposed,
    Accepted,
    Withdrawn,
}

/// What a `match` item's kind requires of a difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// A difference of this kind.
    Kind(Kind),
    /// An `attribute-value-differs` whose values are equal after collapse and trim.
    EqualAfterCollapseAndTrim,
}

impl Condition {
    fn parse(kind: &str) -> Option<Self> {
        if kind == EQUAL_AFTER_COLLAPSE_AND_TRIM {
            return Some(Condition::EqualAfterCollapseAndTrim);
        }
        KINDS
            .iter()
            .find(|candidate| candidate.name() == kind)
            .map(|&kind| Condition::Kind(kind))
    }

    /// The kind as the ledger writes it.
    pub fn name(self) -> &'static str {
        match self {
            Condition::Kind(kind) => kind.name(),
            Condition::EqualAfterCollapseAndTrim => EQUAL_AFTER_COLLAPSE_AND_TRIM,
        }
    }

    fn holds(self, difference: &Difference) -> bool {
        match self {
            Condition::Kind(kind) => difference.kind == kind,
            Condition::EqualAfterCollapseAndTrim => {
                difference.kind == Kind::AttributeValueDiffers
                    && difference.equal_after_collapse_and_trim == Some(true)
            }
        }
    }
}

const KINDS: [Kind; 9] = [
    Kind::ElementMissing,
    Kind::ElementExtra,
    Kind::TagDiffers,
    Kind::AttributeMissing,
    Kind::AttributeExtra,
    Kind::AttributeValueDiffers,
    Kind::TextDiffers,
    Kind::CommentDiffers,
    Kind::DoctypeDiffers,
];

/// The two files of an output an entry was reviewed against, by their SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewed {
    /// The baseline's (Jekyll's) file.
    pub expected_sha256: String,
    /// Folia's file.
    pub actual_sha256: String,
}

/// A DOM entry, checked.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub status: Status,
    pub expected_build: String,
    /// Output path to the files reviewed.
    pub outputs: BTreeMap<String, Reviewed>,
    pub matches: Vec<(String, Condition)>,
    pub follow_up_status: String,
}

impl Entry {
    /// The indices of the `match` items that hold for `difference` on the output `file`, if the
    /// entry lists that output; whether the files are the ones reviewed is [`Entry::binds`].
    fn covers(&self, file: &str, difference: &Difference) -> Vec<usize> {
        if !self.outputs.contains_key(file) {
            return Vec::new();
        }
        self.matches
            .iter()
            .enumerate()
            .filter(|(_, (path, condition))| {
                *path == difference.path && condition.holds(difference)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Whether the expected record and both files of `file` are the ones reviewed: the baseline's
    /// by the hash of its bytes, Folia's by the hash its output manifest records.
    fn binds(&self, expected_record: Option<&str>, file: &FileReport) -> bool {
        expected_record == Some(self.expected_build.as_str())
            && self.outputs.get(&file.path).is_some_and(|reviewed| {
                file.expected_sha256.as_deref() == Some(reviewed.expected_sha256.as_str())
                    && file.actual_manifest_sha256.as_deref()
                        == Some(reviewed.actual_sha256.as_str())
            })
    }
}

/// The DOM entries of a ledger, and what is wrong with any of them.
#[derive(Clone, Debug)]
pub struct Ledger {
    /// The ledger file, as given.
    pub path: String,
    /// The SHA-256 of the ledger file.
    pub sha256: String,
    pub entries: Vec<Entry>,
    /// Each problem with an entry: a ledger with problems fails the check.
    pub problems: Vec<String>,
}

/// Reads the DOM entries of the ledger at `path`.
///
/// # Errors
///
/// Fails when the file cannot be read or is not TOML of `[[deviation]]` tables only. Problems with
/// an entry, including fields the schema does not know, are not errors: they are
/// [`Ledger::problems`], which fail the check.
pub fn load(path: &Path) -> Result<Ledger, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let file: File = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut ledger = Ledger {
        path: path.display().to_string(),
        sha256: hex(&Sha256::digest(&bytes)),
        entries: Vec::new(),
        problems: Vec::new(),
    };
    let mut ids = BTreeSet::new();
    for (index, table) in file.deviation.iter().enumerate() {
        let label = match table.get("id").map(|id| id.as_str()) {
            Some(Some(id)) if !id.trim().is_empty() => {
                if !ids.insert(id.to_owned()) {
                    ledger.problems.push(format!("{id}: the id is used twice"));
                }
                id.to_owned()
            }
            _ => {
                let label = format!("entry {} of the ledger", index + 1);
                ledger
                    .problems
                    .push(format!("{label}: `id` must be a string that is not blank"));
                label
            }
        };
        if table.get("check").and_then(toml::Value::as_str) != Some("dom") {
            continue;
        }
        let raw: Raw = match toml::Value::Table(table.clone()).try_into() {
            Ok(raw) => raw,
            Err(e) => {
                ledger.problems.push(format!(
                    "{label}: not a DOM entry of the known schema: {}",
                    e.to_string().trim()
                ));
                continue;
            }
        };
        match check(&raw) {
            Ok(entry) => ledger.entries.push(entry),
            Err(problems) => ledger
                .problems
                .extend(problems.into_iter().map(|p| format!("{label}: {p}"))),
        }
    }
    Ok(ledger)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Checks an entry is well formed: a known status; for an accepted entry, its date, who accepted
/// it and the decision; a follow-up status, and a follow-up when it is open; an expected build;
/// explicit outputs, each with both files' hashes, none twice; and `match` items with known kinds,
/// none twice. (The id is checked by [`load`], for every entry.)
fn check(raw: &Raw) -> Result<Entry, Vec<String>> {
    let mut problems = Vec::new();
    let present = |value: &Option<String>| value.as_deref().is_some_and(|v| !v.trim().is_empty());
    let status = match raw.status.as_str() {
        "proposed" => Status::Proposed,
        "accepted" => Status::Accepted,
        "withdrawn" => Status::Withdrawn,
        other => {
            problems.push(format!(
                "status {other:?} is not proposed, accepted or withdrawn"
            ));
            Status::Proposed
        }
    };
    if status == Status::Accepted {
        for (name, value) in [
            ("accepted", &raw.accepted),
            ("accepted_by", &raw.accepted_by),
            ("decision", &raw.decision),
        ] {
            if !present(value) {
                problems.push(format!("an accepted entry needs `{name}`"));
            }
        }
    }
    let follow_up_status = raw.follow_up_status.clone().unwrap_or_default();
    match follow_up_status.as_str() {
        "none" | "closed" => {}
        "open" if present(&raw.follow_up) => {}
        "open" => problems.push("an open follow-up needs `follow_up`".to_owned()),
        other => problems.push(format!(
            "follow_up_status {other:?} is not open, closed or none"
        )),
    }
    let expected_build = raw.expected_build.clone().unwrap_or_default();
    if !is_sha256(&expected_build) {
        problems
            .push("`expected_build` must be the SHA-256 of the expected side's record".to_owned());
    }
    let mut outputs = BTreeMap::new();
    if raw.outputs.is_empty() {
        problems.push("`outputs` must name each output explicitly".to_owned());
    }
    for output in &raw.outputs {
        if output.path.is_empty() || output.path.contains('*') {
            problems.push(format!("output {:?} is not an explicit path", output.path));
        }
        for (name, value) in [
            ("expected_sha256", &output.expected_sha256),
            ("actual_sha256", &output.actual_sha256),
        ] {
            if !is_sha256(value) {
                problems.push(format!(
                    "output {}: `{name}` must be a SHA-256",
                    output.path
                ));
            }
        }
        let reviewed = Reviewed {
            expected_sha256: output.expected_sha256.clone(),
            actual_sha256: output.actual_sha256.clone(),
        };
        if outputs.insert(output.path.clone(), reviewed).is_some() {
            problems.push(format!("output {} is listed twice", output.path));
        }
    }
    let mut matches = Vec::new();
    if raw.matches.is_empty() {
        problems.push("`match` must name each difference".to_owned());
    }
    for item in &raw.matches {
        match Condition::parse(&item.kind) {
            _ if item.path.is_empty() => problems.push("a `match` item needs a path".to_owned()),
            Some(condition) if !matches.contains(&(item.path.clone(), condition)) => {
                matches.push((item.path.clone(), condition));
            }
            Some(_) => problems.push(format!("match {} {} is listed twice", item.path, item.kind)),
            None => problems.push(format!("match {}: unknown kind {:?}", item.path, item.kind)),
        }
    }
    if problems.is_empty() {
        Ok(Entry {
            id: raw.id.clone(),
            status,
            expected_build,
            outputs,
            matches,
            follow_up_status,
        })
    } else {
        Err(problems)
    }
}

/// How a difference stands against the ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "verdict", content = "entry")]
pub enum Verdict {
    /// No entry covers it: a failure.
    Unexplained,
    /// One accepted entry explains it, and no other entry does.
    Accepted(String),
    /// Only one proposed entry explains it: a failure, until the entry is accepted.
    Pending(String),
    /// Two or more entries explain it, accepted or proposed: a failure.
    Overlap(Vec<String>),
    /// No entry explains it, but these entries cover it (its output and a `match` item) and were
    /// reviewed against another baseline record or other files: a failure, until reviewed again.
    Stale(Vec<String>),
}

/// Whether a reviewed file, or record, is the one compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Binding {
    /// The file compared has the hash reviewed.
    AsReviewed,
    /// The file compared has another hash.
    Changed,
    /// No such file was compared (the output was not built, a side lacks it, or Folia's output
    /// manifest does not list it).
    Absent,
}

/// An output an entry lists, against the files compared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutputResult {
    pub path: String,
    /// The baseline's file, against the entry's `expected_sha256`.
    pub expected: Binding,
    /// Folia's file, by the hash its output manifest records, against the entry's
    /// `actual_sha256`.
    pub actual: Binding,
}

impl OutputResult {
    fn as_reviewed(&self) -> bool {
        self.expected == Binding::AsReviewed && self.actual == Binding::AsReviewed
    }
}

/// A `match` item that explained no difference on one of the entry's outputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unused {
    pub output: String,
    pub path: String,
    pub kind: &'static str,
}

/// What an entry explained in a comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EntryResult {
    pub id: String,
    pub status: Status,
    /// Each difference it explained, as `(output, path)`.
    pub explained: Vec<(String, String)>,
    /// The baseline's record, against the entry's `expected_build`.
    pub expected_build: Binding,
    /// Each output it lists, with both its files against the hashes reviewed.
    pub outputs: Vec<OutputResult>,
    /// Each pair of a listed output and a `match` item that explained nothing there.
    pub unused: Vec<Unused>,
    /// Whether it is not valid, which fails the run (only accepted and proposed entries can be
    /// stale): it explains nothing, the record is not the one reviewed, an output it lists is
    /// absent or not as reviewed on either side, or a `match` item is unused on an output.
    pub stale: bool,
    pub follow_up_status: String,
}

/// The ledger applied to a report: a verdict per difference, in the report's order, and a result
/// per entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub path: String,
    pub sha256: String,
    pub verdicts: Vec<Vec<Verdict>>,
    pub entries: Vec<EntryResult>,
    pub problems: Vec<String>,
    /// Whether everything passes: every difference accepted, no entry stale, no problem.
    pub passed: bool,
    /// What final acceptance (plan v12, F13) would refuse in this ledger against this comparison,
    /// one line each: a problem, a proposed entry, an open follow-up, and each reason an entry is
    /// stale (it explains nothing, the record, each output not as reviewed, each unused `match`
    /// item). A run fails on problems and stale entries, not on a proposed entry's or an open
    /// follow-up's line; F13 requires it empty.
    pub final_acceptance: Vec<String>,
}

/// Applies `ledger` to `report`.
pub fn apply(ledger: &Ledger, report: &Report) -> Applied {
    let expected_record = report.expected.record.as_ref().map(|r| r.sha256.as_str());
    let active: Vec<&Entry> = ledger
        .entries
        .iter()
        .filter(|entry| entry.status != Status::Withdrawn)
        .collect();
    let mut explained: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    // (entry, output, match item) that explained at least one difference.
    let mut used: BTreeSet<(&str, &str, usize)> = BTreeSet::new();
    let verdicts: Vec<Vec<Verdict>> = report
        .files
        .iter()
        .map(|file| {
            file.differences
                .iter()
                .map(|difference| {
                    let mut covering = Vec::new();
                    let mut explaining: Vec<&Entry> = Vec::new();
                    for &entry in &active {
                        let items = entry.covers(&file.path, difference);
                        if items.is_empty() {
                            continue;
                        }
                        if entry.binds(expected_record, file) {
                            explaining.push(entry);
                            for item in items {
                                used.insert((entry.id.as_str(), file.path.as_str(), item));
                            }
                        } else {
                            covering.push(entry.id.clone());
                        }
                    }
                    for entry in &explaining {
                        explained
                            .entry(entry.id.as_str())
                            .or_default()
                            .push((file.path.clone(), difference.path.clone()));
                    }
                    match explaining.as_slice() {
                        [] if covering.is_empty() => Verdict::Unexplained,
                        [] => Verdict::Stale(covering),
                        [one] if one.status == Status::Accepted => {
                            Verdict::Accepted(one.id.clone())
                        }
                        [one] => Verdict::Pending(one.id.clone()),
                        many => Verdict::Overlap(many.iter().map(|e| e.id.clone()).collect()),
                    }
                })
                .collect()
        })
        .collect();
    let entries: Vec<EntryResult> = ledger
        .entries
        .iter()
        .map(|entry| entry_result(entry, report, expected_record, &explained, &used))
        .collect();
    let passed = ledger.problems.is_empty()
        && entries.iter().all(|entry| !entry.stale)
        && verdicts
            .iter()
            .flatten()
            .all(|verdict| matches!(verdict, Verdict::Accepted(_)));
    let final_acceptance = final_acceptance(&ledger.problems, &entries);
    Applied {
        path: ledger.path.clone(),
        sha256: ledger.sha256.clone(),
        verdicts,
        entries,
        problems: ledger.problems.clone(),
        passed,
        final_acceptance,
    }
}

fn entry_result(
    entry: &Entry,
    report: &Report,
    expected_record: Option<&str>,
    explained: &BTreeMap<&str, Vec<(String, String)>>,
    used: &BTreeSet<(&str, &str, usize)>,
) -> EntryResult {
    let binding = |compared: Option<&str>, reviewed: &str| match compared {
        None => Binding::Absent,
        Some(sha256) if sha256 == reviewed => Binding::AsReviewed,
        Some(_) => Binding::Changed,
    };
    let outputs: Vec<OutputResult> = entry
        .outputs
        .iter()
        .map(|(path, reviewed)| {
            let file = report.files.iter().find(|file| file.path == *path);
            OutputResult {
                path: path.clone(),
                expected: binding(
                    file.and_then(|f| f.expected_sha256.as_deref()),
                    &reviewed.expected_sha256,
                ),
                actual: binding(
                    file.and_then(|f| f.actual_manifest_sha256.as_deref()),
                    &reviewed.actual_sha256,
                ),
            }
        })
        .collect();
    let expected_build = binding(expected_record, &entry.expected_build);
    let explained = explained
        .get(entry.id.as_str())
        .cloned()
        .unwrap_or_default();
    let withdrawn = entry.status == Status::Withdrawn;
    let unused: Vec<Unused> = if withdrawn {
        Vec::new()
    } else {
        entry
            .outputs
            .keys()
            .flat_map(|output| {
                entry
                    .matches
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        !used.contains(&(entry.id.as_str(), output.as_str(), *index))
                    })
                    .map(|(_, (path, condition))| Unused {
                        output: output.clone(),
                        path: path.clone(),
                        kind: condition.name(),
                    })
            })
            .collect()
    };
    let stale = !withdrawn
        && (explained.is_empty()
            || expected_build != Binding::AsReviewed
            || !unused.is_empty()
            || !outputs.iter().all(OutputResult::as_reviewed));
    EntryResult {
        id: entry.id.clone(),
        status: entry.status,
        explained,
        expected_build,
        outputs,
        unused,
        stale,
        follow_up_status: entry.follow_up_status.clone(),
    }
}

/// What final acceptance (F13) would refuse: see [`Applied::final_acceptance`].
fn final_acceptance(problems: &[String], entries: &[EntryResult]) -> Vec<String> {
    let mut out: Vec<String> = problems.iter().map(|p| format!("problem: {p}")).collect();
    for entry in entries.iter().filter(|e| e.status != Status::Withdrawn) {
        let id = &entry.id;
        if entry.status == Status::Proposed {
            out.push(format!("{id}: proposed, not accepted"));
        }
        if entry.follow_up_status == "open" {
            out.push(format!("{id}: its follow-up is open"));
        }
        if entry.stale && entry.explained.is_empty() {
            out.push(format!("{id}: stale, it explains no difference"));
        }
        if entry.expected_build != Binding::AsReviewed {
            out.push(format!(
                "{id}: the baseline's record is not the one reviewed"
            ));
        }
        for output in entry.outputs.iter().filter(|o| !o.as_reviewed()) {
            out.push(format!(
                "{id}: output {}: the baseline's file is {}, Folia's is {}",
                output.path,
                binding_name(output.expected),
                binding_name(output.actual)
            ));
        }
        for item in &entry.unused {
            out.push(format!(
                "{id}: output {}: match {} {} explains nothing",
                item.output, item.path, item.kind
            ));
        }
    }
    out
}

/// A binding in words, for reports.
pub fn binding_name(binding: Binding) -> &'static str {
    match binding {
        Binding::AsReviewed => "as reviewed",
        Binding::Changed => "not the one reviewed",
        Binding::Absent => "absent",
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests;
