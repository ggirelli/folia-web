use super::*;

/// A throwaway site, committed to its own repository, with Folia's configuration and a place for
/// build directories beside it (not inside it); removed when dropped.
pub struct Fixture(pub PathBuf);

const CONFIG: &str = r#"time-zone = "Europe/Stockholm"
templates-dir = "_folia/templates"

[roots]
blog = "_posts"
micro = "_micro"
garden = "_digital_garden"

[site]
url = "https://example.org"
title = "Example"
description = "A site."

[slice]
pages = ["pages/profile.md"]
"#;

/// The page template shows the build's year, so a page tells which time it was built as of.
const PAGE: &str =
    "<title>{{ page.title }} | {{ site.title }}</title><footer>{{ site.year }}</footer>";

impl Fixture {
    pub fn new(test: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("parity-builds-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self(root);
        fixture.write(
            "site/pages/profile.md",
            "---\nlayout: page\ntitle: About\npermalink: /about\n---\nHello.\n",
        );
        fixture.write("site/assets/a.txt", "a");
        fixture.write("site/_folia/templates/page.html", PAGE);
        fixture.write("folia.toml", CONFIG);
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "--quiet", "-m", "site"]);
        fixture
    }

    pub fn site(&self) -> PathBuf {
        self.0.join("site")
    }

    pub fn config_path(&self) -> PathBuf {
        self.0.join("folia.toml")
    }

    pub fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    pub fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.site())
            .args([
                "-c",
                "user.name=Folia",
                "-c",
                "user.email=folia@example.com",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_DATE", "2026-01-04T13:36:24+01:00")
            .env("GIT_COMMITTER_DATE", "2026-01-04T13:36:24+01:00")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    pub fn keep(&self, dest: &Path, at: &str) -> Result<Kept, String> {
        let config = Config::load(&self.config_path()).unwrap();
        keep(&Request {
            site: &self.site(),
            config: &config,
            config_path: &self.config_path(),
            dest,
            at: at.parse().unwrap(),
            expected_templates: None,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_build_is_as_of_the_time_given() {
    let fixture = Fixture::new("time");
    for (at, year) in [
        ("2031-06-01T00:00:00Z", "2031"),
        ("2032-06-01T00:00:00Z", "2032"),
    ] {
        let dest = fixture.0.join(format!("build-{year}"));
        let kept = fixture.keep(&dest, at).unwrap();
        assert_eq!(kept.record.at, at);
        assert_eq!(
            read(&kept.output.join("about.html")),
            format!("<title>About | Example</title><footer>{year}</footer>")
        );
    }
}

#[test]
fn the_output_record_log_and_manifest_are_kept() {
    let fixture = Fixture::new("kept");
    let dest = fixture.0.join("out/build");
    let kept = fixture.keep(&dest, "2026-10-25T14:43:51Z").unwrap();
    assert_eq!(kept.dir, dest.canonicalize().unwrap());
    assert_eq!(kept.output, kept.dir.join("site"));
    assert!(kept.built.is_ok());
    for file in [
        "site/about.html",
        "site/assets/a.txt",
        "sha256.txt",
        "build.json",
        "build.log",
    ] {
        assert!(kept.dir.join(file).is_file(), "{file}");
    }
    let manifest = read(&kept.dir.join("sha256.txt"));
    assert_eq!(manifest, manifest_text(&builds_manifest(&kept.output)));
    assert!(manifest.contains("  ./about.html\n"));
    assert!(manifest.contains(&format!("{}  ./assets/a.txt\n", hex(&Sha256::digest(b"a")))));
    assert_eq!(kept.record.files, Some(manifest.lines().count()));
    let log = read(&kept.dir.join("build.log"));
    assert!(
        log.starts_with("Built 1 pages, 0 redirect stubs, 1 generated files and 1 static files;"),
        "{log}"
    );
}

fn builds_manifest(root: &Path) -> Vec<(String, String)> {
    manifest(root).unwrap()
}

#[test]
fn an_existing_destination_is_refused_and_left_as_it_was() {
    let fixture = Fixture::new("exists");
    let dest = fixture.0.join("evidence");
    fixture.write("evidence/keep.txt", "evidence");
    let error = fixture
        .keep(&dest, "2026-10-25T14:43:51Z")
        .err()
        .expect("refused");
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(read(&dest.join("keep.txt")), "evidence");
    assert_eq!(std::fs::read_dir(&dest).unwrap().count(), 1);
}

#[test]
fn an_empty_existing_directory_is_refused_too() {
    let fixture = Fixture::new("empty");
    let dest = fixture.0.join("empty");
    std::fs::create_dir_all(&dest).unwrap();
    assert!(fixture.keep(&dest, "2026-10-25T14:43:51Z").is_err());
}

#[test]
fn a_destination_inside_the_site_is_refused_before_anything_is_written() {
    let fixture = Fixture::new("inside");
    let inside = fixture.site().join("out/build");
    let error = fixture
        .keep(&inside, "2026-10-25T14:43:51Z")
        .err()
        .expect("refused");
    assert!(error.contains("inside the site"), "{error}");
    assert!(!fixture.site().join("out").exists());
    // Reached through `..`, still inside.
    let sneaky = fixture.0.join("elsewhere/../site/build");
    assert!(fixture.keep(&sneaky, "2026-10-25T14:43:51Z").is_err());
    assert!(!fixture.site().join("build").exists());
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
}

#[test]
fn the_manifest_is_sorted_bytewise_and_stable() {
    let fixture = Fixture::new("manifest");
    for path in ["tree/b", "tree/a/c", "tree/A", "tree/a.b", "tree/a-b"] {
        fixture.write(path, path);
    }
    let tree = fixture.0.join("tree");
    let paths: Vec<String> = builds_manifest(&tree).into_iter().map(|(p, _)| p).collect();
    // Bytewise: `-` (0x2d) < `.` (0x2e) < `/` (0x2f) < `A` < `a` < `b`.
    assert_eq!(paths, ["A", "a-b", "a.b", "a/c", "b"]);
    assert_eq!(builds_manifest(&tree), builds_manifest(&tree));
}

#[test]
fn the_record_names_the_inputs() {
    let fixture = Fixture::new("record");
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    let record = &kept.record;
    let head = fixture.git(&["rev-parse", "HEAD"]);
    assert_eq!(record.site.commit.as_deref(), Some(head.trim()));
    assert_eq!(record.site.status.as_deref(), Some(&[][..]));
    assert_eq!(
        record.config.sha256,
        hex(&Sha256::digest(CONFIG.as_bytes()))
    );
    assert_eq!(record.templates.files, 1);
    assert_eq!(
        record.templates.tree_sha256,
        hex(&Sha256::digest(
            format!("{}  ./page.html\n", hex(&Sha256::digest(PAGE.as_bytes()))).as_bytes()
        ))
    );
    assert!(record.result.ok);
    assert_eq!(record.result.warnings, Vec::<String>::new());
    assert_eq!(record.files, Some(3));
    // The record on disk is the one returned.
    let json: serde_json::Value =
        serde_json::from_str(&read(&kept.dir.join("build.json"))).unwrap();
    assert_eq!(json, serde_json::to_value(record).unwrap());
    assert_eq!(json["at"], "2026-10-25T14:43:51Z");
}

#[test]
fn a_source_outside_a_repository_has_no_commit_and_an_unknown_identity() {
    let record = checkout(Path::new("/"));
    assert_eq!(record.path, "/");
    assert_eq!(record.commit, None);
    assert_eq!(record.status, None);
    match &record.changes {
        Changes::Unknown { error } => {
            assert!(error.starts_with("it has no git identity: "), "{error}");
            assert!(error.contains("git rev-parse HEAD"), "{error}");
        }
        other => panic!("expected an unknown identity, got {other:?}"),
    }
    let json = serde_json::to_value(&record).unwrap();
    assert_eq!(json["commit"], serde_json::Value::Null);
    assert_eq!(json["changes"]["state"], "unknown");
    assert!(json["changes"].get("sha256").is_none());
}

#[test]
fn a_site_that_is_not_a_git_checkout_is_kept_with_its_diagnostics_and_an_unknown_identity() {
    let fixture = Fixture::new("not-git");
    std::fs::remove_dir_all(fixture.site().join(".git")).unwrap();
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    // Folia cannot build it (a page's date is read from git), and the build is kept as a failed
    // one: its record and log say why, and the identity is unknown, never clean or hashed.
    let problems = kept
        .built
        .as_ref()
        .expect_err("a site outside git cannot be built");
    assert!(
        problems.iter().any(|p| p.message.contains("git log")),
        "{problems:?}"
    );
    assert!(kept.dir.join("build.json").exists());
    assert!(kept.dir.join("build.log").exists());
    assert_eq!(kept.record.site.commit, None);
    let reasons = unknown_identities(&kept.record);
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0].starts_with("the site's identity (")
            && reasons[0].contains("is unknown: it has no git identity: "),
        "{reasons:?}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&read(&kept.dir.join("build.json"))).unwrap();
    assert_eq!(json["site"]["commit"], serde_json::Value::Null);
    assert_eq!(json["site"]["changes"]["state"], "unknown");
    let log = read(&kept.dir.join("build.log"));
    assert!(log.contains("are unknown: it has no git identity"), "{log}");
}

#[test]
fn a_failed_build_keeps_its_record_and_log() {
    let fixture = Fixture::new("failed");
    std::fs::remove_file(fixture.site().join("_folia/templates/page.html")).unwrap();
    std::fs::write(fixture.site().join("_folia/templates/other.html"), "x").unwrap();
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    assert!(kept.built.is_err());
    assert!(!kept.record.result.ok);
    assert_eq!(kept.record.files, None);
    assert!(!kept.output.exists());
    assert!(!kept.dir.join("sha256.txt").exists());
    let log = read(&kept.dir.join("build.log"));
    assert!(log.starts_with("The build failed with 1 problems"), "{log}");
    assert!(
        log.contains("problem: ") && log.contains("page.html"),
        "{log}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&read(&kept.dir.join("build.json"))).unwrap();
    assert_eq!(json["result"]["ok"], false);
}

#[test]
fn a_clean_checkout_has_no_changes_and_its_changes_are_identified_by_content() {
    let fixture = Fixture::new("changes");
    assert_eq!(checkout(&fixture.site()).changes, Changes::Clean);
    let changed = |fixture: &Fixture| match checkout(&fixture.site()).changes {
        Changes::Changed { sha256 } => sha256,
        other => panic!("expected changes, got {other:?}"),
    };
    fixture.write("site/_folia/untracked.txt", "one");
    let one = changed(&fixture);
    assert_eq!(changed(&fixture), one);
    fixture.write("site/_folia/untracked.txt", "two");
    let two = changed(&fixture);
    assert_ne!(one, two);
    std::fs::remove_file(fixture.site().join("_folia/untracked.txt")).unwrap();
    fixture.write("site/pages/profile.md", "edited");
    let edited = changed(&fixture);
    assert_ne!(edited, one);
}

#[test]
fn a_git_failure_is_unknown_never_clean_nor_hashed() {
    // Not a repository: `changes` cannot say, and says so.
    let fixture = Fixture::new("failure");
    let outside = fixture.0.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    assert!(matches!(changes(&outside), Changes::Unknown { .. }));
    // A repository whose HEAD resolves but whose index is unreadable: the commit is recorded,
    // and the changes are unknown, not clean, although nothing was changed.
    std::fs::write(fixture.site().join(".git/index"), b"not an index").unwrap();
    let record = checkout(&fixture.site());
    assert!(record.commit.is_some());
    match record.changes {
        Changes::Unknown { error } => assert!(error.contains("git "), "{error}"),
        other => panic!("expected unknown changes, got {other:?}"),
    }
    let json = serde_json::to_value(checkout(&fixture.site())).unwrap();
    assert_eq!(json["changes"]["state"], "unknown");
    assert!(json["changes"].get("sha256").is_none());
}

#[test]
fn a_build_record_says_when_changes_are_unknown() {
    let fixture = Fixture::new("unknown-log");
    std::fs::write(fixture.site().join(".git/index"), b"not an index").unwrap();
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    assert!(matches!(kept.record.site.changes, Changes::Unknown { .. }));
    let log = read(&kept.dir.join("build.log"));
    assert!(
        log.contains("record: the uncommitted changes of") && log.contains("are unknown"),
        "{log}"
    );
}

#[test]
fn templates_other_than_the_expected_are_refused_before_anything_is_written() {
    let fixture = Fixture::new("templates");
    let config = Config::load(&fixture.config_path()).unwrap();
    let dest = fixture.0.join("build");
    let (site, config_path) = (fixture.site(), fixture.config_path());
    let request = |expected| Request {
        site: &site,
        config: &config,
        config_path: &config_path,
        dest: &dest,
        at: "2026-10-25T14:43:51Z".parse().unwrap(),
        expected_templates: expected,
    };
    let error = keep(&request(Some("0000"))).err().expect("refused");
    assert!(error.contains("not the expected 0000"), "{error}");
    assert!(!dest.exists());
    let actual = templates(&fixture.site().join("_folia/templates")).unwrap();
    let kept = keep(&request(Some(&actual.tree_sha256))).unwrap();
    assert_eq!(kept.record.templates.tree_sha256, actual.tree_sha256);
}

/// The hash of the fixture's changes, `None` when it is clean; a git failure fails the test.
fn changes_of(fixture: &Fixture) -> Option<String> {
    changes_in(&fixture.site())
}

fn changes_in(dir: &Path) -> Option<String> {
    let first = changes(dir);
    // Stable: the same tree hashes the same.
    assert_eq!(changes(dir), first);
    match first {
        Changes::Clean => None,
        Changes::Changed { sha256 } => Some(sha256),
        Changes::Unknown { error } => panic!("git failed: {error}"),
    }
}

#[test]
fn staged_unstaged_and_index_only_changes_are_told_apart() {
    let fixture = Fixture::new("staged");
    fixture.write("site/pages/profile.md", "edited\n");
    let unstaged = changes_of(&fixture).expect("changed");
    fixture.git(&["add", "pages/profile.md"]);
    let staged = changes_of(&fixture).expect("changed");
    assert_ne!(staged, unstaged);
    // Staged, then the working tree put back as committed: the index still differs.
    let committed = fixture.git(&["show", "HEAD:pages/profile.md"]);
    fixture.write("site/pages/profile.md", &committed);
    let index_only = changes_of(&fixture).expect("changed");
    assert_ne!(index_only, staged);
    fixture.git(&["reset", "--quiet", "pages/profile.md"]);
    assert_eq!(changes_of(&fixture), None);
}

#[test]
fn deletions_and_renames_are_changes() {
    let fixture = Fixture::new("deletions");
    std::fs::remove_file(fixture.site().join("assets/a.txt")).unwrap();
    let deleted = changes_of(&fixture).expect("changed");
    fixture.git(&["checkout", "--quiet", "--", "assets/a.txt"]);
    assert_eq!(changes_of(&fixture), None);
    fixture.git(&["mv", "assets/a.txt", "assets/b.txt"]);
    let renamed = changes_of(&fixture).expect("changed");
    assert_ne!(renamed, deleted);
    fixture.git(&["mv", "assets/b.txt", "assets/c.txt"]);
    assert_ne!(changes_of(&fixture).expect("changed"), renamed);
}

#[test]
fn binary_changes_are_identified_by_their_bytes() {
    let fixture = Fixture::new("binary");
    std::fs::write(fixture.site().join("assets/b.bin"), [0u8, 1, 2, 255]).unwrap();
    fixture.git(&["add", "assets/b.bin"]);
    fixture.git(&["commit", "--quiet", "-m", "binary"]);
    std::fs::write(fixture.site().join("assets/b.bin"), [0u8, 1, 3, 255]).unwrap();
    let one = changes_of(&fixture).expect("changed");
    std::fs::write(fixture.site().join("assets/b.bin"), [0u8, 1, 4, 255]).unwrap();
    assert_ne!(changes_of(&fixture).expect("changed"), one);
}

#[test]
fn untracked_files_with_unusual_names_and_links_count_and_ignored_ones_do_not() {
    let fixture = Fixture::new("untracked");
    fixture.write("site/.gitignore", "ignored/\n");
    fixture.git(&["add", ".gitignore"]);
    fixture.git(&["commit", "--quiet", "-m", "ignore"]);
    fixture.write("site/ignored/x.txt", "x");
    assert_eq!(changes_of(&fixture), None);
    fixture.write("site/a name\nwith é.txt", "same");
    let unusual = changes_of(&fixture).expect("changed");
    std::fs::rename(
        fixture.site().join("a name\nwith é.txt"),
        fixture.site().join("plain.txt"),
    )
    .unwrap();
    // The same bytes under another name are another tree.
    assert_ne!(changes_of(&fixture).expect("changed"), unusual);
    std::fs::remove_file(fixture.site().join("plain.txt")).unwrap();
    std::os::unix::fs::symlink("nowhere", fixture.site().join("link")).unwrap();
    let link = changes_of(&fixture).expect("a dangling link is a change");
    std::fs::remove_file(fixture.site().join("link")).unwrap();
    std::os::unix::fs::symlink("elsewhere", fixture.site().join("link")).unwrap();
    assert_ne!(changes_of(&fixture).expect("changed"), link);
}

#[test]
fn diff_settings_in_the_repository_do_not_change_the_hash() {
    let fixture = Fixture::new("settings");
    fixture.write("site/pages/profile.md", "edited ü\n");
    fixture.write("site/über.txt", "new");
    fixture.git(&["mv", "assets/a.txt", "assets/moved.txt"]);
    let plain = changes_of(&fixture).expect("changed");
    for (key, value) in [
        ("diff.noprefix", "true"),
        ("diff.mnemonicPrefix", "true"),
        ("diff.renames", "copies"),
        ("color.ui", "always"),
        ("core.quotePath", "false"),
    ] {
        fixture.git(&["config", key, value]);
    }
    assert_eq!(changes_of(&fixture), Some(plain));
}

#[test]
fn diff_layout_settings_do_not_change_the_hash() {
    let fixture = Fixture::new("layout");
    // Two committed files, edits staged and unstaged in places apart, and blank context lines,
    // so that the context, hunk merging, file order, algorithm and blank-line settings would
    // change git's text.
    let lines: String = (1..=30).fold(String::new(), |mut text, n| {
        let _ = write!(text, "{n}\n\n");
        text
    });
    fixture.write("site/a.txt", &lines);
    fixture.write("site/sub/b.txt", &lines);
    fixture.git(&["add", "."]);
    fixture.git(&["commit", "--quiet", "-m", "texts"]);
    fixture.write(
        "site/a.txt",
        &lines.replace("5\n", "five\n").replace("12\n", "twelve\n"),
    );
    fixture.write("site/sub/b.txt", &lines.replace("3\n", "three\n"));
    fixture.git(&["add", "a.txt"]);
    fixture.write("site/a.txt", &lines.replace("20\n", "twenty\n"));
    let plain = changes_of(&fixture).expect("changed");
    fixture.write("order.txt", "sub/b.txt\na.txt\n");
    let order = fixture.0.join("order.txt");
    for (key, value) in [
        ("diff.context", "1"),
        ("diff.interHunkContext", "10"),
        ("diff.orderFile", order.to_str().unwrap()),
        ("diff.algorithm", "histogram"),
        ("diff.indentHeuristic", "false"),
        ("diff.suppressBlankEmpty", "true"),
        ("diff.relative", "true"),
        ("core.abbrev", "12"),
    ] {
        fixture.git(&["config", key, value]);
        assert_eq!(changes_of(&fixture).as_ref(), Some(&plain), "{key}");
    }
    // From a subdirectory, with `diff.relative` set, the whole repository still counts.
    assert_eq!(changes_in(&fixture.site().join("sub")), Some(plain));
}

#[test]
fn an_untracked_file_whose_name_is_not_utf8_is_identified_by_its_contents() {
    use std::os::unix::ffi::OsStrExt as _;
    let fixture = Fixture::new("not-utf8");
    // `caf\xe9.txt`: Latin-1, not UTF-8. A lossy reading of the name would name no file, and
    // hash the same placeholder whatever the file holds.
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9.txt");
    let file = fixture.site().join(name);
    std::fs::write(&file, "one").unwrap();
    let one = changes_of(&fixture).expect("changed");
    std::fs::write(&file, "two").unwrap();
    let two = changes_of(&fixture).expect("changed");
    assert_ne!(one, two, "a change of the file's contents changes the hash");
    std::fs::write(&file, "one").unwrap();
    assert_eq!(changes_of(&fixture).expect("changed"), one);
    // Another name whose lossy reading is the same (`caf\xe8.txt` also reads as `caf�.txt`),
    // with the same contents, is another tree.
    std::fs::remove_file(&file).unwrap();
    std::fs::write(
        fixture
            .site()
            .join(std::ffi::OsStr::from_bytes(b"caf\xe8.txt")),
        "one",
    )
    .unwrap();
    assert_ne!(changes_of(&fixture).expect("changed"), one);
}

#[test]
fn an_untracked_nested_repository_is_refused_as_unknown_never_hashed() {
    let fixture = Fixture::new("nested");
    let nested = fixture.site().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&nested)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "--quiet"]);
    std::fs::write(nested.join("file.txt"), "one").unwrap();
    // Git lists the nested repository as a directory, not its files: its contents cannot be
    // identified, so the changes are unknown, naming it, and never a hash.
    match changes(&fixture.site()) {
        Changes::Unknown { error } => {
            assert!(error.contains("nested/"), "{error}");
            assert!(error.contains("not supported"), "{error}");
        }
        other => panic!("expected unknown changes, got {other:?}"),
    }
    let record = checkout(&fixture.site());
    assert!(matches!(record.changes, Changes::Unknown { .. }));
    // An ignored nested repository is not part of the tree, as any ignored file is not.
    fixture.write("site/.gitignore", "nested/\n");
    fixture.git(&["add", ".gitignore"]);
    fixture.git(&["commit", "--quiet", "-m", "ignore"]);
    assert_eq!(changes_of(&fixture), None);
}

#[test]
fn a_manifest_is_read_back_as_written_and_a_malformed_one_is_refused() {
    let fixture = Fixture::new("read-manifest");
    fixture.write("out/a.html", "a");
    fixture.write("out/b/c.css", "c");
    let manifest = manifest(&fixture.0.join("out")).unwrap();
    fixture.write("sha256.txt", &manifest_text(&manifest));
    let read = read_manifest(&fixture.0.join("sha256.txt")).unwrap();
    assert_eq!(read.into_iter().collect::<Vec<_>>(), manifest);
    for bad in [
        "x  ./a.html\n",
        &format!("{}  a.html\n", "0".repeat(64)),
        &format!("{}  ./\n", "0".repeat(64)),
        &format!("{}  ./a.html\n", "A".repeat(64)),
        &format!("{0}  ./a.html\n{0}  ./a.html\n", "0".repeat(64)),
    ] {
        fixture.write("bad.txt", bad);
        assert!(
            read_manifest(&fixture.0.join("bad.txt")).is_err(),
            "{bad:?}"
        );
    }
}

/// Makes `path`, in the fixture's site, unreadable; `false` when it can still be read (as root),
/// so a test of an unreadable file has nothing to test.
fn make_unreadable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(path).is_ok() {
        eprintln!(
            "{} is still readable (run as root?); not tested",
            path.display()
        );
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
        return false;
    }
    true
}

#[test]
fn an_unreadable_untracked_file_is_unknown_naming_it_and_the_error_never_a_placeholder() {
    let fixture = Fixture::new("unreadable");
    let file = fixture.site().join("_folia/secret.txt");
    fixture.write("site/_folia/secret.txt", "one");
    if !make_unreadable(&file) {
        return;
    }
    match changes(&fixture.site()) {
        Changes::Unknown { error } => {
            assert!(error.contains("_folia/secret.txt"), "{error}");
            assert!(error.contains("cannot be read"), "{error}");
            assert!(
                error.to_lowercase().contains("permission denied"),
                "{error}"
            );
        }
        other => panic!("expected unknown changes, got {other:?}"),
    }
    let json = serde_json::to_value(checkout(&fixture.site())).unwrap();
    assert_eq!(json["changes"]["state"], "unknown");
    assert!(json["changes"].get("sha256").is_none());
    // Readable again, it is identified by its contents.
    std::fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    assert!(changes_of(&fixture).is_some());
}

/// Commits a repository at `sub` in the fixture's site and records it there as a submodule (a
/// gitlink), as `git submodule add` would without its `.gitmodules`.
fn add_submodule(fixture: &Fixture, sub: &str) {
    let dir = fixture.site().join(sub);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("file.txt"), "one").unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args([
                "-c",
                "user.name=Folia",
                "-c",
                "user.email=folia@example.com",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    git(&["init", "--quiet"]);
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "sub"]);
    let head = git(&["rev-parse", "HEAD"]);
    fixture.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{},{sub}", head.trim()),
    ]);
}

