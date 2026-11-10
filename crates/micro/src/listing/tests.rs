use folia_core::content::{DocId, FrontMatter, Mode};
use serde_json::json;

use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
}

fn entry(path: &str, front: &str, body: &str) -> SourceDoc {
    SourceDoc {
        file_time: None,
        id: DocId::new(path),
        mode: Mode::Micro,
        front: FrontMatter::parse(front).unwrap(),
        body: body.to_owned(),
    }
}

fn kinds(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn docs() -> Vec<SourceDoc> {
    vec![
        entry(
            "_micro/notes/2025-06-11-a.md",
            "title: A",
            "One <em>two</em>.\n",
        ),
        entry(
            "_micro/likes/2026-07-06-game.md",
            "title: Game\nlike:\n  url: https://x.org/g",
            "Ownership &amp; games.\n",
        ),
        entry(
            "_micro/bookmarks/2026-07-06-site.md",
            "title: Site\nbookmark:\n  url: https://y.org/s",
            "",
        ),
        entry("_micro/notes/2026-10-26-soon.md", "title: Soon", ""),
        entry(
            "_micro/notes/2025-06-12-b.md",
            "title: B\npublished: false",
            "",
        ),
    ]
}

fn urls(items: &Value) -> Vec<&str> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["url"].as_str().unwrap())
        .collect()
}

#[test]
fn every_kind_is_listed_newest_first_and_same_dates_by_path_descending() {
    let items = listing(
        &docs(),
        "_micro",
        &kinds(&["notes", "likes", "bookmarks", "rsvps"]),
        &clock(),
    )
    .unwrap();
    assert_eq!(
        urls(&items),
        [
            "/likes/2026-07-06-game",
            "/bookmarks/2026-07-06-site",
            "/micro/2025-06-11-a",
        ],
        "future and unpublished entries are left out"
    );
}

#[test]
fn a_listing_of_one_kind_has_only_it_with_its_data_and_texts() {
    let items = listing(&docs(), "_micro", &kinds(&["likes"]), &clock()).unwrap();
    assert_eq!(
        items,
        json!([{
            "url": "/likes/2026-07-06-game",
            "title": "Game",
            "kind": "likes",
            "layout": "like",
            "date": "2026-07-06T00:00:00+02:00",
            "excerpt": "<p>Ownership &amp; games.</p>\n",
            "text": "Ownership &amp; games.\n",
            "summary": "Ownership &amp; games.\n",
            "like": {"url": "https://x.org/g", "label": null},
        }])
    );
}

#[test]
fn a_summary_is_the_first_25_words() {
    let body = (1..=30)
        .map(|n| format!("w{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    let docs = [entry(
        "_micro/bookmarks/2025-06-11-long.md",
        "title: Long\nbookmark:\n  url: u",
        &body,
    )];
    let items = listing(&docs, "_micro", &kinds(&["bookmarks"]), &clock()).unwrap();
    let expected = (1..=25)
        .map(|n| format!("w{n}"))
        .collect::<Vec<_>>()
        .join(" ")
        + "...";
    assert_eq!(items[0]["summary"], expected);
}

#[test]
fn an_unknown_kind_is_refused() {
    let problems = listing(&docs(), "_micro", &kinds(&["notes", "photos"]), &clock()).unwrap_err();
    assert_eq!(
        problems[0].to_string(),
        "a micro listing lists the unknown kind photos"
    );
}
