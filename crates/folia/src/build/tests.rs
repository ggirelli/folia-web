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
        self.build_with(&config())
    }

    fn build_with(&self, config: &Config) -> Result<Built, Vec<Diagnostic>> {
        build(
            &self.site(),
            config,
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
            toml::from_str::<Author>(
                "name = \"Gabriele Girelli\"\ndisplay-name = \"GG\"\navatar = \"gg.webp\"\nweb = \"/\"\ndescription = \"Eclectic.\"\n",
            )
            .unwrap(),
        )]
        .into_iter()
        .collect(),
        slice: Some(Slice {
            pages: vec!["pages/profile.md".into()],
            posts: vec![SLICE_POST.into()],
            notes: vec![
                "_digital_garden/topic/index.md".into(),
                "_digital_garden/topic/note.md".into(),
            ],
            micro: Vec::new(),
        }),
        sass: crate::config::SassConfig::default(),
        micro: crate::config::MicroConfig::default(),
        blog: crate::config::BlogConfig::default(),
        garden: crate::config::GardenConfig::default(),
        feeds: None,
        search: None,
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

/// A whole-site build: no slice.
fn whole_site() -> Config {
    Config {
        slice: None,
        ..config()
    }
}

#[test]
fn without_a_slice_every_document_is_built_and_those_folia_cannot_build_are_skipped() {
    let fixture = Fixture::with(
        "whole",
        &[("base.html", BASE), ("page.html", PAGE)],
        &[
            (
                "pages/raw.html",
                "---\nlayout: page\ntitle: Raw\npermalink: /raw\n---\n<p>raw</p>\n",
            ),
            (
                "pages/liquid.html",
                "---\nlayout: page\ntitle: Liquid\npermalink: /liquid\n---\n{{ site.title }}\n",
            ),
            (
                "pages/fancy.md",
                "---\nlayout: fancy\ntitle: Fancy\npermalink: /fancy\nredirect_from:\n  - /old-fancy\n---\nFancy\n",
            ),
        ],
    );
    let built = fixture.build_with(&whole_site()).unwrap();
    assert_eq!(
        built.pages,
        [
            Route::html("/about").unwrap(),
            Route::html("/now").unwrap(),
            Route::html("/raw").unwrap()
        ]
    );
    assert_eq!(built.stubs, [Route::html("/blog/about").unwrap()]);
    assert!(
        fixture
            .read("now.html")
            .contains("<main><p>Not built yet.</p>\n</main>")
    );
    let skipped: Vec<String> = built.skipped.iter().map(ToString::to_string).collect();
    assert!(
        skipped.contains(&"pages/liquid.html: Liquid in an HTML page is not built yet".to_owned()),
        "{skipped:#?}"
    );
    assert!(
        skipped.iter().any(|s| s
            .starts_with("pages/fancy.md: these templates cannot be used:\n  fancy.html: missing")),
        "{skipped:#?}"
    );
    assert!(!fixture.dest().join("fancy.html").exists());
    assert!(
        !fixture.dest().join("old-fancy.html").exists(),
        "no stub points to a page not built"
    );
}

#[test]
fn without_a_slice_a_problem_of_no_document_still_fails_the_build() {
    let fixture = Fixture::new("whole-failing", &[("base.html", BASE), ("page.html", PAGE)]);
    let config = Config {
        time_zone: "Nowhere/Nothing".into(),
        ..whole_site()
    };
    assert!(fixture.build_with(&config).is_err());
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

#[test]
fn an_author_page_shows_its_author_s_fields_and_covers_and_an_unknown_author_is_skipped() {
    const AUTHOR: &str = "{% extends \"base.html\" %}{% block body %}\
        {% set author = site.authors[page.author] %}<h1>{{ author.name }}</h1>\
        {% if author.nickname %}<p>{{ author.nickname }}</p>{% endif %}\
        {% if author.facebook %}<a href=\"https://www.facebook.com/{{ author.facebook }}\">fb</a>{% endif %}\
        {% if author.hacker_news %}<a href=\"https://news.ycombinator.com/user?id={{ author.hacker_news }}\">hn</a>{% endif %}\
        {{ content | safe }}<script data-covers='{{ author.covers_json | safe }}'></script>{% endblock %}";
    let fixture = Fixture::with(
        "author",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("author.html", AUTHOR),
        ],
        &[
            (
                "me/index.html",
                "---\nlayout: author\ntitle: gg\nauthor: gg\npermalink: /\n---\n<p>Hi.</p>\n",
            ),
            (
                "me/ghost.html",
                "---\nlayout: author\ntitle: ghost\nauthor: nobody\npermalink: /ghost\n---\n<p>Boo.</p>\n",
            ),
        ],
    );
    let mut config = whole_site();
    config.authors.insert(
        "gg".to_owned(),
        toml::from_str(
            "name = \"Gabriele Girelli\"\ndisplay-name = \"GG\"\nnickname = \"GG\"\navatar = \"gg.webp\"\n\
             web = \"/\"\ndescription = \"Eclectic.\"\nhacker-news = \"gg\"\n\
             [[covers]]\npath = \"/c.webp\"\ndescription = \"A <a href=\\\"https://x.org\\\">lake</a> I took\\n\"\n",
        )
        .unwrap(),
    );
    let built = fixture.build_with(&config).unwrap();
    assert!(built.pages.contains(&Route::html("/").unwrap()));
    assert_eq!(
        fixture.read("index.html"),
        "<title>gg | Filopoetica</title><h1>Gabriele Girelli</h1><p>GG</p>\
         <a href=\"https://news.ycombinator.com/user?id=gg\">hn</a><p>Hi.</p>\n\
         <script data-covers='[{\"path\":\"/c.webp\",\"description\":\"A <a href=\\\"https://x.org\\\">lake</a> I took\\n\"}]'></script>\
         <a href=\"/c#journal\">journal (2)</a><a href=\"/c#cinema\">cinema (1)</a><footer>2026</footer>"
    );
    let skipped: Vec<String> = built.skipped.iter().map(ToString::to_string).collect();
    assert!(
        skipped.contains(
            &"me/ghost.html: the author nobody is not one of the configuration's authors"
                .to_owned()
        ),
        "{skipped:#?}"
    );
    assert!(!fixture.dest().join("ghost.html").exists());
}

