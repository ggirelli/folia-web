use super::*;

/// A page shaped like the site's, with the parts the rules treat specially.
const PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="description" content="
Example -

About
" />
    <title>

About

 | Example</title>
    <style>
      body { color: red; }
    </style>
  </head>
  <body class="layout-page">
    <nav><a href="/">Home</a></nav>
    <div class="main-content">
      <!-- Post -->
      <h1>About   me</h1>
      <p>One <em>two</em> three.</p>
      <pre><code class="k">fn  main() {}</code></pre>
      <textarea>  keep
  this </textarea>
    </div>
    <script>
      var a = 1;
    </script>
  </body>
</html>
"#;

fn changed(old: &str, new: &str) -> String {
    assert!(PAGE.contains(old), "{old}");
    PAGE.replacen(old, new, 1)
}

fn kinds(differences: &[Difference]) -> Vec<(&str, Kind)> {
    differences
        .iter()
        .map(|d| (d.path.as_str(), d.kind))
        .collect()
}

#[test]
fn a_document_equals_itself() {
    assert_eq!(compare_documents(PAGE, PAGE), []);
}

#[test]
fn whitespace_between_blocks_and_head_elements_and_at_the_edges_of_the_title_is_insignificant() {
    let compact = PAGE
        .replace("\n    <meta charset", "<meta charset")
        .replace(
            "<title>\n\nAbout\n\n | Example</title>",
            "<title>About | Example</title>",
        )
        .replace("\n    <nav>", "<nav>")
        .replace("<h1>About   me</h1>", "<h1>About me</h1>");
    assert_eq!(compare_documents(PAGE, &compact), []);
}

#[test]
fn a_text_difference_is_reported_at_its_path() {
    let differences = compare_documents(PAGE, &changed("three.", "four."));
    assert_eq!(
        differences,
        [Difference {
            path: "html[1]/body[1]/div[1]/p[1]/#text[2]".to_owned(),
            kind: Kind::TextDiffers,
            expected: Some(" three.".to_owned()),
            actual: Some(" four.".to_owned()),
            equal_after_collapse_and_trim: None,
            ledger: None,
        }]
    );
}

#[test]
fn a_missing_and_an_extra_element_are_reported_without_shifting_the_rest() {
    let differences = compare_documents(
        PAGE,
        &changed("<nav><a href=\"/\">Home</a></nav>", "")
            .replace("<p>One", "<aside>new</aside><p>One"),
    );
    assert_eq!(
        kinds(&differences),
        [
            ("html[1]/body[1]/nav[1]", Kind::ElementMissing),
            ("html[1]/body[1]/div[1]/aside[1]", Kind::ElementExtra),
        ]
    );
}

#[test]
fn a_changed_tag_is_a_tag_difference() {
    let differences = compare_documents(PAGE, &changed("<em>two</em>", "<strong>two</strong>"));
    assert_eq!(
        kinds(&differences),
        [("html[1]/body[1]/div[1]/p[1]/em[1]", Kind::TagDiffers)]
    );
}

#[test]
fn attributes_missing_extra_and_changed_are_reported() {
    let other = changed("<body class=\"layout-page\">", "<body id=\"top\">")
        .replace("<html lang=\"en\">", "<html lang=\"it\">");
    assert_eq!(
        kinds(&compare_documents(PAGE, &other)),
        [
            ("html[1]@lang", Kind::AttributeValueDiffers),
            ("html[1]/body[1]@class", Kind::AttributeMissing),
            ("html[1]/body[1]@id", Kind::AttributeExtra),
        ]
    );
}

#[test]
fn class_order_does_not_matter_but_attribute_whitespace_does() {
    let reordered = changed("class=\"main-content\"", "class=\"main-content  \"");
    // Class tokens are sorted, so the extra spaces in a class do not count.
    assert_eq!(compare_documents(PAGE, &reordered), []);
    let spaced = changed("<html lang=\"en\">", "<html lang=\" en\">");
    let differences = compare_documents(PAGE, &spaced);
    assert_eq!(
        kinds(&differences),
        [("html[1]@lang", Kind::AttributeValueDiffers)]
    );
    assert_eq!(differences[0].equal_after_collapse_and_trim, Some(true));
}

