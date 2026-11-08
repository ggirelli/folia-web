use folia_core::content::{FrontMatter, Mode, SiteClock, SourceDoc};

use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
}

fn post(path: &str, front: &str) -> SourceDoc {
    SourceDoc {
        id: DocId::new(path),
        mode: Mode::Blog,
        front: FrontMatter::parse(front).unwrap(),
        body: String::new(),
    }
}

fn urls(outputs: &[PlannedOutput], generator: Option<&str>) -> Vec<String> {
    outputs
        .iter()
        .filter(|o| o.generator == generator)
        .map(|o| o.route.url().to_owned())
        .collect()
}

#[test]
fn a_post_url_keeps_the_case_of_its_file_name() {
    let outputs = routes(
        &[post("_posts/2023/2023-05-13-Sample-Trip-pt1.md", "")],
        &clock(),
    )
    .unwrap();
    assert_eq!(outputs[0].route.url(), "/blog/2023/05/13/Sample-Trip-pt1");
    assert_eq!(
        outputs[0].route.output_path(),
        "blog/2023/05/13/Sample-Trip-pt1.html"
    );
    assert_eq!(
        outputs[0].source,
        Some(DocId::new("_posts/2023/2023-05-13-Sample-Trip-pt1.md"))
    );
}

#[test]
fn unpublished_and_future_posts_are_left_out() {
    let docs = [
        post("_posts/2026/2026-10-25-today.md", ""),
        post("_posts/2026/2026-10-26-tomorrow.md", ""),
        post("_posts/2021/2021-01-01-draft.md", "published: false"),
    ];
    let outputs = routes(&docs, &clock()).unwrap();
    assert_eq!(urls(&outputs, None), ["/blog/2026/10/25/today"]);
}

#[test]
fn listing_pages_count_only_listed_posts() {
    let mut docs: Vec<_> = (1..=10)
        .map(|day| post(&format!("_posts/2021/2021-01-{day:02}-p.md"), ""))
        .collect();
    assert_eq!(
        urls(&routes(&docs, &clock()).unwrap(), Some("pagination")),
        ["/blog/page2/"]
    );

    docs[9] = post("_posts/2021/2021-01-10-p.md", "hidden: true");
    assert_eq!(
        urls(&routes(&docs, &clock()).unwrap(), Some("pagination")),
        Vec::<String>::new()
    );
}

#[test]
fn a_listing_page_is_written_as_a_directory_index_of_its_template() {
    let docs: Vec<_> = (1..=19)
        .map(|day| post(&format!("_posts/2021/2021-01-{day:02}-p.md"), ""))
        .collect();
    let outputs = routes(&docs, &clock()).unwrap();
    let pages: Vec<_> = outputs
        .iter()
        .filter(|o| o.generator == Some("pagination"))
        .collect();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1].route.output_path(), "blog/page3/index.html");
    assert_eq!(pages[1].source, Some(DocId::new(LISTING_TEMPLATE)));
}

#[test]
fn categories_come_from_both_keys_and_are_slugified() {
    let docs = [
        post(
            "_posts/2021/2021-01-01-a.md",
            "categories: [Self Hosting, code]",
        ),
        post("_posts/2021/2021-01-02-b.md", "category: code"),
    ];
    let archives = urls(&routes(&docs, &clock()).unwrap(), Some("category archive"));
    assert_eq!(
        archives,
        ["/blog/category/self-hosting/", "/blog/category/code/"]
    );
}

#[test]
fn only_plain_tags_get_a_feed() {
    let docs = [post(
        "_posts/2021/2021-01-01-a.md",
        "tags: R self-hosting LLM_2",
    )];
    let feeds = urls(&routes(&docs, &clock()).unwrap(), Some("tag feed"));
    assert_eq!(feeds, ["/blog/tag/R.xml", "/blog/tag/LLM_2.xml"]);
}

#[test]
fn redirects_are_kept_on_the_post() {
    let outputs = routes(
        &[post(
            "_posts/2021/2021-01-01-a.md",
            "redirect_from: [/old, /older/]",
        )],
        &clock(),
    )
    .unwrap();
    let from: Vec<_> = outputs[0].redirect_from.iter().map(Route::url).collect();
    assert_eq!(from, ["/old", "/older/"]);
}

#[test]
fn an_undated_post_is_reported() {
    let problems = routes(&[post("_posts/notes.md", "")], &clock()).unwrap_err();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("start with its date"));
}

