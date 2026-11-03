//! Determinism (plan v12, §5.4, work item A2.5): Folia builds the site twice, as of one pinned
//! time, into two fresh build directories, and the two outputs are compared recursively, byte for
//! byte; a build is also compared with a recorded reference (§4.2), file by file through its
//! manifest.
//!
//! The time is pinned through Folia's library API (`builds::keep`, which calls
//! `folia::build::build` with an explicit time), as the baseline's build time, which is also the
//! time the A0 reference was built as of. Each build is kept, with its record, manifest and log
//! (`builds`), in a new directory that holds both builds and the report:
//!
//! | Path | What it is |
//! |-|-|
//! | `build-1/`, `build-2/` | the two kept builds |
//! | `determinism-report.json` | every difference found, in full |
//! | `determinism-report.md` | the same, to read |
//!
//! What is compared, and what each difference means:
//!
//! - **Inputs**, every field of the two `build.json` records other than the result: the time, the
//!   site's and Folia's checkouts and identities, the configuration, the templates, the
//!   compiler. A difference means the builds were not made from the same inputs, so their
//!   outputs say nothing about determinism; the run fails as *not comparable*.
//! - **Results**: the build's outcome, summary, warnings, problems and file count. A difference
//!   is nondeterminism.
//! - **Outputs**: the two `site/` trees, every entry (directories, files, links), files byte for
//!   byte. A difference is nondeterminism, reported with the path, both sizes and hashes, and the
//!   first differing byte and its line.
//! - **The reference**: each build's manifest (`sha256.txt`) against the reference's, path for
//!   path and hash for hash. A difference is a Folia change, to be approved (§4.2) or fixed.
//!
//! This is repeatability within one checkout: both builds read the same site and Folia trees.
//! Reproducibility across independent checkouts, building from two fresh clones of the final
//! commits (F11's second half), is A2.5's too but runs at acceptance, and is not done here yet.
//!
//! Nothing else is compared: the logs, the absolute paths and the build directories' names are
//! diagnostics. An unknown source identity, or a build that fails, fails the run, as in
//! `parity pages`; the builds and the report are kept either way.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use folia::config::Config;
use serde::Serialize;
use serde_json::Value;

use crate::baseline;
use crate::builds::{self, Kept, Request};
use crate::routes::{check_checkout, short};

/// The names of the two builds in the run's directory.
const BUILDS: [&str; 2] = ["build-1", "build-2"];

/// How many differences of each kind the run prints; the reports hold all of them.
const SHOWN: usize = 20;

/// What to build, and against what.
pub struct Options<'a> {
    /// The site's checkout, which must be the baseline's commit (`check_checkout`).
    pub site: &'a Path,
    /// The baseline whose build time pins the builds' time.
    pub baseline: &'a Path,
    /// Folia's configuration for the site.
    pub config: &'a Path,
    /// The run's directory, which must not exist; by default a new one in the temporary
    /// directory.
    pub out: Option<&'a Path>,
    /// The templates' expected tree hash, as for `parity pages`.
    pub expected_templates: Option<&'a str>,
    /// A recorded reference's manifest, in `sha256.txt`'s format, to compare each build with.
    pub reference: Option<&'a Path>,
}

/// What a run found, written as `determinism-report.json`.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The pinned build time.
    pub at: String,
    /// The builds, by name, in the run's directory.
    pub builds: [&'static str; 2],
    /// Why a build's source identity is unknown, one line per build and source.
    pub unknown_identities: Vec<String>,
    /// The builds that failed, each with its problems.
    pub failed: Vec<String>,
    /// The fields of the records' inputs that differ: the builds are not comparable.
    pub inputs: Vec<Field>,
    /// The fields of the records' results that differ.
    pub results: Vec<Field>,
    /// The comparison of the two output trees; `None` when a build failed.
    pub outputs: Option<Outputs>,
    /// Each build against the reference, when one is given and the builds succeeded.
    pub reference: Option<Reference>,
    /// Whether determinism holds: both builds identified and successful, the same inputs, and
    /// no difference in results, outputs or against the reference.
    pub passed: bool,
}

/// A field of `build.json`, by its dotted path, with both builds' values.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Field {
    pub path: String,
    pub first: Value,
    pub second: Value,
}

