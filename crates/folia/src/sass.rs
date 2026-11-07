//! Stylesheets compiled from Sass with Dart Sass (plan v12, Q7): an external program, pinned to
//! the version Jekyll's `sass-embedded` used, so that the CSS and its source map are Jekyll's.
//!
//! Folia runs `sass` from `PATH`, or the program `FOLIA_SASS` names, and refuses any other
//! version than [`DART_SASS_VERSION`]; there is no fallback compiler. Each stylesheet is compiled
//! from its body (the source without its front matter, as Jekyll reads it), written to a
//! temporary directory at the stylesheet's own path, with the site's Sass directory as its load
//! path. The source map is then rewritten as `jekyll-sass-converter` writes it: sources relative
//! to the stylesheet's output directory, the stylesheet itself by its file name, its sources'
//! contents embedded, and its keys in that converter's order.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::config::SassConfig;

/// The Dart Sass version Folia requires: the one Jekyll's `sass-embedded` 1.100.0 runs.
pub const DART_SASS_VERSION: &str = "1.100.0";

/// Where Dart Sass can be installed from.
const INSTALL: &str = "https://github.com/sass/dart-sass/releases/tag/1.100.0";

/// A stylesheet compiled from Sass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compiled {
    /// The CSS, ending with its `sourceMappingURL` comment.
    pub css: String,
    /// Its source map.
    pub map: String,
}

/// The Dart Sass program, its version checked.
#[derive(Clone, Debug)]
pub struct Sass {
    program: OsString,
}

impl Sass {
    /// Finds Dart Sass: the program `FOLIA_SASS` names, else `sass` on `PATH`; and checks its
    /// version.
    ///
    /// # Errors
    ///
    /// Fails, saying how to install it, when the program cannot be run or is not
    /// [`DART_SASS_VERSION`].
    pub fn find() -> Result<Self, String> {
        let program = std::env::var_os("FOLIA_SASS").unwrap_or_else(|| "sass".into());
        Self::with_program(program)
    }

    /// Uses `program` as Dart Sass, once its version is checked.
    ///
    /// # Errors
    ///
    /// As [`Sass::find`].
    pub fn with_program(program: OsString) -> Result<Self, String> {
        let name = program.to_string_lossy().into_owned();
        let output = Command::new(&program)
            .arg("--version")
            .output()
            .map_err(|e| {
                format!(
                    "Dart Sass {DART_SASS_VERSION} is needed to compile the site's stylesheets, \
                 and `{name}` cannot be run ({e}). Install it from {INSTALL} and put `sass` on \
                 PATH, or set FOLIA_SASS to the program."
                )
            })?;
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !output.status.success() || version != DART_SASS_VERSION {
            return Err(format!(
                "Dart Sass {DART_SASS_VERSION} is needed to compile the site's stylesheets, \
                 and `{name}` is version {version:?}. Install {DART_SASS_VERSION} from {INSTALL}, \
                 or set FOLIA_SASS to it."
            ));
        }
        Ok(Self { program })
    }

    /// Compiles the stylesheet `id` (its path in the site at `root`), whose body is `body`, with
    /// the site's Sass settings.
    ///
    /// # Errors
    ///
    /// Fails with Dart Sass's message when it cannot compile the stylesheet, or when its output
    /// cannot be read.
    pub fn compile(
        &self,
        root: &Path,
        settings: &SassConfig,
        id: &str,
        body: &str,
    ) -> Result<Compiled, String> {
        let work = std::env::temp_dir().join(format!(
            "folia-sass-{}-{}",
            std::process::id(),
            id.replace('/', "_")
        ));
        let _ = std::fs::remove_dir_all(&work);
        let result = self.compile_in(&work, root, settings, id, body);
        let _ = std::fs::remove_dir_all(&work);
        result
    }

