//! Expected warnings (plan v12, §5.4, work item A2.9; F10): every warning Folia gives is listed in
//! `parity/expected-warnings.toml`, and every entry there matches a warning.
//!
//! The file holds one `[[warning]]` table per expected warning:
//!
//! | Key | Meaning |
//! |-|-|
//! | `id` | a unique, not blank name (`W-001`) |
//! | `source` | the document the warning is about, as Folia names it (`_digital_garden/topic/note.md`), or `(site)` for one about no document |
//! | `message` | the warning's text, exactly; or else |
//! | `pattern` | a regular expression the whole text must match, only where exact text cannot be used, with |
//! | `pattern_reason` | why it cannot |
//! | `decision` | the decision that makes the warning expected (`Q3l: preserve, …`) |
//!
//! A warning matches an entry when its source is the entry's and its text is the entry's message
//! (or matches its pattern in full). The check fails on a warning no entry matches (unexpected),
//! an entry no warning matches (stale, or a missing warning), a warning two entries match
//! (ambiguous), and an entry two warnings match (one entry per expected warning). A file that
//! cannot be read, a table that is not an entry, an unknown key, a missing or blank field, both or
//! neither of `message` and `pattern`, a pattern without its reason or that is not a valid
//! regular expression, and an `id` used twice are errors: nothing is built.
//!
//! It checks the warnings Folia returns with a successful build (`Built::warnings`), which keeps
//! every one; a failed build fails the run on its own, and its warnings are not returned yet
//! (plan v12, A1.4).

use std::collections::BTreeSet;
use std::path::Path;

use folia_core::content::Diagnostic;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// What a warning names as its source when it is about no document.
pub const SITE: &str = "(site)";

/// The expected warnings.
#[derive(Debug)]
pub struct Expected {
    entries: Vec<Entry>,
}

/// One expected warning.
#[derive(Debug)]
struct Entry {
    id: String,
    source: String,
    text: Text,
    decision: String,
}

/// How an entry matches a warning's text.
#[derive(Debug)]
enum Text {
    Exact(String),
    Pattern(Regex),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    warning: Vec<Raw>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    id: Option<String>,
    source: Option<String>,
    message: Option<String>,
    pattern: Option<String>,
    pattern_reason: Option<String>,
    decision: Option<String>,
}

/// Reads the expected warnings from `path`.
///
/// # Errors
///
/// Fails when the file cannot be read, is not TOML, or any entry is not well formed (see the
/// module's documentation), naming every problem.
pub fn load(path: &Path) -> Result<Expected, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Result<Expected, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut problems = Vec::new();
    let mut entries = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, raw) in file.warning.into_iter().enumerate() {
        let at = format!("warning {}", index + 1);
        let field = |value: Option<String>, name: &str, problems: &mut Vec<String>| match value {
            Some(value) if !value.trim().is_empty() => Some(value),
            _ => {
                problems.push(format!("{at}: `{name}` is missing or blank"));
                None
            }
        };
        let id = field(raw.id, "id", &mut problems);
        let source = field(raw.source, "source", &mut problems);
        let decision = field(raw.decision, "decision", &mut problems);
        let text = match (raw.message, raw.pattern, raw.pattern_reason) {
            (Some(message), None, None) if !message.is_empty() => Some(Text::Exact(message)),
            (Some(_), None, None) => {
                problems.push(format!("{at}: `message` is empty"));
                None
            }
            (Some(_), None, Some(_)) => {
                problems.push(format!(
                    "{at}: `pattern_reason` is for a `pattern`, not a `message`"
                ));
                None
            }
            (None, Some(pattern), Some(reason)) if !reason.trim().is_empty() => {
                match Regex::new(&format!("^(?:{pattern})$")) {
                    Ok(regex) => Some(Text::Pattern(regex)),
                    Err(error) => {
                        problems.push(format!(
                            "{at}: `pattern` is not a regular expression: {error}"
                        ));
                        None
                    }
                }
            }
            (None, Some(_), _) => {
                problems.push(format!(
                    "{at}: a `pattern` needs a `pattern_reason`, saying why exact text cannot be used"
                ));
                None
            }
            (Some(_), Some(_), _) => {
                problems.push(format!("{at}: has both `message` and `pattern`; give one"));
                None
            }
            _ => {
                problems.push(format!("{at}: has neither `message` nor `pattern`"));
                None
            }
        };
        if let Some(id) = &id
            && !ids.insert(id.clone())
        {
            problems.push(format!("{at}: the id {id} is used twice"));
        }
        if let (Some(id), Some(source), Some(text), Some(decision)) = (id, source, text, decision) {
            entries.push(Entry {
                id,
                source,
                text,
                decision,
            });
        }
    }
    if problems.is_empty() {
        Ok(Expected { entries })
    } else {
        Err(problems.join("; "))
    }
}

