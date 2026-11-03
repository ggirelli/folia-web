//! Folia builds that parity keeps (plan v12, §5.4): one entry point that builds the site with
//! Folia's library at an explicit time, keeps everything it wrote, and records what it was built
//! from, so that a build can be compared, rebuilt and compared again.
//!
//! A build directory, which must not exist beforehand, holds:
//!
//! | Path | What it is |
//! |-|-|
//! | `site/` | Folia's output, whole; absent if the build failed |
//! | `sha256.txt` | one line per file of `site/`, `<sha256>  ./<path>`, sorted by path bytewise (the A0 reference's format) |
//! | `build.json` | the inputs and the result ([`Record`]) |
//! | `build.log` | the summary, then each warning or problem, one per line |
//!
//! Nothing is ever deleted or overwritten: an existing destination, or one inside the site, is
//! refused before anything is written, and a failed build keeps its record and log.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use folia::build::{Built, build};
use folia::config::Config;
use folia_core::content::Diagnostic;
use jiff::Timestamp;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// What to build, and where.
pub struct Request<'a> {
    /// The site's checkout; its git history gives the documents' dates.
    pub site: &'a Path,
    /// Folia's configuration, as read from `config_path`.
    pub config: &'a Config,
    /// The file `config` was read from, which is recorded by its hash.
    pub config_path: &'a Path,
    /// The build directory to create.
    pub dest: &'a Path,
    /// The build time: what is future-dated, and the year the pages show.
    pub at: Timestamp,
    /// The templates' expected tree hash ([`Templates::tree_sha256`]), when the caller knows it;
    /// a different tree is refused before anything is written.
    pub expected_templates: Option<&'a str>,
}

/// A kept build.
pub struct Kept {
    /// The build directory.
    pub dir: PathBuf,
    /// Folia's output in it, `site/`.
    pub output: PathBuf,
    /// What `build.json` records.
    pub record: Record,
    /// What Folia returned.
    pub built: Result<Built, Vec<Diagnostic>>,
}

/// The inputs and the result of a build, written as `build.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Record {
    /// The build time, as given.
    pub at: String,
    /// The site's checkout.
    pub site: Checkout,
    /// The configuration file.
    pub config: Hashed,
    /// The templates directory the configuration names, below the site or absolute.
    pub templates: Templates,
    /// The checkout of the Folia source this parity tool was built from, when it is one.
    pub folia: Checkout,
    /// The Rust compiler this parity tool was built with, when known.
    pub rustc: Option<String>,
    /// Whether the build succeeded, with its summary, warnings and problems.
    pub result: Outcome,
    /// How many files `site/` holds; `None` when the build failed.
    pub files: Option<usize>,
}

/// A directory, and the git commit it is checked out at, when it is in a repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Checkout {
    pub path: String,
    /// `git rev-parse HEAD`; `None` when it is not a checkout or git cannot say.
    pub commit: Option<String>,
    /// `git status --porcelain`, one entry per line; `None` when `commit` is.
    pub status: Option<Vec<String>>,
    /// The identity of the working tree beyond its commit ([`changes`]): clean, changed with the
    /// SHA-256 of its uncommitted changes, or unknown; unknown too when `commit` is `None`, since
    /// a directory without a commit has no git identity.
    pub changes: Changes,
}

/// A working tree's uncommitted changes, as [`changes`] reads them. Written in `build.json` as
/// `{"state": "clean"}`, `{"state": "changed", "sha256": …}` or `{"state": "unknown", "error": …}`:
/// a git failure is never recorded as a clean tree, nor given a hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Changes {
    /// No staged, unstaged or untracked (and not ignored) change.
    Clean,
    /// Changes, identified by the SHA-256 of their text ([`changes`]).
    Changed { sha256: String },
    /// Git could not say, or the tree holds what cannot be identified; `error` is why.
    Unknown { error: String },
}

/// A file and the SHA-256 of its bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Hashed {
    pub path: String,
    pub sha256: String,
}

/// A templates directory, identified by its contents: no commit is recorded, since the
/// templates may be untracked where they are read (`_folia/` in the site, plan v12, §1.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Templates {
    pub path: String,
    pub files: usize,
    /// The SHA-256 of the directory's manifest, in `sha256.txt`'s format.
    pub tree_sha256: String,
}

/// What the build returned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub ok: bool,
    /// Folia's counts, as `folia build` prints them; `None` when the build failed.
    pub summary: Option<String>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
}

