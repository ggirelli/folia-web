use std::path::PathBuf;
use std::process::Command;

use super::*;
use crate::config::{Author, Identity, Roots, Slice};

/// A throwaway site, committed to its own repository on 2026-01-04 at 13:36:24 in Stockholm,
/// with an output directory next to it; removed when dropped.
struct Fixture(PathBuf);

const BASE: &str = "<title>{{ page.title }} | {{ site.title }}</title>\
                    {% block body %}{% endblock %}\
                    {% for category in site.categories %}<a href=\"/c#{{ category.anchor }}\">{{ category.name }} ({{ category.count }})</a>{% endfor %}\
                    <footer>{{ site.year }}</footer>";
const PAGE: &str = "{% extends \"base.html\" %}{% block body %}<main>{{ content | safe }}</main>\
                    <time datetime=\"{{ page.modified }}\">{{ page.modified | date(\"%b %-d, %Y\") }}</time>{% endblock %}";

impl Fixture {
    fn new(test: &str, templates: &[(&str, &str)]) -> Self {
        Self::with(test, templates, &[])
    }

    /// A fixture with more source files, `files`, as paths and texts.
    fn with(test: &str, templates: &[(&str, &str)], files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("folia-build-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self(root);
        fixture.write("pages/profile.md", "---\nlayout: page\ntitle: About me\npermalink: /about\nredirect_from:\n  - /blog/about\n---\n\n### Other things\n");
        fixture.write(
            "pages/current.md",
            "---\nlayout: page\ntitle: Now\npermalink: /now\n---\nNot built yet.\n",
        );
        fixture.write(
            "_posts/2013/2013-05-16-a.md",
            "---\ncategories: [journal]\n---\nA\n",
        );
        fixture.write(
            "_posts/2014/2014-01-08-b.md",
            "---\ncategories: [journal, cinema]\n---\nB\n",
        );
        fixture.write("assets/images/cc.png", "png");
        fixture.write("CNAME", "ggirelli.info\n");
        for (name, text) in templates {
            fixture.write(&format!("_folia/templates/{name}"), text);
        }
        for (path, text) in files {
            fixture.write(path, text);
        }
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "--quiet", "-m", "site"]);
        fixture
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.site().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
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
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn site(&self) -> PathBuf {
        self.0.join("site")
    }

    fn dest(&self) -> PathBuf {
        self.0.join("out")
    }

    fn read(&self, output: &str) -> String {
        std::fs::read_to_string(self.dest().join(output)).unwrap()
    }

