use folia_core::content::{DocId, FrontMatter, Mode};
use serde_json::json;

use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
}

fn entry(path: &str, front: &str, body: &str) -> SourceDoc {
    SourceDoc {
        id: DocId::new(path),
        mode: Mode::Micro,
        front: FrontMatter::parse(front).unwrap(),
        body: body.to_owned(),
    }
}

fn built(doc: &SourceDoc, author: Option<&str>) -> Result<ModeOutput, Vec<Diagnostic>> {
    build(
        std::slice::from_ref(doc),
        "_micro",
        &[doc.id.as_str()],
        author,
        &LastCommits::default(),
        &clock(),
    )
}

fn page(doc: &SourceDoc) -> PageSpec {
    built(doc, Some("gg")).unwrap().pages.remove(0)
}

fn problem(doc: &SourceDoc) -> String {
    built(doc, Some("gg")).unwrap_err()[0].to_string()
}

/// The site's first note.
fn note() -> SourceDoc {
    entry(
        "_micro/notes/2013-08-17-memoria.md",
        "title: \"Le nostre memoria, immagini sfocate contro l'orizzonte.\"\nnarrow_post: true",
        "\nGrammatica volutamente deformata.\n",
    )
}

#[test]
fn a_note_is_a_page_with_every_micro_default() {
    let page = page(&note());
    assert_eq!(page.template, "page.html");
    assert_eq!(page.route.url(), "/micro/2013-08-17-memoria");
    let context = &page.context;
    assert_eq!(context["layout"], "page");
    assert_eq!(context["date"], "2013-08-17T00:00:00+02:00");
    assert_eq!(context["modified"], "2013-08-17T00:00:00+02:00");
    assert_eq!(context["description"], "Grammatica volutamente deformata.");
    assert_eq!(context["author"], "gg");
    assert_eq!(context["narrow_post"], true);
    assert_eq!(
        context["categories"],
        json!([{"name": "notes", "anchor": "notes"}])
    );
    assert_eq!(context["tags"], json!([]));
    assert_eq!(
        context["hide_post_junk"],
        json!({"categories": true, "tags": true})
    );
    assert_eq!(context["pagination"], false);
    assert_eq!(context["previous"], Value::Null);
    assert_eq!(
        context["hide_website_junk"],
        json!({"main_nav": true, "title": true, "cat_jumbotron": true, "footer": false})
    );
    assert_eq!(
        context["corner_backlink"],
        json!({"show": true, "url": "/micro", "icon_class": "fa-solid fa-microscope"})
    );
    assert_eq!(context["syndications"], json!([]));
    assert_eq!(context["interactions"], Value::Null);
    assert_eq!(context["footer"], json!({"extra": null}));
}

#[test]
fn without_a_configured_author_an_entry_has_none() {
    let output = built(&note(), None).unwrap();
    assert_eq!(output.pages[0].context["author"], Value::Null);
    assert_eq!(
        output.entries[0].body_html,
        "\n<p>Grammatica volutamente deformata.</p>\n"
    );
}

#[test]
fn a_like_has_its_link_and_label() {
    let like = entry(
        "_micro/likes/2021-08-18-homebrew-club.md",
        "title: \"Björn Wärmedal\"\nlike:\n  url: https://warmedal.se/~bjorn/\nnarrow_post: true",
        "I met Björn.\n",
    );
    let page = page(&like);
    assert_eq!(page.template, "like.html");
    assert_eq!(page.route.url(), "/likes/2021-08-18-homebrew-club");
    assert_eq!(
        page.context["like"],
        json!({"url": "https://warmedal.se/~bjorn/", "label": null})
    );
    assert_eq!(page.context["categories"][0]["name"], "likes");
}

#[test]
fn a_bookmark_has_its_link_and_syndications() {
    let bookmark = entry(
        "_micro/bookmarks/2021-08-24-ezra-klein-pandemic.md",
        "title: \"Dr. Murthy on<br />the importance of relationships\"\nbookmark:\n  url: https://www.nytimes.com/x.html\n  label: \"The Ezra Klein Show - 2021, Jan 26\"\nsyndications:\n  x-twitter: https://twitter.com/x/status/1",
        "Listened.\n",
    );
    let page = page(&bookmark);
    assert_eq!(page.template, "bookmark.html");
    assert_eq!(
        page.context["bookmark"],
        json!({"url": "https://www.nytimes.com/x.html", "label": "The Ezra Klein Show - 2021, Jan 26"})
    );
    assert_eq!(
        page.context["syndications"],
        json!([{"name": "x-twitter", "url": "https://twitter.com/x/status/1"}])
    );
}

