//! Presentation metadata (plan v8, §5.2, tier 3): the front matter only templates read, resolved
//! once for every mode (plan v8, §12.2 step 2, decisions 1 and 3).
//!
//! A document says what it wants; Rust gives what it leaves out its default, and resolves which
//! presentation applies; templates map that to markup and display text. The defaults are those
//! of the site's `_config.yml`:
//!
//! | Key | Resolved as | When absent |
//! |-|-|-|
//! | `toc` | [`Toc`]: where it goes, its title, its entries; none when not enabled | none |
//! | `cover` | [`Cover`]: its picture, position, whether it shows, its Unsplash credit | after the title, shown, no picture |
//! | `hide_website_junk` | which parts of the frame to hide: `main_nav`, `title`, `cat_jumbotron`, `footer` | none hidden |
//! | `syndications` | [`syndications`]: `{name, url}` of each, in order; [`interactions`]: those the interactions crawler pulls from | none |
//!
//! The TOC's title is display text: none here means the template's own.

use serde_json::{Value, json};

use super::markdown::{TocEntry, analyze, toc};
use super::{Diagnostic, SourceDoc};

/// The levels of headings a table of contents shows, unless a document says otherwise.
const TOC_LEVELS: (u8, u8) = (2, 4);

/// The parts of the site's frame a document may hide.
const FRAME_PARTS: [&str; 4] = ["main_nav", "title", "cat_jumbotron", "footer"];

/// Where a table of contents goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TocPlacement {
    /// Beside the article, on the left.
    Left,
    /// Beside the article, on the right.
    Right,
    /// Above the article.
    Top,
}

impl TocPlacement {
    /// Its name for templates (`left`, `right`, `top`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
        }
    }
}

/// A document's table of contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toc {
    /// Where it goes.
    pub placement: TocPlacement,
    /// Its title, when the document gives one.
    pub title: Option<String>,
    /// Its entries, from the document's headings.
    pub entries: Vec<TocEntry>,
}

impl Toc {
    /// The table of contents as templates see it: `{"placement", "title", "entries"}`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "placement": self.placement.as_str(),
            "title": self.title,
            "entries": self.entries.iter().map(TocEntry::to_json).collect::<Vec<_>>(),
        })
    }
}

/// A document's cover picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    /// The picture, if any.
    pub path: Option<String>,
    /// Where it goes: `after_title` or `behind`.
    pub position: String,
    /// Whether it shows.
    pub show: bool,
    /// The Unsplash user to credit, if any.
    pub unsplash: Option<String>,
}

impl Cover {
    /// The cover as templates see it: `{"path", "position", "show", "unsplash"}`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "path": self.path,
            "position": self.position,
            "show": self.show,
            "unsplash": self.unsplash,
        })
    }
}

/// The table of contents of `doc`, or none when it does not enable one.
///
/// # Errors
///
/// Fails when `toc` is not a mapping, or its `position` or levels are not ones Jekyll's
/// templates know.
pub fn table_of_contents(doc: &SourceDoc) -> Result<Option<Toc>, Diagnostic> {
    let problem = |message: &str| Diagnostic::in_doc(&doc.id, format!("toc: {message}"));
    let settings = match doc.front.json("toc") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(settings)) => settings,
        Some(_) => return Err(problem("should be a mapping")),
    };
    if settings.get("enabled") != Some(&Value::Bool(true)) {
        return Ok(None);
    }
    let placement = match settings.get("position").and_then(Value::as_str) {
        None | Some("top") => TocPlacement::Top,
        Some("left") => TocPlacement::Left,
        Some("right") => TocPlacement::Right,
        Some(other) => return Err(problem(&format!("unknown position {other:?}"))),
    };
    let level = |key: &str, default: u8| match settings.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .and_then(|level| u8::try_from(level).ok())
            .filter(|level| (1..=6).contains(level))
            .ok_or_else(|| problem(&format!("{key} should be a heading level, 1 to 6"))),
    };
    let (min_level, max_level) = (
        level("min_level", TOC_LEVELS.0)?,
        level("max_level", TOC_LEVELS.1)?,
    );
    Ok(Some(Toc {
        placement,
        title: settings
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_owned),
        entries: toc(&analyze(&doc.body, None), min_level, max_level),
    }))
}

/// The cover of `doc`.
///
/// # Errors
///
/// Fails when `cover` is not a mapping.
pub fn cover(doc: &SourceDoc) -> Result<Cover, Diagnostic> {
    let settings = match doc.front.json("cover") {
        None | Some(Value::Null) => serde_json::Map::new(),
        Some(Value::Object(settings)) => settings,
        Some(_) => return Err(Diagnostic::in_doc(&doc.id, "cover: should be a mapping")),
    };
    let text = |key: &str| settings.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(Cover {
        path: text("path"),
        position: text("position").unwrap_or_else(|| "after_title".to_owned()),
        show: settings
            .get("show")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        unsplash: text("unsplash"),
    })
}

/// The front matter `lang` of `doc`, as written (text, or a list of texts like `[italian]`), or
/// nothing without one, so that a template finds `page.lang` undefined.
#[must_use]
pub fn lang(doc: &SourceDoc) -> Option<Value> {
    doc.front.json("lang")
}

