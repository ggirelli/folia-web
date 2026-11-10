//! The pages mode: files with front matter outside every other mode's root.
//!
//! A page's output extension comes from its permalink, when that has one, or else from its
//! source: Markdown becomes `.html`, Sass becomes `.css`, anything else keeps its extension.
//!
//! Routes, as the site has published them under Jekyll:
//!
//! | Page | URL |
//! |-|-|
//! | with a `permalink` | the permalink (`/about`, `/feed.xml`, `/`) |
//! | an HTML page named `index` | its directory (`/dir/`) |
//! | another HTML page | `/dir/name`, without extension |
//! | any other page | `/dir/name.ext` (`/assets/css/main.css`) |
//!
//! Templates (plan v6, §6.4): a page's `layout` names its template, `layout: page` being
//! `page.html`. The template shows the page as `page`:
//!
//! | Key | Value |
//! |-|-|
//! | `url` | the page's URL (`/about`) |
//! | `title` | its front matter `title` |
//! | `layout` | its front matter `layout`, or none: the page is then written as its content alone (`NO_LAYOUT_TEMPLATE`), as Jekyll writes it |
//! | `hide_title` | whether its front matter `hide_title` hides its title, `false` by default |
//! | `modified` | its last commit's time in the site's time zone (`2026-01-04T13:36:24+01:00`), or none |
//! | `toc` | its table of contents, `{placement, title, entries}`, or none (`folia_core`'s `Toc`) |
//! | `cover` | its cover, `{path, position, show, unsplash}` (`folia_core`'s `Cover`) |
//! | `hide_website_junk` | the parts of the frame it hides, each `true` or `false` |
//! | `title_icon` | the classes of the icon its front matter `title_icon` puts before its title, or none |
//! | `corner_backlink` | `{show, url, icon_class}`: the link back in its corner, its front matter over the site's defaults (not shown, to `/`) |
//! | `error_type` | its front matter `error-type`, the error an error page shows (`404 Not Found`), or none |
//! | `author` | its front matter `author`, a key of the site's authors (the author page's, `layout: author`), or none |
//! | `additional_body_class` | its front matter `additional_body_class`, a class the page's `<body>` adds, or none |
//! | `date`, `description`, `previous`, `next` | none: a page has no date, excerpt or neighbours |
//! | `categories`, `tags`, `syndications` | empty |
//!
//! The keys a page lacks are there, empty, because the templates of pages and posts share their
//! parts (plan v8, §12.2 step 2).
//!
//! So far [`build`] builds what the vertical slice needs (plan v8, §12.2): Markdown pages with a
//! layout and a title. Front matter it cannot show is refused, never ignored:
//!
//! | Front matter | Outcome |
//! |-|-|
//! | `layout` (or none), `author`, `additional_body_class`, `title`, `hide_title`, `title_icon`, `corner_backlink`, `error-type`, `permalink`, `redirect_from`, `published`, `hide_website_junk` | built |
//! | an HTML page's body, without Liquid | built: written as it is, as Jekyll converts only Markdown |
//! | a page another mode claims (`claimed`: its source and template) | built with that template, its body not rendered: the template lists what the other mode built, which the builder adds |
//! | `show_author: true` | built: an author box shows whenever a page has an author |
//! | Liquid in an HTML page; a table of contents on one | refused as not built yet |
//! | `cover` without a picture, shown behind the page, or after the title | built |
//! | `toc` disabled, or placed on the left | built |
//! | `comments`, `narrow_post` set to `false` | built: the site's default, which changes nothing |
//! | `comments`, `narrow_post` set to `true`; `last_modified_at`; a table of contents on the right or at the top | refused as not built yet |
//! | a value those keys cannot take (`comments: maybe`, a cover position or TOC placement Jekyll's templates do not know) | refused as invalid |
//! | `narrow-post` set to `true` or `false` | built, and ignored: see [`IGNORED_TYPOS`] |
//! | any other key | refused as unknown, which may be a typo (`narow_post`) |