#[test]
fn a_micro_note_is_built_with_the_configured_author() {
    const NOTE: &str = "{% extends \"base.html\" %}{% block body %}\
        <p>{{ page.author }} {{ page.date }} {{ page.corner_backlink.url }}</p>{{ content | safe }}{% endblock %}";
    let fixture = Fixture::with(
        "micro",
        &[("base.html", BASE), ("page.html", NOTE)],
        &[(
            "_micro/notes/2025-06-11-foo_bar.md",
            "---\ntitle: Foo\n---\nBar.\n",
        )],
    );
    let mut config = whole_site();
    config.micro.author = Some("gg".to_owned());
    let built = fixture.build_with(&config).unwrap();
    assert!(
        built
            .pages
            .contains(&Route::html("/micro/2025-06-11-foo-bar").unwrap()),
        "{:?}",
        built.pages
    );
    assert!(
        fixture
            .read("micro/2025-06-11-foo-bar.html")
            .contains("<p>gg 2025-06-11T00:00:00+02:00 &#x2f;micro</p><p>Bar.</p>"),
        "{}",
        fixture.read("micro/2025-06-11-foo-bar.html")
    );
}

#[test]
fn a_claimed_micro_listing_lists_its_entries_and_keeps_its_stubs() {
    const LISTING: &str = "{% extends \"base.html\" %}{% block body %}\
        {% for item in page.items %}<li>{{ item.title }}</li>{% endfor %}{% endblock %}";
    let fixture = Fixture::with(
        "listing",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("listing.html", LISTING),
        ],
        &[
            ("_micro/notes/2025-06-11-a.md", "---\ntitle: A\n---\nA.\n"),
            ("_micro/notes/2025-06-12-b.md", "---\ntitle: B\n---\nB.\n"),
            (
                "blog/microblog.html",
                "---\nlayout: page\ntitle: Micro\npermalink: /micro\nredirect_from:\n  - /social\n---\n{% for item in site.micro %}{% endfor %}\n",
            ),
        ],
    );
    let mut config = whole_site();
    config.micro.listings.push(crate::config::MicroListing {
        source: "blog/microblog.html".to_owned(),
        template: "listing.html".to_owned(),
        kinds: vec!["notes".to_owned()],
    });
    let built = fixture.build_with(&config).unwrap();
    assert!(built.stubs.contains(&Route::html("/social").unwrap()));
    assert_eq!(
        fixture.read("micro.html"),
        "<title>Micro | Filopoetica</title><li>B</li><li>A</li>\
         <a href=\"/c#journal\">journal (2)</a><a href=\"/c#cinema\">cinema (1)</a><footer>2026</footer>"
    );
}