#[test]
fn categories_are_counted_in_the_order_posts_first_use_them() {
    let counts = category_counts(
        &[
            post(
                "_posts/2014/2014-01-08-b.md",
                "categories: [journal, cinema]",
            ),
            post(
                "_posts/2013/2013-08-07-a.md",
                "categories: [science, cinema]",
            ),
            post("_posts/2013/2013-08-07-0.md", "categories: [journal]"),
            post("_posts/2030/2030-01-01-future.md", "categories: [later]"),
            post(
                "_posts/2014/2014-02-01-c.md",
                "categories: [noise]\npublished: false",
            ),
        ],
        &clock(),
    )
    .unwrap();
    assert_eq!(
        counts,
        [
            ("journal".to_owned(), 2),
            ("science".to_owned(), 1),
            ("cinema".to_owned(), 2),
        ]
    );
}

const SAMPLE_POST: &str = "_posts/2021/2021-08-08-sample-post.md";

fn sample_post() -> SourceDoc {
    SourceDoc {
        body: "Intro.\n\n### One *a*\n\n#### Two\n\n### Three\n".into(),
        ..post(
            SAMPLE_POST,
            "title: \"Webmentions: letting Disqus go\"\nauthor: gg\ncover:\n  path: /c.webp\n  position: behind\n  unsplash: lunarts\ncategories: [code, announcement]\ntags: [webmentions, blog]\nlast_modified_at: 2021-08-24\ntoc:\n  enabled: true\n  position: left\nsyndications:\n  x-twitter: https://x\n  github: https://g\n",
        )
    }
}

fn build_one(docs: &[SourceDoc]) -> Result<ModeOutput, Vec<Diagnostic>> {
    build(
        docs,
        &[SAMPLE_POST],
        &folia_core::content::LastCommits::default(),
        &clock(),
    )
}

use folia_core::publish::ModeOutput;

#[test]
fn a_post_is_built_with_its_dates_and_what_its_template_shows() {
    let docs = [
        post(
            "_posts/2021/2021-07-12-sample-theme.md",
            "title: A new skin!",
        ),
        sample_post(),
        post("_posts/2021/2021-08-17-sample-speed.md", ""),
    ];
    let output = build_one(&docs).unwrap();
    let ([entry], [page]) = (&output.entries[..], &output.pages[..]) else {
        panic!("one entry and one page")
    };
    assert_eq!(entry.route.url(), "/blog/2021/08/08/sample-post");
    assert_eq!(
        clock().xmlschema(entry.published.unwrap()),
        "2021-08-08T00:00:00+02:00"
    );
    let modified = entry.modified.unwrap();
    assert_eq!(
        modified.from,
        folia_core::content::ModifiedFrom::FrontMatter
    );
    assert_eq!(page.template, "post.html");
    let context = &page.context;
    assert_eq!(context["date"], "2021-08-08T00:00:00+02:00");
    assert_eq!(context["modified"], "2021-08-24T00:00:00+02:00");
    assert_eq!(context["description"], "Intro.One aTwoThree");
    assert_eq!(context["author"], "gg");
    assert_eq!(
        context["categories"],
        serde_json::json!([{"name": "announcement", "anchor": "announcement"}, {"name": "code", "anchor": "code"}])
    );
    assert_eq!(context["tags"][0]["name"], "blog");
    assert_eq!(
        context["previous"],
        serde_json::json!({"url": "/blog/2021/07/12/sample-theme", "title": "A new skin!"})
    );
    assert_eq!(context["next"]["title"], "Sample Speed");
    assert_eq!(
        context["toc"],
        serde_json::json!({"placement": "left", "title": null, "entries": [
            {"id": "one-a", "html": "One <em>a</em>", "children": [{"id": "two", "html": "Two", "children": []}]},
            {"id": "three", "html": "Three", "children": []},
        ]})
    );
    assert_eq!(
        context["syndications"],
        serde_json::json!([{"name": "x-twitter", "url": "https://x"}, {"name": "github", "url": "https://g"}])
    );
    assert_eq!(
        context["cover"],
        serde_json::json!({"position": "behind", "show": true, "path": "/c.webp", "unsplash": "lunarts"})
    );
    assert_eq!(
        context["hide_website_junk"],
        serde_json::json!({"main_nav": false, "title": false, "cat_jumbotron": false, "footer": false})
    );
    for absent in [
        "toc_entries",
        "pagination",
        "show_author",
        "section_class",
        "use_dark_layout",
    ] {
        assert!(context.get(absent).is_none(), "{absent}");
    }
}

