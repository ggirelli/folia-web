//! `parity liquid` as the site's CI runs it: its exit status and what it prints.

use std::path::{Path, PathBuf};
use std::process::Output;

const CONFIG: &str = r#"
time-zone = "Europe/Stockholm"

[roots]
blog = "_posts"
micro = "_micro"
garden = "_digital_garden"

[site]
url = "https://example.com"
title = "Example"
description = "A site."
"#;

/// A throwaway site holding `files` and a `folia.toml`, removed when dropped.
struct Site(PathBuf);

impl Site {
    fn new(test: &str, files: &[(&str, &str)]) -> Self {
        let root =
            std::env::temp_dir().join(format!("parity-liquid-cli-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files.iter().chain(&[("folia.toml", CONFIG)]) {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self(root)
    }

    fn lint(&self, config: &Path) -> Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_parity"))
            .arg("liquid")
            .arg("--site")
            .arg(&self.0)
            .arg("--config")
            .arg(self.0.join(config))
            .output()
            .unwrap()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn clean_content_passes() {
    let site = Site::new(
        "clean",
        &[("_posts/2021-08-08-a.md", "---\n---\nUse `{{ x }}`.\n")],
    );
    let output = site.lint(Path::new("folia.toml"));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("Liquid lint: 1 Markdown documents of"));
    assert!(stdout.ends_with("ok: no Liquid outside code.\n"));
}

#[test]
fn liquid_fails_with_each_position() {
    let site = Site::new(
        "liquid",
        &[
            (
                "_posts/2021-08-08-a.md",
                "---\n---\nA {{ x }}.\n\nB {% y %}.\n",
            ),
            ("about.md", "---\ntitle: About\n---\n<!-- {{ z }} -->\n"),
        ],
    );
    let output = site.lint(Path::new("folia.toml"));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let findings: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.split_once(" outside code.").map(|(at, _)| at))
        .collect();
    assert_eq!(
        findings,
        [
            "_posts/2021-08-08-a.md:3:3: `{{`",
            "_posts/2021-08-08-a.md:5:3: `{%`",
            "about.md:4:6: `{{`",
        ]
    );
    assert!(stdout.ends_with("FAILS: 3 Liquid delimiters outside code, in 2 documents.\n"));
}

/// A problem that stops the check is not a finding: exit status 2.
#[test]
fn a_missing_configuration_is_an_error() {
    let site = Site::new("error", &[]);
    let output = site.lint(Path::new("missing.toml"));
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("cannot read "), "{stderr}");
    assert!(stderr.contains("missing.toml"), "{stderr}");
}
