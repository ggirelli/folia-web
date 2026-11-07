use folia_core::content::{FrontMatter, LastCommits, Mode};
use folia_core::publish::{Artifact, ModeOutput};
use serde_json::{Value, json};

use super::*;

const ROOT: &str = "_digital_garden";

fn clock() -> SiteClock {
    clock_at("2026-10-25T09:00:00Z")
}

fn clock_at(now: &str) -> SiteClock {
    SiteClock::new("Europe/Stockholm", now.parse().unwrap()).unwrap()
}

fn note(path: &str, front: &str) -> SourceDoc {
    with_body(path, front, "")
}

fn with_body(path: &str, front: &str, body: &str) -> SourceDoc {
    SourceDoc {
        id: DocId::new(path),
        mode: Mode::Garden,
        front: FrontMatter::parse(front).unwrap(),
        body: body.to_owned(),
    }
}

fn build_all(docs: &[SourceDoc]) -> Result<ModeOutput, Vec<Diagnostic>> {
    let only: Vec<&str> = docs.iter().map(|doc| doc.id.as_str()).collect();
    build(docs, ROOT, &only, &LastCommits::default(), &clock())
}

fn graph(output: &ModeOutput) -> Value {
    match &output.artifacts[..] {
        [Artifact { route, body }] => {
            assert_eq!(route.url(), "/assets/json/notes_graph.json");
            serde_json::from_str(body).unwrap()
        }
        other => panic!("one graph expected, got {other:?}"),
    }
}

/// Two notes that link to each other, and a third that links to one of them twice.
fn mutual() -> Vec<SourceDoc> {
    vec![
        with_body(
            "_digital_garden/a.md",
            "title: Note A\ndate: 2026-01-01",
            "First of A.\n\nSee [[note b|B]].\n",
        ),
        with_body(
            "_digital_garden/b.md",
            "title: Note B\ndate: 2026-01-02",
            "First of B.\n\nBack to [[a]], by its file name.\n",
        ),
        with_body(
            "_digital_garden/c.md",
            "title: A note with a title longer than its label\ndate: 2026-01-03",
            "[[Note A]] and [[Note A|again]], but not `[[Note B]]`.\n",
        ),
    ]
}

#[test]
fn a_note_keeps_its_path_case_and_underscores() {
    let outputs = routes(
        &[note("_digital_garden/watch/topic_note.md", "")],
        ROOT,
        &clock(),
    )
    .unwrap();
    assert_eq!(outputs[0].route.url(), "/dg/watch/topic_note");
}

#[test]
fn an_index_note_keeps_index_in_its_url() {
    let outputs = routes(
        &[note("_digital_garden/topic/index.md", "")],
        ROOT,
        &clock(),
    )
    .unwrap();
    assert_eq!(outputs[0].route.url(), "/dg/topic/index");
    assert_eq!(outputs[0].route.output_path(), "dg/topic/index.html");
}

fn routed(docs: &[SourceDoc], clock: &SiteClock) -> Vec<String> {
    routes(docs, ROOT, clock)
        .unwrap()
        .iter()
        .filter_map(|o| o.source.as_ref())
        .map(|id| id.as_str().to_owned())
        .collect()
}

/// The build is at 2026-10-25 10:00 in Stockholm, UTC+1 since 03:00 that day; a file name's date
/// is at midnight there.
fn dated_around_the_build() -> [SourceDoc; 5] {
    [
        note("_digital_garden/a.md", "date: 2026-10-25 08:00"),
        note("_digital_garden/b.md", "date: 2026-10-25 12:00"),
        note("_digital_garden/quotes/2026-10-25-c.md", ""),
        note("_digital_garden/quotes/2026-10-26-d.md", ""),
        note("_digital_garden/quotes/2026-10-26-e.md", "date: 2026-10-24"),
    ]
}

#[test]
fn a_note_dated_after_the_build_by_its_front_matter_or_its_file_name_has_no_route() {
    assert_eq!(
        routed(&dated_around_the_build(), &clock()),
        [
            "_digital_garden/quotes/2026-10-26-e.md",
            "_digital_garden/quotes/2026-10-25-c.md",
            "_digital_garden/a.md",
        ]
    );
}

