use super::*;
use crate::builds::Changes;
use crate::builds::tests::Fixture;

const AT: &str = "2026-10-25T14:43:51Z";

/// Two kept builds of the fixture's site, as of `first_at` and `second_at`.
fn two_builds(fixture: &Fixture, first_at: &str, second_at: &str) -> (Kept, Kept) {
    let first = fixture
        .keep(&fixture.0.join("run/build-1"), first_at)
        .unwrap();
    let second = fixture
        .keep(&fixture.0.join("run/build-2"), second_at)
        .unwrap();
    (first, second)
}

fn manifest(kept: &Kept) -> BTreeMap<String, String> {
    builds::read_manifest(&kept.dir.join("sha256.txt")).unwrap()
}

/// A tree of files below `root`: `(path, Some(text))` is a file, `(path, None)` a directory.
fn tree(root: &Path, entries: &[(&str, Option<&str>)]) {
    for (path, text) in entries {
        let path = root.join(path);
        match text {
            Some(text) => {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text).unwrap();
            }
            None => std::fs::create_dir_all(path).unwrap(),
        }
    }
}

#[test]
fn two_builds_as_of_one_time_are_identical_and_equal_their_reference() {
    let fixture = Fixture::new("determinism-same");
    let (first, second) = two_builds(&fixture, AT, AT);
    let reference = manifest(&first);
    let path = fixture.0.join("reference.txt");
    let report = judge(&first, &second, Some((&path, &reference))).unwrap();
    assert!(report.passed, "{report:#?}");
    assert!(report.inputs.is_empty() && report.results.is_empty());
    let outputs = report.outputs.unwrap();
    assert_eq!(outputs.differences, []);
    assert_eq!(outputs.files, reference.len());
    // Directories count as entries too: `assets/` at least.
    assert!(outputs.entries > outputs.files);
    let reference = report.reference.unwrap();
    assert_eq!(reference.files, outputs.files);
    assert!(reference.builds.iter().all(|b| b.differences.is_empty()));
}

/// Builds as of two times are not the same inputs: the run says so, and the output's difference
/// (the year the page shows) is reported where it is.
#[test]
fn builds_as_of_two_times_are_not_comparable() {
    let fixture = Fixture::new("determinism-times");
    let (first, second) = two_builds(&fixture, "2031-06-01T00:00:00Z", "2032-06-01T00:00:00Z");
    let report = judge(&first, &second, None).unwrap();
    assert!(!report.passed);
    assert_eq!(
        report.inputs,
        [Field {
            path: "at".to_owned(),
            first: "2031-06-01T00:00:00Z".into(),
            second: "2032-06-01T00:00:00Z".into(),
        }]
    );
    let lines = lines(&report, None);
    assert!(lines[0].starts_with("FAILS: not comparable"), "{lines:?}");
    let differences = report.outputs.unwrap().differences;
    let page = "<title>About | Example</title><footer>2031</footer>";
    assert!(matches!(
        &differences[..],
        [Difference::Bytes { path, offset, line: 1, .. }]
            if path == "about.html" && *offset == page.find("2031").unwrap() + 3
    ));
}

/// A difference in what the build returned is nondeterminism, as is a difference in its output;
/// neither is an input.
#[test]
fn a_changed_result_or_output_is_nondeterminism() {
    let fixture = Fixture::new("determinism-result");
    let (first, mut second) = two_builds(&fixture, AT, AT);
    second.record.result.warnings.push("a warning".to_owned());
    std::fs::write(second.output.join("assets/a.txt"), "b").unwrap();
    let report = judge(&first, &second, None).unwrap();
    assert!(!report.passed);
    assert_eq!(report.inputs, []);
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].path, "result.warnings");
    let differences = &report.outputs.as_ref().unwrap().differences;
    assert_eq!(differences.len(), 1);
    assert_eq!(
        differences[0].to_string(),
        format!(
            "assets/a.txt: bytes differ from byte 0 (line 1): build-1 1 bytes, sha256 {}; build-2 1 bytes, sha256 {}",
            builds::sha256(b"a"),
            builds::sha256(b"b")
        )
    );
    let lines = lines(&report, None);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("FAILS: nondeterminism: the builds' results"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("FAILS: nondeterminism: the two output trees"))
    );
}

#[test]
fn an_unknown_identity_or_a_failed_build_fails() {
    let fixture = Fixture::new("determinism-unknown");
    let (mut first, second) = two_builds(&fixture, AT, AT);
    first.record.site.changes = Changes::Unknown {
        error: "git failed".to_owned(),
    };
    let report = judge(&first, &second, None).unwrap();
    assert!(!report.passed);
    assert_eq!(report.unknown_identities.len(), 1);
    assert!(report.unknown_identities[0].starts_with("build-1: the site's identity"));

    let fixture = Fixture::new("determinism-failed");
    fixture.write("site/_folia/templates/page.html", "{{ page.missing }}");
    let (first, second) = two_builds(&fixture, AT, AT);
    let reference = BTreeMap::new();
    let path = fixture.0.join("reference.txt");
    let report = judge(&first, &second, Some((&path, &reference))).unwrap();
    assert!(!report.passed);
    assert_eq!(report.failed.len(), 2, "{report:#?}");
    assert!(report.outputs.is_none() && report.reference.is_none());
}

