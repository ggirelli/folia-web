//! A site's authors, as its configuration names them and its pages show them.
//!
//! The fields are the site's `authors` of Jekyll's `_config.yml`, typed: each is shown by the
//! templates when it is set, and left out of the page when it is not. In TOML a field is unset
//! by leaving its key out; an empty value is refused, because Jekyll's Liquid showed an empty
//! string (it is true there) while a template would hide it.
//!
//! | Field | What it is |
//! |-|-|
//! | `name`, `display-name`, `nickname` | their name; the one a post's author box shows; a short one |
//! | `avatar` | their picture, relative to the site root |
//! | `web`, `url` | their web page; the URL the author page names as theirs (`itemprop="url"`), if any |
//! | `description`, `default-text` | a line about them; what their picture says on hover |
//! | `blog`, `garden` | where their blog and garden are |
//! | `covers` | the backgrounds of the author page, in order: see [`AuthorCover`] |
//! | `email`, `keybase`, `keyoxide`, `kofi`, `github`, `radicle`, `codeberg`, `hacker-news`, `lobste-rs`, `facebook`, `mastodon`, `bluesky`, `bluesky-did`, `twitter`, `reddit`, `substack`, `instagram`, `unsplash`, `linkedin`, `scholar`, `orcid` | their address or account on each, as the site's buttons put it in a link |

use serde::{Deserialize, Serialize};

/// An author, as pages show them.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all(deserialize = "kebab-case"))]
pub struct Author {
    /// Their name (`Gabriele Girelli`).
    pub name: String,
    /// The name shown in a post's author box.
    pub display_name: String,
    /// A short name (`GG`), which the author page's h-card gives.
    #[serde(default)]
    pub nickname: Option<String>,
    /// Their picture, relative to the site root.
    pub avatar: String,
    /// Their web page.
    pub web: String,
    /// The URL the author page names as theirs, if any.
    #[serde(default)]
    pub url: Option<String>,
    /// A line about them.
    pub description: String,
    /// What their picture on the author page says on hover.
    #[serde(default)]
    pub default_text: Option<String>,
    /// Where their blog is (`/blog`).
    #[serde(default)]
    pub blog: Option<String>,
    /// Where their garden is (`/dg`).
    #[serde(default)]
    pub garden: Option<String>,
    /// The backgrounds of the author page, in the order it shows them.
    #[serde(default)]
    pub covers: Vec<AuthorCover>,
    /// Their e-mail address.
    #[serde(default)]
    pub email: Option<String>,
    /// Their Keybase user.
    #[serde(default)]
    pub keybase: Option<String>,
    /// Their Keyoxide fingerprint.
    #[serde(default)]
    pub keyoxide: Option<String>,
    /// Their Ko-fi page.
    #[serde(default)]
    pub kofi: Option<String>,
    /// Their GitHub user.
    #[serde(default)]
    pub github: Option<String>,
    /// Their Radicle node ID.
    #[serde(default)]
    pub radicle: Option<String>,
    /// Their Codeberg user.
    #[serde(default)]
    pub codeberg: Option<String>,
    /// Their Hacker News user (`hacker-news`).
    #[serde(default)]
    pub hacker_news: Option<String>,
    /// Their Lobsters user (`lobste-rs`).
    #[serde(default)]
    pub lobste_rs: Option<String>,
    /// Their Facebook user.
    #[serde(default)]
    pub facebook: Option<String>,
    /// Their Mastodon address, server and user (`genomic.social/@ggirelli`).
    #[serde(default)]
    pub mastodon: Option<String>,
    /// Their Bluesky handle.
    #[serde(default)]
    pub bluesky: Option<String>,
    /// Their Bluesky DID, which the interactions crawler needs (`bluesky-did`).
    #[serde(default)]
    pub bluesky_did: Option<String>,
    /// Their X (Twitter) user.
    #[serde(default)]
    pub twitter: Option<String>,
    /// Their Reddit user.
    #[serde(default)]
    pub reddit: Option<String>,
    /// Their Substack publication.
    #[serde(default)]
    pub substack: Option<String>,
    /// Their Instagram user.
    #[serde(default)]
    pub instagram: Option<String>,
    /// Their Unsplash user.
    #[serde(default)]
    pub unsplash: Option<String>,
    /// Their `LinkedIn` user.
    #[serde(default)]
    pub linkedin: Option<String>,
    /// Their Google Scholar user ID.
    #[serde(default)]
    pub scholar: Option<String>,
    /// Their ORCID.
    #[serde(default)]
    pub orcid: Option<String>,
}