/// Which parts of the site's frame `doc` hides (`hide_website_junk`), each `true` or `false`.
/// Other keys of the mapping are ignored, as the site's templates ignored them.
#[must_use]
pub fn hide_website_junk(doc: &SourceDoc) -> Value {
    let settings = doc.front.json("hide_website_junk");
    FRAME_PARTS
        .iter()
        .map(|part| {
            let hidden = settings
                .as_ref()
                .and_then(|settings| settings.get(*part))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            ((*part).to_owned(), Value::Bool(hidden))
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// Syndications whose links the site's template turns into an interactions crawler: each with
/// its key in the document's `interactions`.
const INTERACTIONS: [(&str, &str); 3] = [
    ("mastodon", "mastodon"),
    ("bluesky", "bluesky"),
    ("hacker-news", "hacker_news"),
];

/// The URLs of the document's syndications that offer to pull interactions, by
/// `INTERACTIONS`' keys (`{mastodon, bluesky, hacker_news}`, each a URL or none); none when it
/// has none of them, as the site's template shows no crawler then.
#[must_use]
pub fn interactions(doc: &SourceDoc) -> Value {
    let syndications = doc.front.entries("syndications").unwrap_or_default();
    let url = |name: &str| {
        syndications
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, url)| url.clone())
    };
    if INTERACTIONS.iter().all(|(name, _)| url(name).is_none()) {
        return Value::Null;
    }
    INTERACTIONS
        .iter()
        .map(|(name, key)| ((*key).to_owned(), url(name).unwrap_or(Value::Null)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The syndications as `{name, url}`, in their front matter's order: a JSON mapping would sort
/// them. Without a mapping of its own, a document has its default's, which is none.
#[must_use]
pub fn syndications(doc: &SourceDoc) -> Value {
    doc.front
        .entries("syndications")
        .unwrap_or_default()
        .into_iter()
        .map(|(name, url)| json!({"name": name, "url": url}))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{DocId, FrontMatter, Mode};
    use super::*;

    fn doc(front: &str, body: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new("_posts/2021/2021-08-08-a.md"),
            mode: Mode::Blog,
            front: FrontMatter::parse(front).unwrap(),
            body: body.to_owned(),
        }
    }

    #[test]
    fn a_toc_is_none_unless_enabled() {
        assert_eq!(table_of_contents(&doc("", "## A\n")).unwrap(), None);
        let disabled = doc("toc:\n  enabled: false\n  position: left\n", "## A\n");
        assert_eq!(table_of_contents(&disabled).unwrap(), None);
    }

    #[test]
    fn an_enabled_toc_has_its_placement_and_the_default_levels() {
        let left = doc(
            "toc:\n  enabled: true\n  position: left\n",
            "# T\n\n## A\n\n### B\n\n##### C\n",
        );
        let toc = table_of_contents(&left).unwrap().unwrap();
        assert_eq!(toc.placement, TocPlacement::Left);
        assert_eq!(toc.title, None);
        assert_eq!(
            toc.to_json(),
            json!({"placement": "left", "title": null, "entries": [
                {"id": "a", "html": "A", "children": [{"id": "b", "html": "B", "children": []}]},
            ]})
        );
        let top = doc("toc:\n  enabled: true\n  title: In short\n", "## A\n");
        let toc = table_of_contents(&top).unwrap().unwrap();
        assert_eq!(
            (toc.placement, toc.title.as_deref()),
            (TocPlacement::Top, Some("In short"))
        );
    }

    #[test]
    fn a_toc_jekyll_could_not_place_is_a_problem() {
        let problem = |front: &str| table_of_contents(&doc(front, "")).unwrap_err().to_string();
        assert_eq!(
            problem("toc:\n  enabled: true\n  position: middle\n"),
            "_posts/2021/2021-08-08-a.md: toc: unknown position \"middle\""
        );
        assert!(problem("toc:\n  enabled: true\n  max_level: 9\n").contains("max_level"));
        assert!(problem("toc: yes\n").contains("should be a mapping"));
    }

    #[test]
    fn a_cover_keeps_what_it_gives_and_defaults_the_rest() {
        assert_eq!(
            cover(&doc("", "")).unwrap().to_json(),
            json!({"path": null, "position": "after_title", "show": true, "unsplash": null})
        );
        let behind = doc(
            "cover:\n  path: /c.webp\n  position: behind\n  unsplash: lunarts\n",
            "",
        );
        assert_eq!(
            cover(&behind).unwrap(),
            Cover {
                path: Some("/c.webp".into()),
                position: "behind".into(),
                show: true,
                unsplash: Some("lunarts".into()),
            }
        );
    }

    #[test]
    fn every_part_of_the_frame_is_hidden_or_not() {
        assert_eq!(
            hide_website_junk(&doc(
                "hide_website_junk:\n  cat_jumbotron: true\n  extra: 1\n",
                ""
            )),
            json!({"main_nav": false, "title": false, "cat_jumbotron": true, "footer": false})
        );
        assert_eq!(hide_website_junk(&doc("", ""))["footer"], false);
    }
}
