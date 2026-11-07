//! Page parity (plan v6, §9.2, and §12.2 step 1): Folia builds the site as of the baseline's
//! build time, and every file it writes is compared with the one Jekyll wrote at that path.
//!
//! 1. **Routes:** each rendered page, redirect stub and generated file has Jekyll's URL and output
//!    file.
//! 2. **Fingerprints:** a rendered page has Jekyll's title, canonical link, description and
//!    Open Graph values; its headings (level, ID, text, in order), and its link, image and
//!    resource (`link href`, `script src`) multisets, over the whole page; the text of its
//!    `div.main-content`, and of its whole body, so the site's frame (category counts, footer
//!    year) is checked too. Text values are compared with whitespace collapsed.
//! 3. **Main content:** the page's `div.main-content` is equal after spike 1's normalization
//!    (`dom`), and so is its text. A page that has none on either side, one Jekyll writes
//!    without the site's frame, is reported as such (`n/a`) and compared whole by the
//!    whole-document check only; one side alone without it fails.
//! 4. **Other files:** redirect stubs and static files are byte-identical.
//! 5. **Entries** (`entries`): each document's dates equal the baseline's dump; a page's table of
//!    contents equals Jekyll's; its code blocks are its body's, highlighted as spike 1 does, but
//!    on a claimed listing (`[[micro.listings]]`), whose blocks are its entries' (`n/a`).
//! 6. **The garden** (`garden`): the note graph has Jekyll's nodes and edges, and each broken
//!    link on a page built gives one warning.
//! 7. **Links** (`links`, A2.8): every internal `href` and `src` of the HTML written resolves to a
//!    planned output; this compares nothing with Jekyll, so it decides the run but no coverage
//!    class.
//! 8. **Expected warnings** (`warnings`, A2.9): every warning Folia gives is one
//!    `parity/expected-warnings.toml` lists, and every entry there matches one; likewise not a
//!    coverage check.
//!
//! Outputs Folia does not build yet are counted, not compared. The run fails, too, when the
//! build's record cannot identify the site or the Folia source it was built from, a directory
//! that is not a git checkout among them (`builds::unknown_identities`).

mod entries;
mod garden;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use folia::build::Built;
use folia::config::Config;
use folia_core::content::{Route, SiteClock};

use crate::baseline::{self, Row};
use crate::builds::{self, Request};
use crate::coverage::{self, Checks};
use crate::document::assertions::{self, Output};
use crate::document::{self, Report, Scope, Side, ledger};
use crate::links;
use crate::markdown::dom::{self, Node};
use crate::routes::{check_checkout, short};
use crate::warnings;

