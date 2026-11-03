use super::*;

const OWNERS: &str = "owners = [\"Project owner (GG)\"]\n";

fn row(output: &str) -> Row {
    Row {
        kind: "page".to_owned(),
        collection: None,
        class: "Jekyll::Page".to_owned(),
        source: Some(format!("{output}.md")),
        url: format!("/{output}"),
        output: output.to_owned(),
        written: true,
        content: None,
        excerpt: None,
        meta: None,
    }
}

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

/// A coverage entry: `class` and `outputs`, with the fields given in `rest`.
fn entry(id: &str, class: &str, outputs: &[&str], rest: &str) -> String {
    let outputs: Vec<String> = outputs.iter().map(|o| format!("{o:?}")).collect();
    format!(
        "[[deviation]]\nid = \"{id}\"\ncheck = \"coverage\"\nclass = \"{class}\"\noutputs = [{}]\n{rest}\n",
        outputs.join(", ")
    )
}

const APPROVED: &str = "status = \"accepted\"\nrationale = \"why\"\naccepted = \"2026-11-03\"\n\
    accepted_by = \"Project owner (GG)\"\ndecision = \"plan v12, §10.1\"";

/// The site of the tests: `built.html` written and checked, `planned.html` planned only,
/// `waived.html` written, `removed.html` neither.
struct Site {
    rows: Vec<Row>,
    planned: BTreeSet<String>,
    written: BTreeSet<String>,
    accounted: BTreeSet<String>,
    checks: Checks,
}

impl Site {
    fn new() -> Self {
        let mut checks = Checks::default();
        checks.record("built.html", "route", true);
        checks.record("built.html", "dom", true);
        checks.record("waived.html", "route", true);
        Site {
            rows: ["built.html", "planned.html", "waived.html", "removed.html"]
                .into_iter()
                .map(row)
                .collect(),
            planned: set(&["built.html", "planned.html", "waived.html"]),
            written: set(&["built.html", "waived.html"]),
            accounted: set(&["built.html", "waived.html"]),
            checks,
        }
    }

    fn classify(&self, ledger: &str) -> Report {
        let ledger = parse(ledger).unwrap();
        classify(
            &self.rows,
            &Folia {
                planned: &self.planned,
                written: &self.written,
                accounted: &self.accounted,
                not_built: self.planned.difference(&self.written).count(),
            },
            &self.checks,
            &ledger,
        )
    }
}

fn class_of(report: &Report, output: &str) -> Option<Class> {
    report
        .rows
        .iter()
        .find(|row| row.output == output)
        .unwrap()
        .class
}

fn problems_of<'a>(report: &'a Report, output: &str) -> &'a [String] {
    &report
        .rows
        .iter()
        .find(|row| row.output == output)
        .unwrap()
        .problems
}

fn both() -> String {
    format!(
        "{OWNERS}{}{}",
        entry("D-901", "waived", &["waived.html"], APPROVED),
        entry("D-902", "removed", &["removed.html"], APPROVED)
    )
}

#[test]
fn every_row_gets_one_of_the_four_classes_and_the_approval_is_shown() {
    let report = Site::new().classify(&both());
    assert!(report.passed, "{report:#?}");
    assert_eq!(
        class_of(&report, "built.html"),
        Some(Class::BuiltAndChecked)
    );
    assert_eq!(class_of(&report, "planned.html"), Some(Class::NotBuiltYet));
    assert_eq!(class_of(&report, "waived.html"), Some(Class::Waived));
    assert_eq!(class_of(&report, "removed.html"), Some(Class::Removed));
    assert_eq!(
        report.counts,
        Counts {
            rows: 4,
            built_and_checked: 1,
            not_built_yet: 1,
            waived: 1,
            removed: 1,
            unclassified: 0,
        }
    );
    // A waived or removed row carries who approved it; one not built yet carries nothing.
    let waived = report
        .rows
        .iter()
        .find(|r| r.output == "waived.html")
        .unwrap();
    assert_eq!(
        waived.approval.as_ref().unwrap().accepted_by,
        "Project owner (GG)"
    );
    let planned = report
        .rows
        .iter()
        .find(|r| r.output == "planned.html")
        .unwrap();
    assert_eq!(planned.approval, None);
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["rows"][1]["class"], "not-built-yet");
    assert!(json["rows"][1].get("approval").is_none());
    assert_eq!(json["rows"][3]["class"], "waived");
    assert_eq!(json["rows"][3]["approval"]["entry"], "D-901");
    assert!(report.summary().starts_with(
        "ok: coverage of 4 manifest rows: 1 built and checked, 1 not built yet, 1 waived, 1 \
         removed; 0 unclassified, 0 problems"
    ));
}