/// The two output trees compared.
#[derive(Debug, Serialize)]
pub struct Outputs {
    /// How many entries (files, directories, links) the first tree holds.
    pub entries: usize,
    /// How many of them are files.
    pub files: usize,
    pub differences: Vec<Difference>,
}

/// A difference between the two output trees, at a path relative to `site/`.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Difference {
    /// Only the first build has the entry.
    OnlyInFirst { path: String, entry: String },
    /// Only the second build has the entry.
    OnlyInSecond { path: String, entry: String },
    /// The entry is a file in one build and a directory or link in the other.
    Kind {
        path: String,
        first: String,
        second: String,
    },
    /// Both are links, to different targets.
    LinkTarget {
        path: String,
        first: String,
        second: String,
    },
    /// Both are files, with different bytes: `offset` is the first byte that differs (or the
    /// length of the shorter file), and `line` the line it is on in the first file.
    Bytes {
        path: String,
        first: Bytes,
        second: Bytes,
        offset: usize,
        line: usize,
    },
}

/// A file's size and SHA-256.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Bytes {
    pub size: usize,
    pub sha256: String,
}

/// The builds against a recorded reference.
#[derive(Debug, Serialize)]
pub struct Reference {
    /// The reference's manifest, as given.
    pub path: String,
    /// How many files the reference lists.
    pub files: usize,
    /// Each build's differences from it.
    pub builds: Vec<AgainstReference>,
}

/// One build's differences from the reference.
#[derive(Debug, Serialize)]
pub struct AgainstReference {
    pub build: &'static str,
    pub differences: Vec<ReferenceDifference>,
}

/// A difference between a build's manifest and the reference's.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReferenceDifference {
    /// The reference lists a file the build did not write.
    OnlyInReference { path: String },
    /// The build wrote a file the reference does not list.
    OnlyInBuild { path: String },
    /// Both have the file, with different bytes.
    Sha256 {
        path: String,
        reference: String,
        build: String,
    },
}

/// Builds the site twice as `options` says and compares the builds, with each other and with
/// the reference; prints the report, and writes it into the run's directory, which is kept with
/// both builds and printed. Returns whether determinism holds.
///
/// # Errors
///
/// Fails, before building, when the baseline, configuration or reference cannot be read, the
/// site checkout is not the baseline's commit, or the run's directory exists or lies inside the
/// site; and when a build cannot be kept, or its output or the report cannot be read or written.
pub fn run(options: &Options) -> Result<bool, String> {
    let baseline = baseline::load(options.baseline)?;
    check_checkout(options.site, &baseline.commit, baseline.diff.as_deref())?;
    let config = Config::load(options.config).map_err(|e| e.to_string())?;
    let reference = options
        .reference
        .map(|path| builds::read_manifest(path).map(|manifest| (path, manifest)))
        .transpose()?;
    let dest = options
        .out
        .map_or_else(|| builds::fresh_temp_dir("determinism"), Path::to_owned);
    let dir = builds::new_dir(&dest, options.site)?;
    println!(
        "Determinism: {} at {}, built twice as of {}, into {}.",
        options.site.display(),
        short(&baseline.commit),
        baseline.built_at,
        dir.display()
    );
    let keep = |name: &str| {
        builds::keep(&Request {
            site: options.site,
            config: &config,
            config_path: options.config,
            dest: &dir.join(name),
            at: baseline.built_at,
            expected_templates: options.expected_templates,
        })
    };
    let first = keep(BUILDS[0])?;
    let second = keep(BUILDS[1])?;
    let report = judge(
        &first,
        &second,
        reference.as_ref().map(|(path, manifest)| (*path, manifest)),
    )?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    write_new(&dir.join("determinism-report.json"), &json)?;
    write_new(&dir.join("determinism-report.md"), &markdown(&report))?;
    print(&report, &dir);
    Ok(report.passed)
}