fn assert_submodule_refused(fixture: &Fixture, sub: &str) {
    match changes(&fixture.site()) {
        Changes::Unknown { error } => {
            assert!(error.contains(&format!("submodule {sub}")), "{error}");
            assert!(error.contains("not supported"), "{error}");
        }
        other => panic!("expected unknown changes, got {other:?}"),
    }
}

#[test]
fn a_submodule_clean_or_dirty_staged_or_committed_is_unknown() {
    let fixture = Fixture::new("submodule");
    add_submodule(&fixture, "sub");
    // Staged only.
    assert_submodule_refused(&fixture, "sub");
    fixture.git(&["commit", "--quiet", "-m", "add the submodule"]);
    // Committed, its working tree clean: still refused, since a dirty one could not be told from
    // it by the diffs.
    assert_submodule_refused(&fixture, "sub");
    // Dirty, two ways: never hashed, so the two cannot be confused.
    std::fs::write(fixture.site().join("sub/file.txt"), "two").unwrap();
    assert_submodule_refused(&fixture, "sub");
    std::fs::write(fixture.site().join("sub/file.txt"), "three").unwrap();
    assert_submodule_refused(&fixture, "sub");
    // Removed from the index but still in HEAD: still an entry, still refused.
    fixture.git(&["rm", "--cached", "--quiet", "sub"]);
    assert_submodule_refused(&fixture, "sub");
}

