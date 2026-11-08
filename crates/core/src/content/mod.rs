//! What the author wrote and how it maps to the site: source documents, front matter, routes,
//! slugs, dates, git dates, last modifications, presentation metadata and Markdown. It knows
//! nothing of feeds, search or listings.

mod date;
mod diagnostic;
mod front_matter;
mod git;
pub mod markdown;
mod modified;
mod presentation;
mod route;
mod slug;
mod source;

pub use date::{DateError, SiteClock, dated_file_name};
pub use diagnostic::Diagnostic;
pub use front_matter::{FrontMatter, FrontMatterError, has_front_matter, split_front_matter};
pub use git::{GitError, LastCommits};
pub use modified::{Modified, ModifiedFrom, modified};
pub use presentation::{
    Cover, Toc, TocPlacement, cover, hide_website_junk, interactions, syndications,
    table_of_contents,
};
pub use route::{OutputKind, Route, RouteError, encode_path};
pub use slug::{SlugMode, slugify};
pub use source::{DocId, Mode, SourceDoc, SourceError};