/// Builds the site with Folia as of the baseline's build time, keeps the build in `out` (a new
/// directory, `builds::keep`; by default a new one in the temporary directory), and checks what
/// it wrote against the baseline in `baseline_dir`, printing the report. Returns whether every
/// check passed. The build is never deleted: its directory is printed, for the caller to inspect
/// or remove. `expected_templates`, when given, is the templates' tree hash the build requires.
///
/// The pages built are also compared as whole documents (`document`, plan v12, §5.1), judged
/// against the DOM entries of the ledger at `ledger_path` (§5.2) and the hard assertions; that
/// comparison decides the check too. `dom-report.json` and `dom-report.md` are written into the
/// build's directory, with every difference and the ledger's verdict on it.
///
/// # Errors
///
/// Fails when the baseline or configuration cannot be read, the site checkout is not the commit
/// the baseline was built from (`check_checkout`), the build cannot be kept (`out` exists or is
/// inside the site), or Folia's output cannot be read.
pub fn run(
    site_dir: &Path,
    baseline_dir: &Path,
    config_path: &Path,
    out: Option<&Path>,
    expected_templates: Option<&str>,
    ledger_path: &Path,
    expected_warnings: &Path,
) -> Result<bool, String> {
    let ledger = ledger::load(ledger_path)?;
    let expected_warnings = warnings::load(expected_warnings)?;
    let coverage_ledger = coverage::load(ledger_path)?;
    let baseline = baseline::load(baseline_dir)?;
    check_checkout(site_dir, &baseline.commit, baseline.diff.as_deref())?;
    let config = Config::load(config_path).map_err(|e| e.to_string())?;
    println!(
        "Page parity: {} at {}, built {}.",
        site_dir.display(),
        short(&baseline.commit),
        baseline.built_at
    );

    let dest = match out {
        Some(out) => out.to_owned(),
        None => builds::fresh_temp_dir("pages"),
    };
    let kept = builds::keep(&Request {
        site: site_dir,
        config: &config,
        config_path,
        dest: &dest,
        at: baseline.built_at,
        expected_templates,
    })?;
    match kept.record.files {
        Some(files) => println!(
            "Folia's build ({files} files, with its manifest and record) is kept in {}.",
            kept.dir.display()
        ),
        None => println!(
            "Folia's failed build (its record and log) is kept in {}.",
            kept.dir.display()
        ),
    }
    // An unknown source identity fails the run (plan v12, §5.4), and every other check still
    // runs, so the kept build and its reports keep their diagnostics.
    let unknown = builds::unknown_identities(&kept.record);
    for reason in &unknown {
        println!("FAILS: source identity: {reason}");
    }
    let identified = unknown.is_empty();
    let built = match kept.built {
        Ok(built) => built,
        Err(problems) => {
            println!(
                "FAILS: Folia cannot build the site ({} problems):",
                problems.len()
            );
            for problem in &problems {
                println!("  {problem}");
            }
            return Ok(false);
        }
    };
    let clock = SiteClock::new(&config.time_zone, baseline.built_at).map_err(|e| e.to_string())?;
    // Outputs whose comparison an owner waived (A2.1, plan v12, §5.4) are compared by no check
    // but their route.
    let waived = coverage_ledger.waived();
    let planned = planned_routes(site_dir, &config, &baseline)?;
    let mut checks = Checks::default();
    let mut passed = identified;
    passed &= compare(
        &built,
        &kept.output,
        baseline_dir,
        &baseline.rows,
        &clock,
        &planned,
        &waived,
        &claimed(&config),
        &mut checks,
    )?;
    passed &= judge_documents(
        &built,
        &kept.output,
        &kept.dir,
        &baseline_dir.join("site"),
        &ledger,
        &config.site.url,
        &waived,
        &mut checks,
    )?;
    passed &= check_links(&kept.output, &kept.dir, &planned, &config.site.url)?;
    passed &= check_expected_warnings(&expected_warnings, &built, &kept.dir)?;
    passed &= report_coverage(
        &baseline,
        &built,
        &kept.output,
        &kept.dir,
        &planned,
        &checks,
        &coverage_ledger,
    )?;
    println!(
        "{}",
        if passed {
            "Page parity holds for every output Folia builds."
        } else {
            "Page parity FAILS."
        }
    );
    Ok(passed)
}

/// Every output Folia plans for the site, by output path, with its URL.
fn planned_routes(
    site_dir: &Path,
    config: &Config,
    baseline: &baseline::Baseline,
) -> Result<BTreeMap<String, String>, String> {
    let site = folia::site::plan(site_dir, config, baseline.built_at).map_err(|problems| {
        let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
        format!("Folia cannot plan the site: {}", lines.join("; "))
    })?;
    Ok(site
        .routes
        .outputs()
        .iter()
        .map(|planned| (planned.route.output_path(), planned.route.url().to_owned()))
        .collect())
}

/// The sources another mode claims, whose pages list that mode's entries: their bodies are not
/// rendered, so their code blocks are not their bodies'.
fn claimed(config: &Config) -> BTreeSet<String> {
    config
        .micro
        .listings
        .iter()
        .map(|listing| listing.source.clone())
        .chain(config.blog.grouped().map(|(page, _)| page.source.clone()))
        .collect()
}

