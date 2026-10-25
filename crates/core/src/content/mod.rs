//! What the author wrote and how it maps to the site: source documents, front matter, routes,
//! slugs, dates, git dates and Markdown. It knows nothing of feeds, search or listings.

mod date;
mod diagnostic;
mod front_matter;
mod git;
pub mod markdown;
mod route;
mod slug;
mod source;

pub use date::{DateError, SiteClock, dated_file_name};
pub use diagnostic::Diagnostic;
pub use front_matter::{FrontMatter, FrontMatterError, has_front_matter, split_front_matter};
pub use git::{GitError, LastCommits};
pub use route::{OutputKind, Route, RouteError};
pub use slug::{SlugMode, slugify};
pub use source::{DocId, Mode, SourceDoc, SourceError};