use folia_core::content::markdown::to_html;
use folia_core::content::{
    Cover, Diagnostic, LastCommits, Route, SiteClock, SourceDoc, Toc, TocPlacement, cover,
    hide_website_junk, modified, table_of_contents,
};
use folia_core::publish::{EntrySummary, ModeOutput, NO_LAYOUT_TEMPLATE, PageSpec, PlannedOutput};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The front matter keys a page may have, besides the switches.
const KEYS: [&str; 14] = [
    "layout",
    "author",
    "additional_body_class",
    "title",
    "hide_title",
    "title_icon",
    "corner_backlink",
    "error-type",
    "permalink",
    "redirect_from",
    "published",
    "cover",
    "toc",
    "hide_website_junk",
];

/// Switches the page template does not have: a page may only leave them off, as the site's
/// default does.
const SWITCHES: [&str; 2] = ["comments", "narrow_post"];

/// Misspelt keys a page may have, which change nothing, with why each is accepted.
///
/// Folia refuses a front matter key it does not know, because it may be a typo whose page would
/// silently come out wrong. These are the exceptions: typos that a site's published pages
/// already have, which Jekyll ignored, as it ignores every key no template reads. Building them
/// as Jekyll did keeps the page as it was published; refusing them would leave it unbuilt.
/// Each exception is one exact key, on pages only, with a `true` or `false` value; any other
/// unknown key is still refused (owner decision Q3h, 2026-11-07).
pub const IGNORED_TYPOS: [(&str, &str); 1] = [(
    "narrow-post",
    "a typo of narrow_post; no template reads narrow-post, so the page is not narrower",
)];

/// Source extensions that a converter renders as HTML.
const MARKDOWN_EXTENSIONS: [&str; 5] = [".markdown", ".mkdown", ".mkdn", ".mkd", ".md"];