/// Runs the checks of the pages, stubs, generated and static files Folia wrote, recording each
/// one's verdict on each output in `checks`; the outputs in `waived` are compared by none but
/// the route check, which is not a comparison of their contents.
#[expect(
    clippy::too_many_arguments,
    reason = "each is one input of the comparison; a struct would only rename them"
)]
fn compare(
    built: &Built,
    out: &Path,
    baseline_dir: &Path,
    rows: &[Row],
    clock: &SiteClock,
    planned: &BTreeMap<String, String>,
    waived: &BTreeSet<String>,
    claimed: &BTreeSet<String>,
    checks: &mut Checks,
) -> Result<bool, String> {
    let jekyll = &baseline_dir.join("site");
    println!(
        "Folia built {} pages, {} redirect stubs, {} generated files and {} static files; {} planned outputs are not built yet.",
        built.pages.len(),
        built.stubs.len(),
        built.artifacts.len(),
        built.statics,
        built.not_built
    );
    let mut files = Vec::new();
    list_files(out, out, &mut files)?;
    let mut passed = check_routes(&files, planned, rows, checks);
    for route in &built.pages {
        let path = route.output_path();
        if waived.contains(&path) {
            continue;
        }
        let folia = read_text(&out.join(&path))?;
        let Ok(jekyll) = std::fs::read_to_string(jekyll.join(&path)) else {
            println!("FAILS: {path}: Jekyll wrote no such page.");
            passed = false;
            continue;
        };
        let page_ok = check_page(&path, &jekyll, &folia);
        checks.record(&path, "page", page_ok);
        let (toc_ok, _) = entries::check_toc(
            &path,
            &dom::normalize_document(&jekyll),
            &dom::normalize_document(&folia),
        );
        checks.record(&path, "toc", toc_ok);
        passed &= page_ok && toc_ok;
        if let Some(entry) = built.entries.iter().find(|entry| &entry.route == route) {
            if claimed.contains(entry.doc.as_str()) {
                // A listing shows other entries' code; their own pages check it.
                println!(
                    "n/a: {path}: code blocks, which a claimed listing takes from its entries."
                );
                continue;
            }
            let code_ok = entries::check_code(&path, entry, &jekyll, &folia);
            checks.record(&path, "code", code_ok);
            passed &= code_ok;
        }
    }
    passed &= entries::check_dates(&built.entries, rows, &baseline_dir.join("dump"), clock);
    passed &= garden::check_warnings(built, &|path| {
        std::fs::read_to_string(jekyll.join(path)).ok()
    });

    let pages: Vec<String> = built.pages.iter().map(Route::output_path).collect();
    let stubs: Vec<String> = built.stubs.iter().map(Route::output_path).collect();
    let folia_graph = if files.iter().any(|f| f == garden::GRAPH) {
        Some(std::fs::read(out.join(garden::GRAPH)).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let jekyll_graph = std::fs::read(jekyll.join(garden::GRAPH)).ok();
    if !waived.contains(garden::GRAPH) {
        let graph_ok = garden::check_graph_file(jekyll_graph.as_deref(), folia_graph.as_deref());
        if folia_graph.is_some() {
            checks.record(garden::GRAPH, "graph", graph_ok);
        }
        passed &= graph_ok;
    }
    let others: Vec<&String> = files
        .iter()
        .filter(|f| !pages.contains(f) && f.as_str() != garden::GRAPH && !waived.contains(*f))
        .collect();
    let mut differing = Vec::new();
    for path in &others {
        let folia = std::fs::read(out.join(path)).map_err(|e| format!("{path}: {e}"))?;
        let same = match std::fs::read(jekyll.join(path)) {
            Ok(bytes) if bytes == folia => true,
            Ok(_) => {
                differing.push(format!("{path}: differs"));
                false
            }
            Err(_) => {
                differing.push(format!("{path}: Jekyll wrote no such file"));
                false
            }
        };
        checks.record(path, "bytes", same);
    }
    let stub_count = others.iter().filter(|f| stubs.contains(f)).count();
    passed &= report(
        &format!(
            "redirect stubs and static files byte-identical ({stub_count} stubs, {} static files)",
            others.len() - stub_count
        ),
        &differing,
    );
    Ok(passed)
}

/// Each output Folia wrote, static files included, is one Folia planned, at the URL and output
/// file Jekyll gave it: the route check, recorded as `route` (`coverage::ROUTE`). It compares no
/// contents.
fn check_routes(
    files: &[String],
    planned: &BTreeMap<String, String>,
    rows: &[Row],
    checks: &mut Checks,
) -> bool {
    let mut differing = Vec::new();
    for output in files {
        let row = rows.iter().find(|row| row.written && &row.output == output);
        let difference = match (planned.get(output), row) {
            (None, _) => Some(format!("{output}: Folia writes it, but does not plan it")),
            (Some(_), None) => Some(format!("{output}: Jekyll wrote no such output")),
            (Some(url), Some(row)) if *url == row.url => None,
            (Some(url), Some(row)) => Some(format!(
                "{output}: Jekyll's URL is {}, Folia's {url}",
                row.url
            )),
        };
        checks.record(output, coverage::ROUTE, difference.is_none());
        differing.extend(difference);
    }
    report(
        &format!("routes of every output written ({} files)", files.len()),
        &differing,
    )
}

/// What a page is checked by.
#[derive(Debug, PartialEq, Eq)]
struct Fingerprint {
    title: String,
    canonical: Vec<String>,
    description: Vec<String>,
    open_graph: Vec<String>,
    headings: Vec<String>,
    links: Vec<String>,
    images: Vec<String>,
    resources: Vec<String>,
    main_text: String,
    body_text: String,
}

fn check_page(path: &str, jekyll: &str, folia: &str) -> bool {
    let jekyll = dom::normalize_document(jekyll);
    let folia = dom::normalize_document(folia);
    let (want, got) = (fingerprint(&jekyll), fingerprint(&folia));
    let mut differing = Vec::new();
    let mut field = |name: &str, want: &dyn std::fmt::Debug, got: &dyn std::fmt::Debug| {
        let (want, got) = (format!("{want:?}"), format!("{got:?}"));
        if want != got {
            differing.push(format!("{name}: Jekyll {want}\n    Folia  {got}"));
        }
    };
    field("title", &want.title, &got.title);
    field("canonical", &want.canonical, &got.canonical);
    field("description", &want.description, &got.description);
    field("Open Graph", &want.open_graph, &got.open_graph);
    field("headings", &want.headings, &got.headings);
    field("links", &want.links, &got.links);
    field("images", &want.images, &got.images);
    field("resources", &want.resources, &got.resources);
    field("main-content text", &want.main_text, &got.main_text);
    field("body text", &want.body_text, &got.body_text);
    let fingerprint_ok = report(&format!("{path}: fingerprint"), &differing);

    let main = |nodes: &[Node]| find(nodes, &|node| has_class(node, "main-content")).cloned();
    let difference = match (main(&jekyll), main(&folia)) {
        (Some(want), Some(got)) => dom::first_difference(&[want], &[got]),
        (None, None) => {
            // A page Jekyll writes without the site's frame: the whole-document comparison
            // checks it instead (owner decision of 2026-11-07).
            println!("n/a: {path}: div.main-content, which neither page has (no frame).");
            return fingerprint_ok;
        }
        (None, _) => Some("Jekyll's page has no div.main-content".to_owned()),
        (_, None) => Some("Folia's page has no div.main-content".to_owned()),
    };
    let differing: Vec<String> = difference.into_iter().collect();
    let main_ok = report(
        &format!("{path}: div.main-content equal after normalization"),
        &differing,
    );
    fingerprint_ok && main_ok
}

fn fingerprint(page: &[Node]) -> Fingerprint {
    let elements = elements(page);
    let named = |name: &str| -> Vec<&Node> {
        elements
            .iter()
            .copied()
            .filter(|node| matches!(node, Node::Element { name: n, .. } if n == name))
            .collect()
    };
    let values = |name: &str, key: &str, filter: &dyn Fn(&Node) -> bool| -> Vec<String> {
        named(name)
            .into_iter()
            .filter(|node| filter(node))
            .filter_map(|node| attr(node, key))
            .map(|value| collapse(&value))
            .collect()
    };
    let sorted = |mut values: Vec<String>| {
        values.sort();
        values
    };
    let anything = |_: &Node| true;
    Fingerprint {
        title: named("title")
            .into_iter()
            .map(|node| dom::text(children(node)))
            .collect(),
        canonical: values("link", "href", &|node| {
            attr(node, "rel").as_deref() == Some("canonical")
        }),
        description: values("meta", "content", &|node| {
            attr(node, "name").as_deref() == Some("description")
        }),
        open_graph: named("meta")
            .into_iter()
            .filter_map(|node| Some((attr(node, "property")?, attr(node, "content")?)))
            .map(|(property, content)| format!("{property}={}", collapse(&content)))
            .collect(),
        headings: elements
            .iter()
            .filter_map(|node| match node {
                Node::Element {
                    name,
                    attrs,
                    children,
                } if matches!(name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                    let id = attrs.get("id").map_or("", String::as_str);
                    Some(format!("{name}#{id} {}", dom::text(children)))
                }
                _ => None,
            })
            .collect(),
        links: sorted(values("a", "href", &anything)),
        images: sorted(values("img", "src", &anything)),
        resources: sorted(
            values("link", "href", &anything)
                .into_iter()
                .chain(values("script", "src", &anything))
                .collect(),
        ),
        main_text: text_of(page, &|node| has_class(node, "main-content")),
        body_text: text_of(
            page,
            &|node| matches!(node, Node::Element { name, .. } if name == "body"),
        ),
    }
}

/// The text of the first element `wanted` picks, or nothing.
fn text_of(page: &[Node], wanted: &dyn Fn(&Node) -> bool) -> String {
    find(page, wanted)
        .map(|node| dom::text(children(node)))
        .unwrap_or_default()
}

/// Every element, in document order.
fn elements(nodes: &[Node]) -> Vec<&Node> {
    fn gather<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>) {
        for node in nodes {
            if let Node::Element { children, .. } = node {
                out.push(node);
                gather(children, out);
            }
        }
    }
    let mut out = Vec::new();
    gather(nodes, &mut out);
    out
}

fn find<'a>(nodes: &'a [Node], wanted: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
    elements(nodes).into_iter().find(|node| wanted(node))
}

