//! When a document was last modified (plan v7, §7.3), and where Folia found it out.
//!
//! The first available of: the front matter's `last_modified_at`, read in the site's time zone;
//! the time of the last commit that touches the file (`LastCommits`); and when it was published.

use jiff::Timestamp;

use super::{Diagnostic, LastCommits, SiteClock, SourceDoc};

/// A document's last modification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modified {
    /// When it was.
    pub at: Timestamp,
    /// Where it was found.
    pub from: ModifiedFrom,
}

/// Where a last modification was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifiedFrom {
    /// The front matter's `last_modified_at`.
    FrontMatter,
    /// The last commit that touches the file.
    Git,
    /// None of these: the document's publication.
    Published,
}

impl ModifiedFrom {
    /// The name the baseline dump gives it (`front_matter`, `git`, `published`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FrontMatter => "front_matter",
            Self::Git => "git",
            Self::Published => "published",
        }
    }
}

/// When `doc`, published at `published`, was last modified; `None` when nothing dates it.
///
/// # Errors
///
/// Fails when the front matter's `last_modified_at` is not a date Folia reads.
pub fn modified(
    doc: &SourceDoc,
    commits: &LastCommits,
    published: Option<Timestamp>,
    clock: &SiteClock,
) -> Result<Option<Modified>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    if let Some(text) = doc
        .front
        .string("last_modified_at")
        .map_err(|e| problem(&e))?
    {
        let at = clock.parse(&text).map_err(|e| problem(&e))?;
        return Ok(Some(Modified {
            at,
            from: ModifiedFrom::FrontMatter,
        }));
    }
    let found = commits
        .get(&doc.id)
        .map(|at| (at, ModifiedFrom::Git))
        .or(published.map(|at| (at, ModifiedFrom::Published)));
    Ok(found.map(|(at, from)| Modified { at, from }))
}

#[cfg(test)]
mod tests {
    use super::super::{DocId, FrontMatter, Mode};
    use super::*;

    fn doc(front: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new("_posts/2021/2021-08-08-sample-post.md"),
            mode: Mode::Blog,
            front: FrontMatter::parse(front).unwrap(),
            body: String::new(),
        }
    }

    fn clock() -> SiteClock {
        SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
    }

    fn commits() -> LastCommits {
        [(
            DocId::new("_posts/2021/2021-08-08-sample-post.md"),
            "2025-06-17T10:00:00Z".parse().unwrap(),
        )]
        .into_iter()
        .collect()
    }

    #[test]
    fn the_front_matter_comes_first_at_midnight_in_the_site_time_zone() {
        let found = modified(
            &doc("last_modified_at: 2021-08-24"),
            &commits(),
            None,
            &clock(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.from, ModifiedFrom::FrontMatter);
        assert_eq!(clock().xmlschema(found.at), "2021-08-24T00:00:00+02:00");
    }

    #[test]
    fn then_the_last_commit_then_the_publication() {
        let published: Timestamp = "2021-08-07T22:00:00Z".parse().unwrap();
        let git = modified(&doc(""), &commits(), Some(published), &clock())
            .unwrap()
            .unwrap();
        assert_eq!(git.from, ModifiedFrom::Git);
        let fallback = modified(&doc(""), &LastCommits::default(), Some(published), &clock())
            .unwrap()
            .unwrap();
        assert_eq!(
            fallback,
            Modified {
                at: published,
                from: ModifiedFrom::Published
            }
        );
        assert_eq!(
            modified(&doc(""), &LastCommits::default(), None, &clock()).unwrap(),
            None
        );
    }

    #[test]
    fn a_front_matter_date_folia_cannot_read_is_a_problem() {
        let problem =
            modified(&doc("last_modified_at: soon"), &commits(), None, &clock()).unwrap_err();
        assert!(problem.message.contains("\"soon\""), "{problem}");
    }
}
