use std::fmt::Write as _;
use std::path::PathBuf;

use super::*;
use crate::builds::Hashed;
use crate::document::{FileReport, Side, Status as FileStatus, compare_documents};

const EXPECTED_BUILD: &str = "BASELINE-RECORD-SHA256";

/// Jekyll's head: the description over several lines, as Liquid captures write it.
const JEKYLL: &str = "<!DOCTYPE html><html lang=\"en\"><head><title>A | S</title>\
                      <meta name=\"description\" content=\"\n S -\n\n A \n\">\
                      <meta property=\"og:title\" content=\"\n\nA\n\n\"></head>\
                      <body><p>Text.</p></body></html>";
/// Folia's: the same values on one line.
const FOLIA: &str = "<!DOCTYPE html><html lang=\"en\"><head><title>A | S</title>\
                     <meta name=\"description\" content=\"S - A\">\
                     <meta property=\"og:title\" content=\"A\"></head>\
                     <body><p>Text.</p></body></html>";

fn sha(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// A report of `pages` (path, Jekyll's page, Folia's page) with the given expected record.
fn report(expected_record: &str, pages: &[(&str, &str, &str)]) -> Report {
    let files = pages
        .iter()
        .map(|(path, jekyll, folia)| FileReport {
            path: (*path).to_owned(),
            status: FileStatus::Compared,
            expected_sha256: Some(sha(jekyll)),
            actual_sha256: Some(sha(folia)),
            differences: compare_documents(jekyll, folia),
        })
        .collect();
    let side = |label: &str, record: Option<&str>| Side {
        label: label.to_owned(),
        record: record.map(|sha256| Hashed {
            path: "BASELINE.json".to_owned(),
            sha256: sha256.to_owned(),
        }),
        manifest_sha256: "0".repeat(64),
    };
    Report::new(
        side("jekyll", Some(expected_record)),
        side("folia", None),
        files,
    )
}

/// A ledger file in a temporary directory, removed when dropped.
struct LedgerFile(PathBuf);

impl LedgerFile {
    fn new(test: &str, contents: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("parity-ledger-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("deviations.toml"), contents).unwrap();
        Self(dir)
    }

    fn load(&self) -> Ledger {
        load(&self.0.join("deviations.toml")).unwrap()
    }
}

impl Drop for LedgerFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An entry like D-004 for `outputs` (path, Folia page), reviewed against Jekyll's page
/// [`JEKYLL`], with the fields given by `fields`.
fn entry(fields: &str, outputs: &[(&str, &str)], matches: &[(&str, &str)]) -> String {
    let outputs: Vec<(&str, &str, &str)> = outputs
        .iter()
        .map(|&(path, folia)| (path, JEKYLL, folia))
        .collect();
    entry_reviewed(fields, &outputs, matches)
}

/// An entry for `outputs` (path, Jekyll's page, Folia's page reviewed).
fn entry_reviewed(
    fields: &str,
    outputs: &[(&str, &str, &str)],
    matches: &[(&str, &str)],
) -> String {
    let mut text = format!("[[deviation]]\nid = \"D-900\"\ncheck = \"dom\"\n{fields}\n");
    for (path, jekyll, folia) in outputs {
        let _ = write!(
            text,
            "[[deviation.outputs]]\npath = \"{path}\"\nexpected_sha256 = \"{}\"\nactual_sha256 = \"{}\"\n",
            sha(jekyll),
            sha(folia)
        );
    }
    for (path, kind) in matches {
        let _ = write!(
            text,
            "[[deviation.match]]\npath = \"{path}\"\nkind = \"{kind}\"\n"
        );
    }
    text
}

const ACCEPTED: &str = "status = \"accepted\"\naccepted = \"2026-11-01\"\naccepted_by = \"Project owner (GG)\"\ndecision = \"A2.3 step review\"\nfollow_up_status = \"none\"\nexpected_build = \"BASELINE-RECORD-SHA256\"";
const DESCRIPTION: (&str, &str) = (
    "html[1]/head[1]/meta[1]@content",
    EQUAL_AFTER_COLLAPSE_AND_TRIM,
);
const OG_TITLE: (&str, &str) = (
    "html[1]/head[1]/meta[2]@content",
    EQUAL_AFTER_COLLAPSE_AND_TRIM,
);

fn judge(test: &str, ledger: &str, report: &Report) -> Applied {
    apply(&LedgerFile::new(test, ledger).load(), report)
}

#[test]
fn one_entry_explains_the_named_difference_on_every_listed_output() {
    let report = report(
        EXPECTED_BUILD,
        &[("a.html", JEKYLL, FOLIA), ("b/index.html", JEKYLL, FOLIA)],
    );
    assert_eq!(report.totals.differences, 4);
    let applied = judge(
        "explains",
        &entry(
            ACCEPTED,
            &[("a.html", FOLIA), ("b/index.html", FOLIA)],
            &[DESCRIPTION, OG_TITLE],
        ),
        &report,
    );
    assert!(applied.passed, "{applied:?}");
    assert!(
        applied
            .verdicts
            .iter()
            .flatten()
            .all(|v| *v == Verdict::Accepted("D-900".to_owned()))
    );
    assert_eq!(
        applied.entries[0].explained,
        [
            ("a.html".to_owned(), DESCRIPTION.0.to_owned()),
            ("a.html".to_owned(), OG_TITLE.0.to_owned()),
            ("b/index.html".to_owned(), DESCRIPTION.0.to_owned()),
            ("b/index.html".to_owned(), OG_TITLE.0.to_owned()),
        ]
    );
}

#[test]
fn a_proposed_entry_leaves_its_differences_pending_and_fails() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let proposed = ACCEPTED.replace("status = \"accepted\"", "status = \"proposed\"");
    let applied = judge(
        "proposed",
        &entry(&proposed, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    assert!(
        applied
            .verdicts
            .iter()
            .flatten()
            .all(|v| *v == Verdict::Pending("D-900".to_owned()))
    );
}

#[test]
fn a_withdrawn_entry_explains_nothing_and_is_not_stale() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let withdrawn = ACCEPTED.replace("status = \"accepted\"", "status = \"withdrawn\"");
    let applied = judge(
        "withdrawn",
        &entry(&withdrawn, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    assert!(
        applied
            .verdicts
            .iter()
            .flatten()
            .all(|v| *v == Verdict::Unexplained)
    );
    assert!(!applied.entries[0].stale);
}

#[test]
fn a_stale_entry_fails() {
    // The pages are equal: the entry explains nothing.
    let report = report(EXPECTED_BUILD, &[("a.html", FOLIA, FOLIA)]);
    let applied = judge(
        "stale",
        &entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION]),
        &report,
    );
    assert!(!applied.passed);
    assert!(applied.entries[0].stale);
}

#[test]
fn a_second_difference_after_an_accepted_one_fails() {
    let folia = FOLIA.replace("<p>Text.</p>", "<p>Other.</p>");
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, &folia)]);
    let applied = judge(
        "second",
        &entry(ACCEPTED, &[("a.html", &folia)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    let verdicts: Vec<&Verdict> = applied.verdicts.iter().flatten().collect();
    assert_eq!(verdicts.len(), 3);
    assert_eq!(
        verdicts
            .iter()
            .filter(|v| matches!(v, Verdict::Unexplained))
            .count(),
        1
    );
}

#[test]
fn the_same_path_and_kind_with_values_that_differ_after_collapse_fails() {
    // Same node, same comparator kind, but the description now says something else.
    let folia = FOLIA.replace("content=\"S - A\"", "content=\"S - B\"");
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, &folia)]);
    let applied = judge(
        "values",
        &entry(ACCEPTED, &[("a.html", &folia)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    assert_eq!(applied.verdicts[0][0], Verdict::Unexplained);
    assert_eq!(
        applied.verdicts[0][1],
        Verdict::Accepted("D-900".to_owned())
    );
}

#[test]
fn a_difference_at_another_path_or_on_another_output_is_not_explained() {
    let report = report(
        EXPECTED_BUILD,
        &[("a.html", JEKYLL, FOLIA), ("other.html", JEKYLL, FOLIA)],
    );
    let applied = judge(
        "elsewhere",
        &entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION]),
        &report,
    );
    assert!(!applied.passed);
    // a.html: the description accepted, og:title not; other.html: neither.
    assert_eq!(
        applied.verdicts,
        [
            vec![Verdict::Accepted("D-900".to_owned()), Verdict::Unexplained],
            vec![Verdict::Unexplained, Verdict::Unexplained],
        ]
    );
}

