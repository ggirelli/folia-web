//! Live reload: every build `folia serve` makes is a new *version* of the site, and the pages it
//! serves listen for the next one.
//!
//! Each HTML page is served with a small script ([`inject`]), never written to disk, that names
//! the version it was served from and opens a server-sent event stream (`/__folia/events`). The
//! stream sends `reload` when a newer version was built, and `failed`, with the build's problems,
//! when the last build failed: the script then shows them over the page, which stays as it was
//! until a build succeeds.

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

/// The URL path of the event stream.
pub const EVENTS_PATH: &str = "/__folia/events";

/// The versions of the site, shared by the builder, which publishes them, and the event
/// streams, which wait for them.
#[derive(Debug)]
pub struct Versions {
    state: Mutex<Version>,
    changed: Condvar,
}

/// A version of the site: what its last build gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// Names it, unique to this server: a new server never repeats an old one's.
    pub id: String,
    /// The last build's problems, if it failed; the site then is the last good one.
    pub error: Option<String>,
}

impl Versions {
    /// The first version, before anything is built. `server` tells this server's versions from
    /// those of an earlier one, whose pages may still be open.
    #[must_use]
    pub fn new(server: &str) -> Self {
        Self {
            state: Mutex::new(Version {
                id: format!("{server}-0"),
                error: None,
            }),
            changed: Condvar::new(),
        }
    }

    /// The current version.
    #[must_use]
    pub fn current(&self) -> Version {
        self.lock().clone()
    }

    /// Publishes a new version: the outcome of a build, `error` if it failed.
    pub fn publish(&self, error: Option<String>) {
        let mut state = self.lock();
        let (server, count) = state.id.rsplit_once('-').unwrap_or((&state.id, "0"));
        let next = count.parse::<u64>().unwrap_or(0) + 1;
        state.id = format!("{server}-{next}");
        state.error = error;
        self.changed.notify_all();
    }

    /// Waits up to `timeout` for a version other than `seen`, and returns the current one.
    #[must_use]
    pub fn wait_past(&self, seen: &str, timeout: Duration) -> Version {
        let state = self.lock();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| state.id == seen)
            .unwrap_or_else(PoisonError::into_inner);
        state.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Version> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a page that shows the version `seen` is told about the `current` one, if anything:
/// `failed` with the problems while the last build failed, `reload` when there is a newer site.
#[must_use]
pub fn event(seen: &str, current: &Version) -> Option<String> {
    match (&current.error, seen == current.id) {
        (Some(error), _) => Some(sse("failed", error)),
        (None, false) => Some(sse("reload", &current.id)),
        (None, true) => None,
    }
}

/// A server-sent event named `name` carrying `data`, one `data:` line per line of it, whether
/// its lines end with `\n`, `\r\n` or `\r`, as the event stream format reads them all.
#[must_use]
pub fn sse(name: &str, data: &str) -> String {
    let mut text = format!("event: {name}\n");
    let data = data.replace("\r\n", "\n").replace('\r', "\n");
    for line in data.lines() {
        text.push_str("data: ");
        text.push_str(line);
        text.push('\n');
    }
    if data.is_empty() {
        text.push_str("data:\n");
    }
    text.push('\n');
    text
}

/// The version a page asks about, from its event stream's query (`since=<id>`).
#[must_use]
pub fn since(query: &str) -> &str {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("since="))
        .unwrap_or("")
}

/// `html` with the reload script for the version `id` before its closing `</body>`, or at its
/// end without one.
#[must_use]
pub fn inject(html: &str, id: &str) -> String {
    let script = script(id);
    let at = html
        .to_ascii_lowercase()
        .rfind("</body>")
        .unwrap_or(html.len());
    let mut out = String::with_capacity(html.len() + script.len());
    out.push_str(&html[..at]);
    out.push_str(&script);
    out.push_str(&html[at..]);
    out
}