#[test]
fn the_claimed_blog_index_and_the_archives_are_built_by_the_blog() {
    const INDEX: &str = "{% extends \"base.html\" %}{% block body %}\
        {{ page.paginator.page }}/{{ page.paginator.total_pages }}:\
        {% for post in page.items %}<li>{{ post.title }}</li>{% endfor %}{% endblock %}";
    const ARCHIVE: &str = "{% extends \"base.html\" %}{% block body %}\
        {{ page.type }} {{ page.title }}:{% for post in page.items %}<li>{{ post.url }}</li>{% endfor %}{% endblock %}";
    let fixture = Fixture::with(
        "blog-index",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("index.html", INDEX),
            ("archive.html", ARCHIVE),
        ],
        &[(
            "blog/index.html",
            "---\nlayout: default\ntitle: Blog\npagination:\n  enabled: true\n---\n{% for post in paginator.posts %}{% endfor %}\n",
        )],
    );
    let mut config = whole_site();
    config.blog.index = Some(crate::config::ClaimedPage {
        source: "blog/index.html".to_owned(),
        template: "index.html".to_owned(),
    });
    config.blog.archive_template = Some("archive.html".to_owned());
    let built = fixture.build_with(&config).unwrap();
    assert!(
        built.skipped.iter().all(|s| s
            .source
            .as_ref()
            .is_none_or(|id| id.as_str() != "blog/index.html")),
        "{:?}",
        built.skipped
    );
    assert!(
        fixture
            .read("blog/index.html")
            .contains("1/1:<li>B</li><li>A</li>")
    );
    assert!(
        fixture
            .read("blog/category/cinema/index.html")
            .contains("category cinema:<li>&#x2f;blog&#x2f;2014&#x2f;01&#x2f;08&#x2f;b</li>")
    );
}

#[test]
fn the_claimed_tags_page_lists_the_posts_by_tag() {
    const TAGS: &str = "{% extends \"base.html\" %}{% block body %}\
        {% for group in page.groups %}<h2>{{ group.name }} ({{ group.count }})</h2>{% endfor %}{% endblock %}";
    let fixture = Fixture::with(
        "tags",
        &[
            ("base.html", BASE),
            ("page.html", PAGE),
            ("tags.html", TAGS),
        ],
        &[
            (
                "_posts/2020/2020-01-01-t.md",
                "---\ntags: [note, film]\n---\nT\n",
            ),
            (
                "blog/tags.html",
                "---\nlayout: default\ntitle: Tags\npermalink: /blog/tags.html\nadditional_body_class: tags-index\n---\n{% for tag in site.tags %}{% endfor %}\n",
            ),
        ],
    );
    let mut config = whole_site();
    config.blog.tags = Some(crate::config::ClaimedPage {
        source: "blog/tags.html".to_owned(),
        template: "tags.html".to_owned(),
    });
    fixture.build_with(&config).unwrap();
    assert!(
        fixture
            .read("blog/tags.html")
            .contains("<h2>film (1)</h2><h2>note (1)</h2>")
    );
}

#[test]
fn the_planned_feeds_are_written_with_the_entries_built_only() {
    const NOTE: &str =
        "{% extends \"base.html\" %}{% block body %}{{ content | safe }}{% endblock %}";
    let fixture = Fixture::with(
        "feeds",
        &[("base.html", BASE), ("page.html", NOTE)],
        &[(
            "_micro/notes/2025-06-11-foo_bar.md",
            "---\ntitle: Foo\n---\nBar.\n",
        )],
    );
    let mut config = whole_site();
    let without = fixture.build_with(&config).unwrap();
    assert!(!fixture.dest().join("micro/feed.xml").exists());
    config.micro.author = Some("gg".to_owned());
    config.feeds = Some(
        toml::from_str(
            "generator = { name = \"Jekyll\", uri = \"https://jekyllrb.com/\", version = \"4.4.1\" }",
        )
        .unwrap(),
    );
    let built = fixture.build_with(&config).unwrap();
    let feeds = [
        "/blog/feed.xml",
        "/micro/feed.xml",
        "/feed/micro/notes.xml",
        "/feed/micro/likes.xml",
        "/feed/micro/bookmarks.xml",
        "/feed/micro/rsvps.xml",
        "/dg/feed.xml",
    ];
    for feed in feeds {
        assert!(
            built.artifacts.contains(&Route::file(feed).unwrap()),
            "{feed}: {:?}",
            built.artifacts
        );
    }
    assert_eq!(without.not_built - built.not_built, feeds.len());
    let micro = fixture.read("micro/feed.xml");
    assert!(
        micro.contains(
            "<title type=\"html\">Filopoetica | Micro</title><subtitle>An eclectic blog.</subtitle>\
             <entry><title type=\"html\">Foo</title>\
             <link href=\"https://ggirelli.info/micro/2025-06-11-foo-bar\" rel=\"alternate\" type=\"text/html\" title=\"Foo\" />\
             <published>2025-06-11T00:00:00+02:00</published><updated>2026-01-04T13:36:24+01:00</updated>\
             <id>https://ggirelli.info/micro/foo_bar</id>"
        ),
        "{micro}"
    );
    assert!(micro.contains("<author><name>gg</name></author><category term=\"notes\" />"));
    assert_eq!(
        fixture
            .read("feed/micro/notes.xml")
            .matches("<entry>")
            .count(),
        1
    );
    assert_eq!(
        fixture
            .read("feed/micro/likes.xml")
            .matches("<entry>")
            .count(),
        0
    );
    // The posts' template is missing, so they are skipped, and in no feed; the empty feed is
    // as recent as the site's latest entry.
    let blog = fixture.read("blog/feed.xml");
    assert!(!blog.contains("<entry>"), "{blog}");
    assert!(
        blog.contains("<updated>2026-01-04T13:36:24+01:00</updated>"),
        "{blog}"
    );
}

