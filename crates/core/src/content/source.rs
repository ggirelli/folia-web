//! Source documents: what the author wrote, and which mode publishes it.

use std::fmt;
use std::path::Path;

use super::Diagnostic;
use super::front_matter::{FrontMatter, FrontMatterError, split_front_matter};
use super::route::Route;

/// A source file's path relative to the site root, with `/` separators: its stable identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocId(String);

impl DocId {
    /// The identity of the file at `path`, relative to the site root.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The file name, without its directory.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit_once('/').map_or(&self.0, |(_, name)| name)
    }

    /// The directory, without a trailing `/`; empty at the site root.
    #[must_use]
    pub fn dir(&self) -> &str {
        self.0.rsplit_once('/').map_or("", |(dir, _)| dir)
    }

    /// The extension, with its dot (`.md`), or an empty string. A leading dot does not start an
    /// extension: `.htaccess` has none.
    #[must_use]
    pub fn extension(&self) -> &str {
        let name = self.file_name();
        match name.rfind('.') {
            Some(0) | None => "",
            Some(i) => &name[i..],
        }
    }

    /// The file name without its extension or trailing dots.
    #[must_use]
    pub fn stem(&self) -> &str {
        let name = self.file_name();
        name[..name.len() - self.extension().len()].trim_end_matches('.')
    }

    /// The path below `dir`, or `None` when the file is not inside it.
    #[must_use]
    pub fn below(&self, dir: &str) -> Option<&str> {
        self.0.strip_prefix(dir)?.strip_prefix('/')
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The publishing mode a document belongs to, assigned by its source directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    /// Articles.
    Blog,
    /// Notes, likes, bookmarks and RSVPs.
    Micro,
    /// Notes connected by wiki-links.
    Garden,
    /// Individually authored pages.
    Pages,
}

/// Why a source document cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// The file cannot be read as UTF-8 text.
    #[error("{0}: {1}")]
    Io(DocId, std::io::Error),
    /// Its front matter cannot be read.
    #[error("{0}: {1}")]
    FrontMatter(DocId, FrontMatterError),
}

/// A document as the author wrote it.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceDoc {
    /// Its identity.
    pub id: DocId,
    /// The mode that publishes it.
    pub mode: Mode,
    /// Its front matter; empty when it has none.
    pub front: FrontMatter,
    /// The text after the front matter.
    pub body: String,
    /// When the file was last modified; `None` when that is not known. It dates a draft.
    pub file_time: Option<jiff::Timestamp>,
}

/// The directory, at the site's root, whose posts are drafts: read only when drafts are asked
/// for.
pub const DRAFTS_ROOT: &str = "_drafts";

impl SourceDoc {
    /// Reads the document `id` below the site root `root`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read as UTF-8, or its front matter cannot be read.
    pub fn read(root: &Path, id: DocId, mode: Mode) -> Result<Self, SourceError> {
        let path = root.join(id.as_str());
        let text = std::fs::read_to_string(&path).map_err(|e| SourceError::Io(id.clone(), e))?;
        let file_time = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| jiff::Timestamp::try_from(time).ok());
        let (front, body) = match split_front_matter(&text) {
            Some((yaml, body)) => (
                FrontMatter::parse(yaml).map_err(|e| SourceError::FrontMatter(id.clone(), e))?,
                body,
            ),
            None => (FrontMatter::default(), text.as_str()),
        };
        let body = body.to_owned();
        Ok(Self {
            id,
            mode,
            front,
            body,
            file_time,
        })
    }

    /// Whether this is a draft: a document below [`DRAFTS_ROOT`].
    #[must_use]
    pub fn is_draft(&self) -> bool {
        self.id
            .as_str()
            .strip_prefix(DRAFTS_ROOT)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The old URLs that redirect to this document (`redirect_from`).
    ///
    /// # Errors
    ///
    /// Fails when the value is not a URL or a list of them, or a URL is not a valid route.
    pub fn redirect_from(&self) -> Result<Vec<Route>, Diagnostic> {
        let problem = |message: &dyn fmt::Display| Diagnostic::in_doc(&self.id, message);
        self.front
            .strings("redirect_from")
            .map_err(|e| problem(&e))?
            .iter()
            .map(|from| Route::html(from).map_err(|e| problem(&e)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_doc_id_names_its_parts() {
        let id = DocId::new("_posts/2021/2021-08-08-sample-post.md");
        assert_eq!(id.file_name(), "2021-08-08-sample-post.md");
        assert_eq!(id.dir(), "_posts/2021");
        assert_eq!(id.extension(), ".md");
        assert_eq!(id.stem(), "2021-08-08-sample-post");
        assert_eq!(id.below("_posts"), Some("2021/2021-08-08-sample-post.md"));
        assert_eq!(id.below("_micro"), None);
    }

    #[test]
    fn a_dotfile_has_no_extension() {
        let id = DocId::new(".htaccess");
        assert_eq!(id.extension(), "");
        assert_eq!(id.stem(), ".htaccess");
        assert_eq!(DocId::new("CNAME").dir(), "");
    }
}
