//! The register of known source-content link defects: links that resolve to nothing because the
//! site's own source writes them so, and that Jekyll's output has too, so that the fault is not
//! Folia's. It is the site owner's private data, kept in the deviation ledger (`FOLIA_LEDGER`)
//! as `check = "links"` entries:
//!
//! ```toml
//! [[deviation]]
//! id = "L-001"
//! check = "links"
//! class = "known-source-defect"
//! status = "recorded"
//! page = "dg/some/page.html"      # the output file the link is in
//! at = "a@href"                   # the element and attribute
//! value = "github.com"            # the attribute's value, as written
//! source = "_digital_garden/some/page.md:26"
//! rationale = "why this is the source's fault"
//! ```
//!
//! An entry changes nothing about the validation: every link is still resolved, and every other
//! broken link still fails the run. An entry only re-labels the one broken link it names
//! exactly (page, attribute, value) as a *known source-content defect*, and only if the Jekyll
//! baseline's page has the same attribute value, so that it is shown to be no change Folia made.
//! It is **stale**, a failure, when the link no longer is broken or is no longer there, when the
//! baseline lacks it, or when another entry names the same link.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::{Link, Report, Verdict, attributes};

/// The `check` of a register entry.
pub const CHECK: &str = "links";

/// An entry as written; any other field is refused.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "check")]
    _check: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    page: String,
    #[serde(default)]
    at: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    rationale: String,
}

/// One known source-content link defect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    pub id: String,
    pub page: String,
    pub at: String,
    pub value: String,
    pub source: String,
}

/// The register read from a ledger: its entries, and what is wrong with it.
#[derive(Clone, Debug, Default)]
pub struct Register {
    pub entries: Vec<Known>,
    pub problems: Vec<String>,
}

/// Reads the `check = "links"` entries of the ledger at `path`. Withdrawn entries are ignored.
///
/// # Errors
///
/// Fails when the file cannot be read or is not TOML; a malformed entry is a problem in the
/// register, which fails the run, not an error here.
pub fn load(path: &Path) -> Result<Register, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Result<Register, String> {
    let file: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut register = Register::default();
    let entries = match file.get("deviation") {
        Some(toml::Value::Array(entries)) => entries.as_slice(),
        _ => &[],
    };
    for (index, value) in entries.iter().enumerate() {
        if value.get("check").and_then(toml::Value::as_str) != Some(CHECK) {
            continue;
        }
        let name = value
            .get("id")
            .and_then(toml::Value::as_str)
            .map_or_else(|| format!("links entry {}", index + 1), str::to_owned);
        let raw: Raw = match value.clone().try_into() {
            Ok(raw) => raw,
            Err(e) => {
                register.problems.push(format!(
                    "{name}: not a links entry of the known schema: {}",
                    e.to_string().trim()
                ));
                continue;
            }
        };
        if raw.status == "withdrawn" {
            continue;
        }
        let mut wrong = Vec::new();
        if raw.id.trim().is_empty() {
            wrong.push("has no id");
        }
        if raw.status != "recorded" {
            wrong.push("status must be \"recorded\" (or \"withdrawn\"): it records a fault, it approves nothing");
        }
        if raw.class != "known-source-defect" {
            wrong.push("class must be \"known-source-defect\"");
        }
        let before = register.problems.len();
        for (field, text) in [
            ("page", &raw.page),
            ("at", &raw.at),
            ("value", &raw.value),
            ("source", &raw.source),
            ("rationale", &raw.rationale),
        ] {
            if text.trim().is_empty() {
                register.problems.push(format!("{name}: has no {field}"));
            }
        }
        register
            .problems
            .extend(wrong.iter().map(|w| format!("{name}: {w}")));
        if register.problems.len() > before {
            continue;
        }
        register.entries.push(Known {
            id: raw.id,
            page: raw.page,
            at: raw.at,
            value: raw.value,
            source: raw.source,
        });
    }
    Ok(register)
}

impl Report {
    /// Applies `register`: each broken link an entry names exactly becomes a known source-content
    /// defect, provided the Jekyll baseline's page (`jekyll`, its `site/`) has the same attribute.
    /// What is wrong with the register or an entry is added to the report's problems.
    pub fn apply(&mut self, register: &Register, jekyll: &Path) {
        self.problems.extend(register.problems.iter().cloned());
        let mut named: BTreeMap<(&str, &str, &str), &Known> = BTreeMap::new();
        for entry in &register.entries {
            let key = (entry.page.as_str(), entry.at.as_str(), entry.value.as_str());
            if let Some(other) = named.insert(key, entry) {
                self.problems.push(format!(
                    "{} and {} name the same link ({} {}=\"{}\")",
                    other.id, entry.id, entry.page, entry.at, entry.value
                ));
            }
        }
        for entry in named.values() {
            let broken: Vec<&mut Link> = self
                .links
                .iter_mut()
                .filter(|l| {
                    l.page == entry.page
                        && l.at == entry.at
                        && l.value == entry.value
                        && matches!(l.verdict, Verdict::Broken { .. })
                })
                .collect();
            if broken.is_empty() {
                self.problems.push(format!(
                    "{}: stale: {} has no broken {}=\"{}\" any more; remove the entry",
                    entry.id, entry.page, entry.at, entry.value
                ));
                continue;
            }
            if !jekyll_has(jekyll, entry) {
                self.problems.push(format!(
                    "{}: Jekyll's {} has no {}=\"{}\": the link is Folia's own, not a source-content defect",
                    entry.id, entry.page, entry.at, entry.value
                ));
                continue;
            }
            for link in broken {
                if let Verdict::Broken { path } = &link.verdict {
                    link.verdict = Verdict::KnownSource {
                        path: path.clone(),
                        entry: entry.id.clone(),
                        source: entry.source.clone(),
                    };
                }
            }
        }
    }
}

/// Whether the baseline's page of `entry` has its attribute with its value.
fn jekyll_has(jekyll: &Path, entry: &Known) -> bool {
    std::fs::read_to_string(jekyll.join(&entry.page)).is_ok_and(|text| {
        attributes(&text)
            .iter()
            .any(|(at, value)| *at == entry.at && *value == entry.value)
    })
}

#[cfg(test)]
mod tests;