#[test]
fn a_repository_without_submodule_entries_is_identified_even_with_a_gitmodules_file() {
    let fixture = Fixture::new("no-submodule");
    assert_eq!(changes(&fixture.site()), Changes::Clean);
    // A `.gitmodules` that names no gitlink (as a site's may) is a file like any other.
    fixture.write(
        "site/.gitmodules",
        "[submodule \"gone\"]\n\tpath = gone\n\turl = https://example.org/gone.git\n",
    );
    fixture.git(&["add", ".gitmodules"]);
    fixture.git(&["commit", "--quiet", "-m", "gitmodules"]);
    assert_eq!(changes(&fixture.site()), Changes::Clean);
    fixture.write("site/_folia/untracked.txt", "one");
    assert!(changes_of(&fixture).is_some());
}

#[test]
fn an_unknown_site_or_folia_identity_is_named_and_a_known_one_is_not() {
    let fixture = Fixture::new("identities");
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    let known = Record {
        site: Checkout {
            changes: Changes::Clean,
            ..kept.record.site.clone()
        },
        folia: Checkout {
            changes: Changes::Changed {
                sha256: "0".repeat(64),
            },
            ..kept.record.folia.clone()
        },
        ..kept.record.clone()
    };
    assert_eq!(unknown_identities(&known), Vec::<String>::new());
    let unknown = || Changes::Unknown {
        error: "git failed".to_owned(),
    };
    let site = Record {
        site: Checkout {
            changes: unknown(),
            ..known.site.clone()
        },
        ..known.clone()
    };
    let reasons = unknown_identities(&site);
    assert_eq!(reasons.len(), 1);
    assert!(
        reasons[0].starts_with("the site's identity ("),
        "{reasons:?}"
    );
    assert!(
        reasons[0].ends_with("is unknown: git failed"),
        "{reasons:?}"
    );
    let folia = Record {
        folia: Checkout {
            changes: unknown(),
            ..known.folia.clone()
        },
        ..known.clone()
    };
    let reasons = unknown_identities(&folia);
    assert_eq!(reasons.len(), 1);
    assert!(
        reasons[0].starts_with("the Folia source's identity ("),
        "{reasons:?}"
    );
    let both = Record {
        folia: folia.folia.clone(),
        ..site
    };
    assert_eq!(unknown_identities(&both).len(), 2);
}

