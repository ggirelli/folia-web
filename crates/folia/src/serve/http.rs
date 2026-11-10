//! Answering requests: the files of the built site, with the reload script in its pages, and the
//! reload event stream. `GET` and `HEAD` only; everything else is refused with 405.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tiny_http::{Header, Method, Request, Response, Server};

use super::files;
use super::reload::{self, Versions};

/// How often an idle event stream is written to, so that a closed one is noticed.
const HEARTBEAT: Duration = Duration::from_secs(15);

/// The page served when a URL names no file and the site has no `404.html` (as when no build has
/// succeeded yet).
const NOT_FOUND: &str = "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Not found\
                         </title></head><body><h1>Not found</h1><p>No file of the site is served \
                         at this URL.</p></body></html>\n";

const HTML: &str = "text/html; charset=utf-8";

/// Answers each request `server` receives, for the site at `root`, in a thread of its own, as
/// event streams last. Returns when the server stops.
pub fn listen(server: &Server, root: &Path, versions: &Arc<Versions>) {
    for request in server.incoming_requests() {
        let root = root.to_owned();
        let versions = Arc::clone(versions);
        std::thread::spawn(move || respond(request, &root, &versions));
    }
}

/// Answers a request for a file of the site at `root`, or for the event stream.
fn respond(request: Request, root: &Path, versions: &Versions) {
    // Taken before the file is read: a page read after a newer build is then only reloaded once
    // more, never left stale under the newer version.
    let version = versions.current().id;
    let head = match request.method() {
        Method::Get => false,
        Method::Head => true,
        _ => {
            let _ = request.respond(with_headers(
                Response::empty(405),
                &[("Allow", "GET, HEAD")],
            ));
            return;
        }
    };
    let url = request.url().to_owned();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    if path == reload::EVENTS_PATH {
        if head {
            let response = Response::empty(200);
            let _ = request.respond(with_headers(response, &STREAM_HEADERS));
        } else {
            stream_events(request, reload::since(query), versions);
        }
        return;
    }
    let (status, body, content_type) = page(root, path);
    let body = if files::is_html(content_type) {
        match String::from_utf8(body) {
            Ok(html) => reload::inject(&html, &version).into_bytes(),
            Err(not_text) => not_text.into_bytes(),
        }
    } else {
        body
    };
    let response = Response::from_data(body).with_status_code(status);
    let _ = request.respond(with_headers(
        response,
        &[
            ("Content-Type", content_type),
            ("Cache-Control", "no-store"),
        ],
    ));
}

/// The status, body and media type of the answer to `path`: the file it names, if it can be
/// read; otherwise the site's 404 page, or Folia's.
fn page(root: &Path, path: &str) -> (u16, Vec<u8>, &'static str) {
    let read = |path: &str| {
        let file = files::resolve(root, path)?;
        Some((std::fs::read(&file).ok()?, files::content_type(&file)))
    };
    if let Some((body, content_type)) = read(path) {
        return (200, body, content_type);
    }
    let (body, content_type) =
        read("/404.html").unwrap_or_else(|| (NOT_FOUND.as_bytes().to_vec(), HTML));
    (404, body, content_type)
}

fn with_headers<R: std::io::Read>(
    mut response: Response<R>,
    headers: &[(&str, &str)],
) -> Response<R> {
    for (name, value) in headers {
        if let Ok(header) = Header::from_bytes(*name, *value) {
            response.add_header(header);
        }
    }
    response
}

const STREAM_HEADERS: [(&str, &str); 2] = [
    ("Content-Type", "text/event-stream"),
    ("Cache-Control", "no-store"),
];

/// Streams the reload events to a page that shows the version `since`, until it goes away.
fn stream_events(request: Request, since: &str, versions: &Versions) {
    let mut writer = request.into_writer();
    let mut send = |text: &str| {
        writer
            .write_all(text.as_bytes())
            .and_then(|()| writer.flush())
    };
    let mut head = String::from("HTTP/1.1 200 OK\r\n");
    for (name, value) in STREAM_HEADERS {
        head.extend([name, ": ", value, "\r\n"]);
    }
    head.push_str("Connection: close\r\n\r\n");
    if send(&head).is_err() {
        return;
    }
    let mut seen = since.to_owned();
    let mut current = versions.current();
    let mut event = reload::event(&seen, &current);
    loop {
        // A comment line keeps an idle stream open, and tells a closed one.
        if send(event.as_deref().unwrap_or(": idle\n\n")).is_err() {
            return;
        }
        seen.clone_from(&current.id);
        current = versions.wait_past(&seen, HEARTBEAT);
        event = if current.id == seen {
            None
        } else {
            reload::event(&seen, &current)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_unreadable_file_gets_the_404_page() {
        let dir = std::env::temp_dir().join(format!("folia-serve-http-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("about.html"), "about").unwrap();
        let (status, body, _) = page(&dir, "/missing");
        assert_eq!(status, 404);
        assert!(
            String::from_utf8(body)
                .unwrap()
                .contains("<h1>Not found</h1>")
        );

        std::fs::write(dir.join("404.html"), "gone").unwrap();
        assert_eq!(page(&dir, "/about"), (200, b"about".to_vec(), HTML));
        assert_eq!(page(&dir, "/missing"), (404, b"gone".to_vec(), HTML));
        let locked = dir.join("locked.html");
        std::fs::write(&locked, "locked").unwrap();
        std::fs::set_permissions(
            &locked,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o000),
        )
        .unwrap();
        if std::fs::read(&locked).is_err() {
            // (Not as root, who reads it anyway.)
            assert_eq!(page(&dir, "/locked"), (404, b"gone".to_vec(), HTML));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
