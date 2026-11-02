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
//!    (`dom`), and so is its text.
//! 4. **Other files:** redirect stubs and static files are byte-identical.
//! 5. **Entries** (`entries`): each document's dates equal the baseline's dump; a page's table of
//!    contents equals Jekyll's; its code blocks are its body's, highlighted as spike 1 does.
//! 6. **The garden** (`garden`): the note graph has Jekyll's nodes and edges, and each broken
//!    link on a page built gives one warning.
//!
//! Outputs Folia does not build yet are counted, not compared.

mod entries;
mod garden;

use std::path::{Path, PathBuf};

use folia::build::Built;
use folia::config::Config;
use folia_core::content::{Route, SiteClock};

use crate::baseline::{self, Row};
use crate::builds::{self, Request};
use crate::document::assertions::{self, Output};
use crate::document::{self, Report, Scope, Side, ledger};
use crate::markdown::dom::{self, Node};
use crate::routes::{check_checkout, short};

/// Builds the site with Folia as of the baseline's build time, keeps the build in `out` (a new
/// directory, `builds::keep`; by default a new one in the temporary directory), and checks what
/// it wrote against the baseline in `baseline_dir`, printing the report. Returns whether every
/// check passed. The build is never deleted: its directory is printed, for the caller to inspect
/// or remove. `expected_templates`, when given, is the templates' tree hash the build requires.
///
/// The pages built are also compared as whole documents (`document`, plan v11, §5.1), judged
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
) -> Result<bool, String> {
    let ledger = ledger::load(ledger_path)?;
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
        None => fresh_temp_dir(),
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
    for checkout in [&kept.record.site, &kept.record.folia] {
        if let Some(builds::Changes::Unknown { error }) = &checkout.changes {
            println!(
                "warning: the record cannot identify the uncommitted changes of {}: {error}",
                checkout.path
            );
        }
    }
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
    let mut passed = compare(&built, &kept.output, baseline_dir, &baseline.rows, &clock)?;
    passed &= judge_documents(
        &built,
        &kept.output,
        &kept.dir,
        &baseline_dir.join("site"),
        &ledger,
        &config.site.url,
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

fn compare(
    built: &Built,
    out: &Path,
    baseline_dir: &Path,
    rows: &[Row],
    clock: &SiteClock,
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
    let mut passed = check_routes(built, rows);
    for route in &built.pages {
        let path = route.output_path();
        let folia = read_text(&out.join(&path))?;
        let Ok(jekyll) = std::fs::read_to_string(jekyll.join(&path)) else {
            println!("FAILS: {path}: Jekyll wrote no such page.");
            passed = false;
            continue;
        };
        passed &= check_page(&path, &jekyll, &folia);
        let (toc_ok, _) = entries::check_toc(
            &path,
            &dom::normalize_document(&jekyll),
            &dom::normalize_document(&folia),
        );
        passed &= toc_ok;
        if let Some(entry) = built.entries.iter().find(|entry| &entry.route == route) {
            passed &= entries::check_code(&path, entry, &jekyll, &folia);
        }
    }
    passed &= entries::check_dates(&built.entries, rows, &baseline_dir.join("dump"), clock);
    passed &= garden::check_warnings(built, &|path| {
        std::fs::read_to_string(jekyll.join(path)).ok()
    });

    let pages: Vec<String> = built.pages.iter().map(Route::output_path).collect();
    let stubs: Vec<String> = built.stubs.iter().map(Route::output_path).collect();
    let mut files = Vec::new();
    list_files(out, out, &mut files)?;
    let folia_graph = if files.iter().any(|f| f == garden::GRAPH) {
        Some(std::fs::read(out.join(garden::GRAPH)).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let jekyll_graph = std::fs::read(jekyll.join(garden::GRAPH)).ok();
    passed &= garden::check_graph_file(jekyll_graph.as_deref(), folia_graph.as_deref());
    let others: Vec<&String> = files
        .iter()
        .filter(|f| !pages.contains(f) && f.as_str() != garden::GRAPH)
        .collect();
    let mut differing = Vec::new();
    for path in &others {
        let folia = std::fs::read(out.join(path)).map_err(|e| format!("{path}: {e}"))?;
        match std::fs::read(jekyll.join(path)) {
            Ok(bytes) if bytes == folia => {}
            Ok(_) => differing.push(format!("{path}: differs")),
            Err(_) => differing.push(format!("{path}: Jekyll wrote no such file")),
        }
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

/// Each page and stub Folia rendered, and each file a mode generated, has the URL and output
/// file Jekyll gave it.
fn check_routes(built: &Built, rows: &[Row]) -> bool {
    let mut differing = Vec::new();
    for route in built
        .pages
        .iter()
        .chain(&built.stubs)
        .chain(&built.artifacts)
    {
        let output = route.output_path();
        match rows.iter().find(|row| row.written && row.output == output) {
            Some(row) if row.url == route.url() => {}
            Some(row) => differing.push(format!(
                "{output}: Jekyll's URL is {}, Folia's {}",
                row.url,
                route.url()
            )),
            None => differing.push(format!("{output}: Jekyll wrote no such output")),
        }
    }
    report(
        "routes of the pages, stubs and generated files built",
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
/// build's directory, and prints its outcome. Returns whether it passes.
fn judge_documents(
    built: &Built,
    output: &Path,
    dir: &Path,
    jekyll: &Path,
    ledger: &ledger::Ledger,
    site_url: &str,
) -> Result<bool, String> {
    let pages: Vec<String> = built.pages.iter().map(Route::output_path).collect();
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
                .map(|route| (route.output_path(), Output::Stub)),
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
    let (json, _) = report.write(dir)?;
    Ok(print_judgement(&report, &json))
}

/// Prints the outcome of a judged report, whose JSON is at `json`: a summary line, each entry
/// partly stale, and, when it fails, each difference not accepted, each stale entry and problem,
/// and each failed assertion. Returns whether it passes.
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
        "{}: whole-document DOM comparison of {} pages: {} differences, {} accepted by the ledger; {} entries, {} stale, {} partly stale, {} problems; {} assertions, {} failed (see {}).",
        if passed { "ok" } else { "FAILS" },
        totals.compared,
        totals.differences,
        accepted,
        applied.entries.len(),
        applied.entries.iter().filter(|e| e.stale).count(),
        applied.entries.iter().filter(|e| e.partly_stale).count(),
        applied.problems.len(),
        report.assertions.len(),
        failed_assertions,
        json.display()
    );
    // Partial staleness does not fail a run, but is always shown; final acceptance refuses it.
    for entry in applied.entries.iter().filter(|e| e.partly_stale) {
        println!(
            "  ledger: {} is partly stale (final acceptance, F13, refuses it):",
            entry.id
        );
        for line in applied
            .final_acceptance
            .iter()
            .filter(|line| line.starts_with(&format!("{}: ", entry.id)))
        {
            println!("    {line}");
        }
    }
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
            println!("  ledger: {} explains no difference (stale)", entry.id);
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

/// A directory in the temporary directory that does not exist yet, for a build to keep.
fn fresh_temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    (0..u32::MAX)
        .map(|n| base.join(format!("folia-parity-pages-{}-{n}", std::process::id())))
        .find(|dir| dir.symlink_metadata().is_err())
        .expect("some name is free")
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
mod tests;
