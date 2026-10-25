//! Git dates: when each source file was last committed (plan v6, §7.3).
//!
//! As `jekyll-last-modified-at` does, a file's date is the commit time of
//! `git log -n 1 --format=%ct -- <path>`: committed history only, so uncommitted edits keep the
//! file's commit date, and a file no commit touches has no date. Folia never reads file mtimes.
//!
//! One `git log` runs per file; reading all dates in one pass is later risk 6.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use jiff::Timestamp;

use super::DocId;

/// Why git dates cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// `git` cannot be run.
    #[error("cannot run git: {0}")]
    Run(std::io::Error),
    /// `git log` failed, for example outside a repository.
    #[error("git log -- {0} failed: {1}")]
    Log(DocId, String),
    /// `git log` printed something that is not a commit time.
    #[error("git log -- {0} printed {1:?}, not a commit time")]
    Output(DocId, String),
}

/// The time of the last commit that touches each of some source files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LastCommits(BTreeMap<DocId, Timestamp>);

impl LastCommits {
    /// Reads, for each of `ids`, the time of the last commit reachable from `HEAD` that touches
    /// it, in the repository at the site root `root`.
    ///
    /// # Errors
    ///
    /// Fails when `git` cannot be run, `root` is not in a repository, or its output is not a
    /// commit time.
    pub fn read<'a>(
        root: &Path,
        ids: impl IntoIterator<Item = &'a DocId>,
    ) -> Result<Self, GitError> {
        let mut times = BTreeMap::new();
        for id in ids {
            if let Some(time) = last_commit(root, id)? {
                times.insert(id.clone(), time);
            }
        }
        Ok(Self(times))
    }

    /// The time of the last commit that touches `id`, if any does.
    #[must_use]
    pub fn get(&self, id: &DocId) -> Option<Timestamp> {
        self.0.get(id).copied()
    }
}

/// Commit times known in advance, as tests of the modes need them.
impl FromIterator<(DocId, Timestamp)> for LastCommits {
    fn from_iter<I: IntoIterator<Item = (DocId, Timestamp)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

fn last_commit(root: &Path, id: &DocId) -> Result<Option<Timestamp>, GitError> {
    let output = Command::new("git")
        .arg("--literal-pathspecs")
        .arg("-C")
        .arg(root)
        .args(["log", "-n", "1", "--format=%ct", "--", id.as_str()])
        .output()
        .map_err(GitError::Run)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(GitError::Log(id.clone(), stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if stdout.is_empty() {
        return Ok(None);
    }
    stdout
        .parse::<i64>()
        .ok()
        .and_then(|seconds| Timestamp::from_second(seconds).ok())
        .map(Some)
        .ok_or(GitError::Output(id.clone(), stdout))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A throwaway repository, removed when dropped. Git runs without the user's configuration,
    /// so commit signing or hooks set there cannot interfere.
    struct Repo(PathBuf);

    impl Repo {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("folia-git-{}-{test}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            let repo = Self(root);
            repo.git(&["init", "--quiet"], None);
            repo
        }

        fn git(&self, args: &[&str], date: Option<&str>) {
            let mut command = Command::new("git");
            command
                .arg("-C")
                .arg(&self.0)
                .args([
                    "-c",
                    "user.name=Folia",
                    "-c",
                    "user.email=folia@example.com",
                ])
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1");
            if let Some(date) = date {
                command
                    .env("GIT_AUTHOR_DATE", date)
                    .env("GIT_COMMITTER_DATE", date);
            }
            assert!(command.status().unwrap().success(), "git {args:?}");
        }

        fn commit(&self, path: &str, text: &str, date: &str) {
            std::fs::write(self.0.join(path), text).unwrap();
            self.git(&["add", path], None);
            self.git(&["commit", "--quiet", "-m", path], Some(date));
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_file_is_dated_by_its_last_commit() {
        let repo = Repo::new("last");
        repo.commit("a.md", "one", "2026-01-04T13:36:24+01:00");
        repo.commit("b.md", "two", "2026-02-01T00:00:00Z");
        repo.commit("a.md", "three", "2026-03-01T12:00:00Z");
        let a = DocId::new("a.md");
        let b = DocId::new("b.md");
        let commits = LastCommits::read(&repo.0, [&a, &b]).unwrap();
        assert_eq!(
            commits.get(&a),
            Some("2026-03-01T12:00:00Z".parse().unwrap())
        );
        assert_eq!(
            commits.get(&b),
            Some("2026-02-01T00:00:00Z".parse().unwrap())
        );
    }

    #[test]
    fn uncommitted_changes_keep_the_commit_date_and_new_files_have_none() {
        let repo = Repo::new("uncommitted");
        repo.commit("a.md", "one", "2026-01-04T12:36:24Z");
        std::fs::write(repo.0.join("a.md"), "edited").unwrap();
        std::fs::write(repo.0.join("new.md"), "new").unwrap();
        let a = DocId::new("a.md");
        let new = DocId::new("new.md");
        let commits = LastCommits::read(&repo.0, [&a, &new]).unwrap();
        assert_eq!(
            commits.get(&a),
            Some("2026-01-04T12:36:24Z".parse().unwrap())
        );
        assert_eq!(commits.get(&new), None);
    }

    #[test]
    fn a_site_outside_a_repository_is_an_error() {
        let root = std::env::temp_dir().join(format!("folia-git-{}-none", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let result = LastCommits::read(&root, [&DocId::new("a.md")]);
        let _ = std::fs::remove_dir_all(&root);
        assert!(matches!(result, Err(GitError::Log(..))));
    }
}
