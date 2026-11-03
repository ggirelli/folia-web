use super::*;

/// A page shaped like the site's: a head with the fingerprinted values, a frame, and the main
/// content.
const PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <link rel="canonical" href="https://ggirelli.info/about" />
    <meta name="description" content="
Filopoetica -

About me
" />
    <title>

About me

 | Filopoetica</title>
    <meta property="og:title" content="About me" />
    <link href="/assets/css/main.css" rel="stylesheet" />
  </head>
  <body class="layout-page">
    <nav><a href="/"><img src="/favicon.ico" alt="Filopoetica"></a></nav>
    <div class="main-content container">
      <!-- Post -->
      <h1 class="posttitle">
        About me
      </h1>
      <div class="article-post"><h3 id="other-things">Other things</h3><p>Books, <a href="/dg/book-club/index">club</a>.</p></div>
    </div>
    <a class="mt-1 mb-1" href="/blog/categories#journal">journal (16)</a>
    <footer>Copyright © 2012-2026</footer>
    <script src="/assets/js/jquery.min.js"></script>
  </body>
</html>
"#;

fn changed(old: &str, new: &str) -> String {
    assert!(PAGE.contains(old), "{old}");
    PAGE.replacen(old, new, 1)
}

#[test]
fn a_page_equals_itself_whatever_its_whitespace() {
    let compact = PAGE
        .replace("\n\nAbout me\n\n | Filopoetica", "About me | Filopoetica")
        .replace(
            "<h1 class=\"posttitle\">\n        About me\n      </h1>",
            "<h1 class=\"posttitle\">About me</h1>",
        )
        .replace(
            "href=\"https://ggirelli.info/about\"",
            "href=\"https:&#x2f;&#x2f;ggirelli.info&#x2f;about\"",
        );
    assert!(check_page("about.html", PAGE, &compact));
}

#[test]
fn each_part_of_the_fingerprint_is_checked() {
    for (old, new) in [
        ("About me\n\n | Filopoetica", "About | Filopoetica"),
        (
            "https://ggirelli.info/about",
            "https://ggirelli.info/about/",
        ),
        ("Filopoetica -", "Filopoetica |"),
        ("content=\"About me\" />", "content=\"About\" />"),
        ("id=\"other-things\"", "id=\"other\""),
        ("href=\"/dg/book-club/index\"", "href=\"/dg/book-club/\""),
        ("src=\"/favicon.ico\"", "src=\"/favicon.svg\""),
        (
            "src=\"/assets/js/jquery.min.js\"",
            "src=\"/assets/js/jquery.js\"",
        ),
        ("journal (16)", "journal (15)"),
        ("2012-2026", "2012-2025"),
    ] {
        assert!(
            !check_page("about.html", PAGE, &changed(old, new)),
            "{old} -> {new} passed"
        );
    }
}

#[test]
fn the_main_content_is_compared_as_a_tree() {
    for (old, new) in [
        ("<!-- Post -->", "<!-- Article -->"),
        ("<h3 id=", "<h4 id="),
        ("class=\"posttitle\"", "class=\"posttitle p-name\""),
    ] {
        let page = changed(old, new);
        let main = |html: &str| {
            let nodes = dom::normalize_document(html);
            find(&nodes, &|node| has_class(node, "main-content")).cloned()
        };
        assert_ne!(main(PAGE), main(&page), "{old} -> {new}");
        assert!(
            !check_page("about.html", PAGE, &page),
            "{old} -> {new} passed"
        );
    }
}

/// A fixture site with a baseline of it beside: Jekyll's `about.html`, its manifest row and
/// build record, so that `run` checks the build Folia makes.
pub(crate) fn baseline(fixture: &crate::builds::tests::Fixture) -> PathBuf {
    let head = fixture.git(&["rev-parse", "HEAD"]);
    fixture.write(
        "baseline/BASELINE.json",
        &format!(
            r#"{{"created_at": "2026-10-25T14:43:51Z", "site": {{"commit": "{}", "dirty": false}}}}"#,
            head.trim()
        ),
    );
    fixture.write(
        "baseline/dump/manifest.jsonl",
        r#"{"kind": "page", "collection": null, "class": "Jekyll::Page", "source": "pages/profile.md", "url": "/about", "output": "about.html", "written": true, "content": null, "excerpt": null, "meta": null}"#,
    );
    fixture.write(
        "baseline/site/about.html",
        "<title>About | Example</title><footer>2026</footer>",
    );
    fixture.0.join("baseline")
}

#[test]
fn pages_keeps_its_build_where_it_is_told_and_never_overwrites_it() {
    let fixture = crate::builds::tests::Fixture::new("pages-run");
    let baseline = baseline(&fixture);
    let dest = fixture.0.join("pages-build");
    fixture.write("deviations.toml", "");
    let ledger = fixture.0.join("deviations.toml");
    fixture.write("expected-warnings.toml", "");
    let expected_warnings = fixture.0.join("expected-warnings.toml");
    // The check itself may pass or fail on this small site; what matters is the build's life.
    run(
        &fixture.site(),
        &baseline,
        &fixture.config_path(),
        Some(&dest),
        None,
        &ledger,
        &expected_warnings,
    )
    .unwrap();
    for file in [
        "site/about.html",
        "sha256.txt",
        "build.json",
        "build.log",
        "dom-report.json",
        "dom-report.md",
        "links.json",
        "warnings.json",
    ] {
        assert!(dest.join(file).is_file(), "{file} was not kept");
    }
    // Built as of the baseline's time, through the shared entry point.
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("build.json")).unwrap()).unwrap();
    assert_eq!(record["at"], "2026-10-25T14:43:51Z");
    assert_eq!(
        std::fs::read_to_string(dest.join("site/about.html")).unwrap(),
        "<title>About | Example</title><footer>2026</footer>"
    );
    // A second run into the same directory is refused, and the first build stays.
    let manifest = std::fs::read_to_string(dest.join("sha256.txt")).unwrap();
    let error = run(
        &fixture.site(),
        &baseline,
        &fixture.config_path(),
        Some(&dest),
        None,
        &ledger,
        &expected_warnings,
    )
    .expect_err("refused");
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(
        std::fs::read_to_string(dest.join("sha256.txt")).unwrap(),
        manifest
    );
}

#[test]
fn the_default_build_directory_is_new() {
    let dir = crate::builds::fresh_temp_dir("pages");
    assert!(dir.starts_with(std::env::temp_dir()));
    assert!(dir.symlink_metadata().is_err());
}
