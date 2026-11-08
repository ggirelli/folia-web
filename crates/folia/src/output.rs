//! Writing the site (plan v6, §6.5): every output goes into a temporary sibling of the
//! destination, which then takes the destination's place, so a failed build never leaves a
//! partial destination.
//!
//! - The destination must be outside the site, and must not hold it.
//! - **A destination is replaced only when Folia knows it wrote it.** An absent or empty
//!   destination is written. A non-empty one is replaced only if the record
//!   `.<name>.folia-output`, beside it (not in it, so that it is never deployed), names this very
//!   directory and lists every entry the destination holds at its top level: anything else
//!   (user files, a directory moved or edited since) is refused, and nothing is touched. Folia
//!   never deletes what it did not write.
//! - Outputs are written in output path order, into `.<name>.folia-new` next to the destination.
//!   A leftover `.folia-new` or `.folia-old` is refused, never deleted: it may not be Folia's. On
//!   any failure the directory Folia made is removed, and the destination is left as it was.
//! - On success, the old destination is renamed to `.<name>.folia-old`, the new one renamed into
//!   its place, and the old one (which the record vouched for) removed. Between the two renames
//!   the destination is briefly absent, but it is never partial; if the second rename fails, the
//!   old one is put back. Last, the record is rewritten.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use folia_core::content::Diagnostic;

/// What an output holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    /// These bytes.
    Bytes(Vec<u8>),
    /// A copy of this file.
    Copy(PathBuf),
}

/// Writes `outputs`, keyed by output path, as the destination `dest`, replacing whatever it
/// held, for the site at `site_root`.
///
/// # Errors
///
/// Fails, leaving the destination as it was, when the destination is inside the site or holds
/// it, is not a directory, or when any output cannot be written.
pub fn write(
    dest: &Path,
    site_root: &Path,
    outputs: &BTreeMap<String, Content>,
) -> Result<(), Diagnostic> {
    let (parent, name) = resolve(dest, site_root)?;
    let dest = parent.join(&name);
    let new = parent.join(format!(".{name}.folia-new"));
    let old = parent.join(format!(".{name}.folia-old"));
    let record = record_path(&parent, &name);
    for leftover in [&new, &old] {
        if fs::symlink_metadata(leftover).is_ok() {
            return Err(Diagnostic::site(format!(
                "cannot write the site to {}: {} exists, left by an interrupted build or not \
                 Folia's; Folia does not delete it: look at it, remove it, and build again",
                dest.display(),
                leftover.display()
            )));
        }
    }
    check_replaceable(&dest, &record)?;

    if let Err(problem) = fill(&new, outputs) {
        let _ = fs::remove_dir_all(&new);
        return Err(problem);
    }
    let had_dest = swap(&new, &dest, &old)?;
    write_record(&record, &dest, outputs)?;
    if had_dest {
        // The old directory is the previous output the record vouched for: Folia's to remove.
        fs::remove_dir_all(&old).map_err(|e| {
            Diagnostic::site(format!(
                "the site is written to {}, but the previous output {} cannot be removed ({e}); \
                 remove it before the next build",
                dest.display(),
                old.display()
            ))
        })?;
    }
    Ok(())
}

/// Where the record of what Folia wrote to `name` is kept.
fn record_path(parent: &Path, name: &str) -> PathBuf {
    parent.join(format!(".{name}.folia-output"))
}

const RECORD_HEADER: &str = "folia-output 1";

/// The top-level entries the outputs make, sorted and without duplicates.
fn top_level(outputs: &BTreeMap<String, Content>) -> BTreeSet<String> {
    outputs
        .keys()
        .filter_map(|path| path.split('/').next().map(str::to_owned))
        .collect()
}

/// The record's text: the header, the directory it vouches for, and each top-level entry.
fn record_text(dest: &Path, outputs: &BTreeMap<String, Content>) -> String {
    let mut text = format!("{RECORD_HEADER}\n{}\n", dest.display());
    for entry in top_level(outputs) {
        text.push_str(&entry);
        text.push('\n');
    }
    text
}

fn write_record(
    record: &Path,
    dest: &Path,
    outputs: &BTreeMap<String, Content>,
) -> Result<(), Diagnostic> {
    let tmp = record.with_extension("folia-output.tmp");
    fs::write(&tmp, record_text(dest, outputs))
        .and_then(|()| fs::rename(&tmp, record))
        .map_err(|e| {
            let _ = fs::remove_file(&tmp);
            Diagnostic::site(format!(
                "the site is written to {}, but its record {} is not ({e}): the next build will \
                 refuse this directory until it is emptied",
                dest.display(),
                record.display()
            ))
        })
}