#[test]
fn the_description_whitespace_difference_is_reported_and_marked_not_hidden() {
    let one_line = changed(
        "content=\"\nExample -\n\nAbout\n\"",
        "content=\"Example - About\"",
    );
    let differences = compare_documents(PAGE, &one_line);
    assert_eq!(
        kinds(&differences),
        [(
            "html[1]/head[1]/meta[2]@content",
            Kind::AttributeValueDiffers
        )]
    );
    assert_eq!(differences[0].equal_after_collapse_and_trim, Some(true));
    // A real change of the value is marked as such.
    let other = changed(
        "content=\"\nExample -\n\nAbout\n\"",
        "content=\"Example - Contact\"",
    );
    assert_eq!(
        compare_documents(PAGE, &other)[0].equal_after_collapse_and_trim,
        Some(false)
    );
}

#[test]
fn a_title_that_differs_beyond_whitespace_is_reported() {
    let differences = compare_documents(PAGE, &changed(" | Example</title>", " | Other</title>"));
    assert_eq!(
        kinds(&differences),
        [("html[1]/head[1]/title[1]/#text[1]", Kind::TextDiffers)]
    );
}

#[test]
fn a_pre_is_compared_by_its_text_only() {
    // Highlighting markup inside a `pre` does not count; its text does, whitespace and all.
    let spans = changed(
        "<code class=\"k\">fn  main() {}</code>",
        "<code><span class=\"k\">fn</span>  main() {}</code>",
    );
    assert_eq!(compare_documents(PAGE, &spans), []);
    let text = changed("fn  main() {}", "fn main() {}");
    assert_eq!(
        kinds(&compare_documents(PAGE, &text)),
        [("html[1]/body[1]/div[1]/pre[1]/#text[1]", Kind::TextDiffers)]
    );
}

#[test]
fn script_and_style_text_is_compared_collapsed() {
    let collapsed = changed("var a = 1;", "var   a = 1;").replace("color: red;", "color:  red;");
    assert_eq!(compare_documents(PAGE, &collapsed), []);
    let other = changed("var a = 1;", "var a = 2;").replace("color: red;", "color: blue;");
    assert_eq!(
        kinds(&compare_documents(PAGE, &other)),
        [
            ("html[1]/head[1]/style[1]/#text[1]", Kind::TextDiffers),
            ("html[1]/body[1]/script[1]/#text[1]", Kind::TextDiffers),
        ]
    );
}

#[test]
fn a_textarea_is_compared_exactly() {
    let differences = compare_documents(PAGE, &changed("  keep\n  this ", "keep this"));
    assert_eq!(
        kinds(&differences),
        [(
            "html[1]/body[1]/div[1]/textarea[1]/#text[1]",
            Kind::TextDiffers
        )]
    );
}

#[test]
fn a_comment_difference_is_reported() {
    assert_eq!(
        kinds(&compare_documents(
            PAGE,
            &changed("<!-- Post -->", "<!-- Content -->")
        )),
        [("html[1]/body[1]/div[1]/#comment[1]", Kind::CommentDiffers)]
    );
}

#[test]
fn a_doctype_difference_and_a_missing_doctype_are_reported() {
    let legacy = changed(
        "<!DOCTYPE html>",
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"x\">",
    );
    assert_eq!(
        kinds(&compare_documents(PAGE, &legacy)),
        [("#doctype", Kind::DoctypeDiffers)]
    );
    let differences = compare_documents(PAGE, &changed("<!DOCTYPE html>\n", ""));
    assert_eq!(kinds(&differences), [("#doctype", Kind::DoctypeDiffers)]);
    assert_eq!(differences[0].actual, None);
}

#[test]
fn every_difference_of_a_document_is_reported_in_document_order() {
    let many = changed("<html lang=\"en\">", "<html lang=\"it\">")
        .replace("<!-- Post -->", "<!-- Content -->")
        .replace("three.", "four.")
        .replace("var a = 1;", "var b = 1;");
    assert_eq!(
        kinds(&compare_documents(PAGE, &many)),
        [
            ("html[1]@lang", Kind::AttributeValueDiffers),
            ("html[1]/body[1]/div[1]/#comment[1]", Kind::CommentDiffers),
            ("html[1]/body[1]/div[1]/p[1]/#text[2]", Kind::TextDiffers),
            ("html[1]/body[1]/script[1]/#text[1]", Kind::TextDiffers),
        ]
    );
}

