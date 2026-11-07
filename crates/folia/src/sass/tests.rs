use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::config::SassStyle;

/// A throwaway directory, removed when dropped.
struct Dir(PathBuf);

impl Dir {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("folia-sass-test-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, path: &str, text: &str) -> PathBuf {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A program that prints `version` when asked for it.
    fn fake_sass(&self, version: &str) -> OsString {
        let path = self.write("fake-sass", &format!("#!/bin/sh\necho {version}\n"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.into()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_stylesheet_compiles_to_its_name_with_css() {
    assert_eq!(css_name("assets/css/main.scss"), "main.css");
    assert_eq!(css_name("style.sass"), "style.css");
    assert_eq!(css_name("assets/css/dg.styles.scss"), "dg.styles.css");
}

#[test]
fn a_source_is_named_relative_to_the_stylesheets_output_directory() {
    let out = Path::new("/site/assets/css");
    assert_eq!(
        relative(Path::new("/site/_sass/bootstrap/_root.scss"), out),
        "../../_sass/bootstrap/_root.scss"
    );
    assert_eq!(
        relative(Path::new("/site/assets/css/x.scss"), out),
        "x.scss"
    );
    assert_eq!(
        relative(Path::new("/elsewhere/y.scss"), out),
        "../../../elsewhere/y.scss"
    );
}

#[test]
fn a_file_url_is_percent_decoded() {
    assert_eq!(
        file_url_path("file:///a%20b/c%C3%A9.scss"),
        Some(PathBuf::from("/a b/cé.scss"))
    );
    assert_eq!(file_url_path("https://example.com/x.scss"), None);
}

#[test]
fn the_source_map_is_rewritten_as_jekyll_writes_it() {
    let dart = r#"{"version":3,"sourceRoot":"","sources":["file:///site/_sass/_a.scss","file:///tmp/work/assets/css/main.scss"],"names":[],"mappings":"AAAA","file":"main.css","sourcesContent":["a{b:c}","@use \"a\"; // é"]}"#;
    let map = jekyll_map(
        dart,
        Path::new("/tmp/work/assets/css/main.scss"),
        Path::new("/site/assets/css"),
        "main.css",
    )
    .unwrap();
    assert_eq!(
        map,
        r#"{"version":3,"sourceRoot":"","sources":["../../_sass/_a.scss","main.scss"],"names":[],"mappings":"AAAA","sourcesContent":["a{b:c}","@use \"a\"; // é"],"file":"main.css"}"#
    );
}

#[test]
fn a_missing_dart_sass_says_how_to_install_it() {
    let error = Sass::with_program("/nonexistent/folia-sass".into()).unwrap_err();
    assert!(error.contains("Dart Sass 1.100.0 is needed"), "{error}");
    assert!(error.contains("FOLIA_SASS"), "{error}");
    assert!(error.contains(INSTALL), "{error}");
}

#[test]
fn another_version_of_dart_sass_is_refused() {
    // `echo`, whatever it prints for `--version`, is not Dart Sass 1.100.0; no test writes it.
    let error = Sass::with_program("echo".into()).unwrap_err();
    assert!(error.contains("`echo` is version"), "{error}");
    assert!(error.contains("Install 1.100.0"), "{error}");
}

#[test]
fn the_required_version_is_accepted() {
    let dir = Dir::new("version");
    let program = dir.fake_sass(DART_SASS_VERSION);
    // A program just written can be busy while another test's child process still holds it
    // open between fork and exec (ETXTBSY): try again shortly.
    let mut result = Sass::with_program(program.clone());
    for _ in 0..50 {
        match &result {
            Err(error) if error.contains("os error 26") => {
                std::thread::sleep(std::time::Duration::from_millis(20));
                result = Sass::with_program(program.clone());
            }
            _ => break,
        }
    }
    assert!(result.is_ok(), "{result:?}");
}

/// Needs Dart Sass 1.100.0 (`sass` on PATH, or `FOLIA_SASS`): `cargo test -- --ignored`.
#[test]
#[ignore = "needs Dart Sass 1.100.0"]
fn a_stylesheet_and_its_import_compile_with_a_jekyll_source_map() {
    let dir = Dir::new("compile");
    let site = dir.0.join("site");
    dir.write("site/_sass/_colors.scss", "$accent: #00aade;\n");
    let settings = SassConfig {
        dir: "_sass".into(),
        style: SassStyle::Compressed,
    };
    let sass = Sass::find().unwrap();
    let body = "@use \"colors\";\na { color: colors.$accent; }\n";
    let compiled = sass
        .compile(&site, &settings, "assets/css/main.scss", body)
        .unwrap();
    assert_eq!(
        compiled.css,
        "a{color:#00aade}/*# sourceMappingURL=main.css.map */"
    );
    let map: Value = serde_json::from_str(&compiled.map).unwrap();
    assert_eq!(
        map["sources"],
        serde_json::json!(["main.scss", "../../_sass/_colors.scss"]),
        "Dart Sass's order, kept"
    );
    assert_eq!(map["sourcesContent"][0], body);
    assert_eq!(map["file"], "main.css");
    assert!(!compiled.map.contains(&dir.0.to_string_lossy().into_owned()));
}
