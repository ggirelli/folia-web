//! Route-level link validation (plan v12, §5.4, work item A2.8; it begins D1): every internal
//! `href` and `src` in the HTML Folia writes resolves to a planned route, a static file or a
//! redirect source. It needs only the complete route table, so a link to an output Folia plans but
//! does not build yet resolves; each B and C step brings more pages, and so more links, under it.
//!
//! - **Checked:** the `href` and `src` attributes of every element (template contents included)
//!   of every `.html` (or `.htm`, any case) file Folia writes: pages, redirect stubs and static
//!   HTML files.
//! - **Internal:** a link with no scheme and no host (relative or root-relative), or an `http` or
//!   `https` one on the site's own host (`site.url`); a protocol-relative one (`//host/…`) by its
//!   host. Any other host is external, and so is any other scheme (`mailto:`, `tel:`, `data:`,
//!   `javascript:`, …): counted, not checked.
//! - **Not checked yet:** a link that names only a fragment or a query, or nothing, points into
//!   the page it is on; whether its anchor exists is D1's. So is every link in a generated output
//!   (feeds, sitemap).
//! - **Resolved:** the link is resolved against the page's URL (RFC 3986, dot segments removed),
//!   its query and fragment dropped and its path percent-decoded; it resolves when the path is a
//!   planned route's URL, or names a file GitHub Pages would serve for it: the path itself, the
//!   path with `.html`, or its `index.html` (only `index.html` for a path ending in `/`). A page
//!   with a `<base>` element is a problem, not a guess, since it changes every relative link.
//!
//! The check compares nothing with Jekyll, so it is not one of the coverage checks (A2.1): it
//! decides the run on its own. `links.json` in the build's directory lists every link, with its
//! verdict.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use html5ever::tendril::TendrilSink as _;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use serde::Serialize;

/// One `href` or `src` of a page, and what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Link {
    /// The output file it is in, relative to the build's `site/`.
    pub page: String,
    /// The element and attribute: `a@href`, `img@src`, ….
    pub at: String,
    /// The attribute's value, as written.
    pub value: String,
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// What a link is, and for an internal one, whether it resolves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub enum Verdict {
    /// Internal, and it names a planned output: `output`, which Folia `built` or only plans.
    Resolves {
        path: String,
        output: String,
        built: bool,
    },
    /// Internal, and it names nothing Folia plans.
    Broken { path: String },
    /// On another host, or not a web link (`scheme`); not checked.
    External { scheme: String },
    /// Only a fragment, a query or nothing: a place in the page itself; not checked (D1).
    SamePage,
}

/// The links of every HTML file Folia wrote.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// How many HTML files were read.
    pub pages: usize,
    /// Every link, page by page, in document order.
    pub links: Vec<Link>,
    /// What prevented a page's links from being read or resolved.
    pub problems: Vec<String>,
}

impl Report {
    /// The internal links that resolve to nothing planned.
    pub fn broken(&self) -> impl Iterator<Item = &Link> {
        self.links
            .iter()
            .filter(|link| matches!(link.verdict, Verdict::Broken { .. }))
    }

    /// Whether every internal link resolves and every page could be read.
    pub fn passed(&self) -> bool {
        self.problems.is_empty() && self.broken().next().is_none()
    }

    /// The run's lines: one `ok:` or `FAILS:` line, then each broken link and problem.
    pub fn lines(&self) -> Vec<String> {
        let count =
            |test: fn(&Verdict) -> bool| self.links.iter().filter(|l| test(&l.verdict)).count();
        let internal = count(|v| matches!(v, Verdict::Resolves { .. } | Verdict::Broken { .. }));
        let built = count(|v| matches!(v, Verdict::Resolves { built: true, .. }));
        let planned = count(|v| matches!(v, Verdict::Resolves { built: false, .. }));
        let external = count(|v| matches!(v, Verdict::External { .. }));
        let same_page = count(|v| matches!(v, Verdict::SamePage));
        let broken: Vec<&Link> = self.broken().collect();
        let summary = format!(
            "{internal} internal href and src in {} HTML files: {built} resolve to outputs Folia builds, \
             {planned} to outputs it plans; {external} external and {same_page} within their page, not checked",
            self.pages
        );
        let mut out = Vec::new();
        if self.passed() {
            out.push(format!("ok: links: {summary} (see links.json)."));
        } else {
            out.push(format!(
                "FAILS: links: {summary}; {} resolve to no planned route, static file or redirect source, {} problems (see links.json):",
                broken.len(),
                self.problems.len()
            ));
            for link in broken {
                if let Verdict::Broken { path } = &link.verdict {
                    out.push(format!(
                        "  {}: {}=\"{}\" resolves to {path}, which is not planned",
                        link.page, link.at, link.value
                    ));
                }
            }
            out.extend(self.problems.iter().map(|p| format!("  {p}")));
        }
        out
    }
}

