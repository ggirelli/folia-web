use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T14:43:51Z".parse().unwrap()).unwrap()
}

fn config() -> FeedsConfig {
    toml::from_str(
        "lang = \"en\"\n\
         generator = { name = \"Jekyll\", uri = \"https://jekyllrb.com/\", version = \"4.4.1\" }\n",
    )
    .unwrap()
}

const SITE: Site = Site {
    url: "https://example.org",
    title: "Folia's site",
    description: "Notes & posts",
};

fn item(collection: &str, url: &str, published: &str) -> FeedItem {
    let published: Timestamp = published.parse().unwrap();
    FeedItem {
        collection: collection.to_owned(),
        url: url.to_owned(),
        id: url.to_owned(),
        title: format!("Title of {url}"),
        published,
        updated: published,
        content_html: "<p>Body.</p>\n".to_owned(),
        excerpt_html: "<p>Body.</p>\n".to_owned(),
        author: Some("gg".to_owned()),
        categories: Vec::new(),
        tags: Vec::new(),
        lang: None,
        search_text: None,
    }
}

fn feed(generator: &str, url: &str) -> Feed {
    Feed::of(generator, url).unwrap()
}

/// The URLs of the entries of `xml`, in order.
fn listed(xml: &str) -> Vec<&str> {
    xml.split("<link href=\"https://example.org")
        .skip(3)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn a_feed_is_written_as_jekyll_feed_writes_it_minified() {
    let mut post = item("posts", "/blog/2021/08/08/hello", "2021-08-08T10:00:00Z");
    post.id = "/blog/2021/08/08/hello".to_owned();
    post.title = "It's \"new\" -- <em>here</em>".to_owned();
    post.updated = "2021-09-01T10:00:00Z".parse().unwrap();
    post.content_html = "\n<p>Hello, <a href=\"/x\">world</a>.</p>\n".to_owned();
    post.excerpt_html = "<p>Hello,\n  world.</p>\n".to_owned();
    post.categories = vec!["journal".to_owned()];
    post.tags = vec!["web".to_owned()];
    post.lang = Some("italian".to_owned());
    let xml = xml(
        &config(),
        &SITE,
        &feed("blog feed", "/blog/feed.xml"),
        "/blog/feed.xml",
        &[post],
        &clock(),
    );
    assert_eq!(
        xml,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
         <feed xmlns=\"http://www.w3.org/2005/Atom\" xml:lang=\"en\">\
         <generator uri=\"https://jekyllrb.com/\" version=\"4.4.1\">Jekyll</generator>\
         <link href=\"https://example.org/blog/feed.xml\" rel=\"self\" type=\"application/atom+xml\" />\
         <link href=\"https://example.org/\" rel=\"alternate\" type=\"text/html\" hreflang=\"en\" />\
         <updated>2021-09-01T12:00:00+02:00</updated>\
         <id>https://example.org/blog/feed.xml</id>\
         <title type=\"html\">Folia’s site</title>\
         <subtitle>Notes &amp; posts</subtitle>\
         <entry xml:lang=\"italian\">\
         <title type=\"html\">It’s “new” – here</title>\
         <link href=\"https://example.org/blog/2021/08/08/hello\" rel=\"alternate\" type=\"text/html\" title=\"It’s “new” – here\" />\
         <published>2021-08-08T12:00:00+02:00</published><updated>2021-09-01T12:00:00+02:00</updated>\
         <id>https://example.org/blog/2021/08/08/hello</id>\
         <content type=\"html\" xml:base=\"https://example.org/blog/2021/08/08/hello\"><![CDATA[<p>Hello, <a href=\"/x\">world</a>.</p>]]></content>\
         <author><name>gg</name></author>\
         <category term=\"journal\" /><category term=\"web\" />\
         <summary type=\"html\"><![CDATA[Hello, world.]]></summary>\
         </entry></feed>"
    );
}

#[test]
fn a_feed_without_a_language_declares_none() {
    let config = FeedsConfig {
        lang: None,
        ..config()
    };
    let xml = xml(
        &config,
        &SITE,
        &feed("blog feed", "/blog/feed.xml"),
        "/blog/feed.xml",
        &[],
        &clock(),
    );
    assert!(xml.contains("<feed xmlns=\"http://www.w3.org/2005/Atom\" >"));
    assert!(xml.contains("rel=\"alternate\" type=\"text/html\" />"));
}

#[test]
fn a_feed_lists_its_newest_items_and_those_of_a_date_by_url_descending() {
    let mut items: Vec<FeedItem> = (1..=12)
        .map(|day| {
            item(
                "posts",
                &format!("/blog/{day:02}"),
                &format!("2021-08-{day:02}T10:00:00Z"),
            )
        })
        .collect();
    items.push(item("posts", "/blog/12b", "2021-08-12T10:00:00Z"));
    let xml = xml(
        &config(),
        &SITE,
        &feed("blog feed", "/blog/feed.xml"),
        "/blog/feed.xml",
        &items,
        &clock(),
    );
    assert_eq!(
        listed(&xml),
        [
            "/blog/12b",
            "/blog/12",
            "/blog/11",
            "/blog/10",
            "/blog/09",
            "/blog/08",
            "/blog/07",
            "/blog/06",
            "/blog/05",
            "/blog/04"
        ]
    );
}

#[test]
fn each_feed_lists_its_collection_tag_or_kind() {
    let mut tagged = item("posts", "/blog/tagged", "2021-08-01T10:00:00Z");
    tagged.tags = vec!["web".to_owned()];
    let untagged = item("posts", "/blog/untagged", "2021-08-02T10:00:00Z");
    let mut like = item("micro", "/likes/a", "2021-08-03T10:00:00Z");
    like.categories = vec!["likes".to_owned()];
    let mut note = item("micro", "/micro/b", "2021-08-04T10:00:00Z");
    note.categories = vec!["notes".to_owned()];
    let garden = item("digital_garden", "/dg/c", "2021-08-05T10:00:00Z");
    let items = [tagged, untagged, like, note, garden];
    let urls = |generator: &str, path: &str| {
        let xml = xml(
            &config(),
            &SITE,
            &feed(generator, path),
            path,
            &items,
            &clock(),
        );
        listed(&xml)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        urls("blog feed", "/blog/feed.xml"),
        ["/blog/untagged", "/blog/tagged"]
    );
    assert_eq!(urls("tag feed", "/blog/tag/web.xml"), ["/blog/tagged"]);
    assert_eq!(
        urls("micro feed", "/micro/feed.xml"),
        ["/micro/b", "/likes/a"]
    );
    assert_eq!(
        urls("micro kind feed", "/feed/micro/likes.xml"),
        ["/likes/a"]
    );
    assert_eq!(urls("garden feed", "/dg/feed.xml"), ["/dg/c"]);
}

#[test]
fn a_feed_is_titled_by_its_collection_and_kind_capitalized() {
    let title = |generator: &str, path: &str| feed(generator, path).title(&SITE);
    assert_eq!(title("blog feed", "/blog/feed.xml"), "Folia's site");
    assert_eq!(title("tag feed", "/blog/tag/web.xml"), "Folia's site");
    assert_eq!(
        title("micro feed", "/micro/feed.xml"),
        "Folia's site | Micro"
    );
    assert_eq!(
        title("micro kind feed", "/feed/micro/rsvps.xml"),
        "Folia's site | Micro | Rsvps"
    );
    assert_eq!(
        title("garden feed", "/dg/feed.xml"),
        "Folia's site | Digital_garden"
    );
    assert_eq!(Feed::of("note graph", "/dg/graph.json"), None);
    assert_eq!(Feed::of("redirects", "/redirects.json"), None);
}

#[test]
fn a_feed_is_updated_when_its_latest_item_is_never_at_the_build() {
    let mut old = item("posts", "/blog/old", "2021-08-01T10:00:00Z");
    old.updated = "2024-01-01T10:00:00Z".parse().unwrap();
    let newer = item("posts", "/blog/newer", "2022-08-01T10:00:00Z");
    let garden = item("digital_garden", "/dg/c", "2025-03-01T10:00:00Z");
    let items = [old, newer, garden];
    let updated = |generator: &str, path: &str| {
        let xml = xml(
            &config(),
            &SITE,
            &feed(generator, path),
            path,
            &items,
            &clock(),
        );
        let (_, rest) = xml.split_once("<updated>").unwrap();
        rest.split('<').next().unwrap().to_owned()
    };
    // An edit to an older item it lists advances it.
    assert_eq!(
        updated("blog feed", "/blog/feed.xml"),
        "2024-01-01T11:00:00+01:00"
    );
    // A feed that lists nothing takes the latest item of the site.
    assert_eq!(
        updated("micro feed", "/micro/feed.xml"),
        "2025-03-01T11:00:00+01:00"
    );
}

#[test]
fn a_summary_keeps_a_no_break_space_and_an_empty_one_is_still_written() {
    let mut spaced = item("micro", "/micro/a", "2021-08-01T10:00:00Z");
    spaced.excerpt_html = "<p>\u{a0} Studio\n\n Ghibli\u{a0}</p>\n".to_owned();
    let mut empty = item("micro", "/micro/b", "2021-08-02T10:00:00Z");
    empty.excerpt_html = "\n".to_owned();
    let xml = xml(
        &config(),
        &SITE,
        &feed("micro feed", "/micro/feed.xml"),
        "/micro/feed.xml",
        &[spaced, empty],
        &clock(),
    );
    assert!(xml.contains("<summary type=\"html\"><![CDATA[]]></summary>"));
    assert!(xml.contains("<![CDATA[\u{a0} Studio Ghibli\u{a0}]]></summary>"));
}

#[test]
fn an_item_without_an_author_names_none() {
    let mut anonymous = item("posts", "/blog/a", "2021-08-01T10:00:00Z");
    anonymous.author = None;
    let xml = xml(
        &config(),
        &SITE,
        &feed("blog feed", "/blog/feed.xml"),
        "/blog/feed.xml",
        &[anonymous],
        &clock(),
    );
    assert!(xml.contains("<author><name></name></author>"));
}

#[test]
fn liquid_s_filters_are_ruby_s() {
    assert_eq!(xml_escape("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&#39;");
    assert_eq!(normalize_whitespace(" a \t\r\n b\u{c} "), "a b");
    assert_eq!(capitalize("digital_GARDEN"), "Digital_garden");
    assert_eq!(capitalize(""), "");
}

fn combined(items: &[FeedItem]) -> String {
    let collections = ["posts", "digital_garden", "micro"].map(str::to_owned);
    xml(
        &config(),
        &SITE,
        &Feed::combined(&collections),
        "/feed.xml",
        items,
        &clock(),
    )
}

#[test]
fn the_combined_feed_lists_the_posts_the_notes_and_the_micro_entries_newest_first() {
    let items = [
        item("posts", "/blog/a", "2021-01-01T10:00:00Z"),
        item("digital_garden", "/dg/b", "2021-03-01T10:00:00Z"),
        item("micro", "/micro/c", "2021-02-01T10:00:00Z"),
        item("pages", "/about", "2022-01-01T10:00:00Z"),
    ];
    assert_eq!(
        listed(&combined(&items)),
        ["/dg/b", "/micro/c", "/blog/a"],
        "other collections are left out"
    );
}

#[test]
fn the_combined_feed_lists_ten_items_the_same_date_by_url_descending() {
    let mut items: Vec<FeedItem> = (0..12)
        .map(|i| item("micro", &format!("/micro/{i:02}"), "2021-01-01T10:00:00Z"))
        .collect();
    items.push(item("posts", "/blog/old", "2020-01-01T10:00:00Z"));
    let xml = combined(&items);
    let urls = listed(&xml);
    assert_eq!(urls.len(), 10);
    assert_eq!(urls[0], "/micro/11");
    assert_eq!(urls[9], "/micro/02");
}

#[test]
fn the_combined_feed_keeps_the_title_with_its_trailing_separator() {
    let xml = combined(&[item("posts", "/blog/a", "2021-01-01T10:00:00Z")]);
    assert!(
        xml.contains("<title type=\"html\">Folia’s site | </title>"),
        "{xml}"
    );
    assert!(xml.contains("<subtitle>Notes &amp; posts</subtitle>"));
    assert!(xml.contains("<link href=\"https://example.org/feed.xml\" rel=\"self\""));
    assert!(xml.contains("<id>https://example.org/feed.xml</id>"));
}

#[test]
fn the_combined_feed_is_updated_when_its_newest_item_was() {
    let mut old = item("posts", "/blog/a", "2021-01-01T10:00:00Z");
    old.updated = "2021-06-01T10:00:00Z".parse().unwrap();
    let new = item("micro", "/micro/b", "2021-02-01T10:00:00Z");
    let xml = combined(&[old, new]);
    assert!(
        xml.contains("<updated>2021-06-01T12:00:00+02:00</updated><id>"),
        "{xml}"
    );
}

#[test]
fn the_combined_feed_is_read_from_the_configuration() {
    let config: FeedsConfig = toml::from_str(
        "generator = { name = \"Jekyll\", uri = \"u\", version = \"1\" }\n\
         [combined]\nsource = \"feed.xml\"\ncollections = [\"posts\", \"micro\"]\n",
    )
    .unwrap();
    let combined = config.combined.unwrap();
    assert_eq!(combined.source, "feed.xml");
    assert_eq!(combined.collections, ["posts", "micro"]);
}

#[test]
fn a_tag_feed_with_a_space_has_a_percent_encoded_address() {
    let mut post = item("posts", "/blog/a", "2021-01-01T10:00:00Z");
    post.tags = vec!["stem cells".to_owned()];
    let path = "/blog/tag/stem cells.xml";
    let feed = feed("tag feed", path);
    assert_eq!(feed.tag.as_deref(), Some("stem cells"));
    let xml = xml(&config(), &SITE, &feed, path, &[post], &clock());
    let own = "https://example.org/blog/tag/stem%20cells.xml";
    assert!(
        xml.contains(&format!("<link href=\"{own}\" rel=\"self\"")),
        "{xml}"
    );
    assert!(xml.contains(&format!("<id>{own}</id>")), "{xml}");
    assert!(xml.contains("<id>https://example.org/blog/a</id>"));
}

#[test]
fn the_combined_feed_s_entries_have_a_summary_and_no_content() {
    let combined = combined(&[item("posts", "/blog/a", "2021-01-01T10:00:00Z")]);
    assert!(!combined.contains("<content"), "{combined}");
    assert!(combined.contains("<summary type=\"html\"><![CDATA[Body.]]></summary>"));
    let blog = xml(
        &config(),
        &SITE,
        &feed("blog feed", "/blog/feed.xml"),
        "/blog/feed.xml",
        &[item("posts", "/blog/a", "2021-01-01T10:00:00Z")],
        &clock(),
    );
    assert!(
        blog.contains("<content type=\"html\""),
        "other feeds keep it"
    );
}
