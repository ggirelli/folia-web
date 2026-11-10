//! Updating, in place, a destination `folia serve` wrote: only the files whose bytes changed are
//! written, so that a rebuild of a large site costs what it changed, not the whole site.
//!
//! 1. The record is rewritten to vouch for both the entries the destination holds and those the
//!    build makes, so that a serve stopped midway leaves a destination the next one may claim.
//! 2. What the build no longer makes is removed (a file where a directory now goes too, and the
//!    reverse). The record vouched for it: it is Folia's to remove.
//! 3. Each output whose bytes differ from the file in its place is written into the scratch
//!    directory beside the destination (`.<name>.folia-new`, which a serve recovers if it is
//!    left), then renamed into place: the server never serves a file half-written.
//! 4. The record is rewritten for what the destination now holds.
//!
//! A failure partway leaves a destination partly updated, which the next build completes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use folia_core::content::{Diagnostic, Preview};

use super::{Content, Writer, check_relative, fail, top_level, write_record_of};

/// Updates `dest`, whose `record` vouches for it, to hold exactly `outputs`, using `scratch` for
/// the files being written.
pub(super) fn update(
    dest: &Path,
    scratch: &Path,
    record: &Path,
    outputs: &BTreeMap<String, Content>,
    preview: Preview,
) -> Result<(), Diagnostic> {
    for output in outputs.keys() {
        check_relative(output)?;
    }
    let mut entries = held(dest)?;
    entries.extend(top_level(outputs));
    write_record_of(record, dest, &entries, Writer::Serve, preview)?;

    let files: BTreeSet<&str> = outputs.keys().map(String::as_str).collect();
    prune(dest, "", &files, &directories(outputs))?;
    fs::create_dir(scratch).map_err(|e| fail(scratch, &e))?;
    let written = rewrite(dest, scratch, outputs);
    let _ = fs::remove_dir_all(scratch);
    written?;
    write_record_of(record, dest, &top_level(outputs), Writer::Serve, preview)
}

/// The names `dest` holds at its top level.
fn held(dest: &Path) -> Result<BTreeSet<String>, Diagnostic> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(dest).map_err(|e| fail(dest, &e))? {
        let entry = entry.map_err(|e| fail(dest, &e))?;
        names.insert(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// Every directory the outputs are in, as paths relative to the destination.
fn directories(outputs: &BTreeMap<String, Content>) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for output in outputs.keys() {
        let mut end = 0;
        while let Some(slash) = output[end..].find('/') {
            end += slash;
            dirs.insert(output[..end].to_owned());
            end += 1;
        }
    }
    dirs
}

/// Removes from `dir` (at `relative` in the destination) every file that is not one of `files`
/// and every directory that is not one of `dirs`.
fn prune(
    dir: &Path,
    relative: &str,
    files: &BTreeSet<&str>,
    dirs: &BTreeSet<String>,
) -> Result<(), Diagnostic> {
    for entry in fs::read_dir(dir).map_err(|e| partly(dir, &e))? {
        let entry = entry.map_err(|e| partly(dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let inner = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        let is_dir = entry.file_type().map_err(|e| partly(&path, &e))?.is_dir();
        if is_dir && dirs.contains(&inner) {
            prune(&path, &inner, files, dirs)?;
        } else if is_dir {
            fs::remove_dir_all(&path).map_err(|e| partly(&path, &e))?;
        } else if !files.contains(inner.as_str()) {
            fs::remove_file(&path).map_err(|e| partly(&path, &e))?;
        }
    }
    Ok(())
}

/// Writes into `dest` each output whose bytes differ from the file in its place, through
/// `scratch`.
fn rewrite(
    dest: &Path,
    scratch: &Path,
    outputs: &BTreeMap<String, Content>,
) -> Result<(), Diagnostic> {
    for (n, (output, content)) in outputs.iter().enumerate() {
        let path = dest.join(output);
        if unchanged(&path, content) {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| partly(parent, &e))?;
        }
        let tmp = scratch.join(n.to_string());
        match content {
            Content::Bytes(bytes) => fs::write(&tmp, bytes).map_err(|e| partly(&path, &e))?,
            Content::Copy(source) => {
                fs::copy(source, &tmp).map_err(|e| partly(source, &e))?;
            }
        }
        fs::rename(&tmp, &path).map_err(|e| partly(&path, &e))?;
    }
    Ok(())
}

/// Whether the file at `path` already holds `content`.
fn unchanged(path: &Path, content: &Content) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    match content {
        Content::Bytes(bytes) => {
            u64::try_from(bytes.len()).is_ok_and(|len| len == meta.len())
                && fs::read(path).is_ok_and(|held| held == *bytes)
        }
        Content::Copy(source) => {
            fs::metadata(source).is_ok_and(|source| source.len() == meta.len())
                && matches!((fs::read(source), fs::read(path)), (Ok(a), Ok(b)) if a == b)
        }
    }
}

fn partly(path: &Path, error: &io::Error) -> Diagnostic {
    Diagnostic::site(format!(
        "cannot write {}: {error}; the served site may be partly updated until the next build",
        path.display()
    ))
}