/// Checks the links of the HTML files among `written` (output paths, relative to `out`, the
/// build's `site/`), each page at the URL `planned` gives its output; `planned` maps every
/// planned output to its URL, and `site_url` is the site's (`site.url`).
///
/// # Errors
///
/// Fails when an HTML file cannot be read.
pub fn check(
    out: &Path,
    written: &BTreeSet<String>,
    planned: &BTreeMap<String, String>,
    site_url: &str,
) -> Result<Report, String> {
    let host = host_of(site_url);
    let urls: BTreeMap<&str, &str> = planned
        .iter()
        .map(|(output, url)| (url.as_str(), output.as_str()))
        .collect();
    let mut report = Report::default();
    let html = |page: &&String| {
        Path::new(page.as_str())
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
    };
    for page in written.iter().filter(html) {
        let text = std::fs::read(out.join(page)).map_err(|e| format!("{page}: {e}"))?;
        let Ok(text) = String::from_utf8(text) else {
            report
                .problems
                .push(format!("{page}: not UTF-8, so its links cannot be read"));
            continue;
        };
        report.pages += 1;
        let Some(page_url) = planned.get(page) else {
            report.problems.push(format!(
                "{page}: not a planned output, so it has no URL to resolve against"
            ));
            continue;
        };
        let attributes = attributes(&text);
        if attributes.iter().any(|(at, _)| at.starts_with("base@")) {
            report.problems.push(format!(
                "{page}: has a <base> element, which changes how its links resolve; not supported"
            ));
            continue;
        }
        for (at, value) in attributes {
            let verdict = match classify(&value, page_url, host.as_deref()) {
                Target::Path(path) => match resolve(&path, &urls, planned) {
                    Some(output) => Verdict::Resolves {
                        built: written.contains(&output),
                        path,
                        output,
                    },
                    None => Verdict::Broken { path },
                },
                Target::External(scheme) => Verdict::External { scheme },
                Target::SamePage => Verdict::SamePage,
            };
            report.links.push(Link {
                page: page.clone(),
                at,
                value,
                verdict,
            });
        }
    }
    Ok(report)
}

/// Writes `report` as `links.json` into `dir`; an existing file is never overwritten.
///
/// # Errors
///
/// Fails when the file exists or cannot be written.
pub fn write(report: &Report, dir: &Path) -> Result<(), String> {
    use std::io::Write as _;
    let path = dir.join("links.json");
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())? + "\n";
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| file.write_all(json.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Every `href` and `src` of a document, as (`element@attribute`, value), in document order;
/// a `<base>` element is listed as `base@…` whatever its attributes.
fn attributes(html: &str) -> Vec<(String, String)> {
    let dom = parse_document(RcDom::default(), ParseOpts::default()).one(html);
    let mut out = Vec::new();
    walk(&dom.document, &mut out);
    out
}

fn walk(node: &Handle, out: &mut Vec<(String, String)>) {
    if let NodeData::Element {
        name,
        attrs,
        template_contents,
        ..
    } = &node.data
    {
        let element = name.local.to_string();
        if element == "base" {
            out.push(("base@".to_owned(), String::new()));
        }
        for attr in attrs.borrow().iter() {
            let attribute = attr.name.local.to_string();
            if attr.name.ns.is_empty() && (attribute == "href" || attribute == "src") {
                out.push((format!("{element}@{attribute}"), attr.value.to_string()));
            }
        }
        walk_contents(template_contents, out);
    }
    for child in node.children.borrow().iter() {
        walk(child, out);
    }
}

fn walk_contents(contents: &RefCell<Option<Handle>>, out: &mut Vec<(String, String)>) {
    if let Some(contents) = contents.borrow().as_ref() {
        walk(contents, out);
    }
}

/// What a link's value points to.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// A path on the site, resolved, decoded, without query or fragment.
    Path(String),
    /// Another host, or another scheme: the scheme (`https`, `mailto`, …).
    External(String),
    /// A place in the page itself.
    SamePage,
}

