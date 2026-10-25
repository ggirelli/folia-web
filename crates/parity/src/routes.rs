//! Route parity (plan v3, §10 spike 2): Folia must plan exactly the outputs Jekyll built.
//!
//! 1. **Authored outputs:** the multiset of (source, URL, output file) is identical.
//! 2. **Classification:** each source is the same kind of thing: a post, a micro entry, a garden
//!    note, a page or a static file.
//! 3. **Generated outputs:** the set of (URL, output file) of outputs without a source is
//!    identical.
//! 4. **Redirect stubs:** the set of stubs is identical.
//!
//! Folia's own checks, that no URL or output file is planned twice, run when it plans the site.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use folia::config::Config;
use folia::site::{Site, plan};
use folia_core::content::Mode;
use folia_core::publish::PlannedOutput;

use crate::baseline::{self, Row};

/// (source, URL, output file)
type Authored = (String, String, String);
/// (URL, output file)
type Generated = (String, String);

/// Checks Folia's routes for `site` against the baseline in `baseline_dir`, and prints the
/// report. Returns whether every check passed.
///
/// # Errors
///
/// Fails when the baseline or configuration cannot be read, or the site checkout is not the
/// clean commit the baseline was built from.
pub fn run(site_dir: &Path, baseline_dir: &Path, config_path: &Path) -> Result<bool, String> {
    let baseline = baseline::load(baseline_dir)?;
    check_checkout(site_dir, &baseline.commit)?;
    let config = Config::load(config_path).map_err(|e| e.to_string())?;
    println!(
        "Route parity: {} at {}, built {}.",
        site_dir.display(),
        short(&baseline.commit),
        baseline.built_at
    );

    let site = match plan(site_dir, &config, baseline.built_at) {
        Ok(site) => site,
        Err(problems) => {
            println!(
                "FAILS: Folia cannot plan the site ({} problems):",
                problems.len()
            );
            for problem in &problems {
                println!("  {problem}");
            }
            return Ok(false);
        }
    };
    let outputs = site.routes.outputs();
    println!(
        "Folia planned {} outputs, with no URL or output file planned twice.",
        outputs.len()
    );

    let written: Vec<&Row> = baseline.rows.iter().filter(|row| row.written).collect();
    let unwritten = baseline.rows.len() - written.len();
    if unwritten > 0 {
        println!("({unwritten} manifest rows were not written by Jekyll and are not compared.)");
    }
    let results = [
        check_authored(&written, outputs),
        check_classification(&written, &site),
        check_generated(&written, outputs),
        check_redirects(&written, outputs),
    ];
    print_generators(outputs);
    let passed = results.iter().all(|passed| *passed);
    println!(
        "{}",
        if passed {
            "Route parity holds."
        } else {
            "Route parity FAILS."
        }
    );
    Ok(passed)
}

fn check_authored(rows: &[&Row], outputs: &[PlannedOutput]) -> bool {
    let jekyll: Vec<Authored> = rows
        .iter()
        .filter_map(|row| {
            row.source
                .clone()
                .map(|source| (source, row.url.clone(), row.output.clone()))
        })
        .collect();
    let folia: Vec<Authored> = outputs
        .iter()
        .filter_map(|o| {
            o.source.as_ref().map(|id| {
                (
                    id.to_string(),
                    o.route.url().to_owned(),
                    o.route.output_path(),
                )
            })
        })
        .collect();
    report_multisets(
        "Authored outputs (source, URL, output)",
        &jekyll,
        &folia,
        |(s, u, o)| format!("{s}: {u} -> {o}"),
    )
}

/// What a source is, in words both sides share.
fn jekyll_class(row: &Row) -> &'static str {
    match (row.kind.as_str(), row.collection.as_deref()) {
        ("document", Some("posts")) => "post",
        ("document", Some("micro")) => "micro entry",
        ("document", Some("digital_garden")) => "garden note",
        ("page", _) => "page",
        ("static", _) => "static file",
        _ => "something else",
    }
}

fn folia_class(mode: Mode) -> &'static str {
    match mode {
        Mode::Blog => "post",
        Mode::Micro => "micro entry",
        Mode::Garden => "garden note",
        Mode::Pages => "page",
    }
}

fn check_classification(rows: &[&Row], site: &Site) -> bool {
    let jekyll: BTreeMap<&str, &str> = rows
        .iter()
        .filter_map(|row| {
            row.source
                .as_deref()
                .map(|source| (source, jekyll_class(row)))
        })
        .collect();
    let mut folia: BTreeMap<&str, &str> = site
        .discovery
        .documents
        .iter()
        .map(|(mode, id)| (id.as_str(), folia_class(*mode)))
        .collect();
    folia.extend(
        site.discovery
            .statics
            .iter()
            .map(|id| (id.as_str(), "static file")),
    );

    let differing: Vec<String> = jekyll
        .iter()
        .filter(|(source, class)| folia.get(*source) != Some(*class))
        .map(|(source, class)| {
            format!(
                "{source}: Jekyll {class}, Folia {}",
                folia.get(source).unwrap_or(&"nothing")
            )
        })
        .collect();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for class in jekyll.values() {
        *counts.entry(class).or_default() += 1;
    }
    let summary: Vec<String> = counts
        .iter()
        .map(|(class, n)| format!("{class}: {n}"))
        .collect();
    report(
        "Classification of sources",
        jekyll.len(),
        &summary.join(", "),
        &differing,
    )
}

