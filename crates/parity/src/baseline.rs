//! A Jekyll baseline made by `scripts/baseline.sh`: its manifest, and what was built when.

use std::path::Path;

use jiff::Timestamp;
use serde::Deserialize;

/// One output of the Jekyll build, as `parity/jekyll/folia_dump.rb` records it.
#[derive(Clone, Debug, Deserialize)]
pub struct Row {
    /// `page`, `document` or `static`.
    pub kind: String,
    /// The collection of a document (`posts`, `micro`, `digital_garden`).
    pub collection: Option<String>,
    /// The Ruby class, which tells generated pages apart.
    pub class: String,
    /// The source file, or `None` for a generated page.
    pub source: Option<String>,
    /// The URL Jekyll assigned.
    pub url: String,
    /// The file written, relative to the output directory.
    pub output: String,
    /// Whether Jekyll wrote it.
    pub written: bool,
}

/// A baseline's outputs and build.
pub struct Baseline {
    /// Every row of the manifest.
    pub rows: Vec<Row>,
    /// When the build started, which decides what was future-dated.
    pub built_at: Timestamp,
    /// The site commit it was built from.
    pub commit: String,
}

#[derive(Deserialize)]
struct Record {
    created_at: String,
    site: SiteRecord,
}

#[derive(Deserialize)]
struct SiteRecord {
    commit: String,
    dirty: bool,
}

/// Loads the baseline in `dir`.
///
/// # Errors
///
/// Fails when the baseline has no dump, cannot be read, or was built from a dirty tree, whose
/// routes Folia cannot reproduce from the same files.
pub fn load(dir: &Path) -> Result<Baseline, String> {
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .map_err(|e| format!("{}/{name}: {e}", dir.display()))
    };
    let record: Record =
        serde_json::from_str(&read("BASELINE.json")?).map_err(|e| format!("BASELINE.json: {e}"))?;
    if record.site.dirty {
        return Err(format!(
            "{} was built from a dirty tree; route parity needs a clean commit",
            dir.display()
        ));
    }
    let rows = read("dump/manifest.jsonl")?
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|e| format!("manifest.jsonl line {}: {e}", i + 1))
        })
        .collect::<Result<_, _>>()?;
    let built_at = record
        .created_at
        .parse()
        .map_err(|e| format!("BASELINE.json created_at: {e}"))?;
    Ok(Baseline {
        rows,
        built_at,
        commit: record.site.commit,
    })
}