#[test]
fn a_changed_byte_is_reported_with_its_offset_and_line() {
    let fixture = Fixture::new("determinism-bytes");
    let (a, b) = (fixture.0.join("a"), fixture.0.join("b"));
    tree(&a, &[("x.txt", Some("ab\ncd")), ("y.txt", Some("abc"))]);
    tree(&b, &[("x.txt", Some("ab\ncX")), ("y.txt", Some("abcd"))]);
    let differences = compare_trees(&a, &b).unwrap().differences;
    let found: Vec<_> = differences
        .iter()
        .map(|d| match d {
            Difference::Bytes {
                path,
                first,
                second,
                offset,
                line,
            } => (path.as_str(), first.size, second.size, *offset, *line),
            other => panic!("{other:?}"),
        })
        .collect();
    // A file that is a prefix of the other differs where the shorter one ends.
    assert_eq!(found, [("x.txt", 5, 5, 4, 2), ("y.txt", 3, 4, 3, 1)]);
}

/// Every entry counts: one only in either tree, an empty directory, a file in one and a
/// directory in the other, and a link's target (links are not followed).
#[test]
fn entries_are_compared_by_kind_without_following_links() {
    let fixture = Fixture::new("determinism-entries");
    let (a, b) = (fixture.0.join("a"), fixture.0.join("b"));
    tree(
        &a,
        &[
            ("same.txt", Some("same")),
            ("only-a.txt", Some("a")),
            ("empty", None),
            ("k", Some("file")),
        ],
    );
    tree(
        &b,
        &[
            ("same.txt", Some("same")),
            ("only-b.txt", Some("b")),
            ("k/inside.txt", Some("x")),
        ],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("same.txt", a.join("l")).unwrap();
        std::os::unix::fs::symlink("only-b.txt", b.join("l")).unwrap();
    }
    let outputs = compare_trees(&a, &b).unwrap();
    let mut found: Vec<String> = outputs
        .differences
        .iter()
        .map(ToString::to_string)
        .collect();
    found.sort();
    let mut expected = vec![
        "empty: only in build-1 (directory)",
        "k/inside.txt: only in build-2 (file)",
        "k: a file in build-1, a directory in build-2",
        "only-a.txt: only in build-1 (file)",
        "only-b.txt: only in build-2 (file)",
    ];
    #[cfg(unix)]
    expected.push("l: links to same.txt in build-1, to only-b.txt in build-2");
    expected.sort_unstable();
    assert_eq!(found, expected);
    assert_eq!(compare_trees(&a, &a).unwrap().differences, []);
}

#[test]
fn a_build_is_compared_with_the_reference_path_for_path() {
    let reference = BTreeMap::from([
        ("kept.txt".to_owned(), "1".repeat(64)),
        ("changed.txt".to_owned(), "2".repeat(64)),
        ("gone.txt".to_owned(), "3".repeat(64)),
    ]);
    let built = BTreeMap::from([
        ("kept.txt".to_owned(), "1".repeat(64)),
        ("changed.txt".to_owned(), "4".repeat(64)),
        ("new.txt".to_owned(), "5".repeat(64)),
    ]);
    assert_eq!(
        against_reference(&reference, &built),
        [
            ReferenceDifference::Sha256 {
                path: "changed.txt".to_owned(),
                reference: "2".repeat(64),
                build: "4".repeat(64),
            },
            ReferenceDifference::OnlyInReference {
                path: "gone.txt".to_owned()
            },
            ReferenceDifference::OnlyInBuild {
                path: "new.txt".to_owned()
            },
        ]
    );
}

/// `run` builds twice as of the baseline's time into the directory it is given, keeps both
/// builds and the reports, and never overwrites them; a reference that differs fails the run.
#[test]
fn run_keeps_its_builds_and_reports_and_never_overwrites_them() {
    let fixture = Fixture::new("determinism-run");
    let baseline = crate::pages::tests::baseline(&fixture);
    let out = fixture.0.join("determinism");
    let options = |out: &Path, reference: Option<&Path>| {
        run(&Options {
            site: &fixture.site(),
            baseline: &baseline,
            config: &fixture.config_path(),
            out: Some(out),
            expected_templates: None,
            reference,
        })
    };
    assert_eq!(options(&out, None), Ok(true));
    for file in [
        "build-1/site/about.html",
        "build-1/build.json",
        "build-2/site/about.html",
        "build-2/sha256.txt",
        "determinism-report.json",
        "determinism-report.md",
    ] {
        assert!(out.join(file).is_file(), "{file} was not kept");
    }
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("determinism-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["at"], AT);
    assert_eq!(report["passed"], true);
    let markdown = std::fs::read_to_string(out.join("determinism-report.md")).unwrap();
    assert!(markdown.contains("- ok: the two output trees are identical, byte for byte"));

    let kept = std::fs::read_to_string(out.join("determinism-report.json")).unwrap();
    let error = options(&out, None).expect_err("refused");
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(
        std::fs::read_to_string(out.join("determinism-report.json")).unwrap(),
        kept
    );

    // A reference with one hash changed fails the run, naming the file.
    let reference = fixture.0.join("reference.txt");
    let manifest = std::fs::read_to_string(out.join("build-1/sha256.txt")).unwrap();
    let hash = manifest.lines().next().unwrap().split_once("  ").unwrap().0;
    std::fs::write(&reference, manifest.replacen(hash, &"0".repeat(64), 1)).unwrap();
    let second = fixture.0.join("determinism-2");
    assert_eq!(options(&second, Some(&reference)), Ok(false));
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(second.join("determinism-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["reference"]["builds"][0]["differences"][0]["kind"],
        "sha256"
    );
    assert_eq!(report["outputs"]["differences"], Value::Array(Vec::new()));

    // A malformed reference is refused before anything is built.
    std::fs::write(&reference, "not a manifest\n").unwrap();
    let third = fixture.0.join("determinism-3");
    let error = options(&third, Some(&reference)).expect_err("refused");
    assert!(error.contains("not `<sha256>  ./<path>`"), "{error}");
    assert!(third.symlink_metadata().is_err());
}
