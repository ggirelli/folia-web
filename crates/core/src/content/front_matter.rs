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

    /// The entries of a mapping, in the order they are written, with their values as JSON;
    /// `None` when the key is absent or not a mapping.
    #[must_use]
    pub fn entries(&self, key: &str) -> Option<Vec<(String, serde_json::Value)>> {
        match self.get(key)? {
            Yaml::Hash(mapping) => Some(
                mapping
                    .iter()
                    .map(|(key, value)| (json_key(key), to_json(value)))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// The keys that are strings, in the order they are written.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().filter_map(Yaml::as_str)
    }

    /// The value of a key as JSON, for templates: keys become strings, and a number that JSON
    /// cannot hold becomes null. A JSON mapping sorts its keys; [`FrontMatter::entries`] keeps
    /// their order.
    #[must_use]
    pub fn json(&self, key: &str) -> Option<serde_json::Value> {
        self.get(key).map(to_json)
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

fn to_json(value: &Yaml) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Yaml::String(text) => Value::String(text.clone()),
        Yaml::Integer(number) => Value::from(*number),
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number),
        Yaml::Boolean(flag) => Value::Bool(*flag),
        Yaml::Array(items) => Value::Array(items.iter().map(to_json).collect()),
        Yaml::Hash(mapping) => Value::Object(
            mapping
                .iter()
                .map(|(key, value)| (json_key(key), to_json(value)))
                .collect(),
        ),
        Yaml::Null | Yaml::Alias(_) | Yaml::BadValue => Value::Null,
    }
}

fn json_key(key: &Yaml) -> String {
    match key {
        Yaml::String(text) | Yaml::Real(text) => text.clone(),
        Yaml::Integer(number) => number.to_string(),
        Yaml::Boolean(flag) => flag.to_string(),
        _ => String::new(),
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
    fn values_read_as_json_keep_their_shape() {
        let front = FrontMatter::parse(
            "toc:\n  enabled: true\n  min_level: 2\nlist: [a, 1.5]\nlast_modified_at: 2021-08-24\n",
        )
        .unwrap();
        assert_eq!(
            front.json("toc"),
            Some(serde_json::json!({"enabled": true, "min_level": 2}))
        );
        assert_eq!(front.json("list"), Some(serde_json::json!(["a", 1.5])));
        assert_eq!(
            front.json("last_modified_at"),
            Some(serde_json::json!("2021-08-24"))
        );
        assert_eq!(front.json("absent"), None);
        assert_eq!(
            front.entries("toc").unwrap(),
            [
                ("enabled".to_owned(), serde_json::json!(true)),
                ("min_level".to_owned(), serde_json::json!(2))
            ]
        );
        assert_eq!(front.entries("list"), None);
        assert_eq!(
            front.keys().collect::<Vec<_>>(),
            ["toc", "list", "last_modified_at"]
        );
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