#[test]
fn a_build_whose_identity_is_unknown_is_kept_with_its_diagnostics() {
    let fixture = Fixture::new("unknown-kept");
    let file = fixture.site().join("_folia/secret.txt");
    fixture.write("site/_folia/secret.txt", "one");
    if !make_unreadable(&file) {
        return;
    }
    let kept = fixture
        .keep(&fixture.0.join("build"), "2026-10-25T14:43:51Z")
        .unwrap();
    // The build itself succeeds, and is kept whole: its output, manifest, record and log.
    assert!(kept.built.is_ok());
    assert!(kept.output.join("about.html").exists());
    assert!(kept.dir.join("sha256.txt").exists());
    let reasons = unknown_identities(&kept.record);
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(reasons[0].contains("_folia/secret.txt"), "{reasons:?}");
    let json: serde_json::Value =
        serde_json::from_str(&read(&kept.dir.join("build.json"))).unwrap();
    assert_eq!(json["site"]["changes"]["state"], "unknown");
    assert!(
        json["site"]["changes"]["error"]
            .as_str()
            .unwrap()
            .contains("_folia/secret.txt")
    );
    let log = read(&kept.dir.join("build.log"));
    assert!(
        log.contains("are unknown") && log.contains("_folia/secret.txt"),
        "{log}"
    );
}