/// A background of the author page. Its fields are serialized in this order, and an unset one
/// is left out, as Jekyll's `jsonify` wrote the site's covers: see [`Author::covers_json`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorCover {
    /// The picture, as a URL path.
    pub path: String,
    /// A small picture shown while it loads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// What it shows, as HTML.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its CSS `background-position` (`50% 40%`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

impl Author {
    /// The covers as the author page's script reads them (`data-covers`): JSON as Jekyll's
    /// `jsonify | strip_newlines` wrote it, compact, with non-ASCII characters, `/`, `<` and `>`
    /// as they are, and a line break in a value escaped (`\n`), so the text has none to strip.
    /// It is meant to be written as it is, in a single-quoted attribute, as Jekyll did.
    ///
    /// # Panics
    ///
    /// Never: covers are strings, which always serialize.
    #[must_use]
    pub fn covers_json(&self) -> String {
        serde_json::to_string(&self.covers)
            .expect("covers are strings")
            .replace(['\n', '\r'], "")
    }

    /// The names of the fields set to an empty value, which the configuration refuses.
    #[must_use]
    pub fn empty_fields(&self) -> Vec<&'static str> {
        let required = [
            ("name", &self.name),
            ("display-name", &self.display_name),
            ("avatar", &self.avatar),
            ("web", &self.web),
            ("description", &self.description),
        ];
        let optional = [
            ("nickname", &self.nickname),
            ("url", &self.url),
            ("default-text", &self.default_text),
            ("blog", &self.blog),
            ("garden", &self.garden),
            ("email", &self.email),
            ("keybase", &self.keybase),
            ("keyoxide", &self.keyoxide),
            ("kofi", &self.kofi),
            ("github", &self.github),
            ("radicle", &self.radicle),
            ("codeberg", &self.codeberg),
            ("hacker-news", &self.hacker_news),
            ("lobste-rs", &self.lobste_rs),
            ("facebook", &self.facebook),
            ("mastodon", &self.mastodon),
            ("bluesky", &self.bluesky),
            ("bluesky-did", &self.bluesky_did),
            ("twitter", &self.twitter),
            ("reddit", &self.reddit),
            ("substack", &self.substack),
            ("instagram", &self.instagram),
            ("unsplash", &self.unsplash),
            ("linkedin", &self.linkedin),
            ("scholar", &self.scholar),
            ("orcid", &self.orcid),
        ];
        let mut empty: Vec<&'static str> = required
            .into_iter()
            .filter(|(_, value)| value.is_empty())
            .map(|(name, _)| name)
            .collect();
        empty.extend(
            optional
                .into_iter()
                .filter(|(_, value)| value.as_deref() == Some(""))
                .map(|(name, _)| name),
        );
        for cover in &self.covers {
            let fields = [
                (Some(&cover.path), "covers: path"),
                (cover.placeholder.as_ref(), "covers: placeholder"),
                (cover.description.as_ref(), "covers: description"),
                (cover.position.as_ref(), "covers: position"),
            ];
            empty.extend(
                fields
                    .into_iter()
                    .filter(|(value, _)| value.is_some_and(String::is_empty))
                    .map(|(_, name)| name),
            );
        }
        empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn author(toml: &str) -> Result<Author, toml::de::Error> {
        toml::from_str(&format!(
            "name = \"G\"\ndisplay-name = \"GG\"\navatar = \"a.webp\"\nweb = \"/\"\ndescription = \"x\"\n{toml}"
        ))
    }

