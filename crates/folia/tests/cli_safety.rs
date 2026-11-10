//! The `folia` binary's guarantees about the destination and failed builds (release blockers of
//! the stabilization phase): it never deletes what it did not write, a failed build exits
//! non-zero and leaves the previous output as it was, and the diagnostics say why.
//!
//! ```sh
//! cargo test -p folia --test cli_safety
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn example_site() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/minimal-site")
}

/// A scratch directory, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("folia-cli-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A copy of the example site, committed to its own repository.
    fn site_copy(&self) -> PathBuf {
        let site = self.path("site");
        let copied = Command::new("cp")
            .arg("-r")
            .arg(example_site())
            .arg(&site)
            .status()
            .unwrap();
        assert!(copied.success());
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec!["commit", "--quiet", "-m", "site"],
        ] {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&site)
                .args([
                    "-c",
                    "user.name=Folia",
                    "-c",
                    "user.email=folia@example.com",
                ])
                .args(&args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        }
        site
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn folia(site: &Path, dest: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_folia"))
        .arg("build")
        .arg("--site")
        .arg(site)
        .arg("--dest")
        .arg(dest)
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_destination_with_unrelated_files_is_refused_and_everything_survives() {
    let scratch = Scratch::new("unrelated");
    let dest = scratch.path("precious");
    std::fs::create_dir_all(dest.join("photos/2026")).unwrap();
    std::fs::write(dest.join("notes.txt"), "mine").unwrap();
    std::fs::write(dest.join("photos/2026/a.jpg"), "jpg").unwrap();
    let output = folia(&example_site(), &dest);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let text = stderr(&output);
    assert!(
        text.contains("Folia only replaces a directory it wrote itself"),
        "{text}"
    );
    assert!(text.contains("nothing was changed"), "{text}");
    assert_eq!(
        std::fs::read_to_string(dest.join("notes.txt")).unwrap(),
        "mine"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("photos/2026/a.jpg")).unwrap(),
        "jpg"
    );
    assert!(!dest.join("index.html").exists());
}

#[test]
fn a_previous_folia_output_is_replaced_but_not_when_it_holds_user_files() {
    let scratch = Scratch::new("replace");
    let dest = scratch.path("out");
    let first = folia(&example_site(), &dest);
    assert!(first.status.success(), "{}", stderr(&first));
    let second = folia(&example_site(), &dest);
    assert!(second.status.success(), "{}", stderr(&second));
    assert!(dest.join("index.html").exists());

    std::fs::write(dest.join("mine.txt"), "mine").unwrap();
    let third = folia(&example_site(), &dest);
    assert_eq!(third.status.code(), Some(1));
    assert!(
        stderr(&third).contains("Folia did not write"),
        "{}",
        stderr(&third)
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("mine.txt")).unwrap(),
        "mine"
    );
    assert!(dest.join("index.html").exists());
}

#[test]
fn a_broken_template_fails_the_build_and_keeps_the_previous_output() {
    let scratch = Scratch::new("template");
    let site = scratch.site_copy();
    let dest = scratch.path("out");
    let first = folia(&site, &dest);
    assert!(first.status.success(), "{}", stderr(&first));
    let before = std::fs::read_to_string(dest.join("about.html")).unwrap();

    let templates = site.join("_folia/templates");
    let page = std::fs::read_dir(&templates)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().is_some_and(|n| n == "page.html"))
        .expect("the example has a page.html");
    std::fs::write(&page, "{{ page.nosuchkey.x }}").unwrap();
    let broken = folia(&site, &dest);
    let text = stderr(&broken);
    assert_eq!(broken.status.code(), Some(1), "{text}");
    assert!(text.contains("page.html"), "names the template: {text}");
    assert!(
        text.contains("`page.nosuchkey` is undefined"),
        "says why: {text}"
    );
    assert!(text.contains("about.md"), "names a page: {text}");
    assert!(!text.contains("cannot build them yet"), "{text}");
    assert!(text.contains("nothing was written"), "{text}");
    assert!(!text.contains("Built "), "no success line: {text}");
    assert_eq!(
        std::fs::read_to_string(dest.join("about.html")).unwrap(),
        before
    );
    assert!(dest.join("blog/index.html").exists());
}

#[test]
fn a_failed_first_build_leaves_no_destination() {
    let scratch = Scratch::new("first");
    let site = scratch.site_copy();
    std::fs::write(site.join("_folia/templates/page.html"), "{% bogus %}").unwrap();
    let dest = scratch.path("out");
    let output = folia(&site, &dest);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!dest.exists());
    assert!(!scratch.path(".out.folia-new").exists());
    assert!(!scratch.path(".out.folia-output").exists());
}

#[test]
fn version_prints_the_package_version_and_exits_zero() {
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_folia"))
            .arg(flag)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("folia {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(output.stderr.len(), 0);
    }
}