#[test]
fn an_rsvp_has_its_answer_and_its_event_dated_in_the_site_s_zone() {
    let rsvp = entry(
        "_micro/rsvps/2021-08-18-homebrew-club.md",
        "title: \"Impromptu Homebrew Website Club\"\nrsvp: \"yes\"\nevent:\n    date: 2018-08-18\n    time: \"18:30 UTC\"\n    name: \"Impromptu HWC - Europe\"\n    organizer: \"IndieWeb\"\n    url: https://events.indieweb.org/x",
        "",
    );
    let page = page(&rsvp);
    assert_eq!(page.template, "rsvp.html");
    assert_eq!(page.context["rsvp"], "yes");
    assert_eq!(
        page.context["event"],
        json!({
            "date": "2018-08-18T00:00:00+02:00",
            "time": "18:30 UTC",
            "name": "Impromptu HWC - Europe",
            "organizer": "IndieWeb",
            "url": "https://events.indieweb.org/x",
        })
    );
}

#[test]
fn front_matter_an_entry_cannot_show_is_refused() {
    let at = "_micro/notes/2025-06-11-foo-bar.md";
    let cases = [
        (
            "title: T\nlayout: post",
            "the front matter key layout is not built yet on notes",
        ),
        (
            "title: T\nlike:\n  url: u",
            "the front matter key like is not built yet on notes",
        ),
        (
            "narrow_post: true",
            "an entry without a title is not built yet",
        ),
        (
            "title: T\nnarrow_post: maybe",
            "narrow_post should be true or false",
        ),
    ];
    for (front, message) in cases {
        assert_eq!(
            problem(&entry(at, front, "")),
            format!("{at}: {message}"),
            "{front:?}"
        );
    }
    let like = "_micro/likes/2021-09-23-smbc.md";
    for (front, message) in [
        ("title: T\nlike: u", "like should be a mapping with a url"),
        ("title: T\nlike:\n  label: l", "like should have a url"),
        (
            "title: T\nlike:\n  url: u\n  via: v",
            "like: the key via is not built yet",
        ),
    ] {
        assert_eq!(
            problem(&entry(like, front, "")),
            format!("{like}: {message}"),
            "{front:?}"
        );
    }
    let rsvp = "_micro/rsvps/2022-10-06-create-day.md";
    let event = "\nevent:\n  date: 2022-10-06\n  time: t\n  name: n\n  organizer: o";
    for (front, message) in [
        (
            format!("title: T\nrsvp: true{event}\n  url: u"),
            "rsvp should be text, such as yes",
        ),
        (
            format!("title: T\nrsvp: yes{event}"),
            "event: url should be text",
        ),
        (
            format!(
                "title: T\nrsvp: yes{}\n  url: u",
                event.replace("2022-10-06", "soon")
            ),
            "event: \"soon\" is not a date in a form Folia reads (YYYY-MM-DD, optionally with HH:MM or HH:MM:SS)",
        ),
    ] {
        assert_eq!(
            problem(&entry(rsvp, &front, "")),
            format!("{rsvp}: {message}"),
            "{front:?}"
        );
    }
}

#[test]
fn an_entry_outside_the_kinds_is_not_built_yet() {
    let deep = entry("_micro/notes/deep/x_y.md", "title: T", "");
    assert_eq!(
        problem(&deep),
        "_micro/notes/deep/x_y.md: an entry outside notes, likes, bookmarks and rsvps is not built yet"
    );
}

#[test]
fn unpublished_and_future_entries_are_left_out_and_others_not_asked_for() {
    let unpublished = entry(
        "_micro/notes/2025-06-11-foo-bar.md",
        "title: T\npublished: false",
        "",
    );
    let future = entry("_micro/notes/2026-10-26-soon.md", "title: T", "");
    for doc in [unpublished, future] {
        assert_eq!(built(&doc, None).unwrap().pages.len(), 0);
    }
    let output = build(
        &[note()],
        "_micro",
        &[],
        None,
        &LastCommits::default(),
        &clock(),
    )
    .unwrap();
    assert!(output.pages.is_empty(), "not in `only`");
}

#[test]
fn an_entry_s_feed_item_is_filed_under_its_kind_with_jekyll_s_id() {
    let like = entry(
        "_micro/likes/2021-08-18-homebrew-club.md",
        "title: \"Björn Wärmedal\"\nlike:\n  url: https://warmedal.se/~bjorn/",
        "I met Björn.\n",
    );
    let output = built(&like, Some("gg")).unwrap();
    let [item] = &output.feed[..] else {
        panic!("{:?}", output.feed)
    };
    assert_eq!(item.collection, "micro");
    assert_eq!(item.url, "/likes/2021-08-18-homebrew-club");
    assert_eq!(item.id, "/likes/homebrew-club");
    assert_eq!(item.title, "Björn Wärmedal");
    assert_eq!(item.categories, ["likes"]);
    assert_eq!(item.author.as_deref(), Some("gg"));
    assert_eq!(item.published, item.updated);
    assert_eq!(item.content_html, "<p>I met Björn.</p>\n");
}