/// The outputs of the pages: one per published page.
///
/// # Errors
///
/// Returns every problem found: front matter that cannot be read, or a permalink that is not a
/// valid route for the page's output.
pub fn routes(
    docs: &[SourceDoc],
    clock: &SiteClock,
) -> Result<Vec<PlannedOutput>, Vec<Diagnostic>> {
    let mut outputs = Vec::new();
    let mut problems = Vec::new();
    for doc in docs
        .iter()
        .filter(|doc| doc.front.published() || clock.unpublished_allowed())
    {
        match page_output(doc) {
            Ok(output) => outputs.push(output),
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok(outputs)
    } else {
        Err(problems)
    }
}

/// Builds the published pages among `docs`: each one's entry, with its body as HTML, and the
/// page its layout renders. A page's `modified` time is its last commit's (`commits`), shown in
/// the time zone of `clock`. A page in `claimed` (its source, and the template that lists what
/// another mode built) is rendered with that template, and its body is not rendered.
///
/// # Errors
///
/// Returns every problem found: what [`routes`] reports, a page that is not Markdown or has no
/// layout or title, a front matter `last_modified_at`, which Jekyll ignores on pages and Folia
/// does not read yet, and other front matter the page template cannot show (see the module's
/// table).
pub fn build(
    docs: &[SourceDoc],
    commits: &LastCommits,
    clock: &SiteClock,
    claimed: &BTreeMap<String, String>,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    for doc in docs
        .iter()
        .filter(|doc| doc.front.published() || clock.unpublished_allowed())
    {
        let claim = claimed.get(doc.id.as_str()).map(String::as_str);
        match build_page(doc, commits, clock, claim) {
            Ok((entry, page)) => {
                output.entries.push(entry);
                output.pages.push(page);
            }
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok(output)
    } else {
        Err(problems)
    }
}

fn build_page(
    doc: &SourceDoc,
    commits: &LastCommits,
    clock: &SiteClock,
    claim: Option<&str>,
) -> Result<(EntrySummary, PageSpec), Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let planned = page_output(doc)?;
    let extension = doc.id.extension().to_lowercase();
    let markdown = MARKDOWN_EXTENSIONS.contains(&extension.as_str());
    if !markdown && extension != ".html" {
        return Err(problem(&"only Markdown and HTML pages are built yet"));
    }
    if claim.is_none() && !markdown && (doc.body.contains("{%") || doc.body.contains("{{")) {
        return Err(problem(&"Liquid in an HTML page is not built yet"));
    }
    let layout = doc.front.string("layout").map_err(|e| problem(&e))?;
    // Jekyll gives a page without a title none: `page.title` is empty.
    let title = doc
        .front
        .string("title")
        .map_err(|e| problem(&e))?
        .unwrap_or_default();
    if doc.front.get("last_modified_at").is_some() {
        return Err(problem(
            &"last_modified_at is not read on pages yet; the last commit dates them",
        ));
    }
    unknown_or_unbuilt(doc).map_err(|message| problem(&message))?;
    let modified = modified(doc, commits, None, clock)?;
    let toc = table_of_contents(doc)?;
    if !markdown && toc.is_some() {
        return Err(problem(
            &"a table of contents on an HTML page is not built yet",
        ));
    }
    let error_type = doc.front.string("error-type").map_err(|e| problem(&e))?;
    let cover = cover(doc)?;
    unshown(&cover, toc.as_ref()).map_err(|message| problem(&message))?;
    let hide_title = match doc.front.json("hide_title") {
        None => false,
        Some(Value::Bool(hide)) => hide,
        Some(_) => return Err(problem(&"hide_title should be true or false")),
    };
    let title_icon = doc.front.string("title_icon").map_err(|e| problem(&e))?;
    let author = doc.front.string("author").map_err(|e| problem(&e))?;
    let body_class = doc
        .front
        .string("additional_body_class")
        .map_err(|e| problem(&e))?;
    let corner_backlink = corner_backlink(doc).map_err(|message| problem(&message))?;

    let context = json!({
        "url": planned.route.url(),
        "title": title,
        "layout": layout,
        "modified": modified.map(|m| clock.xmlschema(m.at)),
        "toc": toc.as_ref().map(Toc::to_json),
        "cover": cover.to_json(),
        "hide_website_junk": hide_website_junk(doc),
        "narrow_post": false,
        "error_type": error_type,
        "hide_title": hide_title,
        "title_icon": title_icon,
        "corner_backlink": corner_backlink,
        "date": null,
        "description": null,
        "author": author,
        "additional_body_class": body_class,
        "categories": [],
        "tags": [],
        "syndications": [],
        "previous": null,
        "next": null,
    });
    let page = PageSpec {
        route: planned.route.clone(),
        template: match (claim, &layout) {
            (Some(template), _) => template.to_owned(),
            (None, Some(layout)) => format!("{layout}.html"),
            (None, None) => NO_LAYOUT_TEMPLATE.to_owned(),
        },
        source: Some(doc.id.clone()),
        context,
    };
    let entry = EntrySummary {
        doc: doc.id.clone(),
        route: planned.route,
        published: None,
        modified,
        // A claimed page's body is not rendered: its template lists what another mode built.
        // An HTML page is written as it is: Jekyll converts only Markdown.
        body_html: if claim.is_some() {
            String::new()
        } else if markdown {
            to_html(&doc.body, None)
        } else {
            doc.body.clone()
        },
        redirect_from: planned.redirect_from,
    };
    Ok((entry, page))
}

/// Why `doc`'s front matter has a key the page template cannot show, if it has one: a key it
/// does not know, one it knows but has not built, or a switch turned on.
fn unknown_or_unbuilt(doc: &SourceDoc) -> Result<(), String> {
    for key in doc.front.keys() {
        if KEYS.contains(&key) || key == "last_modified_at" {
            continue;
        }
        if IGNORED_TYPOS.iter().any(|(typo, _)| *typo == key) {
            match doc.front.json(key) {
                Some(Value::Bool(_)) => continue,
                _ => return Err(format!("{key} should be true or false")),
            }
        }
        if key == "show_author" {
            match doc.front.json(key) {
                Some(Value::Bool(true)) => continue,
                Some(Value::Bool(false)) => {
                    return Err("show_author: false is not built yet".to_owned());
                }
                _ => return Err("show_author should be true or false".to_owned()),
            }
        }
        if !SWITCHES.contains(&key) {
            return Err(format!(
                "the front matter key {key} is not one pages have (a typo?)"
            ));
        }
        match doc.front.json(key) {
            Some(Value::Bool(false)) => {}
            Some(Value::Bool(true)) => return Err(format!("{key}: true is not built yet")),
            _ => return Err(format!("{key} should be true or false")),
        }
    }
    Ok(())
}

/// The link back a page shows in its corner: its front matter `corner_backlink`, each key over
/// the site's default for pages (`show: false`, `url: "/"`, `icon_class: "fas
/// fa-angle-double-left"`), as Jekyll merged them.
fn corner_backlink(doc: &SourceDoc) -> Result<Value, String> {
    let mut backlink = json!({"show": false, "url": "/", "icon_class": "fas fa-angle-double-left"});
    match doc.front.json("corner_backlink") {
        None | Some(Value::Null) => {}
        Some(Value::Object(set)) => {
            for (key, value) in set {
                match (key.as_str(), &value) {
                    ("show", Value::Bool(_)) | ("url" | "icon_class", Value::String(_)) => {
                        backlink[key.as_str()] = value;
                    }
                    ("show", _) => {
                        return Err("corner_backlink: show should be true or false".to_owned());
                    }
                    ("url" | "icon_class", _) => {
                        return Err(format!("corner_backlink: {key} should be text"));
                    }
                    _ => return Err(format!("corner_backlink: the key {key} is not built yet")),
                }
            }
        }
        Some(_) => return Err("corner_backlink should be a mapping".to_owned()),
    }
    Ok(backlink)
}

/// Why the page template cannot show `cover` or `toc`, if it cannot: the template shows a
/// cover behind the page or after the title, and a table of contents on the left, and nothing
/// else yet.
fn unshown(cover: &Cover, toc: Option<&Toc>) -> Result<(), String> {
    if !matches!(cover.position.as_str(), "behind" | "after_title") {
        return Err(format!("cover: unknown position {:?}", cover.position));
    }
    match toc.map(|toc| toc.placement) {
        None | Some(TocPlacement::Left) => Ok(()),
        Some(placement) => Err(format!(
            "a table of contents placed {} is not built yet",
            match placement {
                TocPlacement::Top => "at the top",
                _ => "on the right",
            }
        )),
    }
}

fn page_output(doc: &SourceDoc) -> Result<PlannedOutput, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let permalink = doc.front.string("permalink").map_err(|e| problem(&e))?;
    let extension = output_extension(doc, permalink.as_deref());
    let html = extension == ".html";
    let dir = doc.id.dir();
    let url = match permalink {
        Some(permalink) => permalink,
        None if !html => format!("/{dir}/{}{extension}", doc.id.stem()),
        None if doc.id.stem() == "index" => format!("/{dir}/"),
        None => format!("/{dir}/{}", doc.id.stem()),
    };
    let route = if html {
        Route::html(&url)
    } else if url.ends_with(&extension) {
        Route::file(&url)
    } else {
        return Err(problem(&format!(
            "the URL {url} of a page written as {extension} must end with {extension}"
        )));
    }
    .map_err(|e| problem(&e))?;
    Ok(PlannedOutput {
        redirect_from: doc.redirect_from()?,
        ..PlannedOutput::authored(route, doc.id.clone())
    })
}

