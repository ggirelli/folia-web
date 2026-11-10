use folia_core::content::FrontMatter;

use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
}

fn doc(path: &str, mode: Mode, front: &str, body: &str) -> SourceDoc {
    SourceDoc {
        id: DocId::new(path),
        mode,
        front: FrontMatter::parse(front).unwrap(),
        body: body.to_owned(),
    }
}

/// `count` posts, one a day from 2021-01-01, in `journal`; the first is also in `code`.
fn posts(count: u32) -> Vec<SourceDoc> {
    (1..=count)
        .map(|n| {
            let categories = if n == 1 {
                "[journal, code]"
            } else {
                "[journal]"
            };
            doc(
                &format!("_posts/2021/2021-01-{n:02}-p{n}.md"),
                Mode::Blog,
                &format!(
                    "title: P{n}\nauthor: gg\ncategories: {categories}\ncover:\n  path: /c{n}.webp"
                ),
                &format!("Post {n}.\n\n<!--more-->\n\nMore."),
            )
        })
        .collect()
}

fn index() -> SourceDoc {
    doc(
        "blog/index.html",
        Mode::Pages,
        "layout: default\ntitle: Blog\npagination:\n  enabled: true",
        "{% for post in paginator.posts %}{% endfor %}",
    )
}

fn built(docs: &[SourceDoc]) -> Vec<PageSpec> {
    let index = index();
    listings(
        docs,
        Some((&index, "blog_index.html")),
        Some("archive.html"),
        &clock(),
    )
    .unwrap()
}

fn titles(page: &PageSpec, key: &str) -> Vec<String> {
    page.context[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["title"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_index_has_nine_posts_a_page_newest_first_and_links_every_page() {
    let pages = built(&posts(10));
    let urls: Vec<&str> = pages.iter().map(|page| page.route.url()).collect();
    assert_eq!(
        urls,
        [
            "/blog/",
            "/blog/page2/",
            "/blog/category/journal/",
            "/blog/category/code/"
        ]
    );
    assert_eq!(
        titles(&pages[0], "items"),
        ["P10", "P9", "P8", "P7", "P6", "P5", "P4", "P3", "P2"]
    );
    assert_eq!(titles(&pages[1], "items"), ["P1"]);
    assert_eq!(pages[0].template, "blog_index.html");
    assert_eq!(
        pages[1].source.as_ref().unwrap().as_str(),
        "blog/index.html"
    );
    assert_eq!(
        pages[1].context["paginator"],
        json!({
            "page": 2,
            "total_pages": 2,
            "previous_page_path": "/blog/",
            "next_page_path": null,
            "paths": ["/blog/", "/blog/page2/"],
        })
    );
    assert_eq!(pages[0].context["title"], "Blog");
    assert_eq!(pages[0].context["layout"], "default");
}

#[test]
fn a_card_has_the_post_s_cover_summary_author_and_date() {
    let pages = built(&posts(1));
    assert_eq!(
        pages[0].context["items"][0],
        json!({
            "url": "/blog/2021/01/01/p1",
            "title": "P1",
            "cover_path": "/c1.webp",
            "box_position": null,
            "summary": "Post 1.\n\n",
            "author": "gg",
            "date": "2021-01-01T00:00:00+01:00",
        })
    );
    let long = (1..=40)
        .map(|n| format!("w{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    let docs = [doc(
        "_posts/2021/2021-01-01-l.md",
        Mode::Blog,
        "title: L",
        &long,
    )];
    let summary = built(&docs)[0].context["items"][0]["summary"].clone();
    assert_eq!(
        summary,
        (1..=30)
            .map(|n| format!("w{n}"))
            .collect::<Vec<_>>()
            .join(" ")
            + "..."
    );
}

#[test]
fn hidden_posts_are_not_listed_but_featured_ones_are_on_the_first_page_only() {
    let mut docs = posts(10);
    docs.push(doc(
        "_posts/2021/2021-02-01-h.md",
        Mode::Blog,
        "title: H\nhidden: true\nfeatured: true\ncategories: [journal]",
        "",
    ));
    docs.push(doc(
        "_posts/2021/2021-01-05-f.md",
        Mode::Blog,
        "title: F\nfeatured: true",
        "",
    ));
    let pages = built(&docs);
    assert!(!titles(&pages[0], "items").contains(&"H".to_owned()));
    assert_eq!(titles(&pages[0], "featured"), ["H", "F"]);
    assert_eq!(titles(&pages[1], "featured"), Vec::<String>::new());
    let journal = pages
        .iter()
        .find(|page| page.route.url() == "/blog/category/journal/")
        .unwrap();
    assert_eq!(
        titles(journal, "items")[0],
        "H",
        "an archive lists hidden posts"
    );
}

#[test]
fn an_archive_lists_its_category_s_posts_and_is_not_dark() {
    let pages = built(&posts(3));
    let code = pages
        .iter()
        .find(|page| page.route.url() == "/blog/category/code/")
        .unwrap();
    assert_eq!(code.template, "archive.html");
    assert_eq!(code.source, None);
    assert_eq!(titles(code, "items"), ["P1"]);
    assert_eq!(code.context["title"], "code");
    assert_eq!(code.context["type"], "category");
    assert_eq!(code.context["use_dark_layout"], false);
}

#[test]
fn same_date_posts_are_listed_by_path_descending() {
    let docs = [
        doc("_posts/2021/2021-01-01-a.md", Mode::Blog, "title: A", ""),
        doc("_posts/2021/2021-01-01-b.md", Mode::Blog, "title: B", ""),
    ];
    assert_eq!(titles(&built(&docs)[0], "items"), ["B", "A"]);
}

#[test]
fn without_their_settings_neither_listing_is_built() {
    assert_eq!(listings(&posts(3), None, None, &clock()).unwrap().len(), 0);
}

#[test]
fn groups_are_by_name_in_byte_order_with_their_posts_newest_first() {
    let docs = [
        doc(
            "_posts/2021/2021-01-01-a.md",
            Mode::Blog,
            "title: A\ntags: [indieweb, stem cells]\ncategories: [code]",
            "",
        ),
        doc(
            "_posts/2021/2021-01-02-b.md",
            Mode::Blog,
            "title: B\ntags: [R, indieweb]\ncategories: [journal, code]",
            "",
        ),
    ];
    let tags = groups(&docs, Grouping::Tags, &clock()).unwrap();
    let names: Vec<&str> = tags
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["R", "indieweb", "stem cells"]);
    assert_eq!(tags[1]["count"], 2);
    assert_eq!(tags[1]["items"][0]["title"], "B");
    assert_eq!(tags[1]["items"][1]["title"], "A");
    assert_eq!(tags[2]["anchor"], "stem -cells");
    let categories = groups(&docs, Grouping::Categories, &clock()).unwrap();
    assert_eq!(categories[0]["name"], "code");
    assert_eq!(categories[0]["count"], 2);
    assert_eq!(categories[1]["name"], "journal");
}
