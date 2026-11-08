//! The site configuration, `folia.toml`: site identity and paths, not publishing rules
//! (plan v3, §3).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

mod author;

pub use author::{Author, AuthorCover};

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
    /// The microblog's settings.
    #[serde(default)]
    pub micro: MicroConfig,
    /// The blog's settings.
    #[serde(default)]
    pub blog: BlogConfig,
    /// The garden's settings.
    #[serde(default)]
    pub garden: GardenConfig,
    /// The site's feeds; without it, none is written.
    #[serde(default)]
    pub feeds: Option<crate::feed::FeedsConfig>,
}

/// The garden's settings: what the site's front matter defaults gave every note.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GardenConfig {
    /// The author of every note, a key of `[authors]`, if any; the feeds name them.
    #[serde(default)]
    pub author: Option<String>,
}

/// The blog's listings (`folia_blog::listings`): each is built only when configured.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BlogConfig {
    /// The blog's index, which the blog claims: its pages, `/blog/` and `/blog/pageN/`.
    #[serde(default)]
    pub index: Option<ClaimedPage>,
    /// The template of the category archives (`archive.html`).
    #[serde(default)]
    pub archive_template: Option<String>,
    /// The page listing the posts by category, which the blog claims (`page.groups`).
    #[serde(default)]
    pub categories: Option<ClaimedPage>,
    /// The page listing the posts by tag, which the blog claims (`page.groups`).
    #[serde(default)]
    pub tags: Option<ClaimedPage>,
}

impl BlogConfig {
    /// The pages the blog claims whose frame the pages mode builds: the categories and tags
    /// pages, each with what groups its posts.
    pub fn grouped(&self) -> impl Iterator<Item = (&ClaimedPage, folia_blog::Grouping)> {
        [
            (self.categories.as_ref(), folia_blog::Grouping::Categories),
            (self.tags.as_ref(), folia_blog::Grouping::Tags),
        ]
        .into_iter()
        .filter_map(|(page, by)| page.map(|page| (page, by)))
    }
}

/// A page of the site a mode claims: the page keeps its route and front matter, its body
/// (Liquid, for Jekyll) is not rendered, and `template` renders what the mode lists.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimedPage {
    /// The page's source, relative to the site root (`blog/index.html`).
    pub source: String,
    /// The template that renders it.
    pub template: String,
}

/// The microblog's settings: what the site's front matter defaults gave every micro entry.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MicroConfig {
    /// The author of every entry, a key of `[authors]`, if any.
    #[serde(default)]
    pub author: Option<String>,
    /// The site's pages that list micro entries (`[[micro.listings]]`).
    #[serde(default)]
    pub listings: Vec<MicroListing>,
}

/// A page of the site that lists micro entries, which micro claims: the page keeps its route and
/// front matter, its body (Liquid, for Jekyll) is not rendered, and `template` renders its
/// frame with the entries of `kinds` as `page.items` (`folia_micro::listing`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MicroListing {
    /// The page's source, relative to the site root (`blog/microblog.html`).
    pub source: String,
    /// The template that renders it (`micro_index.html`).
    pub template: String,
    /// The kinds it lists (`notes`, `likes`, `bookmarks`, `rsvps`).
    pub kinds: Vec<String>,
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
    /// The micro entries the slice builds.
    #[serde(default)]
    pub micro: Vec<String>,
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
    /// An author has a field set to nothing: leave its key out instead.
    #[error(
        "invalid configuration {0}: the author {1} has {2} set to an empty value; leave it out instead"
    )]
    EmptyAuthorField(String, String, String),
}

impl Config {
    /// Loads the configuration at `path`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read, or is not a valid configuration: one an author of
    /// which has a field set to an empty value is not.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let name = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(name.clone(), e))?;
        let config: Self = toml::from_str(&text).map_err(|e| ConfigError::Toml(name.clone(), e))?;
        for (key, author) in &config.authors {
            if let Some(field) = author.empty_fields().first() {
                return Err(ConfigError::EmptyAuthorField(
                    name,
                    key.clone(),
                    (*field).to_owned(),
                ));
            }
        }
        Ok(config)
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
    fn an_author_field_set_to_nothing_is_refused_on_load() {
        let dir = std::env::temp_dir().join(format!("folia-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("folia.toml");
        let base = "time-zone = \"UTC\"\n[roots]\nblog = \"b\"\nmicro = \"m\"\ngarden = \"g\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n[authors.gg]\nname = \"G\"\ndisplay-name = \"GG\"\navatar = \"a.webp\"\nweb = \"/\"\ndescription = \"x\"\n";
        std::fs::write(&path, format!("{base}github = \"gh\"\n")).unwrap();
        assert_eq!(
            Config::load(&path).unwrap().authors["gg"].github.as_deref(),
            Some("gh")
        );
        std::fs::write(&path, format!("{base}facebook = \"\"\n")).unwrap();
        let error = Config::load(&path).unwrap_err().to_string();
        assert!(
            error.ends_with(
                "the author gg has facebook set to an empty value; leave it out instead"
            ),
            "{error}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<Config>("time-zone = \"UTC\"\ntitle = \"x\"\n").is_err());
    }
}
