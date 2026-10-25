//! The renderer (plan v6, §6.4 and §6.5): each page through its template, with `MiniJinja`.
//!
//! Templates are files below the site's template directory, loaded at runtime. A template name
//! is a relative path inside that directory: an absolute name, or one with a `..`, `.` or empty
//! segment, is rejected, by [`Renderer::check`] and by the loader alike. Undefined values are
//! errors, even in a test, so a misspelled key cannot silently render as nothing.
//!
//! A template sees:
//!
//! | Name | Value |
//! |-|-|
//! | `page` | the page's context, from its mode |
//! | `content` | the body HTML of the page's entry, to be shown with `{{ content \| safe }}` |
//! | `site` | the site index, from the site builder |
//!
//! Besides `MiniJinja`'s own filters, `date(format)` writes a time that a mode gave as
//! `2026-01-04T13:36:24+01:00` in a `strftime` format, in its own offset.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use folia_core::content::Diagnostic;
use folia_core::publish::PageSpec;
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Serde;
use minijinja::{Environment, Error, ErrorKind, UndefinedBehavior, context};

/// Renders pages with the templates of one directory.
pub struct Renderer {
    env: Environment<'static>,
}

impl Renderer {
    /// A renderer for the templates below `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        // Block tags on lines of their own leave no blank lines behind. Only custom delimiters
        // can make the builder fail, and these are the default ones.
        if let Ok(syntax) = SyntaxConfig::builder()
            .trim_blocks(true)
            .lstrip_blocks(true)
            .build()
        {
            env.set_syntax(syntax);
        }
        let dir = dir.to_path_buf();
        env.set_loader(move |name| load(&dir, name));
        env.add_filter("date", date);
        Self { env }
    }

    /// Checks, before anything renders, that every page's template has a valid name, exists and
    /// compiles. Templates they extend or include are checked as they render.
    ///
    /// # Errors
    ///
    /// Returns one diagnostic naming every template that fails, why, and the pages that use it.
    pub fn check(&self, pages: &[PageSpec]) -> Result<(), Diagnostic> {
        let mut users: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for page in pages {
            users
                .entry(&page.template)
                .or_default()
                .insert(page.route.url());
        }
        let mut failing = String::new();
        for (name, urls) in &users {
            let problem = match invalid_name(name) {
                Some(reason) => Some(reason.to_owned()),
                None => self
                    .env
                    .get_template(name)
                    .err()
                    .map(|error| match error.kind() {
                        ErrorKind::TemplateNotFound => "missing".to_owned(),
                        _ => describe(&error),
                    }),
            };
            if let Some(problem) = problem {
                let urls: Vec<&str> = urls.iter().copied().collect();
                let _ = write!(
                    failing,
                    "\n  {name}: {problem}; used by {}",
                    urls.join(", ")
                );
            }
        }
        if failing.is_empty() {
            Ok(())
        } else {
            Err(Diagnostic::site(format!(
                "these templates cannot be used:{failing}"
            )))
        }
    }

    /// Renders `page`, showing `content` as its body and `site` as the site index.
    ///
    /// # Errors
    ///
    /// Fails when its template, or one it extends or includes, cannot be loaded or rendered.
    pub fn render(
        &self,
        page: &PageSpec,
        content: &str,
        site: &serde_json::Value,
    ) -> Result<String, Diagnostic> {
        let fail = |error: &Error| {
            Diagnostic::site(format!(
                "{} cannot be rendered with {}: {}",
                page.route,
                page.template,
                describe(error)
            ))
        };
        let template = self
            .env
            .get_template(&page.template)
            .map_err(|e| fail(&e))?;
        template
            .render(context! {
                page => Serde(&page.context),
                content => content,
                site => Serde(site),
            })
            .map_err(|e| fail(&e))
    }
}

/// Why a template name is not a path inside the template directory, if it is not.
fn invalid_name(name: &str) -> Option<&'static str> {
    if name.starts_with('/') || name.contains('\\') || Path::new(name).is_absolute() {
        Some("a template name must be a relative path")
    } else if name
        .split('/')
        .any(|segment| matches!(segment, "" | "." | ".."))
    {
        Some("a template name must not have an empty, `.` or `..` segment")
    } else {
        None
    }
}

fn load(dir: &Path, name: &str) -> Result<Option<String>, Error> {
    if let Some(reason) = invalid_name(name) {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("{name}: {reason}"),
        ));
    }
    let path: PathBuf = dir.join(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("cannot read {}", path.display()),
        )
        .with_source(error)),
    }
}

/// A `MiniJinja` error, with where in which template it happened.
fn describe(error: &Error) -> String {
    let mut out = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        let _ = write!(out, ": {cause}");
        source = cause.source();
    }
    out
}

/// The `date` filter: a time written as `%Y-%m-%dT%H:%M:%S%:z`, in another `strftime` format.
fn date(value: &str, format: &str) -> Result<String, Error> {
    let invalid = |reason: &dyn std::fmt::Display| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("date: {value:?}: {reason}"),
        )
    };
    let zoned = jiff::fmt::strtime::parse("%Y-%m-%dT%H:%M:%S%:z", value)
        .and_then(|time| time.to_zoned())
        .map_err(|e| invalid(&e))?;
    jiff::fmt::strtime::format(format, &zoned).map_err(|e| invalid(&e))
}

#[cfg(test)]
mod tests;