#[test]
fn the_sitemap_lists_the_pages_and_documents_and_static_files_but_404_and_stubs() {
    const NOTE: &str =
        "{% extends \"base.html\" %}{% block body %}{{ content | safe }}{% endblock %}";
    let fixture = Fixture::with(
        "sitemap",
        &[("base.html", BASE), ("page.html", NOTE)],
        &[
            (
                "_micro/notes/2025-06-11-foo_bar.md",
                "---\ntitle: Foo\n---\nBar.\n",
            ),
            ("404.html", "---\nlayout: page\ntitle: Lost\n---\nLost.\n"),
            ("assets/pdf/paper.pdf", "pdf"),
            ("yandex_0.html", "<html>static</html>\n"),
        ],
    );
    let mut config = whole_site();
    config.micro.author = Some("gg".to_owned());
    let built = fixture.build_with(&config).unwrap();
    assert!(
        built
            .artifacts
            .contains(&Route::file("/sitemap.xml").unwrap())
    );
    let sitemap = fixture.read("sitemap.xml");
    let locs: Vec<&str> = sitemap
        .split("<loc>https://ggirelli.info")
        .skip(1)
        .map(|rest| rest.split('<').next().unwrap())
        .collect();
    // `/blog/about` is a stub, and `/404.html` is never listed.
    assert!(locs.contains(&"/about"), "{locs:?}");
    assert!(locs.contains(&"/micro/2025-06-11-foo-bar"), "{locs:?}");
    assert!(!locs.contains(&"/blog/about"), "{locs:?}");
    assert!(!locs.iter().any(|loc| loc.contains("404")), "{locs:?}");
    assert_eq!(
        &locs[locs.len() - 2..],
        ["/assets/pdf/paper.pdf", "/yandex_0.html"]
    );
    assert!(
        sitemap.contains(
            "<loc>https://ggirelli.info/micro/2025-06-11-foo-bar</loc>\n\
             <lastmod>2026-01-04T13:36:24+01:00</lastmod>\n</url>"
        ),
        "{sitemap}"
    );
    assert!(
        sitemap.contains(
            "<loc>https://ggirelli.info/assets/pdf/paper.pdf</loc>\n\
             <lastmod>2026-01-04T13:36:24+01:00</lastmod>\n</url>"
        ),
        "{sitemap}"
    );
}

#[test]
fn the_search_script_lists_the_pages_stubs_and_redirects_then_its_fragment() {
    let fixture = Fixture::with(
        "search",
        &[("base.html", BASE), ("page.html", PAGE)],
        &[
            (
                "assets/js/search.js",
                "---\nlayout: null\nsitemap: false\n---\nvar documents = [{% for page in site.pages %}{% endfor %}];\n",
            ),
            ("_folia/search.js", "\nvar lunr_index = 1;\n"),
        ],
    );
    let mut config = whole_site();
    config.search = Some(crate::search::SearchConfig {
        source: "assets/js/search.js".to_owned(),
        fragment: "_folia/search.js".to_owned(),
    });
    let built = fixture.build_with(&config).unwrap();
    assert!(
        !built
            .skipped
            .iter()
            .any(|s| s.to_string().contains("search.js")),
        "{:?}",
        built.skipped
    );
    let js = fixture.read("assets/js/search.js");
    assert!(js.starts_with("var documents = [\n"), "{js}");
    assert!(js.ends_with("\n];\n\nvar lunr_index = 1;\n"), "{js}");
    assert!(
        js.contains("{\"id\": 0, \"url\": \"https://ggirelli.info/about\", \"title\": \"About me\", \"body\": \"Other things: \"}"),
        "{js}"
    );
    assert!(
        js.contains("\"url\": \"https://ggirelli.info/blog/about\", \"title\": \"\""),
        "{js}"
    );
    assert!(
        js.contains(
            "\"url\": \"https://ggirelli.info/redirects.json\", \"title\": \"\", \"body\": \"{"
        ),
        "{js}"
    );
    assert!(!fixture.dest().join("_folia").exists());
}