fn children(node: &Node) -> &[Node] {
    match node {
        Node::Element { children, .. } => children,
        _ => &[],
    }
}

fn attr(node: &Node, key: &str) -> Option<String> {
    match node {
        Node::Element { attrs, .. } => attrs.get(key).cloned(),
        _ => None,
    }
}

fn has_class(node: &Node, class: &str) -> bool {
    attr(node, "class").is_some_and(|value| value.split(' ').any(|c| c == class))
}

/// Whitespace collapsed, as in text.
fn collapse(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn report(title: &str, differing: &[String]) -> bool {
    if differing.is_empty() {
        println!("ok: {title}.");
        return true;
    }
    println!("FAILS: {title}; {} differences:", differing.len());
    for line in differing {
        println!("  {line}");
    }
    false
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Compares the pages built with Jekyll's as whole documents, judges every difference against
/// the ledger, checks the hard assertions on the pages and stubs, writes the report into the
/// build's directory, and prints its outcome. Returns whether it passes. Each page's and stub's
/// verdicts are recorded in `checks`; the outputs in `waived` are not compared.
#[expect(
    clippy::too_many_arguments,
    reason = "each is one input of the comparison; a struct would only rename them"
)]
fn judge_documents(
    built: &Built,
    output: &Path,
    dir: &Path,
    jekyll: &Path,
    ledger: &ledger::Ledger,
    site_url: &str,
    waived: &BTreeSet<String>,
    checks: &mut Checks,
) -> Result<bool, String> {
    let pages: Vec<String> = built
        .pages
        .iter()
        .map(Route::output_path)
        .filter(|path| !waived.contains(path))
        .collect();
    let mut outcomes = Vec::new();
    let read = |root: &Path, path: &str| std::fs::read(root.join(path)).ok();
    for (path, kind) in pages
        .iter()
        .map(|path| {
            (
                path.clone(),
                Output::Page {
                    root: path == "index.html",
                },
            )
        })
        .chain(
            built
                .stubs
                .iter()
                .map(|route| (route.output_path(), Output::Stub))
                .filter(|(path, _)| !waived.contains(path)),
        )
    {
        let (want, got) = (read(jekyll, &path), read(output, &path));
        outcomes.extend(assertions::check_bytes(
            &path,
            kind,
            want.as_deref(),
            got.as_deref(),
            site_url,
        ));
    }
    let report = Report::new(
        Side::of("jekyll", jekyll)?,
        Side::of("folia", output)?,
        document::compare_trees(jekyll, output, &Scope::Only(&pages))?,
    )
    .judge(ledger, outcomes);
    for file in &report.files {
        let accepted = file.status == document::Status::Compared
            && file.actual_sha256.is_some()
            && file.actual_sha256 == file.actual_manifest_sha256
            && file
                .differences
                .iter()
                .all(|d| matches!(d.ledger, Some(ledger::Verdict::Accepted(_))));
        checks.record(&file.path, "dom", accepted);
    }
    for outcome in &report.assertions {
        checks.record(&outcome.file, "assertions", outcome.passed);
    }
    let (json, _) = report.write(dir)?;
    Ok(print_judgement(&report, &json))
}

