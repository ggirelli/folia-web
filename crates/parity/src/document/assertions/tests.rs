use super::*;

const SITE: &str = "https://example.org";

/// An entry page with everything the page assertions read.
const PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <link rel="canonical" href="https://example.org/about" />
  <title>

About

 | Example</title>
  <meta name="description" content="
Example - About
" />
  <META NAME="ROBOTS" CONTENT="NOINDEX, NOFOLLOW">
</head>
<body>
  <div class="h-entry">
    <h1 class="p-name">About</h1>
    <span class="dt-published"><time datetime="2021-08-08T00:00:00+02:00">8 Aug</time></span>
    <a class="u-url" href="https://example.org/about"></a>
    <div class="e-content"><p>Hi.</p></div>
  </div>
</body>
</html>
"#;

/// The root page, with its identity.
const ROOT: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <link rel="canonical" href="https://example.org/" />
  <title>Example</title>
</head>
<body>
  <div class="h-card d-none">
    <a class="u-url u-uid" rel="me" href="https://example.org/">GG</a>
    <span class="p-name">Gabriele</span>
  </div>
  <a rel="me" href="https://social.example/@gg">social</a>
  <a rel="me noopener" href="https://code.example/gg">code</a>
</body>
</html>
"#;

const STUB: &str = r#"<!DOCTYPE html>
<html lang="en-US">
  <meta charset="utf-8">
  <title>Redirecting&hellip;</title>
  <link rel="canonical" href="https://example.org/about">
  <meta http-equiv="refresh" content="0; url=https://example.org/about">
  <h1>Redirecting&hellip;</h1>
</html>
"#;

const PAGE_OUTPUT: Output = Output::Page { root: false };

fn failed(outcomes: &[Outcome]) -> Vec<&'static str> {
    outcomes
        .iter()
        .filter(|o| !o.passed)
        .map(|o| o.assertion)
        .collect()
}

fn checked(outcomes: &[Outcome]) -> Vec<&'static str> {
    outcomes.iter().map(|o| o.assertion).collect()
}

fn changed(page: &str, old: &str, new: &str) -> String {
    assert!(page.contains(old), "{old}");
    page.replacen(old, new, 1)
}

#[test]
fn a_page_equal_to_jekyll_s_passes_every_assertion_that_applies() {
    let outcomes = check("about.html", PAGE_OUTPUT, PAGE, PAGE, SITE);
    assert_eq!(
        checked(&outcomes),
        [
            "robots",
            "canonical",
            "title",
            "description",
            "lang",
            "h-entry"
        ]
    );
    assert_eq!(failed(&outcomes), Vec::<&str>::new());
}

#[test]
fn robots_is_matched_case_insensitively_and_its_loss_or_change_fails() {
    let lower = changed(
        PAGE,
        "<META NAME=\"ROBOTS\" CONTENT=\"NOINDEX, NOFOLLOW\">",
        "<meta name=\"robots\" content=\"noindex, nofollow\">",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &lower, SITE)),
        Vec::<&str>::new()
    );
    let gone = changed(
        PAGE,
        "<META NAME=\"ROBOTS\" CONTENT=\"NOINDEX, NOFOLLOW\">",
        "",
    );
    let outcomes = check("a.html", PAGE_OUTPUT, PAGE, &gone, SITE);
    assert_eq!(failed(&outcomes), ["robots"]);
    let detail = &outcomes
        .iter()
        .find(|o| o.assertion == "robots")
        .unwrap()
        .detail;
    assert!(detail.contains("NOINDEX, NOFOLLOW"), "{detail}");
    let index = changed(
        PAGE,
        "CONTENT=\"NOINDEX, NOFOLLOW\"",
        "CONTENT=\"INDEX, FOLLOW\"",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &index, SITE)),
        ["robots"]
    );
}

#[test]
fn a_page_jekyll_gives_no_robots_is_not_checked_for_it() {
    let none = changed(
        PAGE,
        "<META NAME=\"ROBOTS\" CONTENT=\"NOINDEX, NOFOLLOW\">",
        "",
    );
    assert!(!checked(&check("a.html", PAGE_OUTPUT, &none, &none, SITE)).contains(&"robots"));
}