#[test]
fn a_changed_expected_build_or_folia_file_makes_the_entry_stale() {
    let ledger = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    // Another baseline record, the same difference.
    let other_baseline = report(&"1".repeat(64), &[("a.html", JEKYLL, FOLIA)]);
    let applied_baseline = judge("baseline", &ledger, &other_baseline);
    assert!(!applied_baseline.passed);
    assert!(applied_baseline.entries[0].stale);
    // Another Folia file (a change outside the head), the same two head differences plus one.
    let folia = FOLIA.replace("Text.", "Text, changed.");
    let changed = report(EXPECTED_BUILD, &[("a.html", JEKYLL, &folia)]);
    let applied_folia = judge("folia", &ledger, &changed);
    assert!(!applied_folia.passed);
    assert!(applied_folia.entries[0].stale);
    assert_eq!(
        applied_folia.entries[0].outputs,
        [OutputResult {
            path: "a.html".to_owned(),
            expected: Binding::AsReviewed,
            actual: Binding::Changed,
        }]
    );
    // The two head differences the entry covers are stale for it; the third is unexplained.
    let stale = Verdict::Stale(vec!["D-900".to_owned()]);
    assert_eq!(
        applied_folia.verdicts[0],
        [stale.clone(), stale.clone(), Verdict::Unexplained]
    );
    assert_eq!(applied_baseline.verdicts[0], [stale.clone(), stale]);
    assert_eq!(applied_baseline.entries[0].expected_build, Binding::Changed);
}

