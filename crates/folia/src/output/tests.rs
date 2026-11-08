use super::*;

/// A throwaway directory, removed when dropped: `site/` for the site, and room for outputs.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let root = std::env::temp_dir().join(format!("folia-output-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("site")).unwrap();
        Self(root)
    }

    fn site(&self) -> PathBuf {
        self.0.join("site")
    }

    /// The names in a directory, sorted.
    fn names(&self, dir: &str) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(self.0.join(dir))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn outputs(entries: &[(&str, Content)]) -> BTreeMap<String, Content> {
    entries
        .iter()
        .map(|(path, content)| ((*path).to_owned(), content.clone()))
        .collect()
}

fn bytes(text: &str) -> Content {
    Content::Bytes(text.as_bytes().to_vec())
}

#[test]
fn outputs_replace_the_destination() {
    let scratch = Scratch::new("replace");
    fs::write(scratch.site().join("CNAME"), "ggirelli.info").unwrap();
    let dest = scratch.0.join("out");
    write(
        &dest,
        &scratch.site(),
        &outputs(&[
            ("about.html", bytes("<p>About</p>")),
            ("blog/about.html", bytes("stub")),
            ("CNAME", Content::Copy(scratch.site().join("CNAME"))),
        ]),
    )
    .unwrap();
    assert_eq!(scratch.names("out"), ["CNAME", "about.html", "blog"]);
    assert_eq!(
        fs::read_to_string(dest.join("blog/about.html")).unwrap(),
        "stub"
    );
    assert_eq!(
        fs::read_to_string(dest.join("CNAME")).unwrap(),
        "ggirelli.info"
    );
    assert_eq!(
        scratch.names(""),
        [".out.folia-output", "out", "site"],
        "the record is left, and no temporary directory"
    );
}

#[test]
fn a_failed_write_leaves_the_destination_as_it_was() {
    let scratch = Scratch::new("failed");
    let dest = scratch.0.join("out");
    write(
        &dest,
        &scratch.site(),
        &outputs(&[("index.html", bytes("previous build"))]),
    )
    .unwrap();
    let record = fs::read_to_string(scratch.0.join(".out.folia-output")).unwrap();
    let problem = write(
        &dest,
        &scratch.site(),
        &outputs(&[
            ("about.html", bytes("<p>About</p>")),
            ("gone.pdf", Content::Copy(scratch.site().join("gone.pdf"))),
        ]),
    )
    .unwrap_err();
    assert!(problem.message.contains("gone.pdf"), "{problem}");
    assert_eq!(scratch.names("out"), ["index.html"]);
    assert_eq!(
        fs::read_to_string(dest.join("index.html")).unwrap(),
        "previous build"
    );
    assert_eq!(
        scratch.names(""),
        [".out.folia-output", "out", "site"],
        "no temporary directory is left"
    );
    assert_eq!(
        fs::read_to_string(scratch.0.join(".out.folia-output")).unwrap(),
        record
    );
}

#[test]
fn a_failed_first_build_leaves_no_destination() {
    let scratch = Scratch::new("first");
    let dest = scratch.0.join("out");
    let result = write(
        &dest,
        &scratch.site(),
        &outputs(&[("gone.pdf", Content::Copy(scratch.site().join("gone.pdf")))]),
    );
    assert!(result.is_err());
    assert_eq!(scratch.names(""), ["site"]);
}

#[test]
fn the_destination_must_be_apart_from_the_site() {
    let scratch = Scratch::new("apart");
    for (dest, reason) in [
        (scratch.site(), "it is inside the site"),
        (scratch.site().join("_site"), "it is inside the site"),
        (scratch.0.clone(), "it holds the site"),
    ] {
        let problem = write(&dest, &scratch.site(), &BTreeMap::new()).unwrap_err();
        assert!(problem.message.ends_with(reason), "{problem}");
    }
    assert_eq!(scratch.names("site"), Vec::<String>::new());
}

#[test]
fn a_destination_that_is_a_file_is_refused() {
    let scratch = Scratch::new("file");
    let dest = scratch.0.join("out");
    fs::write(&dest, "not a directory").unwrap();
    let problem = write(&dest, &scratch.site(), &BTreeMap::new()).unwrap_err();
    assert!(
        problem
            .message
            .ends_with("it exists and is not a directory")
    );
    assert_eq!(fs::read_to_string(&dest).unwrap(), "not a directory");
}

#[test]
fn an_output_path_outside_the_destination_is_refused() {
    let scratch = Scratch::new("escape");
    let dest = scratch.0.join("out");
    let problem = write(
        &dest,
        &scratch.site(),
        &outputs(&[("../escaped.html", bytes("x"))]),
    )
    .unwrap_err();
    assert!(problem.message.contains("not a relative output path"));
    assert_eq!(scratch.names(""), ["site"]);
}

