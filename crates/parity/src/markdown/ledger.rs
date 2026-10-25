//! The deviation ledger (`parity/deviations.toml`, plan v5 §9.4), as the Markdown check reads it:
//! which gate failures of which sources an entry explains. Only accepted entries suppress a
//! failure; a proposed one is reported as pending, and a withdrawn one explains nothing.

use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Ledger {
    deviation: Vec<Entry>,
}

/// One deviation.
#[derive(Deserialize)]
pub struct Entry {
    /// `D-NNN`.
    pub id: String,
    /// `proposed`, `accepted` or `withdrawn`.
    pub status: String,
    /// The sources it applies to.
    #[serde(default)]
    pub sources: Vec<String>,
    /// The Markdown gates whose failures it explains, such as `G3 text`.
    #[serde(default)]
    pub markdown_gates: Vec<String>,
    /// A check the failure must also pass to be explained: `resolved excerpt` means the note's
    /// excerpt is its body and has wiki-links, Jekyll's excerpt differs from Jekyll's body, and
    /// Folia's, rendered with the links resolved, shows what Jekyll's body shows.
    #[serde(default)]
    pub markdown_check: Option<String>,
}

impl Entry {
    /// Whether it is approved.
    pub fn accepted(&self) -> bool {
        self.status == "accepted"
    }

    /// Whether it may explain a failure: accepted, or proposed (and then reported as pending).
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "accepted" | "proposed")
    }
}

/// Reads the ledger at `path`.
///
/// # Errors
///
/// Fails when the file cannot be read or is not a ledger.
pub fn load(path: &Path) -> Result<Vec<Entry>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let ledger: Ledger = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(ledger.deviation)
}