/// Compares two kept builds, and each with the reference (`(path, manifest)`), when given.
///
/// # Errors
///
/// Fails when an output tree or manifest cannot be read.
pub fn judge(
    first: &Kept,
    second: &Kept,
    reference: Option<(&Path, &BTreeMap<String, String>)>,
) -> Result<Report, String> {
    let kept = [first, second];
    let mut unknown_identities = Vec::new();
    let mut failed = Vec::new();
    for (name, build) in BUILDS.iter().zip(kept) {
        for reason in builds::unknown_identities(&build.record) {
            unknown_identities.push(format!("{name}: {reason}"));
        }
        if !build.record.result.ok {
            failed.push(format!(
                "{name}: {}",
                build.record.result.problems.join("; ")
            ));
        }
    }
    let (inputs, results) = compare_records(first, second)?;
    let succeeded = failed.is_empty();
    let outputs = if succeeded {
        Some(compare_trees(&first.output, &second.output)?)
    } else {
        None
    };
    let reference = match reference {
        Some((path, manifest)) if succeeded => Some(Reference {
            path: path.display().to_string(),
            files: manifest.len(),
            builds: BUILDS
                .iter()
                .zip(kept)
                .map(|(name, build)| {
                    let built = builds::read_manifest(&build.dir.join("sha256.txt"))?;
                    Ok(AgainstReference {
                        build: name,
                        differences: against_reference(manifest, &built),
                    })
                })
                .collect::<Result<_, String>>()?,
        }),
        _ => None,
    };
    let passed = unknown_identities.is_empty()
        && succeeded
        && inputs.is_empty()
        && results.is_empty()
        && outputs.as_ref().is_some_and(|o| o.differences.is_empty())
        && reference
            .as_ref()
            .is_none_or(|r| r.builds.iter().all(|b| b.differences.is_empty()));
    Ok(Report {
        at: first.record.at.clone(),
        builds: BUILDS,
        unknown_identities,
        failed,
        inputs,
        results,
        outputs,
        reference,
        passed,
    })
}

/// The differing fields of the two records: the inputs (every field but `result` and `files`),
/// then the results.
fn compare_records(first: &Kept, second: &Kept) -> Result<(Vec<Field>, Vec<Field>), String> {
    let value = |kept: &Kept| serde_json::to_value(&kept.record).map_err(|e| e.to_string());
    let (Value::Object(first), Value::Object(second)) = (value(first)?, value(second)?) else {
        return Err("a build record is not an object".to_owned());
    };
    let mut inputs = Vec::new();
    let mut results = Vec::new();
    let keys: std::collections::BTreeSet<&String> = first.keys().chain(second.keys()).collect();
    for key in keys {
        let out = if key == "result" || key == "files" {
            &mut results
        } else {
            &mut inputs
        };
        differing_fields(
            key,
            first.get(key).unwrap_or(&Value::Null),
            second.get(key).unwrap_or(&Value::Null),
            out,
        );
    }
    Ok((inputs, results))
}

/// The leaves where `first` and `second` differ, by dotted path; arrays are compared whole.
fn differing_fields(path: &str, first: &Value, second: &Value, out: &mut Vec<Field>) {
    match (first, second) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for key in keys {
                differing_fields(
                    &format!("{path}.{key}"),
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        _ if first != second => out.push(Field {
            path: path.to_owned(),
            first: first.clone(),
            second: second.clone(),
        }),
        _ => {}
    }
}

/// An entry of an output tree.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Dir,
    File,
    Link(PathBuf),
    Other,
}

impl Entry {
    fn name(&self) -> String {
        match self {
            Self::Dir => "directory".to_owned(),
            Self::File => "file".to_owned(),
            Self::Link(target) => format!("link to {}", target.display()),
            Self::Other => "special file".to_owned(),
        }
    }
}

/// Compares two trees recursively: every entry, without following links, and every file byte
/// for byte.
///
/// # Errors
///
/// Fails when a directory, file or link cannot be read.
pub fn compare_trees(first: &Path, second: &Path) -> Result<Outputs, String> {
    let a = entries(first)?;
    let b = entries(second)?;
    let mut differences = Vec::new();
    for (path, entry) in &a {
        match b.get(path) {
            None => differences.push(Difference::OnlyInFirst {
                path: path.clone(),
                entry: entry.name(),
            }),
            Some(other) => {
                if let Some(difference) =
                    compare_entry(path, entry, other, &first.join(path), &second.join(path))?
                {
                    differences.push(difference);
                }
            }
        }
    }
    for (path, entry) in &b {
        if !a.contains_key(path) {
            differences.push(Difference::OnlyInSecond {
                path: path.clone(),
                entry: entry.name(),
            });
        }
    }
    differences.sort_by(|x, y| x.path().cmp(y.path()));
    Ok(Outputs {
        entries: a.len(),
        files: a.values().filter(|e| **e == Entry::File).count(),
        differences,
    })
}

