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
    /// The dump of its body HTML, relative to `dump/`.
    pub content: Option<String>,
    /// The dump of its rendered excerpt.
    pub excerpt: Option<String>,
    /// The dump of its metadata.
    pub meta: Option<String>,
}

/// A baseline's outputs and build.
pub struct Baseline {
    /// Every row of the manifest.
    pub rows: Vec<Row>,
    /// The time the build was as of, which decides what was future-dated: the time Jekyll was
    /// pinned to, or else when the build started.
    pub built_at: Timestamp,
    /// The site commit it was built from.
    pub commit: String,
    /// For a build of uncommitted changes, `git diff --binary HEAD` as it was then.
    pub diff: Option<String>,
}

#[derive(Deserialize)]
struct Record {
    created_at: String,
    site: SiteRecord,
    #[serde(default)]
    build: BuildRecord,
}

/// The build's flags; only its pinned time matters here.
#[derive(Default, Deserialize)]
struct BuildRecord {
    /// The time Jekyll was pinned to (`scripts/baseline.sh --at`), when it was.
    #[serde(default)]
    time: Option<String>,
}

#[derive(Deserialize)]
struct SiteRecord {
    commit: String,
    dirty: bool,
    #[serde(default)]
    diff: Option<String>,
}

/// Loads the baseline in `dir`.
///
/// # Errors
///
/// Fails when the baseline has no dump, cannot be read, or was built from a dirty tree without
/// recording its changes (`site.diff`), so that Folia cannot be shown the same files.
pub fn load(dir: &Path) -> Result<Baseline, String> {
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .map_err(|e| format!("{}/{name}: {e}", dir.display()))
    };
    let record: Record =
        serde_json::from_str(&read("BASELINE.json")?).map_err(|e| format!("BASELINE.json: {e}"))?;
    let diff = match (&record.site.diff, record.site.dirty) {
        (Some(name), true) => Some(read(name)?),
        (None, true) => {
            return Err(format!(
                "{} was built from a dirty tree without recording its changes; rebuild it",
                dir.display()
            ));
        }
        (_, false) => None,
    };
    let rows = read("dump/manifest.jsonl")?
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|e| format!("manifest.jsonl line {}: {e}", i + 1))
        })
        .collect::<Result<_, _>>()?;
    // A pinned build is as of its pinned time; any other, as of when it started.
    let (field, time) = match &record.build.time {
        Some(time) => ("build.time", time),
        None => ("created_at", &record.created_at),
    };
    let built_at = time
        .parse()
        .map_err(|e| format!("BASELINE.json {field}: {e}"))?;
    Ok(Baseline {
        rows,
        built_at,
        commit: record.site.commit,
        diff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads a baseline whose `BASELINE.json` is `record`, from a throwaway directory.
    fn baseline(name: &str, record: &str) -> Result<Baseline, String> {
        let dir =
            std::env::temp_dir().join(format!("parity-baseline-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("dump")).unwrap();
        std::fs::write(dir.join("BASELINE.json"), record).unwrap();
        std::fs::write(dir.join("dump/manifest.jsonl"), "").unwrap();
        let loaded = load(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        loaded
    }

    /// A baseline is as of when it was built, unless Jekyll's time was pinned.
    #[test]
    fn a_pinned_time_wins_over_the_build_start() {
        let started =
            r#"{"created_at": "2026-11-03T10:00:00Z", "site": {"commit": "c", "dirty": false}}"#;
        assert_eq!(
            baseline("started", started).unwrap().built_at.to_string(),
            "2026-11-03T10:00:00Z"
        );
        let pinned = r#"{"created_at": "2026-11-03T10:00:00Z", "site": {"commit": "c", "dirty": false},
            "build": {"time": "2026-10-25T14:43:51Z"}}"#;
        assert_eq!(
            baseline("pinned", pinned).unwrap().built_at.to_string(),
            "2026-10-25T14:43:51Z"
        );
        let unpinned = r#"{"created_at": "2026-11-03T10:00:00Z", "site": {"commit": "c", "dirty": false},
            "build": {"time": null}}"#;
        assert_eq!(
            baseline("unpinned", unpinned).unwrap().built_at.to_string(),
            "2026-11-03T10:00:00Z"
        );
        let bad = r#"{"created_at": "2026-11-03T10:00:00Z", "site": {"commit": "c", "dirty": false},
            "build": {"time": "yesterday"}}"#;
        assert!(baseline("bad", bad).is_err_and(|e| e.starts_with("BASELINE.json build.time")));
    }
}