/// The exit status is a contract (docs/guide/building.md, "Exit status"): 0 built, 1 the build
/// failed, 2 usage or configuration error.
#[test]
fn the_exit_status_is_0_for_a_build_1_for_a_failed_build_and_2_for_usage_and_configuration() {
    let scratch = Scratch::new("exit-codes");
    let bin = || Command::new(env!("CARGO_BIN_EXE_folia"));
    let code = |args: &[&str]| bin().args(args).output().unwrap().status.code();

    // 0: a build, and `--help`.
    assert_eq!(code(&["--help"]), Some(0));
    let built = folia(&example_site(), &scratch.path("out"));
    assert_eq!(built.status.code(), Some(0), "{}", stderr(&built));

    // 2: no subcommand, an unknown one, a missing `--dest`, a missing or invalid configuration.
    assert_eq!(code(&[]), Some(2));
    assert_eq!(code(&["bogus"]), Some(2));
    assert_eq!(code(&["build"]), Some(2));
    let missing = bin()
        .args(["build", "--site"])
        .arg(scratch.path("no-such-site"))
        .arg("--dest")
        .arg(scratch.path("o2"))
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2), "{}", stderr(&missing));
    let invalid = scratch.path("invalid.toml");
    std::fs::write(&invalid, "bogus = 1\n").unwrap();
    let output = bin()
        .args(["build", "--site"])
        .arg(example_site())
        .arg("--config")
        .arg(&invalid)
        .arg("--dest")
        .arg(scratch.path("o3"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));

    // 1: a build that fails (here, a destination inside the site), nothing written.
    let inside = example_site().join("never-written");
    let failed = folia(&example_site(), &inside);
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    assert!(!inside.exists());
}

/// `folia build` with more arguments.
fn folia_with(site: &Path, dest: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_folia"))
        .arg("build")
        .arg("--site")
        .arg(site)
        .arg("--dest")
        .arg(dest)
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn build_with_future_marks_the_summary_and_the_record() {
    let scratch = Scratch::new("future");
    let dest = scratch.path("out");
    let output = folia_with(&example_site(), &dest, &["--future"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let summary = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        summary
            .trim_end()
            .ends_with("PREVIEW BUILD (future): not for publishing."),
        "{summary}"
    );
    assert!(stderr(&output).contains("warning: this is a preview build (future)"));
    let record = std::fs::read_to_string(scratch.path(".out.folia-output")).unwrap();
    assert!(
        record.lines().any(|line| line == "/folia future"),
        "{record}"
    );

    let normal = folia(&example_site(), &scratch.path("normal"));
    let summary = String::from_utf8_lossy(&normal.stdout).into_owned();
    assert!(!summary.contains("PREVIEW"), "{summary}");
}

#[test]
fn build_with_unpublished_marks_the_summary_and_the_record() {
    let scratch = Scratch::new("unpublished");
    let dest = scratch.path("out");
    let output = folia_with(&example_site(), &dest, &["--unpublished"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let summary = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        summary
            .trim_end()
            .ends_with("PREVIEW BUILD (unpublished): not for publishing."),
        "{summary}"
    );
    assert!(stderr(&output).contains("warning: this is a preview build (unpublished)"));
    let record = std::fs::read_to_string(scratch.path(".out.folia-output")).unwrap();
    assert!(
        record.lines().any(|line| line == "/folia unpublished"),
        "{record}"
    );
}

#[test]
fn a_normal_build_over_a_preview_destination_is_refused() {
    let scratch = Scratch::new("preview-refused");
    let dest = scratch.path("out");
    let preview = folia_with(&example_site(), &dest, &["--future"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    let normal = folia(&example_site(), &dest);
    assert_eq!(normal.status.code(), Some(1), "{}", stderr(&normal));
    assert!(
        stderr(&normal).contains("it holds a preview build (future)"),
        "{}",
        stderr(&normal)
    );
    assert!(dest.join("index.html").exists());
}

#[test]
fn build_with_future_and_unpublished_names_both_in_the_summary_and_the_record() {
    let scratch = Scratch::new("future-unpublished");
    let dest = scratch.path("out");
    let output = folia_with(&example_site(), &dest, &["--future", "--unpublished"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let summary = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        summary
            .trim_end()
            .ends_with("PREVIEW BUILD (future, unpublished): not for publishing."),
        "{summary}"
    );
    let record = std::fs::read_to_string(scratch.path(".out.folia-output")).unwrap();
    for mark in ["/folia future", "/folia unpublished"] {
        assert!(record.lines().any(|line| line == mark), "{record}");
    }

    let normal = folia(&example_site(), &dest);
    assert_eq!(normal.status.code(), Some(1), "{}", stderr(&normal));
    assert!(
        stderr(&normal).contains("it holds a preview build (future, unpublished)"),
        "{}",
        stderr(&normal)
    );
}

#[test]
fn build_at_sets_the_build_time_and_is_not_a_preview() {
    let scratch = Scratch::new("at");
    let site = example_site();
    let posted = |at: &str, name: &str| {
        let dest = scratch.path(name);
        let output = folia_with(&site, &dest, &["--at", at]);
        assert!(output.status.success(), "{}", stderr(&output));
        let summary = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(!summary.contains("PREVIEW"), "{summary}");
        dest.join("blog/2026/01/10/hello-folia.html").exists()
    };
    // The example's post is dated 2026-01-10.
    assert!(!posted("2026-01-01T00:00:00Z", "before"));
    assert!(posted("2026-03-01T12:00:00+02:00", "after"));
}

#[test]
fn build_at_with_a_bad_time_fails_clearly_and_builds_nothing() {
    let scratch = Scratch::new("at-bad");
    for bad in ["yesterday", "2026-01-01", "2026-13-01T00:00:00Z", ""] {
        let dest = scratch.path("out");
        let output = folia_with(&example_site(), &dest, &["--at", bad]);
        assert_eq!(output.status.code(), Some(2), "{bad}: {}", stderr(&output));
        let message = stderr(&output);
        assert!(
            message.contains("--at") && message.contains("RFC 3339"),
            "{bad}: {message}"
        );
        assert!(!dest.exists(), "{bad}");
    }
}
