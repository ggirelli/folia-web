//! The site configuration, `folia.toml`: site identity and paths, not publishing rules
//! (plan v3, §3).

use std::collections::BTreeMap;
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
    /// The site's authors, by the key a document's `author` names.
    #[serde(default)]
    pub authors: BTreeMap<String, Author>,
    /// The documents the vertical slice builds (plan v9, §12.2). Without it, the build is of the
    /// whole site: every document, except those Folia cannot build yet, which it skips and names.
    #[serde(default)]
    pub slice: Option<Slice>,
    /// How the site's stylesheets are compiled from Sass.
    #[serde(default)]
    pub sass: SassConfig,
}

/// How stylesheets are compiled from Sass, with Jekyll's defaults.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SassConfig {
    /// The directory Sass imports are found in, relative to the site root (`_sass`).
    #[serde(default = "default_sass_dir")]
    pub dir: String,
    /// The output style.
    #[serde(default)]
    pub style: SassStyle,
}

impl Default for SassConfig {
    fn default() -> Self {
        Self {
            dir: default_sass_dir(),
            style: SassStyle::default(),
        }
    }
}

fn default_sass_dir() -> String {
    "_sass".to_owned()
}

/// Dart Sass's output styles.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SassStyle {
    /// One declaration per line (Jekyll's default).
    #[default]
    Expanded,
    /// As little whitespace as possible.
    Compressed,
}

impl std::fmt::Display for SassStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Expanded => "expanded",
            Self::Compressed => "compressed",
        })
    }
}

/// The documents the vertical slice builds, by source path relative to the site root. The site
/// names them, so that the build set is the site's, not Folia's. Each must build: one Folia
/// cannot build fails the build.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    /// The pages the slice builds, such as `pages/profile.md`.
    #[serde(default)]
    pub pages: Vec<String>,
    /// The posts the slice builds.
    #[serde(default)]
    pub posts: Vec<String>,
    /// The notes the slice builds.
    #[serde(default)]
    pub notes: Vec<String>,
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

/// An author, as pages show them.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Author {
    /// Their name (`Gabriele Girelli`).
    pub name: String,
    /// The name shown in a post's author box.
    pub display_name: String,
    /// Their picture, relative to the site root.
    pub avatar: String,
    /// Their web page.
    pub web: String,
    /// A line about them.
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
        assert!(config.authors.is_empty());
        assert_eq!(config.slice, None, "no slice: the whole site is built");
    }

    #[test]
    fn sass_settings_default_to_jekylls_and_are_read() {
        let base = "time-zone = \"UTC\"\n[roots]\nblog = \"b\"\nmicro = \"m\"\ngarden = \"g\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n";
        let config: Config = toml::from_str(base).unwrap();
        assert_eq!(config.sass.dir, "_sass");
        assert_eq!(config.sass.style, SassStyle::Expanded);
        let config: Config =
            toml::from_str(&format!("{base}[sass]\nstyle = \"compressed\"\n")).unwrap();
        assert_eq!(config.sass.style.to_string(), "compressed");
        assert!(toml::from_str::<Config>(&format!("{base}[sass]\nstyle = \"nested\"\n")).is_err());
    }

    #[test]
    fn the_slice_is_read_from_the_configuration() {
        let config: Config = toml::from_str(
            "time-zone = \"UTC\"\n[roots]\nblog = \"b\"\nmicro = \"m\"\ngarden = \"g\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n[slice]\npages = [\"p.md\"]\nnotes = [\"g/a.md\", \"g/b.md\"]\n",
        )
        .unwrap();
        let slice = config.slice.unwrap();
        assert_eq!(slice.pages, ["p.md"]);
        assert_eq!(slice.posts, Vec::<String>::new());
        assert_eq!(slice.notes, ["g/a.md", "g/b.md"]);
    }

    #[test]
    fn authors_are_read_by_their_key() {
        let config: Config = toml::from_str(
            "time-zone = \"UTC\"\n[roots]\nblog = \"b\"\nmicro = \"m\"\ngarden = \"g\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n[authors.gg]\nname = \"G\"\ndisplay-name = \"GG\"\navatar = \"a.webp\"\nweb = \"/\"\ndescription = \"x\"\n",
        )
        .unwrap();
        assert_eq!(config.authors["gg"].display_name, "GG");
        assert!(
            toml::from_str::<Config>(
                "time-zone = \"UTC\"\n[roots]\nblog = \"b\"\nmicro = \"m\"\ngarden = \"g\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n[presentation]\ncomments = false\n"
            )
            .is_err(),
            "presentation defaults are Rust's, not the configuration's"
        );
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<Config>("time-zone = \"UTC\"\ntitle = \"x\"\n").is_err());
    }
}