#[test]
fn a_note_dated_after_the_build_has_a_route_once_the_build_reaches_its_date() {
    let docs = dated_around_the_build();
    assert_eq!(routed(&docs, &clock_at("2026-10-25T10:59:59Z")).len(), 3);
    assert_eq!(routed(&docs, &clock_at("2026-10-25T11:00:00Z")).len(), 4);
    assert_eq!(routed(&docs, &clock_at("2026-10-25T22:59:59Z")).len(), 4);
    assert_eq!(routed(&docs, &clock_at("2026-10-25T23:00:00Z")).len(), 5);
}

#[test]
fn a_date_in_another_form_is_reported() {
    let problems = routes(
        &[note("_digital_garden/a.md", "date: April 2026")],
        ROOT,
        &clock(),
    )
    .unwrap_err();
    assert_eq!(problems.len(), 1);
}

#[test]
fn notes_linking_to_each_other_are_each_others_backlinks() {
    let output = build_all(&mutual()).unwrap();
    let backlinks = |i: usize| output.pages[i].context["backlinks"].clone();
    assert_eq!(
        backlinks(0),
        json!([
            {"title": "Note B", "url": "/dg/b", "excerpt": "First of B."},
            {"title": "A note with a title longer than its label", "url": "/dg/c", "excerpt": "[[Note A]] and [[Note A|again]], but not `[[Note B]]`.\n"},
            {"title": "A note with a title longer than its label", "url": "/dg/c", "excerpt": "[[Note A]] and [[Note A|again]], but not `[[Note B]]`.\n"},
        ])
    );
    assert_eq!(
        backlinks(1),
        json!([{"title": "Note A", "url": "/dg/a", "excerpt": "First of A."}])
    );
    assert_eq!(backlinks(2), json!([]));
    assert_eq!(
        output.entries[0].body_html,
        "<p>First of A.</p>\n\n<p>See <a href=\"/dg/b\" class=\"internal-link\">B</a>.</p>\n"
    );
    assert_eq!(output.warnings, []);
}

#[test]
fn the_graph_has_every_note_and_an_edge_per_link() {
    let output = build_all(&mutual()).unwrap();
    assert_eq!(
        graph(&output),
        json!({
            "nodes": [
                {"id": "/dg/a", "path": "/dg/a", "label": "Note A"},
                {"id": "/dg/b", "path": "/dg/b", "label": "Note B"},
                {"id": "/dg/c", "path": "/dg/c", "label": "A note with a title lo..."},
            ],
            "edges": [
                {"source": "/dg/b", "target": "/dg/a"},
                {"source": "/dg/c", "target": "/dg/a"},
                {"source": "/dg/c", "target": "/dg/a"},
                {"source": "/dg/a", "target": "/dg/b"},
            ],
        })
    );
}

