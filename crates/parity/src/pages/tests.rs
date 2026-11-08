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

/// A page Jekyll writes without the site's frame: no canonical link, title or main content.
const FRAMELESS: &str = "<h2 id=\"changelog\">CHANGELOG</h2>\n<p>Fixed things.</p>\n";

#[test]
fn a_frameless_page_needs_no_main_content_when_neither_side_has_one() {
    assert!(check_page("changelog.html", FRAMELESS, FRAMELESS));
    // Its fingerprint is still checked.
    let other = FRAMELESS.replace("Fixed", "Broke");
    assert!(!check_page("changelog.html", FRAMELESS, &other));
}

#[test]
fn a_main_content_on_one_side_only_fails() {
    assert!(!check_page("changelog.html", FRAMELESS, PAGE));
    assert!(!check_page("about.html", PAGE, FRAMELESS));
    let unframed = changed("class=\"main-content container\"", "class=\"container\"");
    assert!(!check_page("about.html", PAGE, &unframed));
    assert!(!check_page("about.html", &unframed, PAGE));
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

fn report(passed: bool, exact_parity: bool) -> coverage::Report {
    coverage::Report {
        format: 2,
        owners: Vec::new(),
        counts: coverage::Counts::default(),
        problems: Vec::new(),
        rows: Vec::new(),
        folia_only: Vec::new(),
        clean_build: true,
        exact_parity,
        passed,
    }
}

fn verdicts(coverage: &coverage::Report) -> Verdicts<'_> {
    Verdicts {
        identified: true,
        ledger_ok: true,
        links_ok: true,
        warnings_ok: true,
        outputs_compared: true,
        documents_ok: true,
        coverage,
    }
}

#[test]
fn a_run_is_exact_only_when_nothing_differs_and_documented_when_everything_is_classified() {
    let exact = report(true, true);
    assert_eq!(Outcome::of(&verdicts(&exact)), Outcome::Exact);
    // Every difference classified as accepted or approved: it passes, but is not parity.
    let classified = report(true, false);
    assert_eq!(Outcome::of(&verdicts(&classified)), Outcome::Documented);
    // Raw checks that fail for a row the coverage report classified as intentional do not fail
    // the run, and are not exact either.
    let mut v = verdicts(&exact);
    v.outputs_compared = false;
    assert_eq!(Outcome::of(&v), Outcome::Documented);
    v.documents_ok = false;
    assert_eq!(Outcome::of(&v), Outcome::Documented);
    assert!(Outcome::Documented.message().contains("NOT exact parity"));
}

#[test]
fn what_coverage_does_not_classify_and_what_is_not_about_one_output_still_fails_the_run() {
    // A known, defective, evidence-less or unclassified row: coverage does not pass.
    let failing = report(false, false);
    assert_eq!(Outcome::of(&verdicts(&failing)), Outcome::Fails);
    let ok = report(true, false);
    for break_one in [
        |v: &mut Verdicts| v.identified = false,
        |v: &mut Verdicts| v.ledger_ok = false,
        |v: &mut Verdicts| v.links_ok = false,
        |v: &mut Verdicts| v.warnings_ok = false,
    ] {
        let mut v = verdicts(&ok);
        break_one(&mut v);
        assert_eq!(Outcome::of(&v), Outcome::Fails);
    }
}