#[test]
fn a_removed_element_among_equal_siblings_is_found_where_it_was() {
    let list = |items: &[&str]| {
        let items: Vec<String> = items.iter().map(|i| format!("<li>{i}</li>")).collect();
        let items = items.concat();
        format!("<!DOCTYPE html><html><head></head><body><ul>{items}</ul></body></html>")
    };
    assert_eq!(
        kinds(&compare_documents(
            &list(&["a", "b", "c"]),
            &list(&["a", "c"])
        )),
        [("html[1]/body[1]/ul[1]/li[2]", Kind::ElementMissing)]
    );
}

/// Two trees in a fixture directory, removed when dropped.
struct Trees(PathBuf);

impl Trees {
    fn new(test: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("parity-document-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let trees = Self(root);
        trees.write("jekyll/site/a.html", PAGE);
        trees.write("jekyll/site/b/index.html", PAGE);
        trees.write("jekyll/site/only-jekyll.html", PAGE);
        trees.write("jekyll/site/style.css", "x");
        trees.write("jekyll/BASELINE.json", "{}");
        trees.write("folia/site/a.html", PAGE);
        trees.write("folia/site/b/index.html", &changed("three.", "four."));
        trees.write("folia/site/only-folia.html", PAGE);
        trees.write("folia/site/style.css", "y");
        trees.write("folia/build.json", "{}");
        trees.write_manifest();
        trees
    }

    /// Writes Folia's `sha256.txt`, the manifest of its `site/` as a kept build has it.
    fn write_manifest(&self) {
        let manifest = crate::builds::manifest(&self.folia()).unwrap();
        self.write("folia/sha256.txt", &crate::builds::manifest_text(&manifest));
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn jekyll(&self) -> PathBuf {
        self.0.join("jekyll/site")
    }

    fn folia(&self) -> PathBuf {
        self.0.join("folia/site")
    }

    fn report(&self, scope: &Scope) -> Report {
        Report::new(
            Side::of("jekyll", &self.jekyll()).unwrap(),
            Side::of("folia", &self.folia()).unwrap(),
            compare_trees(&self.jekyll(), &self.folia(), scope).unwrap(),
        )
    }
}

impl Drop for Trees {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn trees_are_compared_by_path_with_missing_and_extra_files_apart() {
    let trees = Trees::new("all");
    let report = trees.report(&Scope::All);
    let files: Vec<(&str, Status, usize)> = report
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.status, f.differences.len()))
        .collect();
    assert_eq!(
        files,
        [
            ("a.html", Status::Compared, 0),
            ("b/index.html", Status::Compared, 1),
            ("only-folia.html", Status::Extra, 0),
            ("only-jekyll.html", Status::Missing, 0),
        ]
    );
    // Only HTML files: `style.css` differs, but it is not a document.
    assert_eq!(report.totals.files, 4);
    assert_eq!(
        (
            report.totals.missing,
            report.totals.extra,
            report.totals.differences
        ),
        (1, 1, 1)
    );
}

#[test]
fn a_scope_limits_the_files_compared_and_a_listed_file_absent_is_missing() {
    let trees = Trees::new("scope");
    let only = ["b/index.html".to_owned(), "gone.html".to_owned()];
    let report = trees.report(&Scope::Only(&only));
    let files: Vec<(&str, Status)> = report
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.status))
        .collect();
    assert_eq!(
        files,
        [
            ("b/index.html", Status::Compared),
            ("gone.html", Status::Missing)
        ]
    );
}

#[test]
fn the_report_names_both_sides_by_record_and_manifest() {
    let trees = Trees::new("sides");
    let report = trees.report(&Scope::All);
    let jekyll = &report.expected;
    assert_eq!(jekyll.record.as_ref().unwrap().path, "BASELINE.json");
    assert_eq!(
        jekyll.record.as_ref().unwrap().sha256,
        hex(&Sha256::digest(b"{}"))
    );
    // A baseline has no manifest of its own: one is computed in the kept builds' format.
    let manifest = crate::builds::manifest(&trees.jekyll()).unwrap();
    assert_eq!(
        jekyll.manifest_sha256,
        hex(&Sha256::digest(
            crate::builds::manifest_text(&manifest).as_bytes()
        ))
    );
    assert_eq!(jekyll.manifest, None);
    let folia = &report.actual;
    assert_eq!(folia.record.as_ref().unwrap().path, "build.json");
    let written = std::fs::read(trees.0.join("folia/sha256.txt")).unwrap();
    assert_eq!(folia.manifest_sha256, hex(&Sha256::digest(&written)));
    // Each Folia page carries the hash its manifest records, here that of its bytes.
    for file in report.files.iter().filter(|f| f.actual_sha256.is_some()) {
        assert_eq!(
            file.actual_manifest_sha256, file.actual_sha256,
            "{}",
            file.path
        );
    }
}