#[test]
fn backlinks_and_the_graph_count_notes_whose_pages_are_not_built() {
    let docs = mutual();
    let output = build(
        &docs,
        ROOT,
        &["_digital_garden/a.md"],
        &LastCommits::default(),
        &clock(),
    )
    .unwrap();
    assert_eq!(output.pages.len(), 1);
    assert_eq!(
        output.pages[0].context["backlinks"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(graph(&output)["nodes"].as_array().unwrap().len(), 3);
}

#[test]
fn a_broken_link_is_a_span_and_one_warning() {
    let docs = [with_body(
        "_digital_garden/topic/note.md",
        "title: Rust\ndate: 2025-11-08 12:20",
        "See [[First setup: rust]].\n",
    )];
    let output = build_all(&docs).unwrap();
    assert_eq!(
        output.entries[0].body_html,
        "<p>See <span class=\"broken-link\">First setup: rust</span>.</p>\n"
    );
    assert_eq!(output.warnings.len(), 1);
    assert_eq!(
        output.warnings[0].to_string(),
        "_digital_garden/topic/note.md: the wiki-link [[First setup: rust]] names no note, so it is shown as broken"
    );
}

#[test]
fn a_note_shows_its_dates_and_its_description_keeps_wiki_links_as_written() {
    let docs = [with_body(
        "_digital_garden/topic/index.md",
        "title: Tips\ndate: 2023-01-30 12:20",
        "Tips for [[Rust]],\nsee [[Rust]].\n",
    )];
    let page = &build_all(&docs).unwrap().pages[0];
    assert_eq!(page.template, "garden_post.html");
    assert_eq!(page.context["date"], "2023-01-30T12:20:00+01:00");
    assert_eq!(page.context["modified"], "2023-01-30T12:20:00+01:00");
    assert_eq!(
        page.context["description"],
        "Tips for [[Rust]],see [[Rust]]."
    );
    assert_eq!(page.context["garden_map"], true);
}

#[test]
fn a_description_shows_a_line_with_an_aliased_wiki_link_as_kramdown_s_table_row() {
    let docs = [with_body(
        "_digital_garden/topic/index.md",
        "title: Tips\ndate: 2023-01-30 12:20",
        "Tips for [[Rust|rustaceans]].\n",
    )];
    let page = &build_all(&docs).unwrap().pages[0];
    assert_eq!(
        page.context["description"],
        "            Tips for [[Rust      rustaceans]].      "
    );
    assert_eq!(page.context["garden_map"], true);
}

#[test]
fn the_garden_map_can_be_hidden() {
    let docs = [note(
        "_digital_garden/a.md",
        "title: A\ndate: 2026-01-01\nhide_garden_map: true",
    )];
    assert_eq!(
        build_all(&docs).unwrap().pages[0].context["garden_map"],
        false
    );
}

#[test]
fn front_matter_the_template_cannot_show_is_refused() {
    for (front, message) in [
        (
            "title: A\ndate: 2026-01-01\ncover: x.jpg",
            "the front matter key cover is not built yet",
        ),
        ("title: A", "a note without a date is not built yet"),
        (
            "date: 2026-01-01",
            "a note without a title is not built yet",
        ),
        (
            "title: A\ndate: 2026-01-01\nhide_garden_map: maybe",
            "hide_garden_map must be true or false",
        ),
    ] {
        let problems = build_all(&[note("_digital_garden/a.md", front)]).unwrap_err();
        assert_eq!(problems.len(), 1, "{front}");
        assert!(
            problems[0].message.contains(message),
            "{front}: {}",
            problems[0]
        );
    }
}

/// A dated file name orders a note as a front matter date does. An undated note goes after both:
/// Folia orders it as if dated at the build, by its own choice. Jekyll compared such a note with
/// the others by path only, before dating it, so its place depended on the order Jekyll read the
/// files, here before `b.md`; Folia does not reproduce that.
#[test]
fn a_note_dated_by_its_file_name_is_ordered_by_that_date_and_an_undated_one_last() {
    let docs = [
        note("_digital_garden/b.md", "title: B\ndate: 2016-01-01"),
        note("_digital_garden/quotes/2014-02-13-a.md", "title: A"),
        note("_digital_garden/home.md", "title: Undated"),
    ];
    let output = build_all(&docs).unwrap_err();
    // The undated note cannot be built, but the order is the graph's, which needs no page.
    assert_eq!(output.len(), 1);
    let only = ["_digital_garden/b.md"];
    let built = build(&docs, ROOT, &only, &LastCommits::default(), &clock()).unwrap();
    assert_eq!(
        node_ids(&built),
        ["/dg/quotes/2014-02-13-a", "/dg/b", "/dg/home"]
    );
}

fn node_ids(output: &ModeOutput) -> Vec<String> {
    graph(output)["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect()
}

fn edges(output: &ModeOutput) -> Vec<(String, String)> {
    graph(output)["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let end = |key: &str| edge[key].as_str().unwrap().to_owned();
            (end("source"), end("target"))
        })
        .collect()
}

/// A note due at the build, which links to two notes dated after it: one by its front matter,
/// one by its file name; the first links back.
fn with_future_notes() -> Vec<SourceDoc> {
    vec![
        with_body(
            "_digital_garden/now.md",
            "title: Now\ndate: 2026-10-01",
            "Soon: [[Later]] and [[Tomorrow]].\n",
        ),
        with_body(
            "_digital_garden/later.md",
            "title: Later\ndate: 2026-10-25 12:00",
            "Written before [[Now]].\n",
        ),
        with_body(
            "_digital_garden/quotes/2026-10-26-tomorrow.md",
            "title: Tomorrow",
            "Nothing yet.\n",
        ),
    ]
}

#[test]
fn a_note_dated_after_the_build_is_linked_to_and_links_but_has_no_page() {
    let output = build_all(&with_future_notes()).unwrap();
    let pages: Vec<&str> = output.pages.iter().map(|page| page.route.url()).collect();
    assert_eq!(pages, ["/dg/now"]);
    assert_eq!(
        output.entries[0].body_html,
        "<p>Soon: <a href=\"/dg/later\" class=\"internal-link\">Later</a> and \
         <a href=\"/dg/quotes/2026-10-26-tomorrow\" class=\"internal-link\">Tomorrow</a>.</p>\n"
    );
    assert_eq!(output.warnings, []);
    assert_eq!(
        output.pages[0].context["backlinks"],
        json!([{"title": "Later", "url": "/dg/later", "excerpt": "Written before [[Now]].\n"}])
    );
    assert_eq!(
        node_ids(&output),
        ["/dg/now", "/dg/later", "/dg/quotes/2026-10-26-tomorrow"]
    );
    let edge = |source: &str, target: &str| (source.to_owned(), target.to_owned());
    assert_eq!(
        edges(&output),
        [
            edge("/dg/later", "/dg/now"),
            edge("/dg/now", "/dg/later"),
            edge("/dg/now", "/dg/quotes/2026-10-26-tomorrow"),
        ]
    );
}

#[test]
fn a_note_dated_after_the_build_has_a_page_once_the_build_reaches_its_date() {
    let docs = with_future_notes();
    let only: Vec<&str> = docs.iter().map(|doc| doc.id.as_str()).collect();
    let pages_at = |now: &str| -> Vec<String> {
        let output = build(&docs, ROOT, &only, &LastCommits::default(), &clock_at(now)).unwrap();
        // The graph does not change with the build time: every note is in it.
        assert_eq!(node_ids(&output).len(), 3);
        assert_eq!(edges(&output).len(), 3);
        output
            .pages
            .iter()
            .map(|page| page.route.url().to_owned())
            .collect()
    };
    assert_eq!(pages_at("2026-10-25T09:00:00Z"), ["/dg/now"]);
    assert_eq!(pages_at("2026-10-25T11:00:00Z"), ["/dg/now", "/dg/later"]);
    assert_eq!(
        pages_at("2026-10-25T23:00:00Z"),
        ["/dg/now", "/dg/later", "/dg/quotes/2026-10-26-tomorrow"]
    );
}

/// The plugin indexed the notes in their order, so when two share a key the later one, by date,
/// wins; a title wins over a file name whatever the order.
#[test]
fn a_key_two_notes_share_names_the_later_and_a_title_beats_a_file_name() {
    let docs = [
        with_body(
            "_digital_garden/a/index.md",
            "title: Coding\ndate: 2021-01-01",
            "",
        ),
        with_body(
            "_digital_garden/z/index.md",
            "title: Reading\ndate: 2020-01-01",
            "",
        ),
        with_body(
            "_digital_garden/links.md",
            "title: Links\ndate: 2022-01-01",
            "[[index]], [[Rust]].\n",
        ),
        with_body(
            "_digital_garden/rust.md",
            "title: Oxide\ndate: 2023-01-01",
            "",
        ),
        with_body(
            "_digital_garden/old.md",
            "title: Rust\ndate: 2019-01-01",
            "",
        ),
    ];
    let output = build_all(&docs).unwrap();
    let links = output
        .entries
        .iter()
        .find(|entry| entry.doc.as_str() == "_digital_garden/links.md")
        .unwrap();
    assert_eq!(
        links.body_html,
        "<p><a href=\"/dg/a/index\" class=\"internal-link\">index</a>, \
         <a href=\"/dg/old\" class=\"internal-link\">Rust</a>.</p>\n"
    );
}

#[test]
fn a_note_hides_the_category_menu() {
    let docs = [note("_digital_garden/a.md", "title: A\ndate: 2026-01-01")];
    assert_eq!(
        build_all(&docs).unwrap().pages[0].context["hide_website_junk"],
        json!({"main_nav": false, "title": false, "cat_jumbotron": true, "footer": false})
    );
}

#[test]
fn the_graph_keeps_the_plugins_key_order() {
    let output = build_all(&mutual()).unwrap();
    assert!(
        output.artifacts[0]
            .body
            .starts_with(r#"{"nodes":[{"id":"/dg/a","path":"/dg/a","label":"Note A"},"#)
    );
    assert!(
        output.artifacts[0]
            .body
            .contains(r#"],"edges":[{"source":"/dg/b","target":"/dg/a"},"#)
    );
}