/// Builds the site as `request` says into a new build directory, and keeps it.
///
/// A build that fails is not an error: its record says so, and its directory keeps the record and
/// the log. The `Err` cases are those where nothing could be built or kept.
///
/// # Errors
///
/// Fails, before writing anything, when the configuration or templates cannot be read, the
/// templates are not the expected ones, or the destination exists or lies inside the site (or
/// holds it); and when the build directory or its files cannot be written.
pub fn keep(request: &Request) -> Result<Kept, String> {
    let config = hashed(request.config_path)?;
    let templates = templates(&request.site.join(&request.config.templates_dir))?;
    if let Some(expected) = request.expected_templates
        && expected != templates.tree_sha256
    {
        return Err(format!(
            "the templates in {} have the tree hash {}, not the expected {expected}; nothing was built",
            templates.path, templates.tree_sha256
        ));
    }
    let dir = new_dir(request.dest, request.site)?;
    let output = dir.join("site");
    let built = build(request.site, request.config, &output, request.at);
    let (result, files) = match &built {
        Ok(built) => {
            let manifest = manifest(&output)?;
            write(&dir.join("sha256.txt"), &manifest_text(&manifest))?;
            (
                Outcome {
                    ok: true,
                    summary: Some(summary(built)),
                    warnings: built.warnings.iter().map(ToString::to_string).collect(),
                    problems: Vec::new(),
                },
                Some(manifest.len()),
            )
        }
        Err(problems) => (
            Outcome {
                ok: false,
                summary: None,
                warnings: Vec::new(),
                problems: problems.iter().map(ToString::to_string).collect(),
            },
            None,
        ),
    };
    let record = Record {
        at: request.at.to_string(),
        site: checkout(request.site),
        config,
        templates,
        folia: checkout(&folia_source()),
        rustc: option_env!("PARITY_RUSTC_VERSION")
            .filter(|version| !version.is_empty())
            .map(str::to_owned),
        result,
        files,
    };
    let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())? + "\n";
    write(&dir.join("build.json"), &json)?;
    write(&dir.join("build.log"), &log(&record))?;
    Ok(Kept {
        dir,
        output,
        record,
        built,
    })
}

/// A fresh directory for `dest`, created after checking it is new and apart from `site`.
fn new_dir(dest: &Path, site: &Path) -> Result<PathBuf, String> {
    let refuse = |reason: &str| format!("cannot keep a build in {}: {reason}", dest.display());
    if dest.symlink_metadata().is_ok() {
        return Err(refuse("it already exists, and is never overwritten"));
    }
    let dest = resolved(dest).map_err(|e| refuse(&e))?;
    let site = site
        .canonicalize()
        .map_err(|e| format!("{}: {e}", site.display()))?;
    if dest.starts_with(&site) {
        return Err(refuse("it is inside the site"));
    }
    if site.starts_with(&dest) {
        return Err(refuse("it holds the site"));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| refuse(&e.to_string()))?;
    }
    // `create_dir` fails if the directory appeared meanwhile, so it is never shared.
    std::fs::create_dir(&dest).map_err(|e| refuse(&e.to_string()))?;
    Ok(dest)
}

/// `path` made absolute with its existing ancestors resolved, so it compares with a
/// canonicalized site whatever symbolic links or `..` it was given with.
fn resolved(path: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let mut existing = absolute.as_path();
    let mut rest = Vec::new();
    while existing.symlink_metadata().is_err() {
        let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
            return Err("it has no existing ancestor".to_owned());
        };
        rest.push(name.to_owned());
        existing = parent;
    }
    let mut out = existing.canonicalize().map_err(|e| e.to_string())?;
    for name in rest.iter().rev() {
        if name == ".." || name == "." {
            return Err("it must not end in `.` or `..` segments that do not exist".to_owned());
        }
        out.push(name);
    }
    Ok(out)
}

/// Every file below `root`, as a path relative to it with `/` separators, sorted bytewise.
///
/// # Errors
///
/// Fails when a directory cannot be read.
pub fn manifest_paths(root: &Path) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    files(root, root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

/// Every file below `root`, as a path relative to it with `/` separators and the SHA-256 of its
/// bytes, sorted by path bytewise.
///
/// # Errors
///
/// Fails when a directory or file cannot be read.
pub fn manifest(root: &Path) -> Result<Vec<(String, String)>, String> {
    manifest_paths(root)?
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(root.join(&path)).map_err(|e| format!("{path}: {e}"))?;
            Ok((path, hex(&Sha256::digest(&bytes))))
        })
        .collect()
}