/// The extension a page is written with.
fn output_extension(doc: &SourceDoc, permalink: Option<&str>) -> String {
    let from_permalink = permalink
        .filter(|permalink| !permalink.ends_with('/'))
        .and_then(|permalink| permalink.rsplit('/').next())
        .and_then(|name| {
            name.rfind('.')
                .filter(|&i| i > 0)
                .map(|i| name[i..].to_owned())
        });
    from_permalink.unwrap_or_else(|| {
        let source = doc.id.extension();
        if MARKDOWN_EXTENSIONS.contains(&source.to_lowercase().as_str()) {
            ".html".to_owned()
        } else if source == ".scss" || source == ".sass" {
            ".css".to_owned()
        } else {
            source.to_owned()
        }
    })
}

#[cfg(test)]
mod tests {
    use folia_core::content::{DocId, FrontMatter, Mode, ModifiedFrom};

    use super::*;

    fn page(path: &str, front: &str) -> SourceDoc {
        SourceDoc {
            id: DocId::new(path),
            mode: Mode::Pages,
            front: FrontMatter::parse(front).unwrap(),
            body: String::new(),
        }
    }

    fn clock() -> SiteClock {
        SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
    }

    fn about() -> SourceDoc {
        SourceDoc {
            body: "### Other things\n".into(),
            ..page(
                "pages/profile.md",
                "layout: page\ntitle: About me\npermalink: /about\nredirect_from:\n  - /blog/about",
            )
        }
    }