fn compare_entry(
    path: &str,
    first: &Entry,
    second: &Entry,
    first_path: &Path,
    second_path: &Path,
) -> Result<Option<Difference>, String> {
    Ok(match (first, second) {
        (Entry::File, Entry::File) => {
            let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
            let (x, y) = (read(first_path)?, read(second_path)?);
            (x != y).then(|| {
                let offset = x
                    .iter()
                    .zip(&y)
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| x.len().min(y.len()));
                let line = x[..offset].split(|b| *b == b'\n').count();
                let bytes = |data: &[u8]| Bytes {
                    size: data.len(),
                    sha256: builds::sha256(data),
                };
                Difference::Bytes {
                    path: path.to_owned(),
                    first: bytes(&x),
                    second: bytes(&y),
                    offset,
                    line,
                }
            })
        }
        (Entry::Link(x), Entry::Link(y)) => (x != y).then(|| Difference::LinkTarget {
            path: path.to_owned(),
            first: x.display().to_string(),
            second: y.display().to_string(),
        }),
        (x, y) if x == y => None,
        (x, y) => Some(Difference::Kind {
            path: path.to_owned(),
            first: x.name(),
            second: y.name(),
        }),
    })
}

impl Difference {
    fn path(&self) -> &str {
        match self {
            Self::OnlyInFirst { path, .. }
            | Self::OnlyInSecond { path, .. }
            | Self::Kind { path, .. }
            | Self::LinkTarget { path, .. }
            | Self::Bytes { path, .. } => path,
        }
    }
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OnlyInFirst { path, entry } => write!(f, "{path}: only in build-1 ({entry})"),
            Self::OnlyInSecond { path, entry } => write!(f, "{path}: only in build-2 ({entry})"),
            Self::Kind {
                path,
                first,
                second,
            } => write!(f, "{path}: a {first} in build-1, a {second} in build-2"),
            Self::LinkTarget {
                path,
                first,
                second,
            } => write!(
                f,
                "{path}: links to {first} in build-1, to {second} in build-2"
            ),
            Self::Bytes {
                path,
                first,
                second,
                offset,
                line,
            } => write!(
                f,
                "{path}: bytes differ from byte {offset} (line {line}): build-1 {} bytes, sha256 {}; build-2 {} bytes, sha256 {}",
                first.size, first.sha256, second.size, second.sha256
            ),
        }
    }
}

impl std::fmt::Display for ReferenceDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OnlyInReference { path } => {
                write!(f, "{path}: in the reference, not written")
            }
            Self::OnlyInBuild { path } => write!(f, "{path}: written, not in the reference"),
            Self::Sha256 {
                path,
                reference,
                build,
            } => write!(f, "{path}: sha256 {build}, the reference's {reference}"),
        }
    }
}

/// Every entry below `root`, by its path relative to it with `/` separators.
fn entries(root: &Path) -> Result<BTreeMap<String, Entry>, String> {
    let mut out = BTreeMap::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Entry>) -> Result<(), String> {
    let error = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    for item in std::fs::read_dir(dir).map_err(|e| error(dir, e))? {
        let path = item.map_err(|e| error(dir, e))?.path();
        let kind = path
            .symlink_metadata()
            .map_err(|e| error(&path, e))?
            .file_type();
        let entry = if kind.is_symlink() {
            Entry::Link(std::fs::read_link(&path).map_err(|e| error(&path, e))?)
        } else if kind.is_dir() {
            walk(root, &path, out)?;
            Entry::Dir
        } else if kind.is_file() {
            Entry::File
        } else {
            Entry::Other
        };
        let relative = path.strip_prefix(root).expect("below the root");
        out.insert(relative.to_string_lossy().replace('\\', "/"), entry);
    }
    Ok(())
}

/// A build's manifest against the reference's, sorted by path.
fn against_reference(
    reference: &BTreeMap<String, String>,
    built: &BTreeMap<String, String>,
) -> Vec<ReferenceDifference> {
    let mut out = Vec::new();
    for (path, expected) in reference {
        match built.get(path) {
            None => out.push(ReferenceDifference::OnlyInReference { path: path.clone() }),
            Some(actual) if actual != expected => out.push(ReferenceDifference::Sha256 {
                path: path.clone(),
                reference: expected.clone(),
                build: actual.clone(),
            }),
            Some(_) => {}
        }
    }
    for path in built.keys() {
        if !reference.contains_key(path) {
            out.push(ReferenceDifference::OnlyInBuild { path: path.clone() });
        }
    }
    out.sort_by(|x, y| reference_path(x).cmp(reference_path(y)));
    out
}