#[test]
fn a_changed_baseline_file_makes_the_entry_stale_though_the_record_is_the_same() {
    // The same BASELINE.json, the same Folia file, the same two head differences; but Jekyll's
    // file is not the one reviewed (here, its og:title has other whitespace, still equal after
    // collapse and trim, so the same `match` items hold).
    let jekyll = JEKYLL.replace("content=\"\n\nA\n\n\"", "content=\" A \"");
    let report = report(EXPECTED_BUILD, &[("a.html", &jekyll, FOLIA)]);
    assert_eq!(report.totals.differences, 2);
    let applied = judge(
        "expected-file",
        &entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    let stale = Verdict::Stale(vec!["D-900".to_owned()]);
    assert_eq!(applied.verdicts[0], [stale.clone(), stale]);
    let result = &applied.entries[0];
    assert!(result.stale);
    assert_eq!(result.expected_build, Binding::AsReviewed);
    assert_eq!(
        result.outputs,
        [OutputResult {
            path: "a.html".to_owned(),
            expected: Binding::Changed,
            actual: Binding::AsReviewed,
        }]
    );
    // Reviewed against that file, it explains them again.
    let applied = judge(
        "expected-file-reviewed",
        &entry_reviewed(
            ACCEPTED,
            &[("a.html", &jekyll, FOLIA)],
            &[DESCRIPTION, OG_TITLE],
        ),
        &report,
    );
    assert!(applied.passed, "{applied:?}");
}

#[test]
fn an_output_whose_baseline_file_is_absent_is_not_explained() {
    let mut report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    report.files[0].expected_sha256 = None;
    let applied = judge(
        "absent",
        &entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(!applied.passed);
    assert_eq!(applied.entries[0].outputs[0].expected, Binding::Absent);
    assert!(
        applied
            .verdicts
            .iter()
            .flatten()
            .all(|v| matches!(v, Verdict::Stale(_)))
    );
}

#[test]
fn a_partly_stale_entry_is_reported_and_refused_by_final_acceptance_only() {
    // b.html is now equal to Jekyll's (its file changed and it has no difference left); a.html
    // still has both head differences, which the entry explains.
    let report = report(
        EXPECTED_BUILD,
        &[("a.html", JEKYLL, FOLIA), ("b.html", JEKYLL, JEKYLL)],
    );
    let applied = judge(
        "partly",
        &entry(
            ACCEPTED,
            &[("a.html", FOLIA), ("b.html", FOLIA)],
            &[DESCRIPTION, OG_TITLE],
        ),
        &report,
    );
    assert!(applied.passed, "{applied:?}");
    let result = &applied.entries[0];
    assert!(!result.stale && result.partly_stale);
    assert_eq!(result.outputs[1].actual, Binding::Changed);
    assert_eq!(result.unused.len(), 2);
    assert!(result.unused.iter().all(|u| u.output == "b.html"));
    assert_eq!(
        applied.final_acceptance,
        [
            "D-900: output b.html: the baseline's file is as reviewed, Folia's is not the one reviewed",
            "D-900: output b.html: match html[1]/head[1]/meta[1]@content attribute-value-equal-after-collapse-and-trim explains nothing",
            "D-900: output b.html: match html[1]/head[1]/meta[2]@content attribute-value-equal-after-collapse-and-trim explains nothing",
        ]
    );
}

#[test]
fn a_match_item_unused_on_an_output_as_reviewed_is_partly_stale() {
    // Folia's og:title now equals Jekyll's exactly: one difference left, one `match` item unused.
    let folia = FOLIA.replace("content=\"A\"", "content=\"\n\nA\n\n\"");
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, &folia)]);
    let applied = judge(
        "unused",
        &entry(ACCEPTED, &[("a.html", &folia)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(applied.passed, "{applied:?}");
    let result = &applied.entries[0];
    assert!(result.partly_stale);
    assert!(result.outputs[0].as_reviewed());
    assert_eq!(
        result.unused,
        [Unused {
            output: "a.html".to_owned(),
            path: OG_TITLE.0.to_owned(),
            kind: EQUAL_AFTER_COLLAPSE_AND_TRIM,
        }]
    );
    assert_eq!(applied.final_acceptance.len(), 1);
}

#[test]
fn an_entry_that_explains_everything_it_lists_is_ready_for_final_acceptance() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let ledger = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    let applied = judge("ready", &ledger, &report);
    assert!(applied.passed && !applied.entries[0].partly_stale);
    assert_eq!(applied.final_acceptance, Vec::<String>::new());
    // A proposed entry, or an open follow-up, is not.
    let proposed = ACCEPTED.replace("status = \"accepted\"", "status = \"proposed\"");
    let applied = judge(
        "ready-proposed",
        &entry(&proposed, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert_eq!(applied.final_acceptance, ["D-900: proposed, not accepted"]);
    let open = ACCEPTED.replace(
        "follow_up_status = \"none\"",
        "follow_up_status = \"open\"\nfollow_up = \"a visual check\"",
    );
    let applied = judge(
        "ready-open",
        &entry(&open, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert_eq!(applied.final_acceptance, ["D-900: its follow-up is open"]);
}

#[test]
fn an_exact_kind_matches_whatever_the_values_and_only_that_kind() {
    let folia = FOLIA.replace("content=\"S - A\"", "content=\"S - B\"");
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, &folia)]);
    let applied = judge(
        "exact",
        &entry(
            ACCEPTED,
            &[("a.html", &folia)],
            &[
                ("html[1]/head[1]/meta[1]@content", "attribute-value-differs"),
                (OG_TITLE.0, "attribute-missing"),
            ],
        ),
        &report,
    );
    assert_eq!(
        applied.verdicts[0][0],
        Verdict::Accepted("D-900".to_owned())
    );
    assert_eq!(applied.verdicts[0][1], Verdict::Unexplained);
}

#[test]
fn two_accepted_entries_explaining_one_difference_fail_as_an_overlap() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let one = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    let two = one.replace("D-900", "D-901");
    let applied = judge("overlap", &format!("{one}\n{two}"), &report);
    assert!(!applied.passed);
    assert!(matches!(applied.verdicts[0][0], Verdict::Overlap(_)));
}

#[test]
fn a_proposed_entry_duplicating_an_accepted_one_fails_as_an_overlap() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let accepted = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    let proposed = accepted
        .replace("D-900", "D-901")
        .replace("status = \"accepted\"", "status = \"proposed\"");
    let applied = judge(
        "overlap-proposed",
        &format!("{accepted}\n{proposed}"),
        &report,
    );
    assert!(!applied.passed);
    let overlap = Verdict::Overlap(vec!["D-900".to_owned(), "D-901".to_owned()]);
    assert_eq!(applied.verdicts[0], [overlap.clone(), overlap]);
    // Two proposed entries fail as an overlap too, and neither passes alone.
    let both = format!(
        "{}\n{proposed}",
        accepted.replace("status = \"accepted\"", "status = \"proposed\"")
    );
    let applied = judge("overlap-two-proposed", &both, &report);
    assert!(!applied.passed);
    assert!(matches!(applied.verdicts[0][0], Verdict::Overlap(_)));
    // A withdrawn duplicate is ignored.
    let withdrawn = proposed.replace("status = \"proposed\"", "status = \"withdrawn\"");
    let applied = judge(
        "overlap-withdrawn",
        &format!("{accepted}\n{withdrawn}"),
        &report,
    );
    assert!(applied.passed, "{applied:?}");
}