/// Classifies every manifest row (A2.1, plan v12, §5.4) by what Folia plans and writes, the
/// checks' verdicts and the ledger's coverage entries; writes `coverage.json` and `coverage.md`
/// into the build's directory, and prints the counts and every problem. Returns whether it
/// passes.
fn report_coverage(
    baseline: &baseline::Baseline,
    built: &Built,
    output: &Path,
    dir: &Path,
    planned: &BTreeMap<String, String>,
    checks: &Checks,
    ledger: &coverage::Ledger,
) -> Result<bool, String> {
    let planned: BTreeSet<String> = planned.keys().cloned().collect();
    let mut files = Vec::new();
    list_files(output, output, &mut files)?;
    let written: BTreeSet<String> = files.into_iter().collect();
    // A waived output's bytes are accounted for: Folia's output manifest records it, with the
    // hash of the bytes written.
    let manifest = builds::read_manifest(&dir.join("sha256.txt"))?;
    let mut accounted = BTreeSet::new();
    for path in ledger.waived().intersection(&written) {
        let bytes = std::fs::read(output.join(path)).map_err(|e| format!("{path}: {e}"))?;
        if manifest.get(path) == Some(&builds::sha256(&bytes)) {
            accounted.insert(path.clone());
        }
    }
    let report = coverage::classify(
        &baseline.rows,
        &coverage::Folia {
            planned: &planned,
            written: &written,
            accounted: &accounted,
            not_built: built.not_built,
        },
        checks,
        ledger,
    );
    let (json, _) = report.write(dir)?;
    println!("{} (see {}).", report.summary(), json.display());
    for problem in &report.problems {
        println!("  {problem}");
    }
    for row in report.rows.iter().filter(|row| row.class.is_none()) {
        println!(
            "  {}: unclassified: {}",
            row.output,
            row.problems.join("; ")
        );
    }
    Ok(report.passed)
}

