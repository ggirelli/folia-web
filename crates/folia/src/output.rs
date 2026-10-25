//! Writing the site (plan v6, §6.5): every output goes into a temporary sibling of the
//! destination, which then takes the destination's place, so a failed build never leaves a
//! partial destination.
//!
//! - The destination must be outside the site, and must not hold it: a build replaces the whole
//!   destination.
//! - Outputs are written in output path order, into `.<name>.folia-new` next to the destination.
//!   On any failure that directory is removed, and the destination is left as it was.
//! - On success, the old destination is renamed to `.<name>.folia-old`, the new one renamed into
//!   its place, and the old one removed. Between the two renames the destination is briefly
//!   absent, but it is never partial; if the second rename fails, the old one is put back.

use std::collections::BTreeMap;
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

    remove_if_present(&new).map_err(|e| fail(&new, &e))?;
    if let Err(problem) = fill(&new, outputs) {
        let _ = fs::remove_dir_all(&new);
        return Err(problem);
    }
    swap(&new, &dest, &old)
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

fn swap(new: &Path, dest: &Path, old: &Path) -> Result<(), Diagnostic> {
    let had_dest = dest.exists();
    if had_dest {
        remove_if_present(old).map_err(|e| fail(old, &e))?;
        if let Err(e) = fs::rename(dest, old) {
            let _ = fs::remove_dir_all(new);
            return Err(fail(dest, &e));
        }
    }
    if let Err(e) = fs::rename(new, dest) {
        if had_dest {
            let _ = fs::rename(old, dest);
        }
        let _ = fs::remove_dir_all(new);
        return Err(fail(dest, &e));
    }
    if had_dest {
        fs::remove_dir_all(old).map_err(|e| fail(old, &e))?;
    }
    Ok(())
}

fn remove_if_present(dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn fail(path: &Path, error: &io::Error) -> Diagnostic {
    Diagnostic::site(format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