#[test]
fn an_entry_that_is_not_well_formed_fails() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    for (fields, problem) in [
        (
            ACCEPTED.replace("accepted_by = \"Project owner (GG)\"\n", ""),
            "accepted_by",
        ),
        (
            ACCEPTED.replace("decision = \"A2.3 step review\"\n", ""),
            "decision",
        ),
        (
            ACCEPTED.replace(
                "follow_up_status = \"none\"",
                "follow_up_status = \"later\"",
            ),
            "follow_up_status",
        ),
        (
            ACCEPTED.replace("follow_up_status = \"none\"", "follow_up_status = \"open\""),
            "follow_up",
        ),
        (
            ACCEPTED.replace(EXPECTED_BUILD, "baseline"),
            "expected_build",
        ),
        (
            ACCEPTED.replace("status = \"accepted\"", "status = \"approved\""),
            "status",
        ),
    ] {
        let applied = judge(
            "malformed",
            &entry(&fields, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
            &report,
        );
        assert!(!applied.passed, "{problem}");
        assert!(
            applied.problems.iter().any(|p| p.contains(problem)),
            "{problem}: {:?}",
            applied.problems
        );
    }
    for (text, problem) in [
        (
            entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE])
                .replace(&format!("expected_sha256 = \"{}\"\n", sha(JEKYLL)), ""),
            "`expected_sha256` must be a SHA-256",
        ),
        (
            entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE])
                .replace(&sha(JEKYLL), "jekyll"),
            "`expected_sha256` must be a SHA-256",
        ),
    ] {
        let applied = judge("malformed-hash", &text, &report);
        assert!(!applied.passed, "{problem}");
        assert!(
            applied.problems.iter().any(|p| p.contains(problem)),
            "{problem}: {:?}",
            applied.problems
        );
    }
    for (outputs, matches, problem) in [
        (vec![], vec![DESCRIPTION], "outputs"),
        (vec![("*.html", FOLIA)], vec![DESCRIPTION], "explicit path"),
        (vec![("a.html", FOLIA)], vec![], "`match`"),
        (
            vec![("a.html", FOLIA)],
            vec![(DESCRIPTION.0, "any")],
            "unknown kind",
        ),
    ] {
        let applied = judge("malformed-2", &entry(ACCEPTED, &outputs, &matches), &report);
        assert!(!applied.passed, "{problem}");
        assert!(
            applied.problems.iter().any(|p| p.contains(problem)),
            "{problem}: {:?}",
            applied.problems
        );
    }
}