#[test]
fn the_canonical_link_must_equal_jekyll_s_and_be_absolute_on_the_site() {
    let other = changed(
        PAGE,
        "href=\"https://example.org/about\" />",
        "href=\"https://example.org/contact\" />",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &other, SITE)),
        ["canonical"]
    );
    let missing = changed(
        PAGE,
        "<link rel=\"canonical\" href=\"https://example.org/about\" />",
        "",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &missing, SITE)),
        ["canonical"]
    );
    // Both sides relative: equal, but not on the site's URL.
    let relative = changed(
        PAGE,
        "href=\"https://example.org/about\" />",
        "href=\"/about\" />",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, &relative, &relative, SITE)),
        ["canonical"]
    );
}

#[test]
fn the_title_must_equal_jekyll_s_after_collapse_and_trim() {
    let one_line = changed(
        PAGE,
        "<title>\n\nAbout\n\n | Example</title>",
        "<title>About | Example</title>",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &one_line, SITE)),
        Vec::<&str>::new()
    );
    let other = changed(PAGE, " | Example</title>", " | Other</title>");
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &other, SITE)),
        ["title"]
    );
}

#[test]
fn the_description_must_equal_jekyll_s_after_collapse_and_trim_and_be_present_on_both() {
    let one_line = changed(
        PAGE,
        "content=\"\nExample - About\n\"",
        "content=\"Example - About\"",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &one_line, SITE)),
        Vec::<&str>::new()
    );
    let other = changed(
        PAGE,
        "content=\"\nExample - About\n\"",
        "content=\"Example - Contact\"",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &other, SITE)),
        ["description"]
    );
    let gone = changed(
        PAGE,
        "<meta name=\"description\" content=\"\nExample - About\n\" />",
        "",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &gone, SITE)),
        ["description"]
    );
    // Folia adding one Jekyll's page lacks fails too.
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, &gone, PAGE, SITE)),
        ["description"]
    );
}

#[test]
fn the_language_must_equal_jekyll_s() {
    let other = changed(PAGE, "<html lang=\"en\">", "<html lang=\"it\">");
    let outcomes = check("about.html", PAGE_OUTPUT, PAGE, &other, SITE);
    assert_eq!(failed(&outcomes), ["lang"]);
    let failure = outcomes.iter().find(|o| !o.passed).unwrap();
    assert_eq!(failure.file, "about.html");
    assert!(failure.detail.contains("\"en\"") && failure.detail.contains("\"it\""));
    let gone = changed(PAGE, "<html lang=\"en\">", "<html>");
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &gone, SITE)),
        ["lang"]
    );
}

#[test]
fn the_entry_properties_jekyll_has_must_be_kept() {
    for (old, new) in [
        (
            "<h1 class=\"p-name\">About</h1>",
            "<h1 class=\"p-name\">Other</h1>",
        ),
        (
            "datetime=\"2021-08-08T00:00:00+02:00\"",
            "datetime=\"2021-08-09T00:00:00+02:00\"",
        ),
        (
            "class=\"u-url\" href=\"https://example.org/about\"",
            "class=\"u-url\" href=\"https://example.org/\"",
        ),
        ("<div class=\"e-content\">", "<div class=\"content\">"),
        ("<div class=\"h-entry\">", "<div class=\"entry\">"),
    ] {
        let other = changed(PAGE, old, new);
        assert_eq!(
            failed(&check("a.html", PAGE_OUTPUT, PAGE, &other, SITE)),
            ["h-entry"],
            "{new}"
        );
    }
}

#[test]
fn a_page_without_an_entry_is_not_checked_for_one() {
    let plain = changed(PAGE, "<div class=\"h-entry\">", "<div>");
    assert!(!checked(&check("a.html", PAGE_OUTPUT, &plain, &plain, SITE)).contains(&"h-entry"));
}