/// Fails unless `dest` is absent, empty, or a previous Folia output its record vouches for.
fn check_replaceable(dest: &Path, record: &Path) -> Result<(), Diagnostic> {
    let refuse = |why: &str| {
        Diagnostic::site(format!(
            "cannot write the site to {}: {why}. Folia only replaces a directory it wrote itself; \
             nothing was changed. Build into a new or empty directory, or remove this one \
             yourself",
            dest.display()
        ))
    };
    let mut held = BTreeSet::new();
    match fs::read_dir(dest) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|e| fail(dest, &e))?;
                held.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(fail(dest, &e)),
    }
    if held.is_empty() {
        return Ok(());
    }
    let text = match fs::read_to_string(record) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(refuse(
                "it is not empty, and no record shows a Folia build wrote it",
            ));
        }
        Err(e) => return Err(fail(record, &e)),
    };
    let mut lines = text.lines();
    if lines.next() != Some(RECORD_HEADER) || lines.next() != Some(&dest.display().to_string()) {
        return Err(refuse(
            "it is not empty, and its record is not for this directory",
        ));
    }
    let wrote: BTreeSet<&str> = lines.collect();
    let foreign: Vec<&String> = held
        .iter()
        .filter(|e| !wrote.contains(e.as_str()))
        .collect();
    if let Some(first) = foreign.first() {
        return Err(refuse(&format!(
            "it holds {} entr{} Folia did not write ({first}{})",
            foreign.len(),
            if foreign.len() == 1 { "y" } else { "ies" },
            if foreign.len() > 1 { ", ..." } else { "" }
        )));
    }
    Ok(())
}

/// The destination's absolute parent directory, created if need be, and its name; checked to be
/// a directory or absent, and apart from the site.
fn resolve(dest: &Path, site_root: &Path) -> Result<(PathBuf, String), Diagnostic> {
    let refuse = |reason: &str| {
        Diagnostic::site(format!(
            "cannot write the site to {}: {reason}",
            dest.display()
        ))
    };
    let absolute = std::path::absolute(dest).map_err(|e| fail(dest, &e))?;
    let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) else {
        return Err(refuse("it has no parent directory"));
    };
    if matches!(
        absolute.components().next_back(),
        Some(Component::ParentDir | Component::CurDir)
    ) {
        return Err(refuse("it must name a directory, not `.` or `..`"));
    }
    fs::create_dir_all(parent).map_err(|e| fail(parent, &e))?;
    let parent = parent.canonicalize().map_err(|e| fail(parent, &e))?;
    let name = name.to_string_lossy().into_owned();
    let dest = parent.join(&name);
    let site = site_root.canonicalize().map_err(|e| fail(site_root, &e))?;
    if dest.starts_with(&site) {
        return Err(refuse("it is inside the site"));
    }
    if site.starts_with(&dest) {
        return Err(refuse("it holds the site"));
    }
    match fs::symlink_metadata(&dest) {
        Ok(meta) if !meta.is_dir() => Err(refuse("it exists and is not a directory")),
        Ok(_) => Ok((parent, name)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((parent, name)),
        Err(e) => Err(fail(&dest, &e)),
    }
}

fn fill(dir: &Path, outputs: &BTreeMap<String, Content>) -> Result<(), Diagnostic> {
    fs::create_dir(dir).map_err(|e| fail(dir, &e))?;
    for (output, content) in outputs {
        let relative = Path::new(output);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(Diagnostic::site(format!(
                "{output} is not a relative output path"
            )));
        }
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| fail(parent, &e))?;
        }
        match content {
            Content::Bytes(bytes) => fs::write(&path, bytes).map_err(|e| fail(&path, &e))?,
            Content::Copy(source) => {
                fs::copy(source, &path).map_err(|e| fail(source, &e))?;
            }
        }
    }
    Ok(())
}

/// Puts `new` in the place of `dest`, which, if it exists, becomes `old`. Returns whether it did.
fn swap(new: &Path, dest: &Path, old: &Path) -> Result<bool, Diagnostic> {
    let had_dest = dest.exists();
    if had_dest && let Err(e) = fs::rename(dest, old) {
        let _ = fs::remove_dir_all(new);
        return Err(fail(dest, &e));
    }
    if let Err(e) = fs::rename(new, dest) {
        if had_dest {
            let _ = fs::rename(old, dest);
        }
        let _ = fs::remove_dir_all(new);
        return Err(fail(dest, &e));
    }
    Ok(had_dest)
}

fn fail(path: &Path, error: &io::Error) -> Diagnostic {
    Diagnostic::site(format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