/// Reads a manifest written by [`manifest_text`] (a kept build's `sha256.txt`), as path to the
/// SHA-256 it records.
///
/// # Errors
///
/// Fails when the file cannot be read, or a line is not `<sha256>  ./<path>` with a lowercase
/// hexadecimal SHA-256, or a path is listed twice.
pub fn read_manifest(file: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut out = std::collections::BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let bad = || {
            format!(
                "{} line {}: not `<sha256>  ./<path>`",
                file.display(),
                index + 1
            )
        };
        let (sha256, path) = line.split_once("  ./").ok_or_else(bad)?;
        let hex = sha256.len() == 64
            && sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !hex || path.is_empty() {
            return Err(bad());
        }
        if out.insert(path.to_owned(), sha256.to_owned()).is_some() {
            return Err(format!("{}: {path} is listed twice", file.display()));
        }
    }
    Ok(out)
}

/// A manifest as `sha256sum` writes it for `./`-prefixed paths: `<sha256>  ./<path>`.
pub fn manifest_text(manifest: &[(String, String)]) -> String {
    let mut out = String::new();
    for (path, sha256) in manifest {
        let _ = writeln!(out, "{sha256}  ./{path}");
    }
    out
}

fn files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            files(root, &path, out)?;
        } else {
            let relative = path.strip_prefix(root).expect("below the root");
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn templates(dir: &Path) -> Result<Templates, String> {
    let manifest = manifest(dir).map_err(|e| format!("the templates: {e}"))?;
    Ok(Templates {
        path: dir.display().to_string(),
        files: manifest.len(),
        tree_sha256: hex(&Sha256::digest(manifest_text(&manifest).as_bytes())),
    })
}

fn hashed(path: &Path) -> Result<Hashed, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Hashed {
        path: path.display().to_string(),
        sha256: hex(&Sha256::digest(&bytes)),
    })
}

/// The workspace this parity tool was built from.
fn folia_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `dir`'s commit and status, read with git without taking its index lock.
fn checkout(dir: &Path) -> Checkout {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("--no-optional-locks")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .map_err(|e| format!("git {} could not run ({e})", args.join(" ")))
            .and_then(|out| {
                if out.status.success() {
                    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
                } else {
                    Err(format!(
                        "git {} failed: {}",
                        args.join(" "),
                        String::from_utf8_lossy(&out.stderr).trim()
                    ))
                }
            })
    };
    let head = git(&["rev-parse", "HEAD"]).map(|head| head.trim().to_owned());
    let commit = head.as_ref().ok().cloned();
    let status = commit.as_ref().and_then(|_| {
        git(&["status", "--porcelain"])
            .ok()
            .map(|text| text.lines().map(str::to_owned).collect())
    });
    // Without a commit there is no git identity to record: the directory is not a checkout (or
    // has no commit, or git cannot run), and its identity is unknown, never left out.
    let changes = match head {
        Ok(_) => changes(dir),
        Err(error) => Changes::Unknown {
            error: format!("it has no git identity: {error}"),
        },
    };
    Checkout {
        path: dir
            .canonicalize()
            .unwrap_or_else(|_| dir.to_owned())
            .display()
            .to_string(),
        commit,
        status,
        changes,
    }
}