/// What `value`, a link on the page at `page_url`, points to, on a site at `host`.
fn classify(value: &str, page_url: &str, host: Option<&str>) -> Target {
    let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
    let reference = value.split('#').next().unwrap_or_default();
    let reference = reference.split('?').next().unwrap_or_default();
    if let Some((scheme, rest)) = scheme_of(reference) {
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return Target::External(scheme);
        }
        return match rest.strip_prefix("//") {
            Some(authority_and_path) => on_host(authority_and_path, host, &scheme),
            // `https:foo` is relative in RFC 3986's strict reading; no browser treats it so.
            None => Target::External(scheme),
        };
    }
    if let Some(authority_and_path) = reference.strip_prefix("//") {
        return on_host(authority_and_path, host, "https");
    }
    if reference.is_empty() {
        return Target::SamePage;
    }
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let directory = &page_url[..=page_url.rfind('/').unwrap_or(0)];
        format!("{directory}{reference}")
    };
    Target::Path(percent_decode(&remove_dot_segments(&joined)))
}

/// A link on `authority_and_path` (`host[:port]/path`): a path on the site when the host is the
/// site's, else external.
fn on_host(authority_and_path: &str, host: Option<&str>, scheme: &str) -> Target {
    let (authority, path) = match authority_and_path.find('/') {
        Some(at) => authority_and_path.split_at(at),
        None => (authority_and_path, "/"),
    };
    if host.is_some_and(|host| host == normalized(authority, scheme)) {
        Target::Path(percent_decode(&remove_dot_segments(path)))
    } else {
        Target::External(scheme.to_owned())
    }
}

/// The scheme of a reference and what follows its colon, when it has one (RFC 3986, §3.1).
fn scheme_of(reference: &str) -> Option<(&str, &str)> {
    let colon = reference.find(':')?;
    let scheme = &reference[..colon];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then(|| (scheme, &reference[colon + 1..]))
}

/// The host of `url` (`https://host[:port]/…`), as [`normalized`] gives it.
fn host_of(url: &str) -> Option<String> {
    let (scheme, rest) = scheme_of(url)?;
    let authority = rest.strip_prefix("//")?.split('/').next()?;
    Some(normalized(authority, &scheme.to_ascii_lowercase()))
}

/// An authority without its user information, lowercased, and without the scheme's default port.
fn normalized(authority: &str, scheme: &str) -> String {
    let authority = authority
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let default = if scheme == "http" { ":80" } else { ":443" };
    authority
        .strip_suffix(default)
        .unwrap_or(&authority)
        .to_owned()
}

/// An absolute path with its `.` and `..` segments removed (RFC 3986, §5.2.4); `..` at the root
/// stays at the root.
fn remove_dot_segments(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    for (index, segment) in parts.iter().enumerate() {
        let last = index + 1 == parts.len();
        match *segment {
            "." => {
                if last {
                    segments.push("");
                }
            }
            ".." => {
                segments.pop();
                if last {
                    segments.push("");
                }
            }
            other => segments.push(other),
        }
    }
    format!("/{}", segments.join("/"))
}

/// `%XX` escapes decoded, byte by byte; a sequence that is not UTF-8 is kept as it was written.
fn percent_decode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let hex = |at: usize| bytes.get(at).and_then(|b| char::from(*b).to_digit(16));
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(high), Some(low)) = (hex(i + 1), hex(i + 2))
        {
            out.push(u8::try_from(high * 16 + low).expect("two hex digits make a byte"));
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| path.to_owned())
}

/// The planned output `path` names: a planned route's URL, or a file GitHub Pages serves for it.
fn resolve(
    path: &str,
    urls: &BTreeMap<&str, &str>,
    planned: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(output) = urls.get(path) {
        return Some((*output).to_owned());
    }
    let file = path.trim_start_matches('/');
    let candidates = if file.is_empty() || file.ends_with('/') {
        vec![format!("{file}index.html")]
    } else {
        vec![
            file.to_owned(),
            format!("{file}.html"),
            format!("{file}/index.html"),
        ]
    };
    candidates.into_iter().find(|c| planned.contains_key(c))
}

#[cfg(test)]
mod tests;
