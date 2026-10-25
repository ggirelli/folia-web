//! Site URLs, and the file each one is written to.
//!
//! One policy maps every URL to its output file (plan v3, §6.2), so modes never build file
//! paths. It is Jekyll's, which the host relies on: it serves `/x` from `x.html`.

use std::fmt;

/// How a route's URL maps to a file in the output directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OutputKind {
    /// A rendered HTML page: `/x` is written to `x.html`, `/x/` to `x/index.html`, and `/x.html`
    /// to itself.
    Html,
    /// Any other file, which its URL names exactly: `/feed.xml`, `/CNAME`.
    File,
}

/// A site URL path, such as `/blog/2021/08/08/webmentions` or `/dg/coding/`, and what kind of
/// output it names.
///
/// URLs are kept unescaped: they are the paths themselves, and escaping them is a matter for the
/// HTML that links to them.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Route {
    url: String,
    kind: OutputKind,
}

/// Why a URL cannot be a route.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RouteError {
    /// The URL is not an absolute path.
    #[error("route {0:?} does not start with '/'")]
    NotAbsolute(String),
    /// The URL climbs out of the site, or names the current directory.
    #[error("route {0:?} has a '.' or '..' segment")]
    DotSegment(String),
    /// A file route names a directory.
    #[error("route {0:?} names a file but ends with '/'")]
    FileIsDirectory(String),
}

impl Route {
    /// A route to a rendered HTML page.
    ///
    /// # Errors
    ///
    /// Fails when `url` is not an absolute path, or has a `.` or `..` segment.
    pub fn html(url: &str) -> Result<Self, RouteError> {
        Self::new(url, OutputKind::Html)
    }

    /// A route to any other file.
    ///
    /// # Errors
    ///
    /// Fails like [`Route::html`], and when `url` ends with `/`.
    pub fn file(url: &str) -> Result<Self, RouteError> {
        Self::new(url, OutputKind::File)
    }

    fn new(url: &str, kind: OutputKind) -> Result<Self, RouteError> {
        if !url.starts_with('/') {
            return Err(RouteError::NotAbsolute(url.to_owned()));
        }
        let url = squeeze_slashes(url);
        if url
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        {
            return Err(RouteError::DotSegment(url));
        }
        if kind == OutputKind::File && url.ends_with('/') {
            return Err(RouteError::FileIsDirectory(url));
        }
        Ok(Self { url, kind })
    }

    /// The URL path.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// What kind of output the route names.
    #[must_use]
    pub fn kind(&self) -> OutputKind {
        self.kind
    }

    /// The file the route is written to, relative to the output directory.
    #[must_use]
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "Jekyll compares the extension case-sensitively, and the site's files follow it"
    )]
    pub fn output_path(&self) -> String {
        let path = &self.url[1..];
        match self.kind {
            OutputKind::File => path.to_owned(),
            OutputKind::Html if path.is_empty() || path.ends_with('/') => {
                format!("{path}index.html")
            }
            OutputKind::Html if path.ends_with(".html") => path.to_owned(),
            OutputKind::Html => format!("{path}.html"),
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.url)
    }
}

/// Collapses runs of `/`, as Jekyll does when it builds a URL from a template.
fn squeeze_slashes(url: &str) -> String {
    let mut squeezed = String::with_capacity(url.len());
    for c in url.chars() {
        if !(c == '/' && squeezed.ends_with('/')) {
            squeezed.push(c);
        }
    }
    squeezed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(route: Result<Route, RouteError>) -> String {
        route.unwrap().output_path()
    }

    #[test]
    fn an_extensionless_page_is_written_as_html() {
        assert_eq!(output(Route::html("/about")), "about.html");
        assert_eq!(
            output(Route::html("/blog/2023/05/13/May-train-trip-pt1")),
            "blog/2023/05/13/May-train-trip-pt1.html"
        );
    }

    #[test]
    fn a_directory_page_is_written_as_its_index() {
        assert_eq!(output(Route::html("/")), "index.html");
        assert_eq!(output(Route::html("/blog/page2/")), "blog/page2/index.html");
    }

    #[test]
    fn a_page_named_index_keeps_index_in_its_url() {
        assert_eq!(
            output(Route::html("/dg/coding/index")),
            "dg/coding/index.html"
        );
    }

    #[test]
    fn a_page_with_an_html_url_is_written_as_named() {
        assert_eq!(output(Route::html("/blog/tags.html")), "blog/tags.html");
        assert_eq!(output(Route::html("/404.html")), "404.html");
    }

    #[test]
    fn a_file_is_written_as_named() {
        assert_eq!(output(Route::file("/feed.xml")), "feed.xml");
        assert_eq!(output(Route::file("/CNAME")), "CNAME");
    }

    #[test]
    fn repeated_slashes_collapse() {
        assert_eq!(Route::html("//about").unwrap().url(), "/about");
    }

    #[test]
    fn invalid_urls_are_rejected() {
        assert!(matches!(
            Route::html("about"),
            Err(RouteError::NotAbsolute(_))
        ));
        assert!(matches!(
            Route::html("/a/../b"),
            Err(RouteError::DotSegment(_))
        ));
        assert!(matches!(
            Route::file("/feeds/"),
            Err(RouteError::FileIsDirectory(_))
        ));
    }
}
