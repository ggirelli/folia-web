use super::*;

const HOST: Option<&str> = Some("example.org");

fn path(value: &str, page: &str) -> Target {
    classify(value, page, HOST)
}

fn on_site(path: &str) -> Target {
    Target::Path(path.to_owned())
}

#[test]
fn relative_links_resolve_against_the_pages_url() {
    assert_eq!(
        path("/about", "/blog/2021/08/08/webmentions"),
        on_site("/about")
    );
    assert_eq!(
        path("x", "/blog/2021/08/08/webmentions"),
        on_site("/blog/2021/08/08/x")
    );
    assert_eq!(
        path("../../x", "/blog/2021/08/08/webmentions"),
        on_site("/blog/2021/x")
    );
    assert_eq!(path("./x/", "/dg/coding/"), on_site("/dg/coding/x/"));
    assert_eq!(path("rust", "/dg/coding/"), on_site("/dg/coding/rust"));
    assert_eq!(path("..", "/dg/coding/"), on_site("/dg/"));
    assert_eq!(path(".", "/dg/coding/rust"), on_site("/dg/coding/"));
    // `..` at the root stays at the root.
    assert_eq!(path("../../../x", "/a/b"), on_site("/x"));
}

#[test]
fn the_sites_own_host_is_internal_and_any_other_is_not() {
    assert_eq!(path("https://example.org/a", "/"), on_site("/a"));
    assert_eq!(path("HTTP://Example.org:80/a", "/"), on_site("/a"));
    assert_eq!(path("https://example.org:443", "/x"), on_site("/"));
    assert_eq!(path("//example.org/a", "/"), on_site("/a"));
    assert_eq!(path("https://user@example.org/a", "/"), on_site("/a"));
    for external in [
        "https://www.example.org/a",
        "https://example.org.evil/a",
        "https://example.org:8443/a",
        "//other.org/a",
        "http://other.org",
    ] {
        assert!(
            matches!(path(external, "/"), Target::External(_)),
            "{external}"
        );
    }
}

#[test]
fn other_schemes_are_not_web_links() {
    for (value, scheme) in [
        ("mailto:a@example.org", "mailto"),
        ("tel:+46", "tel"),
        ("data:image/png;base64,AA", "data"),
        ("javascript:void(0)", "javascript"),
        ("JavaScript:x", "javascript"),
    ] {
        assert_eq!(
            path(value, "/"),
            Target::External(scheme.to_owned()),
            "{value}"
        );
    }
}

#[test]
fn fragments_queries_and_nothing_stay_in_the_page() {
    for value in ["#top", "?q=1", "", "  ", "?#x"] {
        assert_eq!(path(value, "/about"), Target::SamePage, "{value:?}");
    }
    // A fragment or a query after a path is dropped from it.
    assert_eq!(path("/a?b=c#d", "/"), on_site("/a"));
    assert_eq!(path(" /a ", "/"), on_site("/a"));
}

#[test]
fn paths_are_percent_decoded_when_they_decode_to_utf8() {
    assert_eq!(path("/a%20b", "/"), on_site("/a b"));
    assert_eq!(path("/caf%C3%A9", "/"), on_site("/café"));
    assert_eq!(path("/a%2", "/"), on_site("/a%2"));
    assert_eq!(path("/a%zz", "/"), on_site("/a%zz"));
    // Not UTF-8 once decoded: kept as written, so it resolves to nothing rather than to a guess.
    assert_eq!(path("/a%FF", "/"), on_site("/a%FF"));
}

fn planned() -> BTreeMap<String, String> {
    [
        ("about.html", "/about"),
        ("index.html", "/"),
        ("blog/index.html", "/blog/"),
        ("feed.xml", "/feed.xml"),
        ("assets/a.png", "/assets/a.png"),
        ("links/index.html", "/links"),
    ]
    .into_iter()
    .map(|(output, url)| (output.to_owned(), url.to_owned()))
    .collect()
}

#[test]
fn a_path_resolves_to_a_planned_url_or_a_file_github_pages_serves() {
    let planned = planned();
    let urls = planned
        .iter()
        .map(|(o, u)| (u.as_str(), o.as_str()))
        .collect();
    let found = |p: &str| resolve(p, &urls, &planned);
    assert_eq!(found("/about").as_deref(), Some("about.html"));
    assert_eq!(found("/about.html").as_deref(), Some("about.html"));
    assert_eq!(found("/").as_deref(), Some("index.html"));
    assert_eq!(found("/blog/").as_deref(), Some("blog/index.html"));
    assert_eq!(found("/blog").as_deref(), Some("blog/index.html"));
    assert_eq!(
        found("/blog/index.html").as_deref(),
        Some("blog/index.html")
    );
    assert_eq!(found("/assets/a.png").as_deref(), Some("assets/a.png"));
    // A redirect stub's source is its route's URL.
    assert_eq!(found("/links").as_deref(), Some("links/index.html"));
    for nothing in ["/about/", "/About", "/assets/b.png", "/feed", "/blog/x"] {
        assert_eq!(found(nothing), None, "{nothing}");
    }
}