#[test]
fn reports_are_identical_when_repeated_and_inputs_are_left_as_they_were() {
    let trees = Trees::new("repeat");
    let before = crate::builds::manifest(&trees.0).unwrap();
    let first = trees.report(&Scope::All);
    let second = trees.report(&Scope::All);
    assert_eq!(first.json(), second.json());
    assert_eq!(first.summary(SUMMARY_LIMIT), second.summary(SUMMARY_LIMIT));
    assert_eq!(crate::builds::manifest(&trees.0).unwrap(), before);
}

#[test]
fn a_report_is_written_beside_the_build_and_never_over_another() {
    let trees = Trees::new("write");
    let report = trees.report(&Scope::All);
    let dir = trees.0.join("folia");
    let (json, summary) = report.write(&dir).unwrap();
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(written["mode"], "report-only");
    assert_eq!(written["totals"]["by_kind"]["text-differs"], 1);
    assert_eq!(
        written["files"][1]["differences"][0]["path"],
        "html[1]/body[1]/div[1]/p[1]/#text[2]"
    );
    assert!(
        std::fs::read_to_string(&summary)
            .unwrap()
            .contains("text-differs")
    );
    let error = report.write(&dir).expect_err("refused");
    assert!(error.contains("never overwritten"), "{error}");
}

#[test]
fn no_difference_is_hidden_by_default() {
    // Report-only mode reports every difference: there is no allowlist to hide one. Each of
    // these differs from the page, and each is reported.
    for other in [
        changed("<meta charset=\"utf-8\" />", ""),
        changed(
            "<body class=\"layout-page\">",
            "<body class=\"layout-post\">",
        ),
        changed("About   me", "About you"),
        changed("<!DOCTYPE html>", "<!DOCTYPE svg>"),
    ] {
        assert!(!compare_documents(PAGE, &other).is_empty(), "{other}");
    }
}

fn list(items: &[&str]) -> String {
    let items: Vec<String> = items.iter().map(|i| format!("<li>{i}</li>")).collect();
    format!(
        "<!DOCTYPE html><html><head></head><body><ul>{}</ul></body></html>",
        items.concat()
    )
}

#[test]
fn an_extra_element_is_named_by_its_index_on_the_actual_side() {
    let differences = compare_documents(&list(&["a", "b"]), &list(&["a", "new", "b"]));
    assert_eq!(
        kinds(&differences),
        [("html[1]/body[1]/ul[1]/li[2]", Kind::ElementExtra)]
    );
    assert_eq!(differences[0].actual.as_deref(), Some("<li>"));
}

#[test]
fn of_equal_siblings_the_last_is_the_one_missing() {
    // Any of the three could be the one removed; ties keep the earliest pairs, so the last is.
    assert_eq!(
        kinds(&compare_documents(
            &list(&["a", "a", "a"]),
            &list(&["a", "a"])
        )),
        [("html[1]/body[1]/ul[1]/li[3]", Kind::ElementMissing)]
    );
}

#[test]
fn swapped_siblings_of_one_tag_are_two_text_differences() {
    // Matching as many siblings as possible comes first, so both pairs are kept, in order.
    assert_eq!(
        kinds(&compare_documents(&list(&["a", "b"]), &list(&["b", "a"]))),
        [
            ("html[1]/body[1]/ul[1]/li[1]/#text[1]", Kind::TextDiffers),
            ("html[1]/body[1]/ul[1]/li[2]/#text[1]", Kind::TextDiffers),
        ]
    );
}

#[test]
fn a_replaced_and_a_removed_sibling_are_reported_against_the_earliest() {
    // Two siblings become one that equals neither: the first is matched and differs, the second
    // is missing. Another reading is as good; the rule picks the earliest, deterministically.
    assert_eq!(
        kinds(&compare_documents(&list(&["a", "b"]), &list(&["c"]))),
        [
            ("html[1]/body[1]/ul[1]/li[1]/#text[1]", Kind::TextDiffers),
            ("html[1]/body[1]/ul[1]/li[2]", Kind::ElementMissing),
        ]
    );
}