/// `dir`'s uncommitted changes: [`Changes::Clean`] when there are none, else the SHA-256 of
/// their text, and [`Changes::Unknown`] when a git command fails (never a clean tree or a hash).
/// The hashed text has three parts, each led by its name and a newline:
///
/// - `staged`: `git diff --cached` (HEAD to the index);
/// - `unstaged`: `git diff` (the index to the working tree);
/// - `untracked`: for each untracked file that is not ignored, in git's order, its path, a NUL,
///   the SHA-256 of its bytes (of `symlink:` and its target for a symbolic link) and a newline.
///   The path is read as git gives it, byte for byte, so a name that is not UTF-8 still names its
///   file (on platforms where a path must be UTF-8, such a name is refused as unknown).
///
/// What cannot be identified this way makes the changes [`Changes::Unknown`], naming it and why,
/// never a hash that would leave it out: an untracked file or link that cannot be read; an
/// untracked nested repository (git lists it as a directory, not its files); and a submodule (a
/// gitlink in the index or `HEAD`, whose working tree the diffs give only as a commit and a
/// `-dirty` marker). Nested repositories and submodules are not supported.
///
/// The diffs are binary with full object ids, of the whole repository whatever the directory
/// (`--no-relative`), with renames off. Every diff setting that configuration can change, and
/// that would change the text, is given on the command line: path quoting (`core.quotePath`),
/// prefixes (`diff.noprefix`, `diff.mnemonicPrefix`), external diff and text conversion drivers,
/// colour, the file order (`diff.orderFile`, by `-O/dev/null`), the algorithm (`diff.algorithm`)
/// and indent heuristic (`diff.indentHeuristic`), the context (`diff.context`, `-U3`;
/// `diff.interHunkContext`, `0`), blank context lines (`diff.suppressBlankEmpty`), and
/// submodules (`diff.ignoreSubmodules` and `submodule.*.ignore`, `diff.submodule`). The
/// environment variable `GIT_DIFF_OPTS` is cleared.
///
/// Not pinned, deliberately: which untracked files are ignored follows `.gitignore`,
/// `.git/info/exclude` and the user's `core.excludesFile`, as `git status` (recorded beside it)
/// does; and content filters (`core.autocrlf`, attributes) apply as git applies them.
pub fn changes(dir: &Path) -> Changes {
    match changes_text(dir) {
        Ok(None) => Changes::Clean,
        Ok(Some(text)) => Changes::Changed {
            sha256: hex(&Sha256::digest(&text)),
        },
        Err(error) => Changes::Unknown { error },
    }
}

