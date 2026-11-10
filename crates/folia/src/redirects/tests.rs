use std::collections::BTreeMap;

use folia_core::content::{FrontMatter, Route, SourceDoc};

use super::*;
use crate::discover::Discovery;
use crate::site::assemble;

/// A document of `mode` at `path`, published at `url`, redirected to from `from`.
struct Doc {
    mode: Mode,
    path: &'static str,
    url: &'static str,
    from: &'static [&'static str],
}

fn site(docs: &[Doc]) -> Site {
    let mut documents: BTreeMap<Mode, Vec<SourceDoc>> = BTreeMap::new();
    let mut planned = Vec::new();
    for doc in docs {
        let id = DocId::new(doc.path);
        documents.entry(doc.mode).or_default().push(SourceDoc {
            file_time: None,
            id: id.clone(),
            mode: doc.mode,
            front: FrontMatter::parse("").unwrap(),
            body: String::new(),
        });
        let mut output = PlannedOutput::authored(Route::html(doc.url).unwrap(), id);
        output.redirect_from = doc.from.iter().map(|f| Route::html(f).unwrap()).collect();
        planned.push(output);
    }
    Site {
        discovery: Discovery::default(),
        documents,
        routes: assemble(planned, &[]).unwrap(),
    }
}

#[test]
fn collections_come_first_then_pages_by_file_name() {
    let site = site(&[
        Doc {
            mode: Mode::Pages,
            path: "pages/gamma.md",
            url: "/g",
            from: &["/old-g", "/older-g"],
        },
        Doc {
            mode: Mode::Pages,
            path: "other/beta.html",
            url: "/b",
            from: &["/old-b"],
        },
        Doc {
            mode: Mode::Pages,
            path: "pages/alpha.md",
            url: "/a",
            from: &["/old-a"],
        },
        Doc {
            mode: Mode::Garden,
            path: "_garden/note.md",
            url: "/garden/note",
            from: &["/old-note"],
        },
        Doc {
            mode: Mode::Micro,
            path: "_micro/kind/2022-01-01-entry.md",
            url: "/kind/entry",
            from: &["/old-entry"],
        },
        Doc {
            mode: Mode::Blog,
            path: "_posts/2020/2020-01-01-post.md",
            url: "/blog/post",
            from: &["/old-post"],
        },
    ]);
    assert_eq!(
        redirects_json("https://example.com", &site),
        concat!(
            r#"{"/old-post":"https://example.com/blog/post","#,
            r#""/old-entry":"https://example.com/kind/entry","#,
            r#""/old-note":"https://example.com/garden/note","#,
            r#""/old-a":"https://example.com/a","#,
            r#""/old-b":"https://example.com/b","#,
            r#""/old-g":"https://example.com/g","/older-g":"https://example.com/g"}"#,
        )
    );
}

#[test]
fn a_site_without_redirects_has_an_empty_object() {
    let site = site(&[Doc {
        mode: Mode::Pages,
        path: "a.md",
        url: "/a",
        from: &[],
    }]);
    assert_eq!(redirects_json("https://example.com", &site), "{}");
}

#[test]
fn a_url_listed_twice_keeps_its_first_place_and_its_last_target() {
    let json = serialize([
        ("/x".to_owned(), "https://e.com/first".to_owned()),
        ("/y".to_owned(), "https://e.com/y".to_owned()),
        ("/x".to_owned(), "https://e.com/last".to_owned()),
    ]);
    assert_eq!(
        json,
        r#"{"/x":"https://e.com/last","/y":"https://e.com/y"}"#
    );
}

#[test]
fn urls_are_escaped_as_json_strings_and_kept_as_utf_8() {
    let json = serialize([(r#"/a "b"\c/é"#.to_owned(), "https://e.com/€".to_owned())]);
    assert_eq!(json, r#"{"/a \"b\"\\c/é":"https://e.com/€"}"#);
}
