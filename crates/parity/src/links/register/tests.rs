use super::*;

fn link(page: &str, value: &str, broken: bool) -> Link {
    Link {
        page: page.to_owned(),
        at: "a@href".to_owned(),
        value: value.to_owned(),
        verdict: if broken {
            Verdict::Broken {
                path: format!("/{value}"),
            }
        } else {
            Verdict::SamePage
        },
    }
}

fn entry(id: &str, page: &str, value: &str) -> String {
    format!(
        "[[deviation]]\nid = \"{id}\"\ncheck = \"links\"\nclass = \"known-source-defect\"\n\
         status = \"recorded\"\npage = \"{page}\"\nat = \"a@href\"\nvalue = \"{value}\"\n\
         source = \"src.md:3\"\nrationale = \"written without a scheme\"\n"
    )
}

/// A baseline directory whose `p.html` has the given links.
struct Baseline(std::path::PathBuf);

impl Baseline {
    fn new(test: &str, links: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!("folia-links-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut body = String::new();
        for l in links {
            body.push_str("<a href=\"");
            body.push_str(l);
            body.push_str("\">x</a>");
        }
        std::fs::write(
            dir.join("p.html"),
            format!("<html><body>{body}</body></html>"),
        )
        .unwrap();
        Self(dir)
    }
}

impl Drop for Baseline {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn report(links: Vec<Link>) -> Report {
    Report {
        pages: 1,
        links,
        problems: Vec::new(),
    }
}

#[test]
fn a_named_broken_link_the_baseline_has_is_a_known_defect_and_no_longer_fails() {
    let baseline = Baseline::new("known", &["github.com"]);
    let register = parse(&entry("L-1", "p.html", "github.com")).unwrap();
    assert_eq!(register.problems.len(), 0, "{:?}", register.problems);
    let mut report = report(vec![link("p.html", "github.com", true)]);
    assert!(!report.passed());
    report.apply(&register, &baseline.0);
    assert!(report.passed(), "{:?}", report.problems);
    assert!(matches!(
        report.links[0].verdict,
        Verdict::KnownSource { ref entry, .. } if entry == "L-1"
    ));
    let lines = report.lines().join("\n");
    assert!(lines.starts_with("ok: links:"), "{lines}");
    assert!(
        lines.contains("known source-content defect (L-1)"),
        "{lines}"
    );
    assert!(lines.contains("written so in src.md:3"), "{lines}");
}

#[test]
fn validation_stays_active_for_every_other_broken_link() {
    let baseline = Baseline::new("others", &["github.com", "codeberg.org"]);
    let register = parse(&entry("L-1", "p.html", "github.com")).unwrap();
    let mut report = report(vec![
        link("p.html", "github.com", true),
        link("p.html", "codeberg.org", true),
        link("q.html", "github.com", true),
    ]);
    report.apply(&register, &baseline.0);
    assert!(!report.passed());
    assert_eq!(
        report.broken().count(),
        2,
        "only the exact link is re-labelled"
    );
}

#[test]
fn an_entry_whose_link_is_no_longer_broken_or_there_is_stale_and_fails() {
    let baseline = Baseline::new("stale", &["github.com"]);
    let register = parse(&entry("L-1", "p.html", "github.com")).unwrap();
    for links in [
        vec![link("p.html", "github.com", false)],
        vec![link("p.html", "other", true)],
        Vec::new(),
    ] {
        let mut report = report(links);
        report.apply(&register, &baseline.0);
        assert!(
            report.problems.iter().any(|p| p.contains("stale")),
            "{:?}",
            report.problems
        );
        assert!(!report.passed());
    }
}

#[test]
fn a_link_the_baseline_lacks_is_folias_own_not_a_source_defect() {
    let baseline = Baseline::new("lacks", &["somewhere-else"]);
    let register = parse(&entry("L-1", "p.html", "github.com")).unwrap();
    let mut report = report(vec![link("p.html", "github.com", true)]);
    report.apply(&register, &baseline.0);
    assert!(!report.passed());
    assert!(report.problems.iter().any(|p| p.contains("Folia's own")));
    assert_eq!(report.broken().count(), 1);
}

#[test]
fn a_duplicate_or_malformed_entry_fails() {
    let baseline = Baseline::new("dup", &["github.com"]);
    let two = format!(
        "{}{}",
        entry("L-1", "p.html", "github.com"),
        entry("L-2", "p.html", "github.com")
    );
    let mut report = report(vec![link("p.html", "github.com", true)]);
    report.apply(&parse(&two).unwrap(), &baseline.0);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.contains("name the same link"))
    );
    for bad in [
        entry("L-1", "p.html", "github.com").replace("recorded", "accepted"),
        entry("L-1", "p.html", "github.com").replace("known-source-defect", "waived"),
        entry("L-1", "p.html", "github.com").replace(
            "rationale = \"written without a scheme\"",
            "rationale = \"\"",
        ),
        entry("L-1", "p.html", "github.com") + "surprise = 1\n",
    ] {
        let register = parse(&bad).unwrap();
        assert_ne!(register.problems.len(), 0, "{bad}");
        assert_eq!(register.entries.len(), 0, "{bad}");
    }
}

#[test]
fn withdrawn_entries_and_other_checks_are_ignored() {
    let text = format!(
        "{}[[deviation]]\nid = \"D-1\"\ncheck = \"dom\"\n",
        entry("L-1", "p.html", "github.com").replace("recorded", "withdrawn")
    );
    let register = parse(&text).unwrap();
    assert_eq!(register.entries.len(), 0);
    assert_eq!(register.problems.len(), 0);
}