#[test]
fn an_equal_sibling_is_matched_across_an_inserted_one_of_another_tag() {
    let page = |body: &str| format!("<!DOCTYPE html><html><head></head><body>{body}</body></html>");
    assert_eq!(
        kinds(&compare_documents(
            &page("<p>a</p><div>b</div>"),
            &page("<p>a</p><span>x</span><div>b</div>")
        )),
        [("html[1]/body[1]/span[1]", Kind::ElementExtra)]
    );
    // An element that changes tag in place, with nothing to match it, differs in tag.
    assert_eq!(
        kinds(&compare_documents(
            &page("<p>a</p><div>b</div>"),
            &page("<p>a</p><section>b</section>")
        )),
        [("html[1]/body[1]/div[1]", Kind::TagDiffers)]
    );
}

#[test]
fn a_text_or_comment_on_one_side_only_is_a_text_or_comment_difference() {
    let page =
        |body: &str| format!("<!DOCTYPE html><html><head></head><body><p>{body}</p></body></html>");
    let differences = compare_documents(&page("Hi <b>x</b>"), &page("<b>x</b>"));
    assert_eq!(
        kinds(&differences),
        [("html[1]/body[1]/p[1]/#text[1]", Kind::TextDiffers)]
    );
    assert_eq!(
        (
            differences[0].expected.as_deref(),
            differences[0].actual.as_deref()
        ),
        (Some("Hi "), Some(""))
    );
    let differences = compare_documents(&page("<b>x</b>"), &page("<!-- c --><b>x</b>"));
    assert_eq!(
        kinds(&differences),
        [("html[1]/body[1]/p[1]/#comment[1]", Kind::CommentDiffers)]
    );
    // The side without it is the empty string (plan v12, §5.1), never `null`.
    assert_eq!(differences[0].expected.as_deref(), Some(""));
    assert_eq!(differences[0].actual.as_deref(), Some("<!-- c -->"));
    let json = serde_json::to_value(&differences[0]).unwrap();
    assert_eq!(json["expected"], "");
    // An element on one side only still has no value on the other.
    let differences = compare_documents(&page("<b>x</b><i>y</i>"), &page("<b>x</b>"));
    assert_eq!(kinds(&differences)[0].1, Kind::ElementMissing);
    assert_eq!(differences[0].actual, None);
}

#[test]
fn indices_count_siblings_of_the_same_tag_only() {
    let page = "<!DOCTYPE html><html><head></head><body><p>a</p><div>x</div><p>b</p></body></html>";
    assert_eq!(
        kinds(&compare_documents(
            page,
            &page.replace("<p>b</p>", "<p>c</p>")
        )),
        [("html[1]/body[1]/p[2]/#text[1]", Kind::TextDiffers)]
    );
}

/// A page whose description `content` is `value` (written as is, so character references work).
fn described(value: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta name=\"description\" content=\"{value}\"></head>\
         <body></body></html>"
    )
}

#[test]
fn html_whitespace_differences_are_equal_after_collapse_and_trim() {
    // Each of the five HTML whitespace characters (§5.1 rule 3), at the ends and inside, written
    // as character references so that the parser keeps them.
    for value in [
        "&#32;S&#9;-&#10;&#10;A&#32;",
        "&#13;S - A&#12;",
        "S&#12;&#13;&#9; - &#10;A",
    ] {
        let differences = compare_documents(&described("S - A"), &described(value));
        assert_eq!(differences.len(), 1, "{value}");
        assert_eq!(differences[0].kind, Kind::AttributeValueDiffers);
        assert_eq!(
            differences[0].equal_after_collapse_and_trim,
            Some(true),
            "{value}"
        );
    }
}

#[test]
fn a_no_break_space_or_other_unicode_space_is_not_html_whitespace() {
    // NBSP, en and em spaces, the ideographic space, Ogham's space mark, the line and paragraph
    // separators, next line and the vertical tab: Unicode whitespace, but not HTML whitespace.
    for space in [
        '\u{a0}', '\u{2002}', '\u{2003}', '\u{3000}', '\u{1680}', '\u{2028}', '\u{2029}', '\u{85}',
        '\u{b}',
    ] {
        let value = format!("S{space}- A");
        let differences = compare_documents(&described("S - A"), &described(&value));
        assert_eq!(differences.len(), 1, "{space:?}");
        assert_eq!(
            differences[0].equal_after_collapse_and_trim,
            Some(false),
            "{space:?}"
        );
        // Nor does it collapse at the ends.
        let differences =
            compare_documents(&described("S - A"), &described(&format!("{space}S - A")));
        assert_eq!(
            differences[0].equal_after_collapse_and_trim,
            Some(false),
            "{space:?}"
        );
    }
}

