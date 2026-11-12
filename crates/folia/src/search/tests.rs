use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T14:43:51Z".parse().unwrap()).unwrap()
}

fn page(url: &str, title: &str, content: &str) -> Page {
    Page {
        url: url.to_owned(),
        title: title.to_owned(),
        content: content.to_owned(),
    }
}

fn item(collection: &str, kind: Option<&str>, url: &str, published: &str) -> FeedItem {
    let published = published.parse().unwrap();
    FeedItem {
        collection: collection.to_owned(),
        url: url.to_owned(),
        id: url.to_owned(),
        title: format!("Title of {url}"),
        published,
        updated: published,
        content_html: format!("<p>Body of {url}</p>\n"),
        excerpt_html: String::new(),
        author: None,
        categories: kind.map_or_default(|kind| vec![kind.to_owned()]),
        tags: Vec::new(),
        lang: None,
        search_text: None,
    }
}

/// The URLs of the records of `js`, in order.
fn urls(js: &str) -> Vec<&str> {
    js.split("\"url\": \"https://example.org")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn the_script_is_the_records_then_the_fragment_as_it_is() {
    let js = script(
        "https://example.org",
        &[page(
            "/about",
            "About \"me\"",
            "<h2>Who</h2>\n<p>I am.Here.</p>\n",
        )],
        &[item("posts", None, "/blog/a", "2021-08-08T10:00:00Z")],
        "\nvar lunr_index = lunr(function () {});\n",
        &clock(),
    );
    assert_eq!(
        js,
        "var documents = [\n\
         \x20   {\"id\": 0, \"url\": \"https://example.org/about\", \"title\": \"About \\\"me\\\"\", \"body\": \"Who: I am. Here. \"},\n\
         \x20   {\"id\": 1, \"url\": \"https://example.org/blog/a\", \"title\": \"Title of /blog/a\", \"body\": \"2021/08/08 - Body of /blog/a \"}\n\
         ];\n\
         \nvar lunr_index = lunr(function () {});\n"
    );
}

#[test]
fn pages_are_listed_by_url_without_feeds_assets_categories_and_tags() {
    let pages = [
        page("/now", "Now", ""),
        page("/blog/tags.html", "Tags", ""),
        page("/blog/categories.html", "Categories", ""),
        page("/blog/category/travel", "Travel", ""),
        page("/assets/js/x", "", ""),
        page("/blog/feed.xml", "", ""),
        page("/about", "About", ""),
        page("/redirects.json", "", "{}"),
    ];
    let js = script("https://example.org", &pages, &[], "", &clock());
    assert_eq!(
        urls(&js),
        ["/about", "/blog/categories.html", "/now", "/redirects.json"]
    );
}

#[test]
fn entries_follow_the_pages_by_kind_then_collection_oldest_first() {
    let items = [
        item("posts", None, "/blog/b", "2021-08-09T10:00:00Z"),
        item("posts", None, "/blog/a", "2021-08-08T10:00:00Z"),
        item("micro", Some("notes"), "/micro/n", "2021-08-01T10:00:00Z"),
        item(
            "micro",
            Some("bookmarks"),
            "/bookmarks/b",
            "2021-08-01T10:00:00Z",
        ),
        item("micro", Some("likes"), "/likes/l", "2021-08-01T10:00:00Z"),
        item("micro", Some("rsvps"), "/rsvps/r", "2021-08-01T10:00:00Z"),
        item("digital_garden", None, "/dg/z", "2021-08-01T10:00:00Z"),
        item("digital_garden", None, "/dg/y", "2021-08-01T10:00:00Z"),
    ];
    let js = script(
        "https://example.org",
        &[page("/", "Home", "")],
        &items,
        "",
        &clock(),
    );
    assert_eq!(
        urls(&js),
        [
            "/",
            "/rsvps/r",
            "/likes/l",
            "/bookmarks/b",
            "/dg/y",
            "/dg/z",
            "/blog/a",
            "/blog/b"
        ]
    );
}

#[test]
fn an_rsvp_s_body_is_its_own_text_after_its_date() {
    let mut rsvp = item("micro", Some("rsvps"), "/rsvps/r", "2021-08-19T22:00:00Z");
    rsvp.search_text = Some("RSVPed yes - 2021-08-20, 18:00: Gardens by Anna".to_owned());
    let js = script("https://example.org", &[], &[rsvp], "", &clock());
    assert!(
        js.contains("\"body\": \"2021/08/20 - RSVPed yes - 2021-08-20, 18:00: Gardens by Anna\""),
        "{js}"
    );
}

#[test]
fn a_body_is_the_script_s_filter_chain() {
    assert_eq!(
        text("<h3>A.</h3>\r\n<p>\"q\"</p>\n<script>x()</script><h4>B</h4>"),
        "A. :  q  B: "
    );
}

#[test]
fn a_hard_line_break_is_a_space_so_the_words_on_either_side_stay_two() {
    // The case that gave `twogarden`: a paragraph wrapped in the source.
    assert_eq!(
        text("<p>a microblog, two\ngarden notes</p>"),
        "a microblog, two garden notes "
    );
    assert_eq!(text("<p>two\r\ngarden</p>"), "two garden ");
    // A run of breaks is one space, and none is added next to a space.
    assert_eq!(text("<p>a\n\n\nb</p>"), "a b ");
    assert_eq!(text("<p>a \nb</p>"), "a b ");
    assert_eq!(text("<p>a\n b</p>"), "a b ");
}

#[test]
fn a_soft_break_a_list_and_inline_markup_do_not_glue_either() {
    // A Markdown hard break (`<br />` then a newline), a list, inline markup across lines.
    assert_eq!(text("<p>one<br />\ntwo</p>"), "one two ");
    assert_eq!(text("<ul>\n<li>a</li>\n<li>b</li>\n</ul>"), "a b ");
    assert_eq!(text("<p><em>x</em>\n<strong>y</strong> z</p>"), "x y z ");
}

#[test]
fn code_blocks_keep_their_lines_apart_and_ordinary_paragraphs_are_as_before() {
    let code = "<pre><code>fn main() {\n    run();\n}\n</code></pre>";
    let text = text(code);
    assert!(
        text.contains("{ run();") || text.contains("{ run(); }"),
        "{text:?}"
    );
    assert!(!text.contains("}fn") && !text.contains("{run"), "{text:?}");
    // Paragraphs separated by a blank line: the break follows the space `</p>` made, so the
    // text is exactly what Jekyll's chain gave.
    assert_eq!(super::text("<p>one</p>\n<p>two</p>"), "one two ");
    assert_eq!(super::text("<h2>A</h2>\n<p>b</p>"), "A: b ");
    // No break, no change.
    assert_eq!(super::text("<p>plain text</p>"), "plain text ");
}