#[test]
fn a_field_the_schema_does_not_know_is_a_problem_wherever_it_is() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let good = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    assert!(judge("known", &good, &report).passed);
    for (text, field) in [
        // An attempt to scope a `match` item to one output: refused, not widened to all.
        (
            good.replacen(
                "kind = \"attribute-value-equal-after-collapse-and-trim\"\n",
                "kind = \"attribute-value-equal-after-collapse-and-trim\"\noutput = \"a.html\"\n",
                1,
            ),
            "output",
        ),
        (
            good.replace("path = \"a.html\"\n", "path = \"a.html\"\nsha256 = \"x\"\n"),
            "sha256",
        ),
        (
            good.replace("check = \"dom\"\n", "check = \"dom\"\nmatches = []\n"),
            "matches",
        ),
    ] {
        let applied = judge("unknown-field", &text, &report);
        assert!(!applied.passed, "{field}");
        assert!(applied.entries.is_empty(), "{field}");
        assert!(
            applied.problems.iter().any(
                |p| p.starts_with("D-900: ") && p.contains(&format!("unknown field `{field}`"))
            ),
            "{field}: {:?}",
            applied.problems
        );
    }
    // A top-level table other than `[[deviation]]` is an error of the file.
    let file = LedgerFile::new(
        "unknown-table",
        &format!("[[deviations]]\nid = \"D-1\"\n{good}"),
    );
    let error = load(&file.0.join("deviations.toml")).expect_err("refused");
    assert!(error.contains("deviations"), "{error}");
}

