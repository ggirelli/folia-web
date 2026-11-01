use std::path::PathBuf;
use std::process::Command;

use super::*;
use crate::config::{Author, Identity, Roots};

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
    }
}

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
        SLICE_POSTS[0],
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
            .contains(&Route::html("/blog/2021/08/08/webmentions").unwrap())
    );
    assert_eq!(
        fixture.read("blog/2021/08/08/webmentions.html"),
        "Webmentions by GG, 08 Aug 2021 (2026-01-04T13:36:24+01:00): left #one \
         after B, before Later\
         <main><h3 id=\"one\">One</h3>\n\n<h4 id=\"two\">Two</h4>\n</main>"
    );
    let entry = built
        .entries
        .iter()
        .find(|entry| entry.doc.as_str() == SLICE_POSTS[0])
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