/// A build directory with `files`, each planned at the URL given.
fn site(test: &str, files: &[(&str, &[u8])]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("parity-links-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (file, bytes) in files {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    dir
}

fn written(files: &[&str]) -> BTreeSet<String> {
    files.iter().map(|f| (*f).to_owned()).collect()
}

#[test]
fn every_href_and_src_of_every_html_file_is_checked() {
    let page = br##"<html><head><link rel="stylesheet" href="assets/a.png">
        <script src="/missing.js"></script></head><body>
        <a href="/blog/">blog</a><a href="#top">top</a><a href="https://other.org/x">x</a>
        <img src="https://example.org/about"><a href="mailto:a@b">mail</a>
        <template><a href="/inside-a-template">t</a></template>
        <a data-href="/not-a-link">d</a></body></html>"##;
    let dir = site(
        "check",
        &[("about.html", page), ("feed.xml", b"<a href=\"/x\"/>")],
    );
    let report = check(
        &dir,
        &written(&["about.html", "feed.xml"]),
        &planned(),
        "https://example.org",
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        report.pages, 1,
        "feed.xml is not HTML: D1 checks generated outputs"
    );
    let seen: Vec<(&str, &str)> = report
        .links
        .iter()
        .map(|l| (l.at.as_str(), l.value.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("link@href", "assets/a.png"),
            ("script@src", "/missing.js"),
            ("a@href", "/blog/"),
            ("a@href", "#top"),
            ("a@href", "https://other.org/x"),
            ("img@src", "https://example.org/about"),
            ("a@href", "mailto:a@b"),
            ("a@href", "/inside-a-template"),
        ]
    );
    let broken: Vec<&str> = report.broken().map(|l| l.value.as_str()).collect();
    assert_eq!(broken, ["/missing.js", "/inside-a-template"]);
    assert!(!report.passed());
    assert_eq!(
        report.links[2].verdict,
        Verdict::Resolves {
            path: "/blog/".to_owned(),
            output: "blog/index.html".to_owned(),
            built: false
        }
    );
    assert_eq!(
        report.links[5].verdict,
        Verdict::Resolves {
            path: "/about".to_owned(),
            output: "about.html".to_owned(),
            built: true
        }
    );
    let lines = report.lines();
    assert!(lines[0].starts_with(
        "FAILS: links: 5 internal href and src in 1 HTML files: 1 resolve to outputs Folia builds, 2 to outputs it plans; 2 external and 1 within their page, not checked; 2 resolve to no planned route"
    ), "{lines:?}");
    assert_eq!(
        lines[1],
        "  about.html: script@src=\"/missing.js\" resolves to /missing.js, which is not planned"
    );
    assert_eq!(lines.len(), 3);
}

#[test]
fn pages_whose_links_cannot_be_resolved_are_problems() {
    let dir = site(
        "problems",
        &[
            (
                "about.html",
                b"<base href=\"/elsewhere/\"><a href=\"x\">x</a>",
            ),
            ("index.html", b"\xff\xfe<a href=\"/\">"),
            ("unplanned.html", b"<a href=\"/\">"),
        ],
    );
    let report = check(
        &dir,
        &written(&["about.html", "index.html", "unplanned.html"]),
        &planned(),
        "https://example.org",
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        report.problems,
        [
            "about.html: has a <base> element, which changes how its links resolve; not supported",
            "index.html: not UTF-8, so its links cannot be read",
            "unplanned.html: not a planned output, so it has no URL to resolve against",
        ]
    );
    assert!(report.links.is_empty() && !report.passed());
}

#[test]
fn a_page_whose_links_all_resolve_passes_and_says_so() {
    let dir = site(
        "passes",
        &[("about.html", b"<a href=\"/\">home</a><a href=\"#x\">x</a>")],
    );
    let report = check(
        &dir,
        &written(&["about.html"]),
        &planned(),
        "https://example.org/",
    )
    .unwrap();
    assert!(report.passed());
    assert_eq!(
        report.lines(),
        [
            "ok: links: 1 internal href and src in 1 HTML files: 0 resolve to outputs Folia builds, 1 to outputs it plans; 0 external and 1 within their page, not checked (see links.json)."
        ]
    );
    write(&report, &dir).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("links.json")).unwrap()).unwrap();
    assert_eq!(json["links"][0]["verdict"], "resolves");
    assert_eq!(json["links"][1]["verdict"], "same-page");
    // links.json is never overwritten.
    assert!(write(&report, &dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