/// A warning, and the entries that match it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub source: String,
    pub message: String,
    /// The ids of the entries that match it: one, when it is expected.
    pub entries: Vec<String>,
}

/// An entry, and how many warnings match it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct EntryReport {
    pub id: String,
    pub source: String,
    pub decision: String,
    /// How many warnings match it: one, when it is in use.
    pub warnings: usize,
}

/// The warnings of a build against the expected ones, written as `warnings.json`.
#[derive(Debug, Serialize)]
pub struct Report {
    pub warnings: Vec<Warning>,
    pub entries: Vec<EntryReport>,
}

impl Expected {
    /// Matches `warnings` (a build's) against the entries.
    pub fn judge(&self, warnings: &[Diagnostic]) -> Report {
        let mut counts = vec![0; self.entries.len()];
        let warnings = warnings
            .iter()
            .map(|warning| {
                let source = warning
                    .source
                    .as_ref()
                    .map_or_else(|| SITE.to_owned(), |doc| doc.as_str().to_owned());
                let entries = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.matches(&source, &warning.message))
                    .map(|(index, entry)| {
                        counts[index] += 1;
                        entry.id.clone()
                    })
                    .collect();
                Warning {
                    source,
                    message: warning.message.clone(),
                    entries,
                }
            })
            .collect();
        let entries = self
            .entries
            .iter()
            .zip(counts)
            .map(|(entry, warnings)| EntryReport {
                id: entry.id.clone(),
                source: entry.source.clone(),
                decision: entry.decision.clone(),
                warnings,
            })
            .collect();
        Report { warnings, entries }
    }
}

impl Entry {
    fn matches(&self, source: &str, message: &str) -> bool {
        self.source == source
            && match &self.text {
                Text::Exact(text) => text == message,
                Text::Pattern(regex) => regex.is_match(message),
            }
    }
}

impl Report {
    /// Whether every warning is expected by exactly one entry, and every entry matches exactly one.
    pub fn passed(&self) -> bool {
        self.warnings.iter().all(|w| w.entries.len() == 1)
            && self.entries.iter().all(|e| e.warnings == 1)
    }

    /// The run's lines: one `ok:` or `FAILS:` line, then each failure.
    pub fn lines(&self) -> Vec<String> {
        let summary = format!(
            "{} warnings, {} expected warnings",
            self.warnings.len(),
            self.entries.len()
        );
        if self.passed() {
            return vec![format!(
                "ok: expected warnings: {summary}; each warning is expected by one entry, and each entry matches one warning (see warnings.json)."
            )];
        }
        let mut failures = Vec::new();
        for warning in &self.warnings {
            match warning.entries.len() {
                0 => failures.push(format!(
                    "unexpected warning: {}: {}",
                    warning.source, warning.message
                )),
                1 => {}
                _ => failures.push(format!(
                    "ambiguous: {} entries expect the warning {}: {} ({})",
                    warning.entries.len(),
                    warning.source,
                    warning.message,
                    warning.entries.join(", ")
                )),
            }
        }
        for entry in &self.entries {
            match entry.warnings {
                0 => failures.push(format!(
                    "{}: expects a warning about {} that Folia did not give (stale, or the warning is missing)",
                    entry.id, entry.source
                )),
                1 => {}
                n => failures.push(format!(
                    "{}: matches {n} warnings; one entry per expected warning",
                    entry.id
                )),
            }
        }
        let mut out = vec![format!(
            "FAILS: expected warnings: {summary}; {} failures (see warnings.json):",
            failures.len()
        )];
        out.extend(failures.into_iter().map(|f| format!("  {f}")));
        out
    }

    /// Writes the report as `warnings.json` into `dir`; an existing file is never overwritten.
    ///
    /// # Errors
    ///
    /// Fails when the file exists or cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(), String> {
        use std::io::Write as _;
        let path = dir.join("warnings.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())? + "\n";
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut file| file.write_all(json.as_bytes()))
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests;