fn check_generated(rows: &[&Row], outputs: &[PlannedOutput]) -> bool {
    let jekyll: Vec<Generated> = rows
        .iter()
        .filter(|row| row.source.is_none())
        .map(|row| (row.url.clone(), row.output.clone()))
        .collect();
    let folia: Vec<Generated> = outputs
        .iter()
        .filter(|o| o.source.is_none())
        .map(|o| (o.route.url().to_owned(), o.route.output_path()))
        .collect();
    report_multisets(
        "Generated outputs (URL, output)",
        &jekyll,
        &folia,
        |(u, o)| format!("{u} -> {o}"),
    )
}

fn check_redirects(rows: &[&Row], outputs: &[PlannedOutput]) -> bool {
    let jekyll: Vec<Generated> = rows
        .iter()
        .filter(|row| row.class == "JekyllRedirectFrom::RedirectPage")
        .map(|row| (row.url.clone(), row.output.clone()))
        .collect();
    let folia: Vec<Generated> = outputs
        .iter()
        .filter(|o| o.generator == Some("redirect stub"))
        .map(|o| (o.route.url().to_owned(), o.route.output_path()))
        .collect();
    report_multisets("Redirect stubs (URL, output)", &jekyll, &folia, |(u, o)| {
        format!("{u} -> {o}")
    })
}

/// Prints how many outputs each generator planned, for the reader.
fn print_generators(outputs: &[PlannedOutput]) {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for output in outputs {
        *counts
            .entry(output.generator.unwrap_or(if output.source.is_some() {
                "authored"
            } else {
                "unknown"
            }))
            .or_default() += 1;
    }
    let summary: Vec<String> = counts
        .iter()
        .map(|(generator, n)| format!("{generator} {n}"))
        .collect();
    println!("Folia's outputs by origin: {}.", summary.join(", "));
}

/// Compares two multisets and prints what is only on one side.
fn report_multisets<T: Ord + Clone>(
    title: &str,
    jekyll: &[T],
    folia: &[T],
    show: impl Fn(&T) -> String,
) -> bool {
    let count = |items: &[T]| {
        let mut counts: BTreeMap<T, usize> = BTreeMap::new();
        for item in items {
            *counts.entry(item.clone()).or_default() += 1;
        }
        counts
    };
    let (j, f) = (count(jekyll), count(folia));
    let keys: BTreeSet<&T> = j.keys().chain(f.keys()).collect();
    let mut differing = Vec::new();
    for key in keys {
        let (in_j, in_f) = (
            j.get(key).copied().unwrap_or(0),
            f.get(key).copied().unwrap_or(0),
        );
        if in_j > in_f {
            differing.push(format!(
                "only in Jekyll: {}{}",
                show(key),
                times(in_j - in_f)
            ));
        } else if in_f > in_j {
            differing.push(format!(
                "only in Folia: {}{}",
                show(key),
                times(in_f - in_j)
            ));
        }
    }
    report(
        title,
        jekyll.len(),
        &format!("Folia {}", folia.len()),
        &differing,
    )
}

fn times(n: usize) -> String {
    if n == 1 {
        String::new()
    } else {
        format!(" ({n} times)")
    }
}

fn report(title: &str, jekyll_count: usize, detail: &str, differing: &[String]) -> bool {
    if differing.is_empty() {
        println!("ok: {title}: Jekyll {jekyll_count}, {detail}, identical.");
        return true;
    }
    println!(
        "FAILS: {title}: Jekyll {jekyll_count}, {detail}; {} differences:",
        differing.len()
    );
    for line in differing {
        println!("  {line}");
    }
    false
}

/// The site checkout must be the clean commit the baseline was built from.
fn check_checkout(site_dir: &Path, commit: &str) -> Result<(), String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(site_dir)
            .args(args)
            .output()
            .map_err(|e| format!("cannot run git: {e}"))
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    let head = git(&["rev-parse", "HEAD"])?;
    if head != commit {
        return Err(format!(
            "{} is at {}, but the baseline was built from {}",
            site_dir.display(),
            short(&head),
            short(commit)
        ));
    }
    if !git(&["status", "--porcelain"])?.is_empty() {
        return Err(format!("{} has uncommitted changes", site_dir.display()));
    }
    Ok(())
}

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(8)]
}