    #[test]
    fn an_author_needs_only_their_names_picture_page_and_line() {
        let minimal = author("").unwrap();
        assert_eq!(minimal.nickname, None);
        assert_eq!(minimal.covers, []);
        assert_eq!(minimal.covers_json(), "[]");
        assert_eq!(minimal.empty_fields(), Vec::<&str>::new());
        assert!(toml::from_str::<Author>("name = \"G\"\n").is_err());
    }

    #[test]
    fn every_field_is_read_with_its_kebab_case_key() {
        let full = author(
            "nickname = \"N\"\nurl = \"https://e.org\"\ndefault-text = \"Hi\"\nblog = \"/blog\"\ngarden = \"/dg\"\n\
             email = \"e@x\"\nkeybase = \"kb\"\nkeyoxide = \"ko\"\nkofi = \"kf\"\ngithub = \"gh\"\nradicle = \"z6\"\n\
             codeberg = \"cb\"\nhacker-news = \"hn\"\nlobste-rs = \"lr\"\nfacebook = \"fb\"\nmastodon = \"m.s/@u\"\n\
             bluesky = \"b.s\"\nbluesky-did = \"did:plc:x\"\ntwitter = \"tw\"\nreddit = \"rd\"\nsubstack = \"ss\"\n\
             instagram = \"ig\"\nunsplash = \"us\"\nlinkedin = \"li\"\nscholar = \"sc\"\norcid = \"0000\"\n",
        )
        .unwrap();
        assert_eq!(full.hacker_news.as_deref(), Some("hn"));
        assert_eq!(full.lobste_rs.as_deref(), Some("lr"));
        assert_eq!(full.default_text.as_deref(), Some("Hi"));
        assert_eq!(full.orcid.as_deref(), Some("0000"));
        // Templates read the fields by their Rust names.
        let shown = serde_json::to_value(&full).unwrap();
        assert_eq!(shown["hacker_news"], "hn");
        assert_eq!(shown["display_name"], "GG");
        assert_eq!(shown["facebook"], "fb");
    }

    #[test]
    fn an_unset_field_is_none_and_an_unknown_one_is_refused() {
        let some = author("github = \"gh\"\n").unwrap();
        assert_eq!(some.facebook, None);
        assert_eq!(
            serde_json::to_value(&some).unwrap()["facebook"],
            serde_json::Value::Null
        );
        assert!(author("gravatar = \"x\"\n").is_err());
        assert!(
            author("hacker_news = \"x\"\n").is_err(),
            "keys are kebab-case"
        );
    }

    #[test]
    fn an_empty_field_is_named() {
        let empty = author(
            "facebook = \"\"\ngithub = \"gh\"\n[[covers]]\npath = \"/c.webp\"\nposition = \"\"\n",
        )
        .unwrap();
        assert_eq!(empty.empty_fields(), ["facebook", "covers: position"]);
        let mut no_name = author("").unwrap();
        no_name.name.clear();
        assert_eq!(no_name.empty_fields(), ["name"]);
    }

    #[test]
    fn covers_are_serialized_as_jekyll_s_jsonify_wrote_them() {
        let with_covers = author(
            "[[covers]]\npath = \"/a.webp\"\nplaceholder = \"/a-tiny.webp\"\n\
             description = \"A shot of <a href=\\\"https://x.org/?a=1&b=2\\\" target=\\\"_blank\\\">Mykines Hólmur</a> I took\\n\"\n\
             position = \"50% 50%\"\n\
             [[covers]]\npath = \"/b.webp\"\ndescription = \"Lake Mashū.\"\n",
        )
        .unwrap();
        assert_eq!(
            with_covers.covers_json(),
            r#"[{"path":"/a.webp","placeholder":"/a-tiny.webp","description":"A shot of <a href=\"https://x.org/?a=1&b=2\" target=\"_blank\">Mykines Hólmur</a> I took\n","position":"50% 50%"},{"path":"/b.webp","description":"Lake Mashū."}]"#
        );
        assert!(author("[[covers]]\npath = \"/a.webp\"\nalt = \"x\"\n").is_err());
    }
}