#[test]
fn every_manifest_row_appears_exactly_once_in_order_and_a_repeated_row_fails() {
    let site = Site::new();
    let report = site.classify(&both());
    let outputs: Vec<&str> = report.rows.iter().map(|r| r.output.as_str()).collect();
    assert_eq!(
        outputs,
        ["built.html", "planned.html", "removed.html", "waived.html"]
    );
    let mut repeated = Site::new();
    repeated.rows.push(row("built.html"));
    let report = repeated.classify(&both());
    assert!(!report.passed);
    assert_eq!(report.rows.len(), 5);
    assert!(
        report
            .problems
            .contains(&"the manifest lists built.html twice".to_owned()),
        "{:?}",
        report.problems
    );
}

#[test]
fn an_unclassified_row_fails() {
    // Not written, not planned, and no entry removes it: never removed by default.
    let report = Site::new().classify(OWNERS);
    assert!(!report.passed);
    assert_eq!(class_of(&report, "removed.html"), None);
    assert_eq!(
        problems_of(&report, "removed.html"),
        ["Folia neither writes nor plans it, and no accepted entry removes it"]
    );
    // Written, with no entry: not waived by default either; it needs its checks.
    let mut unchecked = Site::new();
    unchecked.checks = Checks::default();
    unchecked.checks.record("waived.html", ROUTE, true);
    let report = unchecked.classify(&both());
    assert_eq!(class_of(&report, "built.html"), None);
    assert_eq!(report.counts.unclassified, 1);
    assert!(report.summary().starts_with("FAILS: "));
}

#[test]
fn a_built_row_without_its_output_or_its_checks_passing_is_not_built_and_checked() {
    // No check compared it.
    let mut site = Site::new();
    site.checks = Checks::default();
    site.checks.record("waived.html", "route", true);
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "built.html"), None);
    assert_eq!(
        problems_of(&report, "built.html"),
        ["Folia writes it, but no check compares its contents (a route check is no comparison)"]
    );
    // A check failed on it, once of twice.
    let mut site = Site::new();
    site.checks.record("built.html", "dom", false);
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "built.html"), None);
    assert_eq!(
        problems_of(&report, "built.html"),
        ["Folia writes it, but it fails its checks: dom"]
    );
    assert!(!report.passed);
    // Checked, but not written: its checks do not make it built.
    let mut site = Site::new();
    site.written.remove("built.html");
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "built.html"), Some(Class::NotBuiltYet));
}

#[test]
fn a_row_two_entries_classify_fails() {
    let ledger = format!(
        "{}{}",
        both(),
        entry("D-903", "waived", &["waived.html"], APPROVED)
    );
    let report = Site::new().classify(&ledger);
    assert!(!report.passed);
    assert_eq!(class_of(&report, "waived.html"), None);
    assert_eq!(
        problems_of(&report, "waived.html"),
        ["classified by more than one entry: D-901, D-903"]
    );
    // A proposed entry counts too: it claims the row.
    let proposed = APPROVED.replace("\"accepted\"\n", "\"proposed\"\n");
    let ledger = format!(
        "{}{}",
        both(),
        entry("D-903", "removed", &["waived.html"], &proposed)
    );
    assert_eq!(
        class_of(&Site::new().classify(&ledger), "waived.html"),
        None
    );
}

#[test]
fn an_unknown_classification_fails() {
    for class in ["skipped", "not-built-yet", "built-and-checked", ""] {
        let ledger = format!(
            "{OWNERS}{}{}",
            entry("D-901", class, &["waived.html"], APPROVED),
            entry("D-902", "removed", &["removed.html"], APPROVED)
        );
        let report = Site::new().classify(&ledger);
        assert!(!report.passed, "{class}");
        assert_eq!(class_of(&report, "waived.html"), None, "{class}");
        assert!(
            report.problems.iter().any(|p| p.starts_with(&format!(
                "D-901: class {class:?} is not a coverage classification"
            ))),
            "{:?}",
            report.problems
        );
    }
}

