//! The site's `/sitemap.xml`, as the `jekyll-sitemap` plugin (1.4) wrote it: its template,
//! minified as the plugin minifies it (whitespace after `>` and a line break, or after `}`,
//! removed). Serialized here, not by a template (plan v12, §3.8).
//!
//! It lists every page rendered but `/404.html`, then the static files Jekyll listed: those with
//! the extension `.htm`, `.html`, `.xhtml` or `.pdf`, but `404.html`. A URL ending in
//! `/index.html` is listed as its directory. Redirect stubs are not pages, so they are not
//! listed (the plugin skipped them as `sitemap: false`).
//!
//! A page's `<lastmod>` is, as Jekyll's `last_modified_at` was: an entry's `modified`, or else its
//! date; a claimed page's (a listing's, and its pages') last commit of its source; none for a page
//! without a source, such as a category archive. A static file's is its last commit, where Jekyll
//! wrote the file's modification time, which depends on the checkout (plan v12, Q3g); none
//! when no commit touches it.

use std::fmt::Write;

use folia_core::content::SiteClock;
use jiff::Timestamp;

use crate::feed::xml_escape;

#[cfg(test)]
mod tests;

/// An output the sitemap lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    /// Its URL, from the site root (`/blog/`).
    pub url: String,
    /// When it was last modified, if Folia knows.
    pub lastmod: Option<Timestamp>,
}

/// The extensions of the static files the sitemap lists.
pub const STATIC_EXTENSIONS: [&str; 4] = [".htm", ".html", ".xhtml", ".pdf"];

/// The sitemap of `urls`, in their order, for the site at `site_url` (without a trailing `/`).
#[must_use]
pub fn xml(site_url: &str, urls: &[Url], clock: &SiteClock) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://www.sitemaps.org/schemas/sitemap/0.9 \
         http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd\" \
         xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for url in urls {
        let path = url.url.replace("/index.html", "/");
        let _ = writeln!(
            out,
            "<url>\n<loc>{}</loc>",
            xml_escape(&format!("{site_url}{path}"))
        );
        if let Some(lastmod) = url.lastmod {
            let _ = writeln!(out, "<lastmod>{}</lastmod>", clock.xmlschema(lastmod));
        }
        out.push_str("</url>\n");
    }
    out.push_str("</urlset>\n");
    out
}
