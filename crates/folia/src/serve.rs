//! `folia serve`: builds the site, serves it over HTTP on the local machine, and builds it again,
//! whole, each time its sources change, telling the open pages to reload.
//!
//! URLs are mapped to files as the host maps them ([`files`]); [`reload`] is the live reload,
//! whose script goes into the HTML pages as they are served, never into the files; [`watch`]
//! tells the builder when the sources changed.

pub mod files;
pub mod reload;
pub mod watch;