fn tree(dir: &Path) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut todo = vec![dir.to_owned()];
    while let Some(dir) = todo.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else {
                found.insert(
                    path.display().to_string(),
                    fs::read_to_string(&path).unwrap(),
                );
            }
        }
    }
    found
}

#[test]
fn a_destination_with_user_files_is_refused_and_nothing_is_touched() {
    let scratch = Scratch::new("user-files");
    let dest = scratch.0.join("out");
    fs::create_dir_all(dest.join("photos/2026")).unwrap();
    fs::write(dest.join("notes.txt"), "mine").unwrap();
    fs::write(dest.join("photos/2026/a.jpg"), "jpg").unwrap();
    let before = tree(&scratch.0);
    let problem = write(
        &dest,
        &scratch.site(),
        &outputs(&[("index.html", bytes("x"))]),
    )
    .unwrap_err();
    assert!(
        problem
            .message
            .contains("no record shows a Folia build wrote it"),
        "{problem}"
    );
    assert!(problem.message.contains("nothing was changed"), "{problem}");
    assert_eq!(tree(&scratch.0), before, "every file survives");
    assert_eq!(
        scratch.names(""),
        ["out", "site"],
        "no record, no temporary"
    );
}

#[test]
fn a_previous_folia_output_with_a_user_file_added_is_not_cleaned() {
    let scratch = Scratch::new("mixed");
    let dest = scratch.0.join("out");
    write(
        &dest,
        &scratch.site(),
        &outputs(&[("index.html", bytes("first"))]),
    )
    .unwrap();
    fs::write(dest.join("mine.txt"), "mine").unwrap();
    fs::create_dir(dest.join("mydir")).unwrap();
    fs::write(dest.join("mydir/keep"), "keep").unwrap();
    let before = tree(&scratch.0);
    let problem = write(
        &dest,
        &scratch.site(),
        &outputs(&[("index.html", bytes("second"))]),
    )
    .unwrap_err();
    assert!(
        problem
            .message
            .contains("holds 2 entries Folia did not write"),
        "{problem}"
    );
    assert_eq!(tree(&scratch.0), before);
}

#[test]
fn a_previous_folia_output_is_replaced_and_the_record_follows() {
    let scratch = Scratch::new("again");
    let dest = scratch.0.join("out");
    for text in ["first", "second"] {
        write(
            &dest,
            &scratch.site(),
            &outputs(&[("index.html", bytes(text)), ("blog/a.html", bytes(text))]),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(dest.join("index.html")).unwrap(), text);
    }
    // A build that no longer writes `blog/` replaces it, then `blog` is not a record's entry.
    write(
        &dest,
        &scratch.site(),
        &outputs(&[("only.html", bytes("third"))]),
    )
    .unwrap();
    assert_eq!(scratch.names("out"), ["only.html"]);
    assert_eq!(scratch.names(""), [".out.folia-output", "out", "site"]);
}

#[test]
fn an_empty_destination_is_written_and_a_moved_one_is_refused() {
    let scratch = Scratch::new("empty-moved");
    let dest = scratch.0.join("out");
    fs::create_dir(&dest).unwrap();
    write(&dest, &scratch.site(), &outputs(&[("a.html", bytes("a"))])).unwrap();
    // The same output, copied to another name, has no record of its own.
    let copy = scratch.0.join("copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("a.html"), "a").unwrap();
    assert!(write(&copy, &scratch.site(), &outputs(&[("a.html", bytes("b"))])).is_err());
    assert_eq!(fs::read_to_string(copy.join("a.html")).unwrap(), "a");
    // A record for another directory is not this directory's.
    fs::copy(
        scratch.0.join(".out.folia-output"),
        scratch.0.join(".copy.folia-output"),
    )
    .unwrap();
    let problem = write(&copy, &scratch.site(), &outputs(&[("a.html", bytes("b"))])).unwrap_err();
    assert!(
        problem.message.contains("not for this directory"),
        "{problem}"
    );
    assert_eq!(fs::read_to_string(copy.join("a.html")).unwrap(), "a");
}

#[test]
fn leftover_temporary_directories_are_refused_not_deleted() {
    let scratch = Scratch::new("leftover");
    let dest = scratch.0.join("out");
    for leftover in [".out.folia-new", ".out.folia-old"] {
        let path = scratch.0.join(leftover);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("precious"), "p").unwrap();
        let problem =
            write(&dest, &scratch.site(), &outputs(&[("a.html", bytes("a"))])).unwrap_err();
        assert!(problem.message.contains("does not delete it"), "{problem}");
        assert_eq!(fs::read_to_string(path.join("precious")).unwrap(), "p");
        assert!(!dest.exists());
        fs::remove_dir_all(&path).unwrap();
    }
}