#[test]
fn every_entry_needs_an_id_that_is_not_blank() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let good = entry(ACCEPTED, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]);
    for (label, text) in [
        ("missing", good.replace("id = \"D-900\"\n", "")),
        ("empty", good.replace("id = \"D-900\"", "id = \"\"")),
        ("blank", good.replace("id = \"D-900\"", "id = \" \\t \"")),
        ("not a string", good.replace("id = \"D-900\"", "id = 900")),
        (
            "of another check",
            format!("{good}\n[[deviation]]\nstatus = \"accepted\"\nsources = [\"a.md\"]\n"),
        ),
    ] {
        let applied = judge("id", &text, &report);
        assert!(!applied.passed, "{label}");
        assert!(
            applied
                .problems
                .iter()
                .any(|p| p.contains("`id` must be a string that is not blank")),
            "{label}: {:?}",
            applied.problems
        );
    }
}

#[test]
fn an_open_follow_up_with_its_description_does_not_fail_a_run() {
    let report = report(EXPECTED_BUILD, &[("a.html", JEKYLL, FOLIA)]);
    let open = ACCEPTED.replace(
        "follow_up_status = \"none\"",
        "follow_up_status = \"open\"\nfollow_up = \"a visual check\"",
    );
    let applied = judge(
        "open",
        &entry(&open, &[("a.html", FOLIA)], &[DESCRIPTION, OG_TITLE]),
        &report,
    );
    assert!(applied.passed, "{:?}", applied.problems);
    assert_eq!(applied.entries[0].follow_up_status, "open");
}

#[test]
fn entries_of_other_checks_are_not_read_but_their_ids_are_unique() {
    let report = report(EXPECTED_BUILD, &[("a.html", FOLIA, FOLIA)]);
    let markdown = "[[deviation]]\nid = \"D-001\"\nstatus = \"accepted\"\nsources = [\"a.md\"]\n";
    let applied = judge("others", markdown, &report);
    assert!(applied.passed && applied.entries.is_empty());
    let twice = judge("twice", &format!("{markdown}{markdown}"), &report);
    assert!(!twice.passed);
}

#[test]
fn the_repository_ledger_is_well_formed() {
    let ledger =
        load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parity/deviations.toml"))
            .unwrap();
    assert_eq!(ledger.problems, Vec::<String>::new());
    assert!(ledger.entries.iter().any(|entry| entry.id == "D-004"));
}