    fn build(&self) -> Result<Built, Vec<Diagnostic>> {
        build(
            &self.site(),
            &config(),
            &self.dest(),
            "2026-10-25T14:43:51Z".parse().unwrap(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn config() -> Config {
    Config {
        time_zone: "Europe/Stockholm".into(),
        include: vec![],
        exclude: vec![],
        templates_dir: "_folia/templates".into(),
        roots: Roots {
            blog: "_posts".into(),
            micro: "_micro".into(),
            garden: "_digital_garden".into(),
        },
        site: Identity {
            url: "https://ggirelli.info".into(),
            title: "Filopoetica".into(),
            description: "An eclectic blog.".into(),
        },
        authors: [(
            "gg".to_owned(),
            Author {
                name: "Gabriele Girelli".into(),
                display_name: "GG".into(),
                avatar: "gg.webp".into(),
                web: "/".into(),
                description: "Eclectic.".into(),
            },
        )]
        .into_iter()
        .collect(),
        slice: Slice {
            pages: vec!["pages/profile.md".into()],
            posts: vec![SLICE_POST.into()],
            notes: vec![
                "_digital_garden/topic/index.md".into(),
                "_digital_garden/topic/note.md".into(),
            ],
        },
    }
}

/// The post the slice builds.
const SLICE_POST: &str = "_posts/2021/2021-08-08-sample-post.md";

#[test]
fn the_slice_builds_its_page_its_stub_and_the_static_files() {
    let fixture = Fixture::new("slice", &[("base.html", BASE), ("page.html", PAGE)]);
    let built = fixture.build().unwrap();
    assert_eq!(built.pages, [Route::html("/about").unwrap()]);
    assert_eq!(built.stubs, [Route::html("/blog/about").unwrap()]);
    assert_eq!(built.statics, 2);
    assert_eq!(
        fixture.read("about.html"),
        "<title>About me | Filopoetica</title>\
         <main><h3 id=\"other-things\">Other things</h3>\n</main>\
         <time datetime=\"2026-01-04T13:36:24+01:00\">Jan 4, 2026</time>\
         <a href=\"/c#journal\">journal (2)</a><a href=\"/c#cinema\">cinema (1)</a>\
         <footer>2026</footer>"
    );
    assert_eq!(
        fixture.read("blog/about.html"),
        redirect_stub("https://ggirelli.info/about")
    );
    assert_eq!(fixture.read("CNAME"), "ggirelli.info\n");
    assert_eq!(fixture.read("assets/images/cc.png"), "png");
    assert!(!fixture.dest().join("now.html").exists());
    assert!(built.not_built > 0);
}

/// What a post's template shows of it, and of the site.
const POST: &str = "{{ page.title }} by {{ site.authors[page.author].display_name }}, \
                    {{ page.date | date(\"%d %b %Y\") }} ({{ page.modified }}): \
                    {{ page.toc.placement }} {% for entry in page.toc.entries %}#{{ entry.id }} {% endfor %}\
                    after {{ page.previous.title }}, before {{ page.next.title }}\
                    <main>{{ content | safe }}</main>";

#[test]
fn the_slice_builds_its_post_with_its_dates_contents_and_neighbours() {
    let fixture = Fixture::new(
        "post",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("post.html", POST),
        ],
    );
    fixture.write(
        SLICE_POST,
        "---\ntitle: Webmentions\nauthor: gg\ntoc:\n  enabled: true\n  position: left\n---\n### One\n\n#### Two\n",
    );
    fixture.write(
        "_posts/2021/2021-08-17-later.md",
        "---\ntitle: Later\n---\nL\n",
    );
    fixture.git(&["add", "."]);
    fixture.git(&["commit", "--quiet", "-m", "posts"]);
    let built = fixture.build().unwrap();
    assert!(
        built
            .pages
            .contains(&Route::html("/blog/2021/08/08/sample-post").unwrap())
    );
    assert_eq!(
        fixture.read("blog/2021/08/08/sample-post.html"),
        "Webmentions by GG, 08 Aug 2021 (2026-01-04T13:36:24+01:00): left #one \
         after B, before Later\
         <main><h3 id=\"one\">One</h3>\n\n<h4 id=\"two\">Two</h4>\n</main>"
    );
    let entry = built
        .entries
        .iter()
        .find(|entry| entry.doc.as_str() == SLICE_POST)
        .unwrap();
    assert_eq!(
        entry.published,
        Some("2021-08-07T22:00:00Z".parse().unwrap())
    );
    assert_eq!(
        entry.modified.map(|m| m.from),
        Some(folia_core::content::ModifiedFrom::Git)
    );
}

#[test]
fn a_missing_template_fails_before_anything_is_written() {
    let fixture = Fixture::new("missing", &[("base.html", BASE)]);
    let problems = fixture.build().unwrap_err();
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].message,
        "these templates cannot be used:\n  page.html: missing; used by /about"
    );
    assert!(!fixture.dest().exists());
}

#[test]
fn a_page_that_fails_to_render_leaves_the_previous_build() {
    let fixture = Fixture::new("render", &[("page.html", "{% extends \"default.html\" %}")]);
    std::fs::create_dir_all(fixture.dest()).unwrap();
    std::fs::write(fixture.dest().join("about.html"), "previous").unwrap();
    let problems = fixture.build().unwrap_err();
    assert!(
        problems[0]
            .message
            .starts_with("/about cannot be rendered with page.html: template not found"),
        "{}",
        problems[0]
    );
    assert_eq!(fixture.read("about.html"), "previous");
}

#[test]
fn a_category_anchor_is_escaped_as_liquid_escaped_it() {
    assert_eq!(anchor("journal"), "journal");
    assert_eq!(anchor("open source"), "open%20source");
    assert_eq!(anchor("café"), "caf%C3%A9");
}

/// What a note's template shows of it.
const NOTE: &str = "{{ page.title }}: {% for backlink in page.backlinks %}{{ backlink.title }} {% endfor %}\
                    <main>{{ content | safe }}</main>";

/// The slice's two notes, and a third dated after the build, which both link to and from.
const NOTES: [(&str, &str); 3] = [
    (
        "_digital_garden/topic/index.md",
        "---\ntitle: Coding\ndate: 2023-01-30 12:20\n---\nTips for [[Rust]] and [[Later]].\n",
    ),
    (
        "_digital_garden/topic/note.md",
        "---\ntitle: Rust\ndate: 2025-11-08 12:20\n---\nSee [[First setup: rust]] and [[Coding]].\n",
    ),
    (
        "_digital_garden/later.md",
        "---\ntitle: Later\ndate: 2026-12-01\n---\nAbout [[Rust]].\n",
    ),
];

#[test]
fn the_slice_writes_its_notes_and_the_graph_and_returns_the_warnings() {
    let fixture = Fixture::with(
        "garden",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("garden_post.html", NOTE),
        ],
        &NOTES,
    );
    let built = fixture.build().unwrap();
    assert_eq!(
        built.pages,
        [
            Route::html("/about").unwrap(),
            Route::html("/dg/topic/index").unwrap(),
            Route::html("/dg/topic/note").unwrap(),
        ]
    );
    assert_eq!(
        built.artifacts,
        [Route::file("/assets/json/notes_graph.json").unwrap()]
    );
    assert_eq!(
        fixture.read("assets/json/notes_graph.json"),
        r#"{"nodes":[{"id":"/dg/topic/index","path":"/dg/topic/index","label":"Coding"},{"id":"/dg/topic/note","path":"/dg/topic/note","label":"Rust"},{"id":"/dg/later","path":"/dg/later","label":"Later"}],"edges":[{"source":"/dg/topic/note","target":"/dg/topic/index"},{"source":"/dg/topic/index","target":"/dg/topic/note"},{"source":"/dg/later","target":"/dg/topic/note"},{"source":"/dg/topic/index","target":"/dg/later"}]}"#
    );
    assert_eq!(
        fixture.read("dg/topic/note.html"),
        "Rust: Coding Later <main><p>See <span class=\"broken-link\">First setup: \
         rust</span> and <a href=\"/dg/topic/index\" class=\"internal-link\">Coding</a>.</p>\n</main>"
    );
    // The note dated after the build is linked to, but its page is not written.
    assert!(
        fixture
            .read("dg/topic/index.html")
            .contains("href=\"/dg/later\"")
    );
    assert!(!fixture.dest().join("dg/later.html").exists());
    let warnings: Vec<String> = built.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "_digital_garden/topic/note.md: the wiki-link [[First setup: rust]] names no note, so it \
          is shown as broken"
        ]
    );
}

#[test]
fn an_output_built_twice_is_refused_not_overwritten() {
    let mut outputs = BTreeMap::new();
    add(
        &mut outputs,
        "a.html".into(),
        Content::Bytes(b"one".to_vec()),
    )
    .unwrap();
    let clash = add(
        &mut outputs,
        "a.html".into(),
        Content::Bytes(b"two".to_vec()),
    )
    .unwrap_err();
    assert_eq!(clash.to_string(), "a.html is built twice");
    assert!(matches!(&outputs["a.html"], Content::Bytes(bytes) if bytes == b"one"));
}
