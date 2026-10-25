//! YAML front matter: telling it apart, splitting it from the body, and reading its values with
//! Jekyll's conventions.
//!
//! The YAML is read as YAML 1.2, where Jekyll's Ruby reads YAML 1.1: a bare `no` or `off` is a
//! string here, not `false`. The site's front matter uses neither.

use yaml_rust2::yaml::{Hash, Yaml};
use yaml_rust2::{ScanError, YamlLoader};

/// Why front matter cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum FrontMatterError {
    /// The YAML does not parse.
    #[error("front matter is not valid YAML: {0}")]
    Yaml(#[from] ScanError),
    /// The YAML is not a mapping of keys to values.
    #[error("front matter is not a mapping")]
    NotAMapping,
    /// A key holds a value of the wrong shape.
    #[error("front matter key {key:?} {expected}")]
    Shape {
        /// The key.
        key: String,
        /// What the value should have been.
        expected: &'static str,
    },
}

/// Whether a file starts with a front matter fence: Jekyll's test for a page rather than a
/// static file. The first line must be `---`, optionally followed by whitespace.
#[must_use]
pub fn has_front_matter(bytes: &[u8]) -> bool {
    let Some(end) = bytes.iter().position(|&b| b == b'\n') else {
        return false;
    };
    let line = &bytes[..end];
    line.starts_with(b"---") && line[3..].iter().all(u8::is_ascii_whitespace)
}

/// Splits text into its front matter YAML and its body. Returns `None` when the text does not
/// open with a front matter block closed by a `---` or `...` line.
///
/// As in Jekyll (`^(---|\.\.\.)\s*$\n?`), the body starts after the blank lines that follow
/// the closing line; the first line with content keeps its indentation.
#[must_use]
pub fn split_front_matter(text: &str) -> Option<(&str, &str)> {
    let first_end = text.find('\n')?;
    if text[..first_end].trim_end() != "---" {
        return None;
    }
    let mut start = first_end + 1;
    while start <= text.len() {
        let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let line = text[start..end].trim_end();
        if line == "---" || line == "..." {
            let after = &text[start + 3..];
            let blank = after.len() - after.trim_start().len();
            let body = match after[..blank].rfind('\n') {
                Some(i) if blank < after.len() => start + 3 + i + 1,
                _ => start + 3 + blank,
            };
            return Some((&text[first_end + 1..start], &text[body..]));
        }
        if end == text.len() {
            return None;
        }
        start = end + 1;
    }
    None
}

/// A document's front matter: the YAML mapping, read as Jekyll reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontMatter(Hash);

impl FrontMatter {
    /// Parses front matter YAML. Empty YAML is empty front matter.
    ///
    /// # Errors
    ///
    /// Fails when the YAML does not parse, or is not a mapping.
    pub fn parse(yaml: &str) -> Result<Self, FrontMatterError> {
        match YamlLoader::load_from_str(yaml)?.into_iter().next() {
            None | Some(Yaml::Null) => Ok(Self::default()),
            Some(Yaml::Hash(mapping)) => Ok(Self(mapping)),
            Some(_) => Err(FrontMatterError::NotAMapping),
        }
    }

    /// The raw value of a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        self.0.get(&Yaml::String(key.to_owned()))
    }

    /// A scalar value as a string; `None` when the key is absent or null.
    ///
    /// # Errors
    ///
    /// Fails when the value is a list or a mapping.
    pub fn string(&self, key: &str) -> Result<Option<String>, FrontMatterError> {
        self.get(key).map_or(Ok(None), |value| scalar(key, value))
    }

    /// Whether a key is set to something Ruby considers true: anything but null and `false`.
    #[must_use]
    pub fn truthy(&self, key: &str) -> bool {
        !matches!(
            self.get(key),
            None | Some(Yaml::Null | Yaml::Boolean(false))
        )
    }

    /// Whether the document is published: only `published: false` unpublishes it.
    #[must_use]
    pub fn published(&self) -> bool {
        !matches!(self.get("published"), Some(Yaml::Boolean(false)))
    }

    /// A value that may be one string or a list of them, such as `redirect_from`.
    ///
    /// # Errors
    ///
    /// Fails when the value, or one of its items, is a mapping or a nested list.
    pub fn strings(&self, key: &str) -> Result<Vec<String>, FrontMatterError> {
        match self.get(key) {
            None | Some(Yaml::Null) => Ok(Vec::new()),
            Some(Yaml::Array(items)) => list(key, items),
            Some(value) => Ok(scalar(key, value)?.into_iter().collect()),
        }
    }

    /// A list given as a list, or as one string of words separated by whitespace (`tags: a b`).
    ///
    /// # Errors
    ///
    /// Fails when the value, or one of its items, is a mapping or a nested list.
    pub fn words(&self, key: &str) -> Result<Vec<String>, FrontMatterError> {
        match self.get(key) {
            Some(Yaml::String(text)) => Ok(text.split_whitespace().map(str::to_owned).collect()),
            _ => self.strings(key),
        }
    }

    /// A list given by a singular or a plural key, as Jekyll reads `tag`/`tags`: the singular
    /// key wins as it is, and the plural one is read as [`FrontMatter::words`].
    ///
    /// # Errors
    ///
    /// Fails when the value, or one of its items, is a mapping or a nested list.
    pub fn pluralized(
        &self,
        singular: &str,
        plural: &str,
    ) -> Result<Vec<String>, FrontMatterError> {
        if self.get(singular).is_some() {
            self.strings(singular)
        } else {
            self.words(plural)
        }
    }
}

