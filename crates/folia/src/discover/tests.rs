use std::path::PathBuf;

use super::*;
use crate::config::Roots;

/// A throwaway site directory holding the given files, removed when dropped.
struct Site(PathBuf);

impl Site {
    fn new(test: &str, files: &[(&str, &str)]) -> Self {
        let root =
            std::env::temp_dir().join(format!("folia-discover-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self(root)
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn config(exclude: &[&str]) -> Config {
    Config {
        time_zone: "Europe/Stockholm".into(),
        include: vec![".htaccess".into()],
        exclude: exclude.iter().map(|p| (*p).to_owned()).collect(),
        roots: Roots {
            blog: "_posts".into(),
            micro: "_micro".into(),
            garden: "_digital_garden".into(),
        },
        templates_dir: "_folia/templates".into(),
        site: crate::config::Identity {
            url: "https://example.com".into(),
            title: "Example".into(),
            description: "A site.".into(),
        },
        authors: std::collections::BTreeMap::new(),
        slice: None,
        sass: crate::config::SassConfig::default(),
        micro: crate::config::MicroConfig::default(),
        blog: crate::config::BlogConfig::default(),
        garden: crate::config::GardenConfig::default(),
        feeds: None,
        search: None,
    }
}

fn ids(found: &[(Mode, DocId)], mode: Mode) -> Vec<&str> {
    found
        .iter()
        .filter(|(m, _)| *m == mode)
        .map(|(_, id)| id.as_str())
        .collect()
}

const FRONT: &str = "---\ntitle: x\n---\nbody\n";

#[test]
fn files_are_documents_pages_or_static_by_place_and_front_matter() {
    let site = Site::new(
        "classify",
        &[
            ("_posts/2021/2021-08-08-sample-post.md", FRONT),
            ("_posts/2021/README.md", "notes"),
            ("_micro/notes/2026-04-08-sample-note.md", FRONT),
            ("_digital_garden/topic/index.md", FRONT),
            ("pages/profile.md", FRONT),
            ("assets/css/main.scss", "---\n---\n@import 'x';\n"),
            ("assets/img/a.png", "png"),
            ("CNAME", "ggirelli.info\n"),
        ],
    );
    let found = discover(&site.0, &config(&[])).unwrap();
    assert_eq!(
        ids(&found.documents, Mode::Blog),
        ["_posts/2021/2021-08-08-sample-post.md"]
    );
    assert_eq!(
        ids(&found.documents, Mode::Micro),
        ["_micro/notes/2026-04-08-sample-note.md"]
    );
    assert_eq!(
        ids(&found.documents, Mode::Garden),
        ["_digital_garden/topic/index.md"]
    );
    assert_eq!(
        ids(&found.documents, Mode::Pages),
        ["assets/css/main.scss", "pages/profile.md"]
    );
    let statics: Vec<_> = found.statics.iter().map(DocId::as_str).collect();
    assert_eq!(statics, ["CNAME", "assets/img/a.png"]);
    assert!(found.excluded.contains(&(
        "_posts/2021/README.md".into(),
        "not named with a date, so not a post"
    )));
}

#[test]
fn hidden_names_are_left_out_unless_included() {
    let site = Site::new(
        "hidden",
        &[
            (".git/config", "x"),
            ("_site/index.html", FRONT),
            ("_layouts/article.html", FRONT),
            ("notes.md~", "x"),
            (".htaccess", "x"),
        ],
    );
    let found = discover(&site.0, &config(&[])).unwrap();
    assert_eq!(found.documents, []);
    let statics: Vec<_> = found.statics.iter().map(DocId::as_str).collect();
    assert_eq!(statics, [".htaccess"]);
    let excluded: Vec<_> = found.excluded.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(excluded, [".git", "_layouts", "_site", "notes.md~"]);
}

#[test]
fn exclude_patterns_match_paths_and_whole_directories() {
    let site = Site::new(
        "exclude",
        &[
            ("vendor/bundle/x.rb", "x"),
            ("LICENSE-example.txt", "x"),
            ("assets/json/notes_graph.json", "{}"),
            ("assets/json/other.json", "{}"),
        ],
    );
    let found = discover(
        &site.0,
        &config(&["/vendor", "LICENSE*", "/assets/json/notes_graph.json"]),
    )
    .unwrap();
    let statics: Vec<_> = found.statics.iter().map(DocId::as_str).collect();
    assert_eq!(statics, ["assets/json/other.json"]);
    assert_eq!(found.excluded.len(), 3);
}

#[test]
fn a_star_stays_within_one_segment() {
    let site = Site::new("star", &[("docs/a.md", "x"), ("docs/deep/b.md", "x")]);
    let found = discover(&site.0, &config(&["docs/*.md"])).unwrap();
    let statics: Vec<_> = found.statics.iter().map(DocId::as_str).collect();
    assert_eq!(statics, ["docs/deep/b.md"]);
}

#[test]
fn a_garden_file_without_front_matter_is_reported() {
    let site = Site::new("garden-static", &[("_digital_garden/img/map.png", "png")]);
    let problems = discover(&site.0, &config(&[])).unwrap_err();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("not published yet"));
}