/// The reload script for a page served from the version `id`.
fn script(id: &str) -> String {
    // Version ids are made of digits and `-` (`Versions`), so they need no escaping here.
    format!(
        "<script>/* folia serve: live reload */(() => {{\n\
         const events = new EventSource(\"{EVENTS_PATH}?since={id}\");\n\
         events.addEventListener(\"reload\", () => location.reload());\n\
         events.addEventListener(\"failed\", (event) => {{\n\
         let box = document.getElementById(\"folia-serve-error\");\n\
         if (!box) {{\n\
         box = document.createElement(\"pre\");\n\
         box.id = \"folia-serve-error\";\n\
         box.style.cssText = \"position:fixed;inset:0;z-index:2147483647;margin:0;padding:2em;\
         overflow:auto;background:rgba(20,20,20,.95);color:#f88;font:14px/1.5 monospace;\
         white-space:pre-wrap\";\n\
         document.body.appendChild(box);\n\
         }}\n\
         box.textContent = \"folia serve: the build failed, and the last good build, if any, is still served.\\n\\n\" + event.data;\n\
         }});\n\
         }})();</script>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_goes_before_the_closing_body() {
        let html = inject("<html><body><p>Hi</p></BODY></html>", "7-1");
        assert!(html.starts_with("<html><body><p>Hi</p><script>"), "{html}");
        assert!(html.ends_with("</script>\n</BODY></html>"), "{html}");
        assert!(html.contains("/__folia/events?since=7-1"), "{html}");
    }

    #[test]
    fn the_script_goes_at_the_end_of_a_page_without_a_body_tag() {
        let html = inject("<p>Hi</p>", "7-1");
        assert!(html.starts_with("<p>Hi</p><script>"), "{html}");
        assert!(html.ends_with("</script>\n"), "{html}");
    }

    #[test]
    fn the_last_closing_body_is_the_one_used() {
        let html = inject("<body><code></body></code></body>", "1-1");
        assert!(
            html.starts_with("<body><code></body></code><script>"),
            "{html}"
        );
    }

    #[test]
    fn an_event_carries_each_line_of_its_data() {
        assert_eq!(sse("reload", "1-2"), "event: reload\ndata: 1-2\n\n");
        assert_eq!(
            sse("failed", "one\ntwo"),
            "event: failed\ndata: one\ndata: two\n\n"
        );
        assert_eq!(sse("failed", ""), "event: failed\ndata:\n\n");
        assert_eq!(
            sse("failed", "a\rb\r\nc"),
            "event: failed\ndata: a\ndata: b\ndata: c\n\n",
            "a bare carriage return ends a line too"
        );
    }

    #[test]
    fn a_page_is_told_to_reload_only_for_a_newer_good_build() {
        let good = Version {
            id: "1-2".to_owned(),
            error: None,
        };
        assert_eq!(event("1-2", &good), None);
        assert_eq!(
            event("1-1", &good).as_deref(),
            Some("event: reload\ndata: 1-2\n\n")
        );
        assert_eq!(
            event("0-5", &good).as_deref(),
            Some("event: reload\ndata: 1-2\n\n"),
            "a page from an earlier server reloads"
        );
    }

    #[test]
    fn a_page_is_shown_the_problems_while_the_last_build_failed() {
        let failed = Version {
            id: "1-3".to_owned(),
            error: Some("a.md: no template".to_owned()),
        };
        let shown = Some("event: failed\ndata: a.md: no template\n\n");
        assert_eq!(event("1-2", &failed).as_deref(), shown);
        assert_eq!(event("1-3", &failed).as_deref(), shown);
    }

    #[test]
    fn each_build_publishes_a_new_version() {
        let versions = Versions::new("42");
        assert_eq!(versions.current().id, "42-0");
        versions.publish(None);
        assert_eq!(versions.current().id, "42-1");
        versions.publish(Some("broken".to_owned()));
        assert_eq!(
            versions.current(),
            Version {
                id: "42-2".to_owned(),
                error: Some("broken".to_owned())
            }
        );
    }

    #[test]
    fn waiting_returns_as_soon_as_a_version_is_published() {
        let versions = std::sync::Arc::new(Versions::new("1"));
        let publisher = std::sync::Arc::clone(&versions);
        let handle = std::thread::spawn(move || publisher.publish(None));
        let version = versions.wait_past("1-0", Duration::from_secs(30));
        handle.join().unwrap();
        assert_eq!(version.id, "1-1");
        assert_eq!(
            versions.wait_past("1-0", Duration::from_secs(30)).id,
            "1-1",
            "a version already past is returned at once"
        );
        assert_eq!(
            versions.wait_past("1-1", Duration::from_millis(1)).id,
            "1-1"
        );
    }

    #[test]
    fn the_version_asked_about_is_read_from_the_query() {
        assert_eq!(since("since=1-2"), "1-2");
        assert_eq!(since("x=y&since=3-4"), "3-4");
        assert_eq!(since(""), "");
    }
}