/// Prints the outcome of a judged report, whose JSON is at `json`: a summary line and, when it
/// fails, each difference not accepted, each stale entry with why it is stale, each problem, each
/// page whose bytes are not what Folia's output manifest records, and each failed assertion.
/// Returns whether it passes.
fn print_judgement(report: &Report, json: &Path) -> bool {
    let totals = &report.totals;
    let applied = report.ledger.as_ref().expect("judged");
    let accepted = report
        .files
        .iter()
        .flat_map(|file| &file.differences)
        .filter(|d| matches!(d.ledger, Some(ledger::Verdict::Accepted(_))))
        .count();
    let failed_assertions = report.assertions.iter().filter(|o| !o.passed).count();
    let passed = report.passed == Some(true);
    println!(
        "{}: whole-document DOM comparison of {} pages: {} differences, {} accepted by the ledger; {} entries, {} stale, {} problems; {} manifest mismatches; {} assertions, {} failed (see {}).",
        if passed { "ok" } else { "FAILS" },
        totals.compared,
        totals.differences,
        accepted,
        applied.entries.len(),
        applied.entries.iter().filter(|e| e.stale).count(),
        applied.problems.len(),
        report.manifest_mismatches.len(),
        report.assertions.len(),
        failed_assertions,
        json.display()
    );
    if !passed {
        for file in &report.files {
            for difference in &file.differences {
                if !matches!(difference.ledger, Some(ledger::Verdict::Accepted(_))) {
                    println!(
                        "  {}: {} {}: {:?}",
                        file.path,
                        difference.path,
                        difference.kind.name(),
                        difference.ledger
                    );
                }
            }
        }
        for entry in applied.entries.iter().filter(|e| e.stale) {
            if entry.explained.is_empty() {
                println!("  ledger: {} explains no difference (stale)", entry.id);
            } else {
                println!(
                    "  ledger: {} is stale: it explains {} differences, but not all it lists",
                    entry.id,
                    entry.explained.len()
                );
            }
            if entry.expected_build != ledger::Binding::AsReviewed {
                println!(
                    "    the baseline's record is {}",
                    ledger::binding_name(entry.expected_build)
                );
            }
            for output in entry.outputs.iter().filter(|o| {
                o.expected != ledger::Binding::AsReviewed || o.actual != ledger::Binding::AsReviewed
            }) {
                println!(
                    "    output {}: the baseline's file is {}, Folia's is {}",
                    output.path,
                    ledger::binding_name(output.expected),
                    ledger::binding_name(output.actual)
                );
            }
            for item in &entry.unused {
                println!(
                    "    output {}: match {} {} explains nothing",
                    item.output, item.path, item.kind
                );
            }
        }
        for mismatch in &report.manifest_mismatches {
            println!("  Folia's output manifest: {mismatch}");
        }
        for problem in &applied.problems {
            println!("  ledger: {problem}");
        }
        for outcome in report.assertions.iter().filter(|o| !o.passed) {
            println!(
                "  {}: assertion {}: {}",
                outcome.file, outcome.assertion, outcome.detail
            );
        }
    }
    passed
}