#[test]
fn a_post_without_a_toc_or_cover_has_none_and_the_default_cover() {
    let output = build_one(&[post(SAMPLE_POST, "title: T")]).unwrap();
    let context = &output.pages[0].context;
    assert_eq!(context["toc"], serde_json::Value::Null);
    assert_eq!(context["cover"]["position"], "after_title");
    assert_eq!(context["syndications"], serde_json::json!([]));
    assert_eq!(context["narrow_post"], false);
}

#[test]
fn keys_that_change_nothing_on_the_page_are_accepted_and_not_shown() {
    let plain = build_one(&[post(SAMPLE_POST, "title: T")]).unwrap();
    let front =
        "title: T\ntrip: somewhere-2023\nfeatured: true\nhidden: false\nuse_dark_layout: true";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    assert_eq!(output.pages[0].context, plain.pages[0].context);
}

#[test]
fn a_footer_line_is_in_the_context_and_none_by_default() {
    let output = build_one(&[post(
        SAMPLE_POST,
        "title: T\nfooter:\n  extra: A fish named Elvis!",
    )])
    .unwrap();
    assert_eq!(
        output.pages[0].context["footer"],
        serde_json::json!({"extra": "A fish named Elvis!"})
    );
    let plain = build_one(&[post(SAMPLE_POST, "title: T")]).unwrap();
    assert_eq!(
        plain.pages[0].context["footer"],
        serde_json::json!({"extra": null})
    );
}

#[test]
fn a_cover_at_another_position_is_built_and_passed_on_as_it_is() {
    let front = "title: T\ncover:\n  path: /c.webp\n  show: false\n  position: top";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    assert_eq!(
        output.pages[0].context["cover"],
        serde_json::json!({"path": "/c.webp", "position": "top", "show": false, "unsplash": null})
    );
    let without = build_one(&[post(SAMPLE_POST, "title: T")]).unwrap();
    assert_eq!(
        without.pages[0].context["cover"]["path"],
        serde_json::Value::Null
    );
}

#[test]
fn syndications_that_pull_interactions_are_named_for_the_crawler() {
    let front = "title: T\nsyndications:\n  mastodon: https://social.example/@me/1\n  \
                 linkedin: https://linkedin.example/p/2\n  hacker-news: https://news.example/item?id=3";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    let context = &output.pages[0].context;
    assert_eq!(
        context["interactions"],
        serde_json::json!({
            "mastodon": "https://social.example/@me/1",
            "bluesky": null,
            "hacker_news": "https://news.example/item?id=3",
        })
    );
    assert_eq!(
        context["syndications"],
        serde_json::json!([
            {"name": "mastodon", "url": "https://social.example/@me/1"},
            {"name": "linkedin", "url": "https://linkedin.example/p/2"},
            {"name": "hacker-news", "url": "https://news.example/item?id=3"},
        ]),
        "every syndication is still linked, in order"
    );
}

#[test]
fn a_post_without_such_syndications_has_no_interactions() {
    let front = "title: T\nsyndications:\n  linkedin: https://linkedin.example/p/2";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    assert_eq!(
        output.pages[0].context["interactions"],
        serde_json::Value::Null
    );
    let plain = build_one(&[post(SAMPLE_POST, "title: T")]).unwrap();
    assert_eq!(
        plain.pages[0].context["interactions"],
        serde_json::Value::Null
    );
}

#[test]
fn a_single_category_is_one_of_the_posts_categories() {
    let output = build_one(&[post(SAMPLE_POST, "title: T\ncategory: travel")]).unwrap();
    assert_eq!(
        output.pages[0].context["categories"],
        serde_json::json!([{"name": "travel", "anchor": "travel"}])
    );
}

#[test]
fn a_narrow_post_with_a_cover_after_its_title_is_built() {
    let front = "narrow_post: true\ncover:\n  path: /c.webp\n  unsplash: someone";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    let context = &output.pages[0].context;
    assert_eq!(context["narrow_post"], true);
    assert_eq!(
        context["cover"],
        serde_json::json!({"position": "after_title", "show": true, "path": "/c.webp", "unsplash": "someone"})
    );
}

