//! Slugs: the URL-safe form of a title or file name, as Jekyll makes them (`Utils.slugify`).

use std::sync::LazyLock;

use regex::Regex;

/// Which characters a slug keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlugMode {
    /// Letters, marks and decimal digits; anything else becomes `-`.
    Default,
    /// Also keeps `._~!$&'()+,;=@`, as in post titles.
    Pretty,
}

static DEFAULT_RUNS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{M}\p{L}\p{Nd}]+").unwrap());
static PRETTY_RUNS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{M}\p{L}\p{Nd}._~!$&'()+,;=@]+").unwrap());

/// The slug of `text`: each run of characters the mode does not keep becomes one `-`, a leading
/// and a trailing `-` are dropped, and the result is lowercased unless `cased`.
#[must_use]
pub fn slugify(text: &str, mode: SlugMode, cased: bool) -> String {
    let runs = match mode {
        SlugMode::Default => &*DEFAULT_RUNS,
        SlugMode::Pretty => &*PRETTY_RUNS,
    };
    let replaced = runs.replace_all(text, "-");
    let trimmed = replaced.strip_prefix('-').unwrap_or(&replaced);
    let trimmed = trimmed.strip_suffix('-').unwrap_or(trimmed);
    if cased {
        trimmed.to_owned()
    } else {
        trimmed.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_mode_lowercases_and_replaces_underscores() {
        assert_eq!(
            slugify(
                "2025-06-16-internet_resiliency_club",
                SlugMode::Default,
                false
            ),
            "2025-06-16-internet-resiliency-club"
        );
        assert_eq!(
            slugify("Self Hosting", SlugMode::Default, false),
            "self-hosting"
        );
    }

    #[test]
    fn the_pretty_mode_keeps_case_and_punctuation_when_cased() {
        assert_eq!(
            slugify("May-train-trip-pt1", SlugMode::Pretty, true),
            "May-train-trip-pt1"
        );
        assert_eq!(
            slugify("v1.2 (beta)", SlugMode::Pretty, true),
            "v1.2-(beta)"
        );
    }

    #[test]
    fn edge_hyphens_are_dropped() {
        assert_eq!(
            slugify("  hello world!  ", SlugMode::Default, false),
            "hello-world"
        );
    }

    #[test]
    fn letters_beyond_ascii_are_kept() {
        assert_eq!(
            slugify("Cos’è l’autismo", SlugMode::Default, false),
            "cos-è-l-autismo"
        );
    }
}