/// Checks that every internal link of the HTML Folia wrote resolves to a planned output (A2.8,
/// `links`), writes `links.json` into the build's directory, and prints the verdict.
fn check_links(
    output: &Path,
    dir: &Path,
    planned: &BTreeMap<String, String>,
    site_url: &str,
) -> Result<bool, String> {
    let written: BTreeSet<String> = builds::manifest_paths(output)?.into_iter().collect();
    let report = links::check(output, &written, planned, site_url)?;
    links::write(&report, dir)?;
    for line in report.lines() {
        println!("{line}");
    }
    Ok(report.passed())
}

/// Checks that every warning of the build is expected, and every expected warning given (A2.9,
/// `warnings`), beside the check that each broken link on a page built gives one warning
/// (`garden::check_warnings`); writes `warnings.json` into the build's directory and prints the
/// verdict.
fn check_expected_warnings(
    expected: &warnings::Expected,
    built: &Built,
    dir: &Path,
) -> Result<bool, String> {
    let report = expected.judge(&built.warnings);
    report.write(dir)?;
    for line in report.lines() {
        println!("{line}");
    }
    Ok(report.passed())
}

/// The files below `dir`, as paths relative to `root` with `/` separators.
fn list_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path: PathBuf = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            list_files(root, &path, out)?;
        } else {
            let relative = path.strip_prefix(root).expect("below the root");
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    out.sort();
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