#[test]
fn only_the_posts_asked_for_are_built_and_the_first_has_no_previous() {
    let docs = [sample_post(), post("_posts/2020/2020-01-01-other.md", "")];
    let output = build(
        &docs,
        &["_posts/2020/2020-01-01-other.md"],
        &folia_core::content::LastCommits::default(),
        &clock(),
    )
    .unwrap();
    assert_eq!(output.pages.len(), 1);
    assert_eq!(output.pages[0].context["previous"], serde_json::Value::Null);
    assert_eq!(
        output.pages[0].context["next"]["url"],
        "/blog/2021/08/08/sample-post"
    );
    assert_eq!(
        output.entries[0].modified.unwrap().from,
        folia_core::content::ModifiedFrom::Published
    );
}

#[test]
fn front_matter_the_template_cannot_show_is_not_built_yet() {
    let cases = [
        ("rating: 4", "the front matter key rating is not built yet"),
        (
            "layout: page",
            "a post with the layout page is not built yet",
        ),
        ("narrow_post: maybe", "narrow_post should be true or false"),
        // Pages accept this typo (Q3h); posts never had it, so they still refuse it.
        (
            "narrow-post: true",
            "the front matter key narrow-post is not built yet",
        ),
        ("featured: yes please", "featured should be true or false"),
        ("hidden: 1", "hidden should be true or false"),
        ("trip: [a, b]", "trip should be text"),
        (
            "use_dark_layout: false",
            "use_dark_layout: false is not built yet",
        ),
        ("footer: plain", "footer should be a mapping"),
        ("footer:\n  extra: [x]", "footer: extra should be text"),
        (
            "footer:\n  credits: x",
            "footer: the key credits is not built yet",
        ),
        (
            "toc:\n  enabled: true",
            "a table of contents not on the left is not built yet",
        ),
        (
            "show_author: false",
            "the front matter key show_author is not built yet",
        ),
        (
            "toc:\n  enabled: true\n  position: middle",
            "toc: unknown position \"middle\"",
        ),
    ];
    for (front, message) in cases {
        let problems = build_one(&[post(SAMPLE_POST, front)]).unwrap_err();
        assert_eq!(problems[0].to_string(), format!("{SAMPLE_POST}: {message}"));
    }
}

#[test]
fn a_post_s_feed_item_has_its_dates_author_terms_and_jekyll_s_id() {
    let output = build_one(&[sample_post()]).unwrap();
    let [item] = &output.feed[..] else {
        panic!("{:?}", output.feed)
    };
    assert_eq!(item.collection, "posts");
    assert_eq!(item.url, "/blog/2021/08/08/sample-post");
    assert_eq!(item.id, "/blog/2021/08/08/sample-post");
    assert_eq!(item.title, "Webmentions: letting Disqus go");
    assert_eq!(
        clock().xmlschema(item.published),
        "2021-08-08T00:00:00+02:00"
    );
    assert_eq!(clock().xmlschema(item.updated), "2021-08-24T00:00:00+02:00");
    assert_eq!(item.author.as_deref(), Some("gg"));
    assert_eq!(item.categories, ["code", "announcement"]);
    assert_eq!(item.tags, ["webmentions", "blog"]);
    assert_eq!(item.content_html, output.entries[0].body_html);
    assert_eq!(
        item.excerpt_html,
        folia_core::content::markdown::excerpt_html(&sample_post().body, None)
    );
    assert_eq!(item.lang, None);
}

#[test]
fn the_feed_reads_a_post_s_singular_category_alone() {
    let front = "title: T\ncategory: travel\ncategories: [journal]";
    let output = build_one(&[post(SAMPLE_POST, front)]).unwrap();
    assert_eq!(output.feed[0].categories, ["travel"]);
}

#[test]
fn a_post_s_lang_is_its_text_or_its_list_s_texts_joined() {
    let lang = |front: &str| {
        let output = build_one(&[post(SAMPLE_POST, front)])
            .map_err(|problems| problems.iter().map(ToString::to_string).collect::<Vec<_>>());
        output.map(|output| output.feed[0].lang.clone())
    };
    assert_eq!(lang("lang: italian"), Ok(Some("italian".to_owned())));
    assert_eq!(lang("lang: [italian]"), Ok(Some("italian".to_owned())));
    assert_eq!(lang("lang: [it, aly]"), Ok(Some("italy".to_owned())));
    let refused = lang("lang: 3").unwrap_err();
    assert!(
        refused[0].contains("lang should be text or a list of texts"),
        "{refused:?}"
    );
}