#[test]
fn the_root_page_keeps_its_rel_me_links_and_its_h_card() {
    let root = Output::Page { root: true };
    let outcomes = check("index.html", root, ROOT, ROOT, SITE);
    assert_eq!(
        checked(&outcomes),
        ["canonical", "title", "lang", "rel-me", "h-card"]
    );
    assert_eq!(failed(&outcomes), Vec::<&str>::new());
    // Order does not matter, a lost or changed link does.
    let reordered = ROOT.replace(
        "<a rel=\"me\" href=\"https://social.example/@gg\">social</a>\n  <a rel=\"me noopener\" href=\"https://code.example/gg\">code</a>",
        "<a rel=\"me noopener\" href=\"https://code.example/gg\">code</a>\n  <a rel=\"me\" href=\"https://social.example/@gg\">social</a>",
    );
    assert_eq!(
        failed(&check("index.html", root, ROOT, &reordered, SITE)),
        Vec::<&str>::new()
    );
    let lost = changed(
        ROOT,
        "<a rel=\"me\" href=\"https://social.example/@gg\">social</a>",
        "",
    );
    assert_eq!(
        failed(&check("index.html", root, ROOT, &lost, SITE)),
        ["rel-me"]
    );
    let card = changed(
        ROOT,
        "<span class=\"p-name\">Gabriele</span>",
        "<span class=\"p-name\">GG</span>",
    );
    assert_eq!(
        failed(&check("index.html", root, ROOT, &card, SITE)),
        ["h-card"]
    );
    let no_card = changed(ROOT, "class=\"h-card d-none\"", "class=\"d-none\"");
    assert_eq!(
        failed(&check("index.html", root, ROOT, &no_card, SITE)),
        ["h-card"]
    );
}

#[test]
fn rel_me_and_h_card_are_checked_on_the_root_page_only() {
    let outcomes = check("about.html", PAGE_OUTPUT, ROOT, ROOT, SITE);
    assert!(!checked(&outcomes).contains(&"rel-me"));
    assert!(!checked(&outcomes).contains(&"h-card"));
}

#[test]
fn a_stub_keeps_its_target_and_canonical() {
    let outcomes = check("blog/about.html", Output::Stub, STUB, STUB, SITE);
    assert_eq!(checked(&outcomes), ["stub"]);
    assert_eq!(failed(&outcomes), Vec::<&str>::new());
    let target = changed(
        STUB,
        "url=https://example.org/about\"",
        "url=https://example.org/\"",
    );
    assert_eq!(
        failed(&check("blog/about.html", Output::Stub, STUB, &target, SITE)),
        ["stub"]
    );
    let canonical = changed(
        STUB,
        "<link rel=\"canonical\" href=\"https://example.org/about\">",
        "",
    );
    assert_eq!(
        failed(&check(
            "blog/about.html",
            Output::Stub,
            STUB,
            &canonical,
            SITE
        )),
        ["stub"]
    );
}

#[test]
fn a_file_that_is_not_utf8_or_absent_is_a_failed_read_never_skipped() {
    let mut bad = PAGE.as_bytes().to_vec();
    bad.insert(PAGE.find("Hi.").unwrap(), 0xff);
    for (jekyll, folia, side) in [
        (
            Some(PAGE.as_bytes()),
            Some(&bad[..]),
            "Folia's file is not valid UTF-8",
        ),
        (
            Some(&bad[..]),
            Some(PAGE.as_bytes()),
            "Jekyll's file is not valid UTF-8",
        ),
        (Some(PAGE.as_bytes()), None, "Folia's file is absent"),
        (None, Some(PAGE.as_bytes()), "Jekyll's file is absent"),
    ] {
        let outcomes = check_bytes("a.html", PAGE_OUTPUT, jekyll, folia, SITE);
        assert_eq!(failed(&outcomes), ["read"], "{side}");
        assert_eq!(outcomes.len(), 1, "{side}");
        assert_eq!(outcomes[0].file, "a.html");
        assert!(
            outcomes[0].detail.contains(side)
                && outcomes[0].detail.contains("no assertion was checked"),
            "{}",
            outcomes[0].detail
        );
    }
    // A stub is read the same way.
    let outcomes = check_bytes(
        "s.html",
        Output::Stub,
        Some(STUB.as_bytes()),
        Some(&bad),
        SITE,
    );
    assert_eq!(failed(&outcomes), ["read"]);
    // Valid UTF-8 on both sides is checked as text.
    let outcomes = check_bytes(
        "a.html",
        PAGE_OUTPUT,
        Some(PAGE.as_bytes()),
        Some(PAGE.as_bytes()),
        SITE,
    );
    assert_eq!(outcomes, check("a.html", PAGE_OUTPUT, PAGE, PAGE, SITE));
    assert_eq!(failed(&outcomes), [] as [&str; 0]);
}