/// `approval` replaces the approved fields; each must leave `class` unapproved.
fn unapproved() -> Vec<(&'static str, String)> {
    vec![
        (
            "not an owner",
            APPROVED.replace("\"Project owner (GG)\"", "\"Claude (implementer)\""),
        ),
        (
            "no accepted_by",
            APPROVED.replace("accepted_by = \"Project owner (GG)\"\n", ""),
        ),
        (
            "no decision",
            APPROVED.replace("decision = \"plan v12, §10.1\"", ""),
        ),
        (
            "a blank decision",
            APPROVED.replace("\"plan v12, §10.1\"", "\" \""),
        ),
        (
            "no date",
            APPROVED.replace("accepted = \"2026-11-03\"\n", ""),
        ),
        ("not a date", APPROVED.replace("2026-11-03", "yesterday")),
        (
            "no rationale",
            APPROVED.replace("rationale = \"why\"\n", ""),
        ),
        (
            "proposed",
            APPROVED.replace("\"accepted\"\n", "\"proposed\"\n"),
        ),
    ]
}

#[test]
fn a_waiver_without_an_owners_approval_fails() {
    for (case, fields) in unapproved() {
        let ledger = format!(
            "{OWNERS}{}{}",
            entry("D-901", "waived", &["waived.html"], &fields),
            entry("D-902", "removed", &["removed.html"], APPROVED)
        );
        let report = Site::new().classify(&ledger);
        assert!(!report.passed, "{case}");
        assert_eq!(class_of(&report, "waived.html"), None, "{case}");
        assert_eq!(report.counts.waived, 0, "{case}");
        assert!(
            !problems_of(&report, "waived.html").is_empty(),
            "{case}: {report:?}"
        );
    }
    // A ledger that names no owner approves nothing.
    let report = Site::new().classify(&both().replace(OWNERS, ""));
    assert!(!report.passed);
    assert_eq!(report.counts.waived + report.counts.removed, 0);
    assert!(
        report
            .problems
            .contains(&"D-901: is accepted, but the ledger names no `owners`".to_owned())
    );
}

#[test]
fn a_removal_without_an_owners_approval_fails() {
    for (case, fields) in unapproved() {
        let ledger = format!(
            "{OWNERS}{}{}",
            entry("D-901", "waived", &["waived.html"], APPROVED),
            entry("D-902", "removed", &["removed.html"], &fields)
        );
        let report = Site::new().classify(&ledger);
        assert!(!report.passed, "{case}");
        assert_eq!(class_of(&report, "removed.html"), None, "{case}");
        assert_eq!(report.counts.removed, 0, "{case}");
    }
    // The implementer's name in `accepted_by` is refused by name.
    let fields = APPROVED.replace("\"Project owner (GG)\"", "\"Claude (implementer)\"");
    let ledger = format!(
        "{OWNERS}{}",
        entry("D-902", "removed", &["removed.html"], &fields)
    );
    let report = Site::new().classify(&ledger);
    assert!(
        report.problems.iter().any(|p| p.contains(
            "is accepted by \"Claude (implementer)\", who is not an owner (Project owner (GG))"
        )),
        "{:?}",
        report.problems
    );
}