    #[test]
    fn an_html_page_is_written_as_it_is_with_its_error_type() {
        let doc = SourceDoc {
            body: "\n<p class=\"x\">Not *Markdown*</p>\n".to_owned(),
            ..page(
                "404.html",
                "layout: error\ntitle: 404\npermalink: /404.html\nerror-type: 404 Not Found",
            )
        };
        let output = build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new()).unwrap();
        assert_eq!(
            output.entries[0].body_html,
            "\n<p class=\"x\">Not *Markdown*</p>\n"
        );
        assert_eq!(output.pages[0].template, "error.html");
        assert_eq!(output.pages[0].context["error_type"], "404 Not Found");
    }

    #[test]
    fn an_author_page_names_its_author_and_other_pages_none() {
        let author = |front| {
            let doc = page("me/index.html", front);
            build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new()).map(|output| {
                (
                    output.pages[0].template.clone(),
                    output.pages[0].context["author"].clone(),
                )
            })
        };
        assert_eq!(
            author("layout: author\ntitle: gg\nauthor: gg\npermalink: /").unwrap(),
            ("author.html".to_owned(), json!("gg"))
        );
        assert_eq!(
            author("layout: page\ntitle: T\npermalink: /p").unwrap().1,
            Value::Null
        );
        let problems = author("layout: author\ntitle: gg\nauthor: [gg]").unwrap_err();
        assert_eq!(
            problems[0].to_string(),
            "me/index.html: front matter key \"author\" should be a single value"
        );
    }

    #[test]
    fn a_claimed_page_takes_the_claim_s_template_and_no_body() {
        let doc = SourceDoc {
            body: "{% for item in site.micro %}{{ item.title }}{% endfor %}\n".to_owned(),
            ..page(
                "blog/microblog.html",
                "layout: page\ntitle: A microblog.\nauthor: gg\nshow_author: true\npermalink: /micro\nredirect_from:\n  - /social\ncomments: false",
            )
        };
        let claimed: BTreeMap<String, String> = [(
            "blog/microblog.html".to_owned(),
            "micro_index.html".to_owned(),
        )]
        .into();
        let output = build(
            std::slice::from_ref(&doc),
            &LastCommits::default(),
            &clock(),
            &claimed,
        )
        .unwrap();
        assert_eq!(output.pages[0].template, "micro_index.html");
        assert_eq!(output.pages[0].route.url(), "/micro");
        assert_eq!(output.pages[0].context["author"], "gg");
        assert_eq!(output.entries[0].body_html, "");
        assert_eq!(output.entries[0].redirect_from[0].url(), "/social");
        let unclaimed = build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new());
        assert_eq!(
            unclaimed.unwrap_err()[0].to_string(),
            "blog/microblog.html: Liquid in an HTML page is not built yet"
        );
    }

    #[test]
    fn an_additional_body_class_is_passed_on() {
        let doc = page(
            "blog/tags.html",
            "layout: default\ntitle: Tags\nadditional_body_class: tags-index",
        );
        let output = build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new()).unwrap();
        assert_eq!(
            output.pages[0].context["additional_body_class"],
            "tags-index"
        );
    }

    #[test]
    fn show_author_is_built_only_on() {
        let shown = |value: &str| {
            build(
                &[page(
                    "pages/p.md",
                    &format!("layout: page\ntitle: T\nshow_author: {value}"),
                )],
                &LastCommits::default(),
                &clock(),
                &BTreeMap::new(),
            )
            .map_err(|problems| problems[0].to_string())
        };
        assert!(shown("true").is_ok());
        assert_eq!(
            shown("false").unwrap_err(),
            "pages/p.md: show_author: false is not built yet"
        );
        assert_eq!(
            shown("maybe").unwrap_err(),
            "pages/p.md: show_author should be true or false"
        );
    }

    #[test]
    fn a_title_icon_is_passed_on_and_none_by_default() {
        let built = |front| {
            let doc = page("pages/p.md", front);
            build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new())
                .unwrap()
                .pages[0]
                .context["title_icon"]
                .clone()
        };
        assert_eq!(
            built("layout: page\ntitle: T\npermalink: /p\ntitle_icon: fas fa-code"),
            "fas fa-code"
        );
        assert_eq!(built("layout: page\ntitle: T\npermalink: /p"), Value::Null);
    }

    #[test]
    fn a_corner_backlink_is_the_front_matter_over_the_defaults() {
        let doc = page(
            "me/p.md",
            "layout: page\ntitle: T\npermalink: /me/p\ncorner_backlink:\n  show: true",
        );
        let output = build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new()).unwrap();
        assert_eq!(
            output.pages[0].context["corner_backlink"],
            json!({"show": true, "url": "/", "icon_class": "fas fa-angle-double-left"})
        );
    }

    #[test]
    fn a_page_is_built_with_the_template_its_layout_names() {
        let output = build(
            &[about()],
            &LastCommits::default(),
            &clock(),
            &BTreeMap::new(),
        )
        .unwrap();
        let [entry] = &output.entries[..] else {
            panic!("one entry")
        };
        let [page] = &output.pages[..] else {
            panic!("one page")
        };
        assert_eq!(entry.route.url(), "/about");
        assert_eq!(
            entry.body_html,
            "<h3 id=\"other-things\">Other things</h3>\n"
        );
        assert_eq!(entry.redirect_from, [Route::html("/blog/about").unwrap()]);
        assert_eq!(page.template, "page.html");
        assert_eq!(page.source, Some(DocId::new("pages/profile.md")));
        assert_eq!(
            page.context,
            json!({
                "url": "/about", "title": "About me", "layout": "page", "modified": null,
                "toc": null,
                "cover": {"path": null, "position": "after_title", "show": true, "unsplash": null},
                "hide_website_junk": {
                    "main_nav": false, "title": false, "cat_jumbotron": false, "footer": false,
                },
                "narrow_post": false,
                "error_type": null,
                "hide_title": false,
                "title_icon": null,
                "corner_backlink": {"show": false, "url": "/", "icon_class": "fas fa-angle-double-left"},
                "date": null, "description": null, "author": null, "additional_body_class": null, "categories": [], "tags": [],
                "syndications": [], "previous": null, "next": null,
            })
        );
        assert_eq!((entry.published, entry.modified), (None, None));
    }

    #[test]
    fn a_page_is_modified_at_its_last_commit_in_the_site_time_zone() {
        let commits: LastCommits = [(
            DocId::new("pages/profile.md"),
            "2026-01-04T12:36:24Z".parse().unwrap(),
        )]
        .into_iter()
        .collect();
        let output = build(&[about()], &commits, &clock(), &BTreeMap::new()).unwrap();
        assert_eq!(
            output.pages[0].context["modified"],
            "2026-01-04T13:36:24+01:00"
        );
        assert_eq!(
            output.entries[0].modified.map(|m| m.from),
            Some(ModifiedFrom::Git)
        );
    }

    #[test]
    fn a_page_without_a_layout_is_written_as_its_content() {
        let doc = page(
            "pages/log.md",
            "title: Log\npermalink: /log\nhide_title: true",
        );
        let output = build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new()).unwrap();
        assert_eq!(output.pages[0].template, NO_LAYOUT_TEMPLATE);
        assert_eq!(output.pages[0].context["layout"], Value::Null);
        assert_eq!(output.pages[0].context["hide_title"], true);
    }

    #[test]
    fn a_page_with_liquid_or_last_modified_at_is_not_built_yet() {
        let problems = build(
            &[
                SourceDoc {
                    body: "{% for post in site.posts %}{% endfor %}".to_owned(),
                    ..page("blog/c.html", "layout: page\ntitle: C")
                },
                page(
                    "pages/d.md",
                    "layout: page\ntitle: D\nlast_modified_at: 2026-01-01",
                ),
            ],
            &LastCommits::default(),
            &clock(),
            &BTreeMap::new(),
        )
        .unwrap_err();
        let messages: Vec<_> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "blog/c.html: Liquid in an HTML page is not built yet",
                "pages/d.md: last_modified_at is not read on pages yet; the last commit dates them",
            ]
        );
    }

    #[test]
    fn a_page_without_a_title_has_an_empty_one_as_in_jekyll_and_a_titled_page_keeps_its_own() {
        let output = build(
            &[
                page("pages/b.md", "layout: page"),
                page("pages/c.md", "layout: page\ntitle:"),
                page("pages/d.md", "layout: page\ntitle: Named"),
            ],
            &LastCommits::default(),
            &clock(),
            &BTreeMap::new(),
        )
        .unwrap();
        let titles: Vec<_> = output
            .pages
            .iter()
            .map(|p| p.context["title"].clone())
            .collect();
        assert_eq!(titles, ["", "", "Named"]);
    }

    /// A page shaped like `pages/disclosure.md`, with a table of contents on the left.
    fn policy(toc: &str) -> SourceDoc {
        SourceDoc {
            body: "# Policy\n\n## Purpose\n\n## Scope\n\n### Out of scope\n\n##### Deep\n\n\
                   ## Timelines & response\n"
                .into(),
            ..page(
                "pages/disclosure.md",
                &format!(
                    "layout: page\ntitle: Vulnerability Disclosure Policy\npermalink: /security-policy\n{toc}"
                ),
            )
        }
    }

    fn build_one(doc: SourceDoc) -> Result<ModeOutput, Vec<Diagnostic>> {
        build(&[doc], &LastCommits::default(), &clock(), &BTreeMap::new())
    }

    #[test]
    fn a_page_with_a_toc_on_the_left_gets_its_placement_and_entries() {
        let output = build_one(policy("toc:\n  enabled: true\n  position: left\n")).unwrap();
        assert_eq!(
            output.pages[0].context["toc"],
            json!({"placement": "left", "title": null, "entries": [
                {"id": "purpose", "html": "Purpose", "children": []},
                {"id": "scope", "html": "Scope", "children": [
                    {"id": "out-of-scope", "html": "Out of scope", "children": []},
                ]},
                {"id": "timelines--response", "html": "Timelines &amp; response", "children": []},
            ]})
        );
    }

    #[test]
    fn a_page_without_an_enabled_toc_has_none() {
        for toc in ["", "toc:\n  enabled: false\n  position: left\n"] {
            let output = build_one(policy(toc)).unwrap();
            assert_eq!(output.pages[0].context["toc"], Value::Null, "{toc:?}");
        }
        let about = build_one(about()).unwrap();
        assert_eq!(about.pages[0].context["toc"], Value::Null);
    }

    #[test]
    fn switches_left_off_and_a_cover_behind_the_page_are_built() {
        let front = "comments: false\nnarrow_post: false\nhide_website_junk:\n  title: true\n\
                     cover:\n  path: /c.webp\n  position: behind\n";
        let output = build_one(policy(front)).unwrap();
        assert_eq!(output.pages[0].context["cover"]["position"], "behind");
        assert_eq!(output.pages[0].context["hide_website_junk"]["title"], true);
    }

    #[test]
    fn the_narrow_post_typo_is_built_as_jekyll_ignored_it() {
        // The site's security hall of fame has `narrow-post: true` (Q3h): Jekyll's templates
        // read only `narrow_post`, so the page was published at its usual width.
        for typo in ["narrow-post: true", "narrow-post: false"] {
            let output = build_one(policy(typo)).unwrap();
            let typoless = build_one(policy("")).unwrap();
            assert_eq!(
                output.pages[0].context, typoless.pages[0].context,
                "{typo:?}"
            );
            assert_eq!(output.pages[0].context["narrow_post"], false);
        }
    }

    #[test]
    fn front_matter_the_page_template_cannot_show_is_refused_with_its_key() {
        let cases = [
            (
                "title_icon: [fas]",
                "front matter key \"title_icon\" should be a single value",
            ),
            (
                "corner_backlink:\n  label: back",
                "corner_backlink: the key label is not built yet",
            ),
            ("hide_title: maybe", "hide_title should be true or false"),
            ("comments: true", "comments: true is not built yet"),
            ("narrow_post: maybe", "narrow_post should be true or false"),
            ("narrow-post: maybe", "narrow-post should be true or false"),
            (
                "narrow_posts: true",
                "the front matter key narrow_posts is not one pages have (a typo?)",
            ),
            (
                "narrow-posts: true",
                "the front matter key narrow-posts is not one pages have (a typo?)",
            ),
            (
                "cover:\n  path: /c.webp\n  position: top",
                "cover: unknown position \"top\"",
            ),
            (
                "toc:\n  enabled: true\n  position: right",
                "a table of contents placed on the right is not built yet",
            ),
            (
                "toc:\n  enabled: true",
                "a table of contents placed at the top is not built yet",
            ),
            (
                "toc:\n  enabled: true\n  position: middle",
                "toc: unknown position \"middle\"",
            ),
        ];
        for (front, message) in cases {
            let problems = build_one(policy(front)).unwrap_err();
            assert_eq!(
                problems[0].to_string(),
                format!("pages/disclosure.md: {message}"),
                "{front:?}"
            );
        }
    }

    fn route(doc: SourceDoc) -> (String, String) {
        let output = routes(&[doc], &clock()).unwrap().remove(0);
        (output.route.url().to_owned(), output.route.output_path())
    }

    #[test]
    fn a_root_permalink_is_the_site_index() {
        assert_eq!(
            route(page("profile/index.html", "permalink: /")),
            ("/".into(), "index.html".into())
        );
    }

    #[test]
    fn a_permalink_with_an_extension_names_the_file() {
        assert_eq!(
            route(page("pages/rss.xml", "permalink: /feed.xml")),
            ("/feed.xml".into(), "feed.xml".into())
        );
        assert_eq!(
            route(page("blog/tags.html", "permalink: /blog/tags.html")),
            ("/blog/tags.html".into(), "blog/tags.html".into())
        );
    }

    #[test]
    fn an_extensionless_permalink_is_an_html_page() {
        assert_eq!(
            route(page("pages/profile.md", "permalink: /about")),
            ("/about".into(), "about.html".into())
        );
    }

    #[test]
    fn without_a_permalink_the_path_names_the_page() {
        assert_eq!(
            route(page("blog/index.html", "")),
            ("/blog/".into(), "blog/index.html".into())
        );
        assert_eq!(
            route(page("blog/microblog.html", "")),
            ("/blog/microblog".into(), "blog/microblog.html".into())
        );
        assert_eq!(
            route(page("assets/css/main.scss", "")),
            ("/assets/css/main.css".into(), "assets/css/main.css".into())
        );
        assert_eq!(
            route(page("assets/js/lunrsearchengine.js", "")),
            (
                "/assets/js/lunrsearchengine.js".into(),
                "assets/js/lunrsearchengine.js".into()
            )
        );
    }

    #[test]
    fn unpublished_pages_are_left_out() {
        assert_eq!(
            routes(&[page("pages/draft.md", "published: false")], &clock()).unwrap(),
            []
        );
    }

    #[test]
    fn an_unpublished_page_is_kept_when_unpublished_is_allowed() {
        let docs = [page("pages/draft.md", "published: false")];
        let kept = routes(&docs, &clock().with_unpublished(true)).unwrap();
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn a_non_html_page_needs_its_extension_in_the_url() {
        let problems = routes(&[page("pages/rss.xml", "permalink: /feed")], &clock()).unwrap_err();
        assert!(problems[0].message.contains("must end with .xml"));
    }
}
