//! Builds the public example site, `examples/minimal-site`, which the manual testing guide
//! (`docs/development/manual-testing.md`) builds by hand, and checks every mode, listing and
//! generated file it has is written.
//!
//! ```sh
//! cargo test -p folia --test example_site
//! ```

use std::path::{Path, PathBuf};

use folia::build::build;
use folia::config::Config;

/// The example site, from this crate's directory.
fn example_site() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/minimal-site")
}

/// A new, empty directory for the build, removed when dropped.
struct Dest(PathBuf);

impl Drop for Dest {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_example_site_builds_every_mode_with_one_expected_warning() {
    let site = example_site();
    let config = Config::load(&site.join("_folia/folia.toml")).unwrap();
    let dest = Dest(std::env::temp_dir().join(format!("folia-example-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&dest.0);

    let built = build(
        &site,
        &config,
        &dest.0,
        "2026-03-01T12:00:00Z".parse().unwrap(),
    )
    .unwrap();

    assert!(built.skipped.is_empty(), "{:?}", built.skipped);
    let warnings: Vec<String> = built.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "_garden/topic/index.md: the wiki-link [[Missing note]] names no note, so it is shown as broken"
        ]
    );
    for output in [
        "index.html",
        "about.html",
        "me.html",
        "404.html",
        "search.html",
        "blog/2026/01/10/hello-folia.html",
        "blog/index.html",
        "blog/category/journal/index.html",
        "blog/categories.html",
        "blog/tags.html",
        "micro.html",
        "micro/2026-01-12-a-note.html",
        "likes/2026-01-15-a-like.html",
        "bookmarks/2026-01-20-a-bookmark.html",
        "rsvps/2026-02-01-an-rsvp.html",
        "dg/topic/index.html",
        "dg/topic/leaf.html",
        "assets/json/notes_graph.json",
        "blog/feed.xml",
        "blog/tag/folia.xml",
        "blog/tag/two words.xml",
        "micro/feed.xml",
        "feed/micro/likes.xml",
        "dg/feed.xml",
        "feed.xml",
        "sitemap.xml",
        "redirects.json",
        "assets/js/search.js",
        "assets/css/style.css",
    ] {
        assert!(dest.0.join(output).is_file(), "{output} is not written");
    }
    let leaf = std::fs::read_to_string(dest.0.join("dg/topic/leaf.html")).unwrap();
    assert!(
        leaf.contains("Backlinks"),
        "the leaf note shows its backlink"
    );
    let search = std::fs::read_to_string(dest.0.join("assets/js/search.js")).unwrap();
    assert!(search.starts_with("var documents = [\n"), "{search}");
    // A listing is indexed by its content block (no navigation, no footer), a redirect stub by
    // nothing.
    let record = |url: &str| {
        let key = format!("\"url\": \"https://example.org{url}\"");
        let line = search.lines().find(|line| line.contains(&key));
        line.unwrap_or_else(|| panic!("no record for {url}"))
            .to_owned()
    };
    for url in ["/micro", "/blog/categories.html"] {
        let line = record(url);
        assert!(!line.contains("\"body\": \"\""), "{url} has a body: {line}");
        assert!(
            !line.contains("Categories:") && !line.contains("Home"),
            "{line}"
        );
    }
    assert!(record("/micro").contains("The template engine Folia renders pages with"));
    assert!(
        record("/micro").contains("Ada's short note") && !record("/micro").contains("&#x"),
        "the templates' escaping does not reach the index: {}",
        record("/micro")
    );
    assert!(
        record("/me").ends_with("\"body\": \"\"},") || record("/me").ends_with("\"body\": \"\"}")
    );
    assert!(!search.contains("Redirecting"), "a stub has no text");
    // A tag with a space has a feed, linked percent-encoded; a category has no feed link.
    let tags = std::fs::read_to_string(dest.0.join("blog/tags.html")).unwrap();
    assert!(tags.contains("two%20words.xml\">feed</a>"), "{tags}");
    let categories = std::fs::read_to_string(dest.0.join("blog/categories.html")).unwrap();
    assert!(!categories.contains(".xml"), "{categories}");
    // The combined feed lists the posts, notes and micro entries; its title keeps the separator.
    let combined = std::fs::read_to_string(dest.0.join("feed.xml")).unwrap();
    assert!(
        combined.contains("<title type=\"html\">Folia example | </title>"),
        "{combined}"
    );
    assert!(combined.contains("/blog/2026/01/10/hello-folia"), "a post");
    assert!(combined.contains("/dg/topic/leaf"), "a note");
    assert!(
        combined.contains("/micro/2026-01-12-a-note"),
        "a micro entry"
    );
    assert!(!combined.contains("example.org/about\""), "no page");
    assert!(!dest.0.join("README.md").exists(), "the README is excluded");
}

/// The example's configuration names Folia's own version as the feeds' generator; it follows the
/// workspace version, so a version bump cannot leave it behind.
#[test]
fn the_example_names_the_workspace_version_as_its_feeds_generator() {
    let config = std::fs::read_to_string(example_site().join("_folia/folia.toml")).unwrap();
    let wanted = format!("version = \"{}\"", env!("CARGO_PKG_VERSION"));
    assert!(
        config
            .lines()
            .any(|l| l.starts_with("generator") && l.contains(&wanted)),
        "the example's `generator` should say {wanted}"
    );
}
