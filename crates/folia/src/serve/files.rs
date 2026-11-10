//! Which file of the built site a URL is served from, as the host serves it (and as
//! `scripts/serve.py` does): a URL without an extension from its `.html` file (`/about` from
//! `about.html`), a directory from its `index.html`. A URL that names no file inside the site,
//! including one that tries to climb out of it, finds nothing, and gets the site's 404 page.

use std::path::{Path, PathBuf};

/// The file of the site at `root` the URL `path` (without its query) is served from, if any.
///
/// The file must be inside `root` once symbolic links are resolved.
#[must_use]
pub fn resolve(root: &Path, path: &str) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    candidates(path)?
        .into_iter()
        .filter_map(|candidate| root.join(candidate).canonicalize().ok())
        .find(|file| file.is_file() && file.starts_with(&root))
}

/// The paths, relative to the site, a URL path may be served from, in order: the path itself,
/// with `.html`, and its `index.html`. `None` when the path cannot name a file of the site: it
/// is not valid percent-encoded UTF-8, or has a `..` segment or a NUL.
#[must_use]
pub fn candidates(path: &str) -> Option<Vec<String>> {
    let decoded = percent_decode(path)?;
    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            _ if segment.contains('\0') => return None,
            _ => segments.push(segment),
        }
    }
    let relative = segments.join("/");
    let index = if relative.is_empty() {
        "index.html".to_owned()
    } else {
        format!("{relative}/index.html")
    };
    if relative.is_empty() || decoded.ends_with('/') {
        return Some(vec![index]);
    }
    Some(vec![relative.clone(), format!("{relative}.html"), index])
}

/// `text` with its `%XX` escapes decoded; `None` if one is malformed or the result is not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// The media type a file is served with, by its extension.
#[must_use]
pub fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

/// Whether a file of this media type is an HTML page, into which the reload script goes.
#[must_use]
pub fn is_html(content_type: &str) -> bool {
    content_type.starts_with("text/html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_url_is_served_from_itself_its_html_file_or_its_index() {
        assert_eq!(
            candidates("/about").unwrap(),
            ["about", "about.html", "about/index.html"]
        );
        assert_eq!(
            candidates("/blog/2026/01/10/hello").unwrap(),
            [
                "blog/2026/01/10/hello",
                "blog/2026/01/10/hello.html",
                "blog/2026/01/10/hello/index.html"
            ]
        );
    }

    #[test]
    fn a_directory_url_is_served_from_its_index() {
        assert_eq!(candidates("/").unwrap(), ["index.html"]);
        assert_eq!(candidates("").unwrap(), ["index.html"]);
        assert_eq!(candidates("/blog/").unwrap(), ["blog/index.html"]);
    }

    #[test]
    fn a_percent_encoded_url_is_decoded() {
        assert_eq!(
            candidates("/blog/tag/stem%20cells.xml").unwrap()[0],
            "blog/tag/stem cells.xml"
        );
        assert_eq!(candidates("/caf%C3%A9").unwrap()[0], "café");
    }

    #[test]
    fn a_url_that_climbs_out_of_the_site_names_nothing() {
        assert_eq!(candidates("/../etc/passwd"), None);
        assert_eq!(candidates("/blog/../../secret"), None);
        assert_eq!(candidates("/%2e%2e/secret"), None);
        assert_eq!(candidates("/a%00b"), None);
    }

    #[test]
    fn a_malformed_escape_names_nothing() {
        assert_eq!(candidates("/a%2"), None);
        assert_eq!(candidates("/a%zz"), None);
        assert_eq!(candidates("/%ff"), None, "not UTF-8");
    }

    #[test]
    fn files_are_resolved_inside_the_site_only() {
        let dir = std::env::temp_dir().join(format!("folia-serve-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("site");
        std::fs::create_dir_all(root.join("blog")).unwrap();
        std::fs::write(root.join("index.html"), "home").unwrap();
        std::fs::write(root.join("about.html"), "about").unwrap();
        std::fs::write(root.join("blog/index.html"), "blog").unwrap();
        std::fs::write(dir.join("secret.html"), "secret").unwrap();
        std::os::unix::fs::symlink(dir.join("secret.html"), root.join("leak.html")).unwrap();

        let found =
            |path: &str| resolve(&root, path).map(|file| std::fs::read_to_string(file).unwrap());
        assert_eq!(found("/").as_deref(), Some("home"));
        assert_eq!(found("/about").as_deref(), Some("about"));
        assert_eq!(found("/about.html").as_deref(), Some("about"));
        assert_eq!(found("/blog").as_deref(), Some("blog"));
        assert_eq!(found("/blog/").as_deref(), Some("blog"));
        assert_eq!(found("/missing"), None);
        assert_eq!(found("/../secret.html"), None);
        assert_eq!(
            found("/leak"),
            None,
            "a link out of the site is not followed"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn media_types_follow_the_extension() {
        assert_eq!(
            content_type(Path::new("a/b.HTML")),
            "text/html; charset=utf-8"
        );
        assert_eq!(content_type(Path::new("feed.xml")), "application/xml");
        assert_eq!(
            content_type(Path::new("no-extension")),
            "application/octet-stream"
        );
        assert!(is_html(content_type(Path::new("404.html"))));
        assert!(!is_html(content_type(Path::new("style.css"))));
    }
}
