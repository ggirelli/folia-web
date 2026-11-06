use folia_core::content::DocId;

use super::*;

const RUST: &str = "_digital_garden/topic/note.md";
const BROKEN: &str = "the wiki-link [[First setup: rust]] names no note, so it is shown as broken";

fn warning(source: &str, message: &str) -> Diagnostic {
    Diagnostic::in_doc(&DocId::new(source), message)
}

fn entry(id: &str, source: &str, text: &str) -> String {
    format!(
        "[[warning]]\nid = \"{id}\"\nsource = \"{source}\"\n{text}\ndecision = \"Q3l: preserve\"\n"
    )
}

fn exact(id: &str, source: &str, message: &str) -> String {
    entry(id, source, &format!("message = {message:?}"))
}

#[test]
fn an_unexpected_warning_fails_naming_it() {
    let expected = parse(&exact("W-001", RUST, BROKEN)).unwrap();
    let report = expected.judge(&[
        warning(RUST, BROKEN),
        warning("pages/x.md", "something new"),
    ]);
    assert!(!report.passed());
    assert_eq!(
        report.lines()[1..],
        ["  unexpected warning: pages/x.md: something new"]
    );
    // Nothing expected at all: every warning is unexpected.
    let none = parse("").unwrap().judge(&[warning(RUST, BROKEN)]);
    assert!(!none.passed());
    assert!(none.lines()[1].starts_with("  unexpected warning: _digital_garden/topic/note.md:"));
}

/// An entry whose warning Folia no longer gives (fixed, or lost) fails: missing or stale alike.
#[test]
fn an_entry_that_matches_no_warning_fails() {
    let expected = parse(&exact("W-001", RUST, BROKEN)).unwrap();
    let report = expected.judge(&[]);
    assert!(!report.passed());
    assert_eq!(
        report.lines()[1..],
        [
            "  W-001: expects a warning about _digital_garden/topic/note.md that Folia did not give (stale, or the warning is missing)"
        ]
    );
    // The same text about another document is not a match.
    let elsewhere = expected.judge(&[warning("pages/x.md", BROKEN)]);
    assert_eq!(elsewhere.lines().len(), 3, "{:?}", elsewhere.lines());
}

#[test]
fn a_warning_two_entries_expect_and_an_entry_two_warnings_match_fail() {
    let both = format!(
        "{}{}",
        exact("W-001", RUST, BROKEN),
        entry(
            "W-002",
            RUST,
            "pattern = 'the wiki-link \\[\\[.*\\]\\] names no note, so it is shown as broken'\npattern_reason = 'test'"
        )
    );
    let report = parse(&both).unwrap().judge(&[warning(RUST, BROKEN)]);
    assert!(!report.passed());
    assert!(
        report
            .lines()
            .iter()
            .any(|l| l.contains("ambiguous: 2 entries expect the warning"))
    );

    let pattern = entry(
        "W-001",
        RUST,
        "pattern = 'the wiki-link \\[\\[.*\\]\\] names no note, so it is shown as broken'\npattern_reason = 'test'",
    );
    let expected = parse(&pattern).unwrap();
    assert!(expected.judge(&[warning(RUST, BROKEN)]).passed());
    let two = expected.judge(&[
        warning(RUST, BROKEN),
        warning(
            RUST,
            "the wiki-link [[Other]] names no note, so it is shown as broken",
        ),
    ]);
    assert!(!two.passed());
    assert!(
        two.lines()
            .iter()
            .any(|l| l == "  W-001: matches 2 warnings; one entry per expected warning")
    );
}

/// A pattern matches the whole text, never a part of it.
#[test]
fn a_pattern_matches_the_whole_text() {
    let expected = parse(&entry(
        "W-001",
        RUST,
        "pattern = 'names no note'\npattern_reason = 'test'",
    ))
    .unwrap();
    assert!(!expected.judge(&[warning(RUST, BROKEN)]).passed());
}

#[test]
fn a_warning_about_no_document_is_the_sites() {
    let expected = parse(&exact("W-001", SITE, "the site says so")).unwrap();
    assert!(
        expected
            .judge(&[Diagnostic::site("the site says so")])
            .passed()
    );
    assert!(
        !expected
            .judge(&[warning("x.md", "the site says so")])
            .passed()
    );
}

#[test]
fn malformed_files_and_entries_are_refused_before_anything_is_built() {
    let base = |text: &str| entry("W-001", RUST, text);
    for (text, problem) in [
        ("not toml = ".to_owned(), "expected"),
        ("[other]\nx = 1\n".to_owned(), "unknown field `other`"),
        (
            format!("{}colour = \"red\"\n", exact("W-001", RUST, BROKEN)),
            "unknown field `colour`",
        ),
        (
            "[[warning]]\nsource = \"a\"\nmessage = \"m\"\ndecision = \"d\"\n".to_owned(),
            "warning 1: `id` is missing or blank",
        ),
        (
            "[[warning]]\nid = \" \"\nsource = \"a\"\nmessage = \"m\"\ndecision = \"d\"\n"
                .to_owned(),
            "warning 1: `id` is missing or blank",
        ),
        (
            "[[warning]]\nid = \"W\"\nmessage = \"m\"\ndecision = \"d\"\n".to_owned(),
            "warning 1: `source` is missing or blank",
        ),
        (
            "[[warning]]\nid = \"W\"\nsource = \"a\"\nmessage = \"m\"\n".to_owned(),
            "warning 1: `decision` is missing or blank",
        ),
        (base(""), "warning 1: has neither `message` nor `pattern`"),
        (base("message = \"\""), "warning 1: `message` is empty"),
        (
            base("message = \"m\"\npattern = \"m\""),
            "warning 1: has both `message` and `pattern`",
        ),
        (
            base("pattern = \"m\""),
            "warning 1: a `pattern` needs a `pattern_reason`",
        ),
        (
            base("pattern = \"m\"\npattern_reason = \" \""),
            "warning 1: a `pattern` needs a `pattern_reason`",
        ),
        (
            base("message = \"m\"\npattern_reason = \"r\""),
            "`pattern_reason` is for a `pattern`",
        ),
        (
            base("pattern = \"(\"\npattern_reason = \"r\""),
            "warning 1: `pattern` is not a regular expression",
        ),
        (
            format!(
                "{}{}",
                exact("W-001", RUST, BROKEN),
                exact("W-001", "x.md", "other")
            ),
            "warning 2: the id W-001 is used twice",
        ),
    ] {
        let error = parse(&text).expect_err(&text);
        assert!(error.contains(problem), "{text:?}: {error}");
    }
}

#[test]
fn the_report_is_kept_and_never_overwritten() {
    let dir = std::env::temp_dir().join(format!("parity-warnings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let report = parse(&exact("W-001", RUST, BROKEN))
        .unwrap()
        .judge(&[warning(RUST, BROKEN)]);
    report.write(&dir).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("warnings.json")).unwrap()).unwrap();
    assert_eq!(json["warnings"][0]["entries"][0], "W-001");
    assert_eq!(json["entries"][0]["warnings"], 1);
    assert!(report.write(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