#[test]
fn an_output_that_does_not_exist_cannot_be_waived_or_removed() {
    // No manifest row.
    for class in ["waived", "removed"] {
        let ledger = format!(
            "{}{}",
            both(),
            entry("D-903", class, &["nowhere.html"], APPROVED)
        );
        let report = Site::new().classify(&ledger);
        assert!(!report.passed, "{class}");
        assert!(
            report.problems.contains(
                &"D-903: nowhere.html is no manifest row: only an output Jekyll writes can be \
                  waived or removed"
                    .to_owned()
            ),
            "{:?}",
            report.problems
        );
        assert_eq!(report.rows.len(), 4);
    }
    // Waived, but Folia does not write it: there is no output whose comparison to waive.
    let ledger = format!(
        "{OWNERS}{}{}",
        entry("D-901", "waived", &["removed.html"], APPROVED),
        entry("D-902", "waived", &["waived.html"], APPROVED)
    );
    let report = Site::new().classify(&ledger);
    assert_eq!(class_of(&report, "removed.html"), None);
    assert_eq!(
        problems_of(&report, "removed.html"),
        [
            "D-901 waives its comparison, but Folia does not write it: a waiver never excuses a missing output"
        ]
    );
    // Removed, but Folia writes it.
    let ledger = format!(
        "{OWNERS}{}{}",
        entry("D-901", "waived", &["waived.html"], APPROVED),
        entry(
            "D-902",
            "removed",
            &["removed.html", "built.html"],
            APPROVED
        )
    );
    let report = Site::new().classify(&ledger);
    assert_eq!(class_of(&report, "built.html"), None);
    assert_eq!(
        problems_of(&report, "built.html"),
        ["D-902 removes it, but Folia writes it"]
    );
}

#[test]
fn an_entry_not_of_the_schema_or_naming_a_pattern_fails_and_a_withdrawn_one_classifies_nothing() {
    let unknown = format!(
        "{}{}",
        both(),
        entry(
            "D-903",
            "waived",
            &["planned.html"],
            &format!("{APPROVED}\nscope = \"all\"")
        )
    );
    let report = Site::new().classify(&unknown);
    assert!(!report.passed);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.starts_with("D-903: not a coverage entry of the known schema")),
        "{:?}",
        report.problems
    );
    assert_eq!(class_of(&report, "planned.html"), None);
    let pattern = format!(
        "{}{}",
        both(),
        entry("D-903", "removed", &["*.html"], APPROVED)
    );
    assert!(
        Site::new()
            .classify(&pattern)
            .problems
            .contains(&"D-903: output \"*.html\" is not an explicit path".to_owned())
    );
    let withdrawn = format!(
        "{}{}",
        both(),
        entry(
            "D-903",
            "removed",
            &["planned.html"],
            "status = \"withdrawn\"\nwithdrawn = \"2026-11-03\""
        )
    );
    let report = Site::new().classify(&withdrawn);
    assert!(report.passed, "{report:?}");
    assert_eq!(class_of(&report, "planned.html"), Some(Class::NotBuiltYet));
}

#[test]
fn an_output_folia_writes_outside_the_manifest_or_a_wrong_count_fails() {
    let mut site = Site::new();
    site.written.insert("extra.html".to_owned());
    site.planned.insert("extra.html".to_owned());
    let report = site.classify(&both());
    assert!(!report.passed);
    assert!(
        report
            .problems
            .contains(&"Folia writes extra.html, which is no manifest row".to_owned())
    );
    let site = Site::new();
    let report = classify(
        &site.rows,
        &Folia {
            planned: &site.planned,
            written: &site.written,
            accounted: &site.accounted,
            not_built: 7,
        },
        &site.checks,
        &parse(&both()).unwrap(),
    );
    assert!(!report.passed);
}

#[test]
fn the_report_is_deterministic_and_every_row_is_in_the_markdown() {
    let site = Site::new();
    let (a, b) = (site.classify(&both()), site.classify(&both()));
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    let md = a.markdown();
    assert_eq!(md, b.markdown());
    for output in ["built.html", "planned.html", "removed.html", "waived.html"] {
        assert_eq!(md.matches(&format!("| `{output}` |")).count(), 1, "{md}");
    }
    assert!(
        md.contains(
            "| `waived.html` | waived | D-901, accepted 2026-11-03 by Project owner (GG) |"
        )
    );
    assert!(md.contains("| `planned.html` | not built yet |"));
    let dir = std::env::temp_dir().join(format!("parity-coverage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (json, _) = a.write(&dir).unwrap();
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(written["counts"]["rows"], 4);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_ledger_names_its_owner_and_holds_no_waiver_or_removal() {
    let ledger = load(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../parity/deviations.toml"
    )))
    .unwrap();
    assert_eq!(ledger.owners, ["Project owner (GG)"]);
    assert!(ledger.problems.is_empty(), "{:?}", ledger.problems);
    assert!(ledger.entries.is_empty());
    assert!(ledger.waived().is_empty());
}

#[test]
fn a_route_check_alone_is_no_comparison() {
    let mut site = Site::new();
    site.checks = Checks::default();
    site.checks.record("built.html", ROUTE, true);
    site.checks.record("waived.html", ROUTE, true);
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "built.html"), None);
    assert!(!report.passed);
    // Any check of its contents makes it compared.
    site.checks.record("built.html", "bytes", true);
    assert_eq!(
        class_of(&site.classify(&both()), "built.html"),
        Some(Class::BuiltAndChecked)
    );
}

