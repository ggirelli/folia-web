//! Folia's content model and publishing contracts (plan v3, §5).
//!
//! - [`content`] describes what the author wrote and how it maps to the site.
//! - [`publish`] holds the types a mode hands to the site builder.
//!
//! Neither holds mode logic: modes decide what gets published, and the `folia` crate builds it.

pub mod content;
pub mod publish;
