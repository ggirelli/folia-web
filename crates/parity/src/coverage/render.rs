//! The coverage report as text: its summary, its verdict, `coverage.json` and `coverage.md`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::{Class, Report};

impl Report {
    /// The summary line `parity pages` prints on every run.
    pub fn summary(&self) -> String {
        let c = &self.counts;
        format!(
            "{}: coverage of {} manifest rows: {} built and checked, {} not built yet, {} waived, \
             {} removed, {} intentional, {} known and excluded, {} defects, {} missing evidence; \
             {} unclassified, {} problems",
            if self.passed { "ok" } else { "FAILS" },
            c.rows,
            c.built_and_checked,
            c.not_built_yet,
            c.waived,
            c.removed,
            c.intentional,
            c.known_excluded,
            c.defects,
            c.missing_evidence,
            c.unclassified,
            self.problems.len()
        )
    }

    /// The verdict `parity pages` prints after the counts: a clean build is not exact parity.
    pub fn verdict(&self) -> String {
        let c = &self.counts;
        let yes = |b: bool| if b { "yes" } else { "NO" };
        format!(
            "Clean build (every output accounted for, none a defect or lacking evidence): {}. Exact Jekyll parity: {} ({} of {} rows identical; {} built and \
             checked with ledger-accepted differences, {} intentional, {} known and excluded, {} \
             defects, {} missing evidence, {} unclassified; {} Folia-only outputs). Zero failed \
             assertions is not exact parity, and a documented difference is not identical output.",
            yes(self.clean_build),
            yes(self.exact_parity),
            c.built_and_checked - c.built_with_accepted_differences,
            c.rows,
            c.built_with_accepted_differences,
            c.intentional,
            c.known_excluded,
            c.defects,
            c.missing_evidence,
            c.unclassified,
            self.folia_only.len()
        )
    }

    /// Writes `coverage.json` and `coverage.md` into `dir`, and returns their paths.
    ///
    /// # Errors
    ///
    /// Fails when a file cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
        let json = dir.join("coverage.json");
        let md = dir.join("coverage.md");
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())? + "\n";
        std::fs::write(&json, text).map_err(|e| format!("{}: {e}", json.display()))?;
        std::fs::write(&md, self.markdown()).map_err(|e| format!("{}: {e}", md.display()))?;
        Ok((json, md))
    }

    /// The report as Markdown: the counts, the problems, and every row with its class.
    pub fn markdown(&self) -> String {
        let c = &self.counts;
        let mut out = String::from("# Coverage\n\n");
        let _ = writeln!(out, "{}.\n", self.summary());
        let _ = writeln!(out, "**{}**\n", self.verdict());
        let _ = writeln!(
            out,
            "Owners, who alone approve a waiver or a removal: {}.\n",
            if self.owners.is_empty() {
                "none".to_owned()
            } else {
                self.owners.join(", ")
            }
        );
        out.push_str("| Class | Rows |\n|-|-:|\n");
        for (name, count) in [
            ("built and checked", c.built_and_checked),
            ("not built yet", c.not_built_yet),
            ("waived (written, comparison waived; approved)", c.waived),
            ("removed (approved)", c.removed),
            (Class::Intentional.name(), c.intentional),
            (Class::KnownExcluded.name(), c.known_excluded),
            (Class::Defect.name(), c.defects),
            (Class::MissingEvidence.name(), c.missing_evidence),
            ("**unclassified** (fails)", c.unclassified),
        ] {
            let _ = writeln!(out, "| {name} | {count} |");
        }
        let _ = writeln!(out, "| total | {} |", c.rows);
        let _ = writeln!(
            out,
            "\nOf the {} built and checked, {} have differences the ledger accepts (not identical \
             to Jekyll's output); {} are identical. Folia-only outputs, which are no row: {}.",
            c.built_and_checked,
            c.built_with_accepted_differences,
            c.built_and_checked - c.built_with_accepted_differences,
            self.folia_only.len()
        );
        if !self.folia_only.is_empty() {
            out.push_str("\n## Folia-only outputs (no Jekyll baseline)\n\n");
            for f in &self.folia_only {
                let _ = writeln!(
                    out,
                    "- `{}`: {}, accepted {} by {}",
                    f.output, f.approval.entry, f.approval.accepted, f.approval.accepted_by
                );
            }
        }
        if !self.problems.is_empty() {
            out.push_str("\n## Problems\n\n");
            for problem in &self.problems {
                let _ = writeln!(out, "- FAILS: {problem}");
            }
        }
        out.push_str("\n## Rows\n\n| Output | Class | Approval or checks |\n|-|-|-|\n");
        for row in &self.rows {
            let class = row
                .class
                .map_or_else(|| "**UNCLASSIFIED**".to_owned(), |c| c.name().to_owned());
            let detail = match (&row.approval, row.problems.is_empty()) {
                (Some(a), _) => {
                    format!("{}, accepted {} by {}", a.entry, a.accepted, a.accepted_by)
                }
                (None, true) if row.entry.is_some() => format!(
                    "{} (recorded, not approved): {}",
                    row.entry.as_deref().unwrap_or_default(),
                    row.checks
                        .iter()
                        .map(|v| v.check)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                (None, true) if row.accepted_differences > 0 => format!(
                    "{} (ledger-accepted differences: {})",
                    row.checks
                        .iter()
                        .map(|v| v.check)
                        .collect::<Vec<_>>()
                        .join(", "),
                    row.accepted_differences
                ),
                (None, false) => row.problems.join("; "),
                (None, true) => row
                    .checks
                    .iter()
                    .map(|v| v.check)
                    .collect::<Vec<_>>()
                    .join(", "),
            };
            let _ = writeln!(
                out,
                "| `{}` | {class} | {} |",
                row.output.replace('|', "\\|"),
                detail.replace('|', "\\|")
            );
        }
        out
    }
}