/// The text [`changes`] hashes, `None` when there are no changes.
fn changes_text(dir: &Path) -> Result<Option<Vec<u8>>, String> {
    let git = |args: &[&str]| -> Result<Vec<u8>, String> {
        let out = Command::new("git")
            .arg("--no-optional-locks")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIFF_OPTS")
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(format!(
                "git {} failed ({}): {}",
                args.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    };
    let diff = |cached: bool| {
        let mut args = vec![
            "-c",
            "core.quotePath=true",
            "-c",
            "diff.suppressBlankEmpty=false",
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--no-color",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "-O/dev/null",
            "--diff-algorithm=myers",
            "--no-indent-heuristic",
            "-U3",
            "--inter-hunk-context=0",
            "--no-relative",
            "--ignore-submodules=none",
            "--submodule=short",
        ];
        if cached {
            args.push("--cached");
        }
        git(&args)
    };
    let top = git(&["rev-parse", "--show-toplevel"])?;
    let top = PathBuf::from(String::from_utf8_lossy(&top).trim());
    refuse_submodules(
        &git(&["ls-files", "--stage", "-z", "--full-name", ":/"])?,
        "index",
    )?;
    refuse_submodules(
        &git(&["ls-tree", "-r", "-z", "--full-tree", "HEAD"])?,
        "HEAD",
    )?;
    let (staged, unstaged) = (diff(true)?, diff(false)?);
    let untracked = git(&[
        "ls-files",
        "--others",
        "--exclude-standard",
        "-z",
        "--full-name",
        ":/",
    ])?;
    let mut text = b"staged\n".to_vec();
    text.extend(&staged);
    text.extend(b"unstaged\n");
    text.extend(&unstaged);
    text.extend(b"untracked\n");
    let any_untracked = untracked_text(&top, &untracked, &mut text)?;
    Ok((any_untracked || !staged.is_empty() || !unstaged.is_empty()).then_some(text))
}

/// Appends to `text` each untracked file of `listing` (`git ls-files --others -z`, paths from the
/// repository's top `top`): its path, a NUL, the SHA-256 of its contents and a newline. Returns
/// whether there was any; fails on what cannot be identified (see [`changes`]).
fn untracked_text(top: &Path, listing: &[u8], text: &mut Vec<u8>) -> Result<bool, String> {
    let mut any_untracked = false;
    for path in listing
        .split(|&byte| byte == 0)
        .filter(|path| !path.is_empty())
    {
        any_untracked = true;
        let file = top.join(git_path(path)?);
        let metadata = file.symlink_metadata();
        if path.ends_with(b"/") || metadata.as_ref().is_ok_and(std::fs::Metadata::is_dir) {
            return Err(format!(
                "the untracked directory {} is a nested repository, whose contents cannot be \
                 identified; nested repositories are not supported",
                String::from_utf8_lossy(path)
            ));
        }
        let unreadable = |e: std::io::Error| {
            format!(
                "the untracked file {} cannot be read ({e}), so its contents cannot be identified",
                String::from_utf8_lossy(path)
            )
        };
        let content = match metadata {
            Ok(meta) if meta.file_type().is_symlink() => std::fs::read_link(&file)
                .map(|target| [b"symlink:", target.as_os_str().as_encoded_bytes()].concat())
                .map_err(unreadable)?,
            _ => std::fs::read(&file).map_err(unreadable)?,
        };
        text.extend(path);
        text.push(0);
        text.extend(hex(&Sha256::digest(&content)).as_bytes());
        text.push(b'\n');
    }
    Ok(any_untracked)
}

/// Refuses a submodule: an entry of mode `160000` (a gitlink) in `listing`, the `-z` output of
/// `git ls-files --stage` or `git ls-tree -r` (each entry starts with its mode). A submodule's
/// working tree is not in the diffs, which give only its commit and a `-dirty` marker, so two
/// different states of it would hash the same.
fn refuse_submodules(listing: &[u8], place: &str) -> Result<(), String> {
    let gitlink = listing
        .split(|&byte| byte == 0)
        .find(|entry| entry.starts_with(b"160000 "));
    match gitlink {
        None => Ok(()),
        Some(entry) => {
            let path = entry
                .iter()
                .position(|&byte| byte == b'\t')
                .map_or(entry, |tab| &entry[tab + 1..]);
            Err(format!(
                "the submodule {} (in the {place}) cannot be identified: its working tree is not \
                 part of the diffs; submodules are not supported",
                String::from_utf8_lossy(path)
            ))
        }
    }
}

/// Why each source of `record`, the site's and Folia's checkout, has no complete identity: one
/// line for each whose changes are [`Changes::Unknown`]. A parity run fails on any (plan v12,
/// §5.4): its result cannot be trusted when what it was built from cannot be said.
pub fn unknown_identities(record: &Record) -> Vec<String> {
    [("site", &record.site), ("Folia source", &record.folia)]
        .into_iter()
        .filter_map(|(name, checkout)| match &checkout.changes {
            Changes::Unknown { error } => Some(format!(
                "the {name}'s identity ({}) is unknown: {error}",
                checkout.path
            )),
            _ => None,
        })
        .collect()
}

/// A path as git writes it with `-z`, relative to the repository's top: its bytes as they are.
#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as on platforms where a path must be UTF-8"
)]
fn git_path(bytes: &[u8]) -> Result<PathBuf, String> {
    use std::os::unix::ffi::OsStrExt as _;
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

/// A path as git writes it with `-z`: elsewhere a path must be UTF-8, and one that is not is
/// refused rather than read as another name.
#[cfg(not(unix))]
fn git_path(bytes: &[u8]) -> Result<PathBuf, String> {
    std::str::from_utf8(bytes).map(PathBuf::from).map_err(|_| {
        format!(
            "the untracked path {} is not UTF-8",
            String::from_utf8_lossy(bytes)
        )
    })
}

/// Folia's counts, worded as `folia build` prints them, without the destination.
fn summary(built: &Built) -> String {
    format!(
        "Built {} pages, {} redirect stubs, {} generated files and {} static files; {} planned outputs are not built yet.",
        built.pages.len(),
        built.stubs.len(),
        built.artifacts.len(),
        built.statics,
        built.not_built
    )
}

fn log(record: &Record) -> String {
    let result = &record.result;
    let mut out = String::new();
    match &result.summary {
        Some(summary) => {
            let _ = writeln!(out, "{summary}");
        }
        None => {
            let _ = writeln!(
                out,
                "The build failed with {} problems; Folia wrote nothing.",
                result.problems.len()
            );
        }
    }
    for warning in &result.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    for problem in &result.problems {
        let _ = writeln!(out, "problem: {problem}");
    }
    for checkout in [&record.site, &record.folia] {
        if let Changes::Unknown { error } = &checkout.changes {
            let _ = writeln!(
                out,
                "record: the uncommitted changes of {} are unknown: {error}",
                checkout.path
            );
        }
    }
    out
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The SHA-256 of `bytes`, in lowercase hexadecimal, as the output manifest records it.
pub fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
pub(crate) mod tests;
