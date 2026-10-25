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
    fs::create_dir_all(dest.join("stale")).unwrap();
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
        ["out", "site"],
        "no temporary directory is left"
    );
}

#[test]
fn a_failed_write_leaves_the_destination_as_it_was() {
    let scratch = Scratch::new("failed");
    let dest = scratch.0.join("out");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("index.html"), "previous build").unwrap();
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
        ["out", "site"],
        "no temporary directory is left"
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
