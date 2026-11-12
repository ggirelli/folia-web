//! Documents without a title, as Jekyll 4.4.1 titles them (checked against a Jekyll build of the
//! same file names: see `docs/internals/differences-from-jekyll.md`): a page has an empty title; a post, a garden
//! note and a micro entry are titled from their file name.
//!
//! ```sh
//! cargo test -p folia --test untitled
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use folia::build::build;
use folia::config::Config;

const TEMPLATE: &str = "[{{ page.title }}]{{ content | safe }}";

/// A throwaway site, committed to its own repository, and its output; removed when dropped.
struct Site(PathBuf);

impl Site {
    fn new(files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("folia-untitled-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let site = Self(root);
        let config = "time-zone = \"UTC\"\n[roots]\nblog = \"_posts\"\nmicro = \"_micro\"\n\
                      garden = \"_garden\"\n[site]\nurl = \"https://example.org\"\n\
                      title = \"T\"\ndescription = \"D\"\n";
        site.write("_folia/folia.toml", config);
        for template in ["page.html", "post.html", "garden_post.html"] {
            site.write(&format!("_folia/templates/{template}"), TEMPLATE);
        }
        for (path, text) in files {
            site.write(path, text);
        }
        for args in [
            &["init", "--quiet"][..],
            &["add", "."],
            &["commit", "--quiet", "-m", "site"],
        ] {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&site.0)
                .args([
                    "-c",
                    "user.name=Folia",
                    "-c",
                    "user.email=folia@example.com",
                ])
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        }
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn built(&self, dest: &Path) -> folia::build::Built {
        let config = Config::load(&self.0.join("_folia/folia.toml")).unwrap();
        build(
            &self.0,
            &config,
            dest,
            "2026-03-01T12:00:00Z".parse().unwrap(),
        )
        .unwrap()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_dir_all(self.0.with_extension("out"));
    }
}

fn read(dest: &Path, path: &str) -> String {
    std::fs::read_to_string(dest.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn untitled_documents_are_titled_as_jekyll_titles_them_and_titled_ones_are_unchanged() {
    let untitled = "---\nlayout: page\n---\nbody\n";
    let site = Site::new(&[
        (
            "pages/untitled.md",
            &untitled.replace("layout: page", "layout: page\npermalink: /untitled"),
        ),
        (
            "pages/empty.md",
            "---\nlayout: page\ntitle:\npermalink: /empty\n---\nbody\n",
        ),
        (
            "pages/named.md",
            "---\nlayout: page\ntitle: Named\npermalink: /named\n---\nbody\n",
        ),
        (
            "_posts/2026-01-02-my-first-post.md",
            "---\nlayout: post\n---\nbody\n",
        ),
        (
            "_posts/2026-01-03-titled-post.md",
            "---\nlayout: post\ntitle: Own\n---\nbody\n",
        ),
        (
            "_garden/topic/Mixed-CASE-name.md",
            "---\ndate: 2026-01-01\n---\nbody\n",
        ),
        (
            "_garden/topic/index.md",
            "---\ndate: 2026-01-01\n---\nbody\n",
        ),
        (
            "_garden/topic/2026-01-05-dated-note.md",
            "---\ndate: 2026-01-01\n---\nbody\n",
        ),
        (
            "_micro/notes/2026-01-12-a-note_x.md",
            "---\nnarrow_post: true\n---\nbody\n",
        ),
    ]);
    let dest = site.0.with_extension("out");
    let built = site.built(&dest);
    assert_eq!(
        built.skipped.len(),
        0,
        "nothing is skipped: {:?}",
        built.skipped
    );
    // Jekyll's `page.title` of each, from a Jekyll 4.4.1 build of the same names.
    for (output, title) in [
        ("untitled.html", ""),
        ("empty.html", ""),
        ("named.html", "Named"),
        ("blog/2026/01/02/my-first-post.html", "My First Post"),
        ("blog/2026/01/03/titled-post.html", "Own"),
        ("dg/topic/Mixed-CASE-name.html", "Mixed Case Name"),
        ("dg/topic/index.html", "Index"),
        ("dg/topic/2026-01-05-dated-note.html", "Dated Note"),
        ("micro/2026-01-12-a-note-x.html", "A Note_x"),
    ] {
        assert!(
            read(&dest, output).starts_with(&format!("[{title}]")),
            "{output}: {}",
            read(&dest, output)
        );
    }
}