#[test]
fn a_waived_row_needs_its_output_written_its_bytes_recorded_and_its_route_check_passing() {
    // Its route check fails.
    let mut site = Site::new();
    site.checks.record("waived.html", ROUTE, false);
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "waived.html"), None);
    assert_eq!(
        problems_of(&report, "waived.html"),
        ["D-901 waives its comparison, but its route check fails"]
    );
    // Its route check did not run.
    let mut site = Site::new();
    site.checks = Checks::default();
    site.checks.record("built.html", "dom", true);
    let report = site.classify(&both());
    assert_eq!(
        problems_of(&report, "waived.html"),
        ["D-901 waives its comparison, but its route check did not run"]
    );
    // Folia's output manifest does not record its bytes.
    let mut site = Site::new();
    site.accounted.remove("waived.html");
    let report = site.classify(&both());
    assert_eq!(
        problems_of(&report, "waived.html"),
        ["D-901 waives its comparison, but Folia's output manifest does not record its bytes"]
    );
    // Folia does not write it: a waiver never stands for a missing output.
    let mut site = Site::new();
    site.written.remove("waived.html");
    site.accounted.remove("waived.html");
    let report = site.classify(&both());
    assert_eq!(class_of(&report, "waived.html"), None);
    assert!(
        problems_of(&report, "waived.html")[0].ends_with("a waiver never excuses a missing output")
    );
    // A waived row's other checks are not required, and not counted against it.
    let mut site = Site::new();
    site.checks.record("waived.html", "dom", false);
    assert_eq!(
        class_of(&site.classify(&both()), "waived.html"),
        Some(Class::Waived)
    );
}

#[test]
fn a_removal_cannot_stand_for_an_output_folia_still_plans() {
    // planned.html is not built yet: a removal must not hide it.
    let ledger = format!(
        "{OWNERS}{}{}",
        entry("D-901", "waived", &["waived.html"], APPROVED),
        entry(
            "D-902",
            "removed",
            &["removed.html", "planned.html"],
            APPROVED
        )
    );
    let report = Site::new().classify(&ledger);
    assert!(!report.passed);
    assert_eq!(class_of(&report, "planned.html"), None);
    assert_eq!(
        problems_of(&report, "planned.html"),
        [
            "D-902 removes it, but Folia still plans it: an output Folia plans and does not write is \
          not built yet, not removed"
        ]
    );
    assert_eq!(class_of(&report, "removed.html"), Some(Class::Removed));
}

#[test]
fn only_an_approved_waiver_waives_any_check() {
    let proposed = APPROVED.replace("\"accepted\"\n", "\"proposed\"\n");
    let withdrawn = APPROVED.replace("\"accepted\"\n", "\"withdrawn\"\n");
    let not_owner = APPROVED.replace("\"Project owner (GG)\"", "\"Claude (implementer)\"");
    for fields in [&proposed, &withdrawn, &not_owner] {
        let ledger = parse(&format!(
            "{OWNERS}{}",
            entry("D-901", "waived", &["waived.html"], fields)
        ))
        .unwrap();
        assert!(ledger.waived().is_empty(), "{fields}");
    }
    let ledger = parse(&format!(
        "{OWNERS}{}",
        entry("D-901", "waived", &["waived.html"], APPROVED)
    ))
    .unwrap();
    assert_eq!(ledger.waived(), set(&["waived.html"]));
    // A removal waives nothing.
    let ledger = parse(&format!(
        "{OWNERS}{}",
        entry("D-902", "removed", &["removed.html"], APPROVED)
    ))
    .unwrap();
    assert!(ledger.waived().is_empty());
}

