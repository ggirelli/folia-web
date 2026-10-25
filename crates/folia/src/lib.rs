//! Folia, the site builder (plan v6, §4), and the `folia` command (`main.rs`).
//!
//! - [`config`]: the site's `folia.toml`.
//! - [`discover`] and [`site`]: the site's files, and the route table every output is planned in.
//! - [`build`]: the pipeline, so far for the first vertical slice (plan v6, §12.2 step 1).
//! - [`render`]: pages through their `MiniJinja` templates.
//! - [`output`]: the destination, written whole or not at all.

pub mod build;
pub mod config;
pub mod discover;
pub mod output;
pub mod render;
pub mod site;