/// [`PAGE`] with a nested author h-card first in the entry, whose own `p-name`, `u-url` and
/// `dt-published` differ from the entry's.
fn with_author(page: &str, author_name: &str) -> String {
    changed(
        page,
        "<div class=\"h-entry\">\n",
        &format!(
            "<div class=\"h-entry\">\n    <span class=\"p-author h-card\">\
             <span class=\"p-name\">{author_name}</span>\
             <a class=\"u-url\" href=\"https://example.org/me\"></a>\
             <time class=\"dt-published\" datetime=\"1999-01-01\"></time></span>\n"
        ),
    )
}

#[test]
fn a_nested_item_s_properties_do_not_stand_for_the_entry_s() {
    let jekyll = with_author(PAGE, "Gabriele");
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, &jekyll, &jekyll, SITE)),
        [] as [&str; 0]
    );
    // The entry's own name is lost: the author's `p-name`, first in the document, must not
    // satisfy the assertion.
    let folia = changed(&jekyll, "<h1 class=\"p-name\">About</h1>", "<h1>About</h1>");
    let outcomes = check("a.html", PAGE_OUTPUT, &jekyll, &folia, SITE);
    assert_eq!(failed(&outcomes), ["h-entry"]);
    let detail = &outcomes
        .iter()
        .find(|o| o.assertion == "h-entry")
        .unwrap()
        .detail;
    assert!(
        detail.contains("p-name: Jekyll \"About\", Folia None"),
        "{detail}"
    );
    // A change to the author's name alone is not the entry's.
    let renamed = with_author(PAGE, "Someone");
    assert!(!failed(&check("a.html", PAGE_OUTPUT, &jekyll, &renamed, SITE)).contains(&"h-entry"));
    // The same for the entry's date and URL.
    for (old, new) in [
        (
            "<span class=\"dt-published\"><time datetime=\"2021-08-08T00:00:00+02:00\">8 Aug</time></span>",
            "",
        ),
        (
            "<a class=\"u-url\" href=\"https://example.org/about\"></a>",
            "",
        ),
    ] {
        let folia = changed(&jekyll, old, new);
        assert_eq!(
            failed(&check("a.html", PAGE_OUTPUT, &jekyll, &folia, SITE)),
            ["h-entry"],
            "{old}"
        );
    }
}

#[test]
fn a_property_on_a_nested_item_itself_belongs_to_the_parent_and_layout_classes_are_not_items() {
    // `h-100` (Bootstrap) is not a root class name: properties inside it are the entry's.
    let wrapped = changed(
        PAGE,
        "<h1 class=\"p-name\">About</h1>",
        "<div class=\"h-100\"><h1 class=\"p-name\">About</h1></div>",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, &wrapped, &wrapped, SITE)),
        [] as [&str; 0]
    );
    let entries_found = entries(&dom::normalize_document_with(&wrapped, &dom::DOCUMENT).1);
    assert!(
        entries_found[0]
            .iter()
            .any(|(name, value)| name == "p-name" && value == "About")
    );
    for (class, root) in [
        ("h-card", true),
        ("h-entry", true),
        ("h-x-test", true),
        ("h-100", false),
        ("h-", false),
        ("h-Card", false),
        ("p-name", false),
    ] {
        assert_eq!(is_root_class(class), root, "{class}");
    }
    // A nested item that is itself the property (`p-name h-card`) is the entry's p-name.
    let nested = changed(
        PAGE,
        "<h1 class=\"p-name\">About</h1>",
        "<h1 class=\"p-name h-card\">About</h1>",
    );
    let found = entries(&dom::normalize_document_with(&nested, &dom::DOCUMENT).1);
    assert!(
        found[0]
            .iter()
            .any(|(name, value)| name == "p-name" && value == "About")
    );
}

