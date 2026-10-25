//! Page parity (plan v6, §9.2, and §12.2 step 1): Folia builds the site as of the baseline's
//! build time, and every file it writes is compared with the one Jekyll wrote at that path.
//!
//! 1. **Routes:** each rendered page and redirect stub has Jekyll's URL and output file.
//! 2. **Fingerprints:** a rendered page has Jekyll's title, canonical link, description and
//!    Open Graph values; its headings (level, ID, text, in order), and its link, image and
//!    resource (`link href`, `script src`) multisets, over the whole page; the text of its
//!    `div.main-content`, and of its whole body, so the site's frame (category counts, footer
//!    year) is checked too. Text values are compared with whitespace collapsed.
//! 3. **Main content:** the page's `div.main-content` is equal after spike 1's normalization
//!    (`dom`), and so is its text.
//! 4. **Other files:** redirect stubs and static files are byte-identical.
//!
//! Outputs Folia does not build yet are counted, not compared.

use std::path::{Path, PathBuf};

use folia::build::{Built, build};
use folia::config::Config;
use folia_core::content::Route;

use crate::baseline::{self, Row};
use crate::markdown::dom::{self, Node};
use crate::routes::{check_checkout, short};

/// Builds the site with Folia and checks what it wrote against the baseline in `baseline_dir`,
/// printing the report. Returns whether every check passed.
///
/// # Errors
///
/// Fails when the baseline or configuration cannot be read, the site checkout is not the commit
/// the baseline was built from (`check_checkout`), or Folia's output cannot be read.
pub fn run(site_dir: &Path, baseline_dir: &Path, config_path: &Path) -> Result<bool, String> {
    let baseline = baseline::load(baseline_dir)?;
    check_checkout(site_dir, &baseline.commit, baseline.diff.as_deref())?;
    let config = Config::load(config_path).map_err(|e| e.to_string())?;
    println!(
        "Page parity: {} at {}, built {}.",
        site_dir.display(),
        short(&baseline.commit),
        baseline.built_at
    );

    let out = std::env::temp_dir().join(format!("folia-parity-pages-{}", std::process::id()));
    let built = match build(site_dir, &config, &out, baseline.built_at) {
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
    let result = compare(&built, &out, &baseline_dir.join("site"), &baseline.rows);
    let _ = std::fs::remove_dir_all(&out);
    let passed = result?;
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

fn compare(built: &Built, out: &Path, jekyll: &Path, rows: &[Row]) -> Result<bool, String> {
    println!(
        "Folia built {} pages, {} redirect stubs and {} static files; {} planned outputs are not built yet.",
        built.pages.len(),
        built.stubs.len(),
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
    }

    let pages: Vec<String> = built.pages.iter().map(Route::output_path).collect();
    let stubs: Vec<String> = built.stubs.iter().map(Route::output_path).collect();
    let mut files = Vec::new();
    list_files(out, out, &mut files)?;
    let others: Vec<&String> = files.iter().filter(|f| !pages.contains(f)).collect();
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

/// Each page and stub Folia rendered has the URL and output file Jekyll gave it.
fn check_routes(built: &Built, rows: &[Row]) -> bool {
    let mut differing = Vec::new();
    for route in built.pages.iter().chain(&built.stubs) {
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
    report("routes of the pages and stubs built", &differing)
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