#[test]
fn a_withdrawn_entry_classifies_nothing_even_with_its_approval() {
    let withdrawn = APPROVED.replace("\"accepted\"\n", "\"withdrawn\"\n");
    let ledger = format!(
        "{OWNERS}{}",
        entry("D-901", "waived", &["waived.html"], &withdrawn)
    );
    let report = Site::new().classify(&ledger);
    // waived.html, written and checked only by its route, is then compared by nothing.
    assert_eq!(class_of(&report, "waived.html"), None);
    assert_eq!(report.counts.waived, 0);
}

#[test]
fn the_owners_are_the_only_names_that_approve_and_none_approve_nothing() {
    // An empty list, a list of something else, a blank name.
    for owners in [
        "owners = []\n",
        "owners = \"Project owner (GG)\"\n",
        "owners = [\" \"]\n",
        "owners = [1]\n",
    ] {
        let report = Site::new().classify(&both().replace(OWNERS, owners));
        assert!(!report.passed, "{owners}");
        assert_eq!(report.counts.waived + report.counts.removed, 0, "{owners}");
    }
    // A name that differs only in spacing or case is not the owner's.
    for by in ["Project owner (GG) ", "project owner (gg)", "Project owner"] {
        let fields = APPROVED.replace("\"Project owner (GG)\"", &format!("{by:?}"));
        let ledger = format!(
            "{OWNERS}{}{}",
            entry("D-901", "waived", &["waived.html"], &fields),
            entry("D-902", "removed", &["removed.html"], APPROVED)
        );
        let report = Site::new().classify(&ledger);
        assert_eq!(class_of(&report, "waived.html"), None, "{by}");
    }
    // A second owner, declared, approves; the report records who the owners were.
    let ledger = format!(
        "owners = [\"Project owner (GG)\", \"Co-owner\"]\n{}{}",
        entry(
            "D-901",
            "waived",
            &["waived.html"],
            &APPROVED.replace("\"Project owner (GG)\"", "\"Co-owner\"")
        ),
        entry("D-902", "removed", &["removed.html"], APPROVED)
    );
    let report = Site::new().classify(&ledger);
    assert!(report.passed, "{report:?}");
    assert_eq!(report.owners, ["Project owner (GG)", "Co-owner"]);
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["owners"][1], "Co-owner");
    assert!(report.markdown().contains(
        "Owners, who alone approve a waiver or a removal: Project owner (GG), Co-owner."
    ));
    // `owners` belongs at the top of the ledger: inside an entry it is an unknown field.
    let inside = format!(
        "{}{}",
        entry(
            "D-901",
            "waived",
            &["waived.html"],
            &format!("{APPROVED}\nowners = [\"Claude (implementer)\"]")
        ),
        entry("D-902", "removed", &["removed.html"], APPROVED)
    );
    let report = Site::new().classify(&inside);
    assert_eq!(report.counts.waived + report.counts.removed, 0);
}

#[test]
fn malformed_approval_metadata_fails() {
    for (case, fields) in [
        (
            "accepted as a boolean",
            APPROVED.replace("accepted = \"2026-11-03\"", "accepted = true"),
        ),
        (
            "an impossible date",
            APPROVED.replace("2026-11-03", "2026-02-30"),
        ),
        (
            "a date and time",
            APPROVED.replace("2026-11-03", "2026-11-03T10:00:00Z"),
        ),
        (
            "accepted_by as a list",
            APPROVED.replace(
                "accepted_by = \"Project owner (GG)\"",
                "accepted_by = [\"Project owner (GG)\"]",
            ),
        ),
        (
            "decision as a number",
            APPROVED.replace("decision = \"plan v12, §10.1\"", "decision = 1"),
        ),
        (
            "an unknown status",
            APPROVED.replace("\"accepted\"\n", "\"approved\"\n"),
        ),
    ] {
        let ledger = format!(
            "{OWNERS}{}{}",
            entry("D-901", "waived", &["waived.html"], &fields),
            entry("D-902", "removed", &["removed.html"], APPROVED)
        );
        let report = Site::new().classify(&ledger);
        assert!(!report.passed, "{case}");
        assert_eq!(class_of(&report, "waived.html"), None, "{case}");
        assert!(
            report.problems.iter().any(|p| p.starts_with("D-901: ")),
            "{case}: {:?}",
            report.problems
        );
    }
}
