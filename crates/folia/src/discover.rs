//! Discovery: which of the site's files are documents, which are static files, and which are
//! not part of the site (plan v3, §11 step 2).
//!
//! The rules are Jekyll's, which the site was written for:
//!
//! - A name that starts with `.`, `_`, `#` or `~`, or ends with `~`, is hidden; so is anything
//!   below it. `include` publishes such a name anyway; a name ending with `.` never is.
//! - `exclude` patterns remove paths, and everything below a matching directory.
//! - Below a mode root, every file is a document of that mode. Below the blog root, only files
//!   named with a date (`2021-08-08-title.md`) are posts; Jekyll ignores the others.
//! - Anywhere else, a file with front matter is a page, and any other file is static.
//!
//! Jekyll applies the hidden-name rule below the micro and garden roots to the top directory and
//! the file name only; Folia applies it to every segment.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use folia_core::content::{Diagnostic, DocId, Mode, dated_file_name, has_front_matter};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::config::Config;

/// What discovery found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    /// Documents, with the mode each belongs to.
    pub documents: Vec<(Mode, DocId)>,
    /// Files copied as they are.
    pub statics: Vec<DocId>,
    /// Paths left out of the site, and why. A directory stands for everything below it.
    pub excluded: Vec<(String, &'static str)>,
}

/// Finds the documents and static files of the site at `root`.
///
/// # Errors
///
/// Returns every problem found: unreadable directories or file names that are not UTF-8,
/// invalid `exclude` or `include` patterns, and files without front matter in the micro or
/// garden root, which Folia does not publish yet.
pub fn discover(root: &Path, config: &Config) -> Result<Discovery, Vec<Diagnostic>> {
    let mut walker = Walker {
        root,
        roots: [
            (Mode::Blog, &config.roots.blog),
            (Mode::Micro, &config.roots.micro),
            (Mode::Garden, &config.roots.garden),
        ],
        include: glob_set(&config.include).map_err(|e| vec![e])?,
        exclude: glob_set(&config.exclude).map_err(|e| vec![e])?,
        found: Discovery::default(),
        problems: Vec::new(),
    };
    walker.walk("");
    if walker.problems.is_empty() {
        Ok(walker.found)
    } else {
        Err(walker.problems)
    }
}

struct Walker<'a> {
    root: &'a Path,
    roots: [(Mode, &'a String); 3],
    include: GlobSet,
    exclude: GlobSet,
    found: Discovery,
    problems: Vec<Diagnostic>,
}

impl Walker<'_> {
    /// Walks a directory outside the mode roots.
    fn walk(&mut self, dir: &str) {
        for (name, is_dir) in self.entries(dir) {
            let path = join(dir, &name);
            if let Some(mode) = self.mode_root(&path) {
                self.walk_mode_root(mode, &path, &path);
            } else if let Some(reason) = self.excluded(&path, &name) {
                self.found.excluded.push((path, reason));
            } else if is_dir {
                self.walk(&path);
            } else if self.has_front_matter(&path) {
                self.found.documents.push((Mode::Pages, DocId::new(path)));
            } else {
                self.found.statics.push(DocId::new(path));
            }
        }
    }

    /// Walks a directory below a mode root, where every file is a document of that mode.
    fn walk_mode_root(&mut self, mode: Mode, mode_root: &str, dir: &str) {
        for (name, is_dir) in self.entries(dir) {
            let path = join(dir, &name);
            if let Some(reason) = self.excluded(&path, &name) {
                self.found.excluded.push((path, reason));
            } else if is_dir {
                self.walk_mode_root(mode, mode_root, &path);
            } else if mode == Mode::Blog {
                if matches!(dated_file_name(&name), Ok(None)) {
                    self.found
                        .excluded
                        .push((path, "not named with a date, so not a post"));
                } else {
                    self.found.documents.push((mode, DocId::new(path)));
                }
            } else if self.has_front_matter(&path) {
                self.found.documents.push((mode, DocId::new(path)));
            } else {
                self.problems.push(Diagnostic::site(format!(
                    "{path}: files without front matter in the {mode_root} root are not published yet"
                )));
            }
        }
    }

    /// The entries of a directory, sorted by name, with whether each is a directory.
    fn entries(&mut self, dir: &str) -> Vec<(String, bool)> {
        let read = match std::fs::read_dir(self.root.join(dir)) {
            Ok(read) => read,
            Err(error) => {
                self.problems.push(Diagnostic::site(format!(
                    "cannot read the directory {dir:?}: {error}"
                )));
                return Vec::new();
            }
        };
        let mut entries = Vec::new();
        for entry in read {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    self.problems.push(Diagnostic::site(format!(
                        "cannot read the directory {dir:?}: {error}"
                    )));
                    continue;
                }
            };
            match entry.file_name().into_string() {
                Ok(name) => entries.push((name, entry.path().is_dir())),
                Err(name) => self.problems.push(Diagnostic::site(format!(
                    "{dir}/{}: the file name is not UTF-8",
                    name.display()
                ))),
            }
        }
        entries.sort();
        entries
    }

    fn mode_root(&self, path: &str) -> Option<Mode> {
        self.roots
            .iter()
            .find(|(_, root)| root.trim_matches('/') == path)
            .map(|(mode, _)| *mode)
    }

    /// Why a path is left out of the site, or `None` when it is part of it.
    fn excluded(&self, path: &str, name: &str) -> Option<&'static str> {
        if name.ends_with('.') {
            return Some("the name ends with a dot");
        }
        if self.include.is_match(path) || self.include.is_match(name) {
            return None;
        }
        if self.exclude.is_match(path) {
            return Some("excluded by the configuration");
        }
        if name.starts_with(['.', '_', '#', '~']) || name.ends_with('~') {
            return Some("a hidden name");
        }
        None
    }

    /// Whether a file opens with a front matter fence, reading only its first line.
    fn has_front_matter(&mut self, path: &str) -> bool {
        let mut first_line = Vec::new();
        let read = std::fs::File::open(self.root.join(path))
            .and_then(|file| BufReader::new(file.take(4096)).read_until(b'\n', &mut first_line));
        match read {
            Ok(_) => has_front_matter(&first_line),
            Err(error) => {
                self.problems
                    .push(Diagnostic::site(format!("cannot read {path}: {error}")));
                false
            }
        }
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// The patterns as one set, anchored at the site root; `*` stays within a path segment.
fn glob_set(patterns: &[String]) -> Result<GlobSet, Diagnostic> {
    let mut set = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern.trim_start_matches('/'))
            .literal_separator(true)
            .build()
            .map_err(|e| Diagnostic::site(format!("invalid pattern {pattern:?}: {e}")))?;
        set.add(glob);
    }
    set.build()
        .map_err(|e| Diagnostic::site(format!("invalid patterns: {e}")))
}

#[cfg(test)]
mod tests;