fn scalar(key: &str, value: &Yaml) -> Result<Option<String>, FrontMatterError> {
    match value {
        Yaml::Null => Ok(None),
        Yaml::String(text) | Yaml::Real(text) => Ok(Some(text.clone())),
        Yaml::Boolean(flag) => Ok(Some(flag.to_string())),
        Yaml::Integer(number) => Ok(Some(number.to_string())),
        _ => Err(FrontMatterError::Shape {
            key: key.to_owned(),
            expected: "should be a single value",
        }),
    }
}

fn list(key: &str, items: &[Yaml]) -> Result<Vec<String>, FrontMatterError> {
    let mut strings = Vec::with_capacity(items.len());
    for item in items {
        if matches!(item, Yaml::Array(_) | Yaml::Hash(_)) {
            return Err(FrontMatterError::Shape {
                key: key.to_owned(),
                expected: "should list single values",
            });
        }
        strings.extend(scalar(key, item)?);
    }
    Ok(strings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_line_marks_front_matter() {
        assert!(has_front_matter(b"---\ntitle: x\n---\n"));
        assert!(has_front_matter(b"---  \r\nx"));
        assert!(!has_front_matter(b"--- title\n"));
        assert!(!has_front_matter(b"---"));
        assert!(!has_front_matter(b"body\n"));
    }

    #[test]
    fn front_matter_splits_from_the_body() {
        assert_eq!(
            split_front_matter("---\na: 1\n---\nbody\n"),
            Some(("a: 1\n", "body\n"))
        );
        assert_eq!(split_front_matter("---\na: 1\n...\n"), Some(("a: 1\n", "")));
        assert_eq!(split_front_matter("---\n---\nbody"), Some(("", "body")));
        assert_eq!(
            split_front_matter("---\na: 1\n---  \n\n \n  body\n"),
            Some(("a: 1\n", "  body\n"))
        );
        assert_eq!(split_front_matter("---\na: 1\n"), None);
        assert_eq!(split_front_matter("body"), None);
    }

    #[test]
    fn values_read_with_jekyll_conventions() {
        let front = FrontMatter::parse(
            "tags: indieweb LLM\ncategory: code\nhidden: false\nredirect_from: /x\n",
        )
        .unwrap();
        assert_eq!(
            front.pluralized("tag", "tags").unwrap(),
            ["indieweb", "LLM"]
        );
        assert_eq!(
            front.pluralized("category", "categories").unwrap(),
            ["code"]
        );
        assert!(!front.truthy("hidden"));
        assert_eq!(front.strings("redirect_from").unwrap(), ["/x"]);
        assert_eq!(front.words("tags").unwrap(), ["indieweb", "LLM"]);
        assert!(front.published());
    }

    #[test]
    fn only_published_false_unpublishes() {
        assert!(!FrontMatter::parse("published: false").unwrap().published());
        assert!(
            FrontMatter::parse("published: no thanks")
                .unwrap()
                .published()
        );
    }

    #[test]
    fn empty_front_matter_is_empty() {
        assert_eq!(FrontMatter::parse("").unwrap(), FrontMatter::default());
        assert!(matches!(
            FrontMatter::parse("- a"),
            Err(FrontMatterError::NotAMapping)
        ));
    }
}