fn reference_path(difference: &ReferenceDifference) -> &str {
    match difference {
        ReferenceDifference::OnlyInReference { path }
        | ReferenceDifference::OnlyInBuild { path }
        | ReferenceDifference::Sha256 { path, .. } => path,
    }
}

/// The report's lines: `ok:` or `FAILS:` per comparison, each failure followed by its
/// differences, indented; up to `limit` of each list when given (as the run prints them).
fn lines(report: &Report, limit: Option<usize>) -> Vec<String> {
    let mut out = Vec::new();
    let list = |out: &mut Vec<String>, items: Vec<String>| {
        let total = items.len();
        let shown = limit.unwrap_or(total);
        out.extend(
            items
                .into_iter()
                .take(shown)
                .map(|item| format!("  {item}")),
        );
        if total > shown {
            out.push(format!("  … and {} more, in the report", total - shown));
        }
    };
    for reason in &report.unknown_identities {
        out.push(format!("FAILS: source identity: {reason}"));
    }
    for build in &report.failed {
        out.push(format!("FAILS: Folia cannot build the site: {build}"));
    }
    if report.inputs.is_empty() {
        out.push("ok: the two builds have the same inputs (build.json).".to_owned());
    } else {
        out.push(format!(
            "FAILS: not comparable: the builds' inputs differ in {} fields, so their outputs say nothing about determinism:",
            report.inputs.len()
        ));
        list(&mut out, report.inputs.iter().map(field_line).collect());
    }
    if report.results.is_empty() {
        out.push(
            "ok: the two builds have the same result (summary, warnings, problems, file count)."
                .to_owned(),
        );
    } else {
        out.push(format!(
            "FAILS: nondeterminism: the builds' results differ in {} fields:",
            report.results.len()
        ));
        list(&mut out, report.results.iter().map(field_line).collect());
    }
    if let Some(outputs) = &report.outputs {
        if outputs.differences.is_empty() {
            out.push(format!(
                "ok: the two output trees are identical, byte for byte: {} entries, {} files.",
                outputs.entries, outputs.files
            ));
        } else {
            out.push(format!(
                "FAILS: nondeterminism: the two output trees differ in {} entries:",
                outputs.differences.len()
            ));
            list(
                &mut out,
                outputs
                    .differences
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            );
        }
    }
    if let Some(reference) = &report.reference {
        for build in &reference.builds {
            if build.differences.is_empty() {
                out.push(format!(
                    "ok: {} equals the reference {}: {} files.",
                    build.build, reference.path, reference.files
                ));
            } else {
                out.push(format!(
                    "FAILS: {} differs from the reference {} in {} files (a Folia change, to approve or fix):",
                    build.build,
                    reference.path,
                    build.differences.len()
                ));
                list(
                    &mut out,
                    build.differences.iter().map(ToString::to_string).collect(),
                );
            }
        }
    }
    out
}

fn field_line(field: &Field) -> String {
    format!("{}: {} / {}", field.path, field.first, field.second)
}

fn print(report: &Report, dir: &Path) {
    for line in lines(report, Some(SHOWN)) {
        println!("{line}");
    }
    if report.passed {
        println!(
            "Determinism holds; the builds and the report are kept in {}.",
            dir.display()
        );
    } else {
        println!(
            "Determinism does not hold; the builds and the report are kept in {} (see determinism-report.md).",
            dir.display()
        );
    }
}

/// The report as Markdown: every comparison's verdict, and every difference.
fn markdown(report: &Report) -> String {
    let mut out = String::from("# Determinism\n\n");
    let _ = writeln!(
        out,
        "Built twice as of `{}`, into `{}` and `{}`. Determinism {}.\n",
        report.at,
        report.builds[0],
        report.builds[1],
        if report.passed {
            "holds"
        } else {
            "does not hold"
        }
    );
    for line in lines(report, None) {
        let _ = match line.strip_prefix("  ") {
            Some(item) => writeln!(out, "  - {item}"),
            None => writeln!(out, "- {line}"),
        };
    }
    out
}

/// Writes a new file; an existing one is never overwritten.
fn write_new(path: &Path, text: &str) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests;