#[test]
fn values_are_kept_in_full_in_the_json_and_shortened_in_the_summary() {
    let long = "word ".repeat(80); // 400 characters
    let differences = compare_documents(&described("short"), &described(long.trim()));
    let value = differences[0].actual.clone().unwrap();
    assert_eq!(value.chars().count(), 399);
    let file = FileReport {
        path: "a.html".to_owned(),
        status: Status::Compared,
        expected_sha256: None,
        actual_sha256: None,
        actual_manifest_sha256: None,
        differences,
    };
    let side = |label: &str| Side {
        label: label.to_owned(),
        record: None,
        manifest_sha256: "0".repeat(64),
        manifest: None,
    };
    let report = Report::new(side("jekyll"), side("folia"), vec![file]);
    let json: serde_json::Value = serde_json::from_str(&report.json()).unwrap();
    assert_eq!(json["files"][0]["differences"][0]["actual"], value.as_str());
    let summary = report.summary(SUMMARY_LIMIT);
    assert!(!summary.contains(&value), "{summary}");
    let shortened: String = value.chars().take(VALUE_LENGTH).collect();
    assert!(summary.contains(&format!("`{shortened}…`")), "{summary}");
    // The summary stays one line per difference.
    let line = summary.lines().find(|l| l.contains("@content")).unwrap();
    assert!(line.chars().count() < 2 * VALUE_LENGTH, "{line}");
}

/// An empty ledger, judging nothing but the report itself.
fn no_ledger() -> ledger::Ledger {
    ledger::Ledger {
        path: "deviations.toml".to_owned(),
        sha256: "0".repeat(64),
        entries: Vec::new(),
        problems: Vec::new(),
    }
}

#[test]
fn a_page_whose_bytes_are_not_what_folia_s_manifest_records_fails_when_judged() {
    let trees = Trees::new("manifest");
    let only = ["a.html".to_owned()];
    // As written, the manifest records a.html's bytes: the page is equal to Jekyll's, and passes.
    let report = trees
        .report(&Scope::Only(&only))
        .judge(&no_ledger(), Vec::new());
    assert_eq!(report.passed, Some(true), "{}", report.json());
    assert_eq!(report.manifest_mismatches, Vec::<String>::new());
    // The manifest records another hash for it: the same page fails, naming it.
    let manifest = trees.0.join("folia/sha256.txt");
    let text = std::fs::read_to_string(&manifest).unwrap();
    let sha = hex(&Sha256::digest(PAGE.as_bytes()));
    trees.write(
        "folia/sha256.txt",
        &text.replace(
            &format!("{sha}  ./a.html"),
            &format!("{}  ./a.html", "1".repeat(64)),
        ),
    );
    let report = trees
        .report(&Scope::Only(&only))
        .judge(&no_ledger(), Vec::new());
    assert_eq!(report.passed, Some(false));
    assert_eq!(report.manifest_mismatches.len(), 1);
    assert!(
        report.manifest_mismatches[0].starts_with("a.html: the manifest records 1111"),
        "{:?}",
        report.manifest_mismatches
    );
    assert!(
        report
            .summary(SUMMARY_LIMIT)
            .contains("FAILS: Folia's output manifest: a.html")
    );
    // The manifest does not list it: it fails too.
    trees.write(
        "folia/sha256.txt",
        &text
            .lines()
            .filter(|line| !line.ends_with("./a.html"))
            .fold(String::new(), |out, line| out + line + "\n"),
    );
    let report = trees
        .report(&Scope::Only(&only))
        .judge(&no_ledger(), Vec::new());
    assert_eq!(report.passed, Some(false));
    assert_eq!(
        report.manifest_mismatches,
        ["a.html: the manifest does not list it"]
    );
    // A file changed after the manifest was written is a mismatch as well.
    trees.write_manifest();
    trees.write("folia/site/a.html", &changed("three.", "3."));
    let report = trees
        .report(&Scope::Only(&only))
        .judge(&no_ledger(), Vec::new());
    assert_eq!(report.manifest_mismatches.len(), 1);
}

#[test]
fn a_malformed_manifest_is_refused() {
    let trees = Trees::new("bad-manifest");
    trees.write("folia/sha256.txt", "manifest\n");
    let error = Side::of("folia", &trees.folia()).expect_err("refused");
    assert!(error.contains("not `<sha256>  ./<path>`"), "{error}");
}