    fn compile_in(
        &self,
        work: &Path,
        root: &Path,
        settings: &SassConfig,
        id: &str,
        body: &str,
    ) -> Result<Compiled, String> {
        let source = work.join(id);
        let css_name = css_name(id);
        let out = source.with_file_name(&css_name);
        std::fs::create_dir_all(source.parent().unwrap_or(work)).map_err(|e| e.to_string())?;
        std::fs::write(&source, body).map_err(|e| e.to_string())?;
        let load_path = absolute(&root.join(&settings.dir))?;
        let mut arg = OsString::from(&source);
        arg.push(":");
        arg.push(&out);
        let output = Command::new(&self.program)
            .arg(format!("--style={}", settings.style))
            .arg("--load-path")
            .arg(&load_path)
            .args([
                "--embed-sources",
                "--source-map-urls=absolute",
                "--quiet",
                "--no-error-css",
            ])
            .arg(arg)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Dart Sass cannot compile it: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let css = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let map = std::fs::read_to_string(out.with_file_name(format!("{css_name}.map")))
            .map_err(|e| e.to_string())?;
        let out_dir = absolute(&root.join(id))?
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let source = std::fs::canonicalize(&source).map_err(|e| e.to_string())?;
        Ok(Compiled {
            css: css.strip_suffix('\n').unwrap_or(&css).to_owned(),
            map: jekyll_map(&map, &source, &out_dir, &css_name)?,
        })
    }
}

/// The CSS file a stylesheet `id` compiles to: its file name with `.css` for `.scss` or `.sass`.
fn css_name(id: &str) -> String {
    let name = id.rsplit('/').next().unwrap_or(id);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    format!("{stem}.css")
}

/// `path`, absolute and without `.` or `..`, its links resolved when it exists.
fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).or_else(|_| std::path::absolute(path).map_err(|e| e.to_string()))
}

/// Dart Sass's source map `map`, rewritten as `jekyll-sass-converter` writes it: each source a
/// path relative to `out_dir`, the stylesheet compiled (`main`, in its temporary directory) by
/// its file name, and the keys `version`, `sourceRoot`, `sources`, `names`, `mappings`,
/// `sourcesContent` and `file` (`css_name`), in that order, compact, as UTF-8.
fn jekyll_map(map: &str, main: &Path, out_dir: &Path, css_name: &str) -> Result<String, String> {
    let map: Value = serde_json::from_str(map).map_err(|e| format!("its source map: {e}"))?;
    let sources: Vec<String> = map["sources"]
        .as_array()
        .ok_or("its source map has no sources")?
        .iter()
        .map(|url| {
            let url = url.as_str().ok_or("a source is not a string")?;
            let path =
                file_url_path(url).ok_or_else(|| format!("a source is not a file: {url}"))?;
            Ok(if path == main {
                main.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                relative(&path, out_dir)
            })
        })
        .collect::<Result<_, String>>()?;
    let field = |key: &str| serde_json::to_string(&map[key]).map_err(|e| e.to_string());
    Ok(format!(
        "{{\"version\":{},\"sourceRoot\":\"\",\"sources\":{},\"names\":{},\"mappings\":{},\"sourcesContent\":{},\"file\":{}}}",
        field("version")?,
        serde_json::to_string(&sources).map_err(|e| e.to_string())?,
        field("names")?,
        field("mappings")?,
        field("sourcesContent")?,
        serde_json::to_string(css_name).map_err(|e| e.to_string())?,
    ))
}

/// The path of a `file:` URL, percent-decoded.
fn file_url_path(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("file://")?;
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = path
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            decoded.push(byte);
            i += 3;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(decoded).ok().map(PathBuf::from)
}

/// `path` relative to the directory `from`, both absolute, with `/` separators.
fn relative(path: &Path, from: &Path) -> String {
    let path: Vec<Component> = path.components().collect();
    let from: Vec<Component> = from.components().collect();
    let common = path.iter().zip(&from).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_owned(); from.len() - common];
    parts.extend(
        path[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

#[cfg(test)]
mod tests;
