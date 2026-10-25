//! The site configuration, `folia.toml`: site identity and paths, not publishing rules
//! (plan v3, §3).

use std::path::Path;

use serde::Deserialize;

/// A site's configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// The time zone dates are read in, such as `Europe/Stockholm`.
    pub time_zone: String,
    /// Paths published even though their name would hide them (`.htaccess`).
    #[serde(default)]
    pub include: Vec<String>,
    /// Paths that are not part of the site: globs relative to the site root, where `*` stays
    /// within one path segment. A pattern that matches a directory excludes all of it.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// The directory of the site's templates, relative to the site root.
    #[serde(default = "default_templates_dir")]
    pub templates_dir: String,
    /// The directory each mode's documents live in.
    pub roots: Roots,
    /// Who the site is: what templates show, and the URL redirect stubs point to.
    pub site: Identity,
}

/// Where templates live unless the configuration says otherwise: in the site, next to the
/// configuration, in a directory Jekyll ignores (plan v6, §14 Q5).
fn default_templates_dir() -> String {
    "_folia/templates".to_owned()
}

/// The source directory of each mode with a root; every other file with front matter is a page.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Roots {
    /// Blog posts, such as `_posts`.
    pub blog: String,
    /// Microblog entries, such as `_micro`.
    pub micro: String,
    /// Garden notes, such as `_digital_garden`.
    pub garden: String,
}

/// The site's identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// The URL the site is published at, without a trailing `/` (`https://ggirelli.info`).
    pub url: String,
    /// Its name (`Filopoetica`).
    pub title: String,
    /// Its tagline.
    pub description: String,
}

/// Why the configuration cannot be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file cannot be read.
    #[error("cannot read {0}: {1}")]
    Io(String, std::io::Error),
    /// The file is not a valid configuration.
    #[error("invalid configuration {0}: {1}")]
    Toml(String, toml::de::Error),
}

impl Config {
    /// Loads the configuration at `path`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read, or is not a valid configuration.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let name = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(name.clone(), e))?;
        toml::from_str(&text).map_err(|e| ConfigError::Toml(name, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_parses() {
        let config: Config = toml::from_str(
            "time-zone = \"Europe/Stockholm\"\nexclude = [\"vendor\"]\n[roots]\nblog = \"_posts\"\nmicro = \"_micro\"\ngarden = \"_digital_garden\"\n[site]\nurl = \"https://example.com\"\ntitle = \"Example\"\ndescription = \"A site.\"\n",
        )
        .unwrap();
        assert_eq!(config.roots.garden, "_digital_garden");
        assert_eq!(config.include, Vec::<String>::new());
        assert_eq!(config.templates_dir, "_folia/templates");
        assert_eq!(config.site.url, "https://example.com");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<Config>("time-zone = \"UTC\"\ntitle = \"x\"\n").is_err());
    }
}