#[test]
fn the_root_card_s_url_and_name_are_its_own() {
    // A nested h-card inside the root card, with its own `u-url u-uid` and `p-name`, listed first.
    let nested = changed(
        ROOT,
        "<div class=\"h-card d-none\">\n",
        "<div class=\"h-card d-none\">\n    <div class=\"p-org h-card\"><a class=\"u-url u-uid\" href=\"https://org.example/\"></a><span class=\"p-name\">Org</span></div>\n",
    );
    assert_eq!(
        root_card(&dom::normalize_document_with(&nested, &dom::DOCUMENT).1),
        Some(("https://example.org/".to_owned(), "Gabriele".to_owned()))
    );
}

#[test]
fn title_and_description_collapse_html_whitespace_only() {
    // The five HTML whitespace characters (as character references, so the parser keeps them)
    // collapse: both assertions hold.
    let html_whitespace = changed(
        &changed(
            PAGE,
            "<title>\n\nAbout\n\n | Example</title>",
            "<title>&#9;About&#12;&#13;|&#10;Example&#32;</title>",
        ),
        "content=\"\nExample - About\n\"",
        "content=\"&#12;Example&#9;-&#13;About&#10;\"",
    );
    assert_eq!(
        failed(&check("a.html", PAGE_OUTPUT, PAGE, &html_whitespace, SITE)),
        Vec::<&str>::new()
    );
    // A no-break space, or another Unicode space, is a character like any other: each fails.
    for space in ['\u{a0}', '\u{2003}', '\u{3000}', '\u{b}'] {
        let title = changed(
            PAGE,
            "<title>\n\nAbout\n\n | Example</title>",
            &format!("<title>About{space}| Example</title>"),
        );
        assert_eq!(
            failed(&check("a.html", PAGE_OUTPUT, PAGE, &title, SITE)),
            ["title"],
            "{space:?}"
        );
        let description = changed(
            PAGE,
            "content=\"\nExample - About\n\"",
            &format!("content=\"Example{space}- About\""),
        );
        assert_eq!(
            failed(&check("a.html", PAGE_OUTPUT, PAGE, &description, SITE)),
            ["description"],
            "{space:?}"
        );
        // Nor is one trimmed at an end.
        let trailing = changed(
            PAGE,
            "content=\"\nExample - About\n\"",
            &format!("content=\"Example - About{space}\""),
        );
        assert_eq!(
            failed(&check("a.html", PAGE_OUTPUT, PAGE, &trailing, SITE)),
            ["description"],
            "{space:?}"
        );
    }
}

#[test]
fn the_assertions_and_the_ledger_kind_share_one_whitespace_operation() {
    // The same pairs of values, judged by the description assertion and by the comparator's
    // collapse-and-trim flag, get the same answer.
    for (jekyll, folia) in [
        ("\nExample - About\n", "Example - About"),
        ("Example&#9;-&#12;About", "Example - About"),
        ("Example\u{a0}- About", "Example - About"),
        ("Example - About\u{2003}", "Example - About"),
    ] {
        let page = |value: &str| {
            format!(
                "<!DOCTYPE html><html lang=\"en\"><head><link rel=\"canonical\" \
                 href=\"https://example.org/a\"><title>A</title><meta name=\"description\" \
                 content=\"{value}\"></head><body></body></html>"
            )
        };
        let assertion = check("a.html", PAGE_OUTPUT, &page(jekyll), &page(folia), SITE)
            .into_iter()
            .find(|o| o.assertion == "description")
            .unwrap()
            .passed;
        let differences = crate::document::compare_documents(&page(jekyll), &page(folia));
        let ledger = differences
            .iter()
            .find(|d| d.path.ends_with("@content"))
            .and_then(|d| d.equal_after_collapse_and_trim);
        assert_eq!(Some(assertion), ledger, "{jekyll:?} / {folia:?}");
    }
}
