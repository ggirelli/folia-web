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
    assert_eq!(
        outputs[0].route.url(),
        "/blog/2023/05/13/May-train-trip-pt1"
    );
    assert_eq!(
        outputs[0].route.output_path(),
        "blog/2023/05/13/May-train-trip-pt1.html"
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
