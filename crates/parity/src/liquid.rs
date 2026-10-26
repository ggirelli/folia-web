//! The content lint (plan v7, §7.2): no Liquid outside code in the site's Markdown.
//!
//! Spike 3 replaced the Liquid in the site's Markdown with what it produced, because Folia does
//! not run Liquid. Jekyll still does until the switch, so a new `{{ … }}` or `{% … %}` in prose
//! would work there and show as written in Folia. This check finds those.
//!
//! - **What is checked:** the body, after the front matter, of every Markdown document Folia
//!   discovers (`folia::discover`), so not excluded files, hidden directories, or Markdown
//!   files without front matter, which Jekyll copies as they are. HTML pages, layouts and
//!   includes are not checked, though Jekyll runs Liquid in them too.
//! - **What is found:** `{{` and `{%`, in the source as Liquid reads it: Jekyll runs Liquid
//!   before kramdown, so a Markdown escape (`\{{`) or raw HTML does not hide a delimiter.
//! - **What is exempt:** code, as Folia's Markdown reads it (`code_ranges`): fenced and indented
//!   code blocks and code spans, but not a code span kramdown keeps as text, such as one inside
//!   `<u>`, nor HTML's `<code>`.
//!
//! The exemption keeps code examples right for Folia, which shows them as written. Jekyll runs
//! Liquid inside code too, so until the switch such an example may still need `{% raw %}` to
//! show as intended there; the check neither requires nor adds it.
use std::path::Path;

use folia::config::Config;
use folia::discover::discover;
use folia_core::content::markdown::code_ranges;
use folia_core::content::split_front_matter;

use crate::markdown::is_markdown;

/// A Liquid delimiter outside code.
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    /// The document, relative to the site.
    pub source: String,
    /// Where it is in the file, from 1; the column counts characters.
    pub line: usize,
    pub column: usize,
    /// `{{` or `{%`.
    pub delimiter: &'static str,
}

/// Checks the Markdown of the site in `site_dir`, printing each finding. Returns whether there
/// was none.
///
/// # Errors
///
/// Fails when the configuration or a document cannot be read, or discovery finds a problem.
pub fn run(site_dir: &Path, config_path: &Path) -> Result<bool, String> {
    let config = Config::load(config_path).map_err(|e| e.to_string())?;
    let (checked, findings) = check(site_dir, &config)?;
    println!(
        "Liquid lint: {} Markdown documents of {}.",
        checked,
        site_dir.display()
    );
    for finding in &findings {
        println!(
            "{}:{}:{}: `{}` outside code. Folia does not run Liquid, so it would show as \
             written: use the value it produces, or put it in code.",
            finding.source, finding.line, finding.column, finding.delimiter
        );
    }
    if findings.is_empty() {
        println!("ok: no Liquid outside code.");
        return Ok(true);
    }
    let mut documents: Vec<&str> = findings.iter().map(|f| f.source.as_str()).collect();
    documents.dedup();
    println!(
        "FAILS: {} Liquid delimiters outside code, in {} documents.",
        findings.len(),
        documents.len()
    );
    Ok(false)
}

/// The number of Markdown documents of the site, and the findings in them, by document and
/// position.
///
/// # Errors
///
/// As [`run`].
pub fn check(site_dir: &Path, config: &Config) -> Result<(usize, Vec<Finding>), String> {
    let discovery = discover(site_dir, config).map_err(|problems| {
        problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut sources: Vec<&str> = discovery
        .documents
        .iter()
        .map(|(_, id)| id.as_str())
        .filter(|source| is_markdown(source))
        .collect();
    sources.sort_unstable();
    let mut findings = Vec::new();
    for source in &sources {
        let text =
            std::fs::read_to_string(site_dir.join(source)).map_err(|e| format!("{source}: {e}"))?;
        findings.extend(
            find(&text)
                .into_iter()
                .map(|(line, column, delimiter)| Finding {
                    source: (*source).to_owned(),
                    line,
                    column,
                    delimiter,
                }),
        );
    }
    Ok((sources.len(), findings))
}

/// The Liquid delimiters outside code in a document's `text`, after its front matter, by line,
/// column and delimiter. A delimiter counts as code only when both its characters are.
pub fn find(text: &str) -> Vec<(usize, usize, &'static str)> {
    let body = split_front_matter(text).map_or(text, |(_, body)| body);
    let start = text.len() - body.len();
    let code = code_ranges(body);
    let in_code = |at: usize| code.iter().any(|range| range.contains(&at));
    let bytes = body.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        let delimiter = match &bytes[at..at + 2] {
            b"{{" => "{{",
            b"{%" => "{%",
            _ => {
                at += 1;
                continue;
            }
        };
        if !(in_code(at) && in_code(at + 1)) {
            let (line, column) = position(text, start + at);
            found.push((line, column, delimiter));
        }
        at += 2;
    }
    found
}

/// The line and column of byte `at` in `text`, from 1.
fn position(text: &str, at: usize) -> (usize, usize) {
    let before = &text[..at];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}

#[cfg(test)]
mod tests;
