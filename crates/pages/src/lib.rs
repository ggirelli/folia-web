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
//! | `layout` | its front matter `layout` |
//! | `modified` | its last commit's time in the site's time zone (`2026-01-04T13:36:24+01:00`), or none |
//!
//! So far [`build`] builds what the first vertical slice needs (plan v6, §12.2 step 1): Markdown
//! pages with a layout and a title. Anything else it reports as not supported yet.

use folia_core::content::markdown::to_html;
use folia_core::content::{Diagnostic, LastCommits, Route, SiteClock, SourceDoc};
use folia_core::publish::{EntrySummary, ModeOutput, PageSpec, PlannedOutput};
use serde_json::json;

/// Source extensions that a converter renders as HTML.
const MARKDOWN_EXTENSIONS: [&str; 5] = [".markdown", ".mkdown", ".mkdn", ".mkd", ".md"];

/// The outputs of the pages: one per published page.
///
/// # Errors
///
/// Returns every problem found: front matter that cannot be read, or a permalink that is not a
/// valid route for the page's output.
pub fn routes(docs: &[SourceDoc]) -> Result<Vec<PlannedOutput>, Vec<Diagnostic>> {
    let mut outputs = Vec::new();
    let mut problems = Vec::new();
    for doc in docs.iter().filter(|doc| doc.front.published()) {
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
/// the time zone of `clock`.
///
/// # Errors
///
/// Returns every problem found: what [`routes`] reports, a page that is not Markdown or has no
/// layout or title, and a front matter `last_modified_at`, which Jekyll ignores on pages and
/// Folia does not read yet.
pub fn build(
    docs: &[SourceDoc],
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    for doc in docs.iter().filter(|doc| doc.front.published()) {
        match build_page(doc, commits, clock) {
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
) -> Result<(EntrySummary, PageSpec), Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let planned = page_output(doc)?;
    if !MARKDOWN_EXTENSIONS.contains(&doc.id.extension().to_lowercase().as_str()) {
        return Err(problem(&"only Markdown pages are built yet"));
    }
    let required = |key: &str| {
        doc.front
            .string(key)
            .map_err(|e| problem(&e))?
            .ok_or_else(|| problem(&format!("a page without a {key} is not built yet")))
    };
    let layout = required("layout")?;
    let title = required("title")?;
    if doc.front.get("last_modified_at").is_some() {
        return Err(problem(
            &"last_modified_at is not read on pages yet; the last commit dates them",
        ));
    }
    let modified = commits.get(&doc.id).map(|time| clock.xmlschema(time));

    let page = PageSpec {
        route: planned.route.clone(),
        template: format!("{layout}.html"),
        source: Some(doc.id.clone()),
        context: json!({
            "url": planned.route.url(),
            "title": title,
            "layout": layout,
            "modified": modified,
        }),
    };
    let entry = EntrySummary {
        doc: doc.id.clone(),
        route: planned.route,
        body_html: to_html(&doc.body, None),
        redirect_from: planned.redirect_from,
    };
    Ok((entry, page))
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
    use folia_core::content::{DocId, FrontMatter, Mode};

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
    fn a_page_is_built_with_the_template_its_layout_names() {
        let output = build(&[about()], &LastCommits::default(), &clock()).unwrap();
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
            json!({"url": "/about", "title": "About me", "layout": "page", "modified": null})
        );
    }

    #[test]
    fn a_page_is_modified_at_its_last_commit_in_the_site_time_zone() {
        let commits: LastCommits = [(
            DocId::new("pages/profile.md"),
            "2026-01-04T12:36:24Z".parse().unwrap(),
        )]
        .into_iter()
        .collect();
        let output = build(&[about()], &commits, &clock()).unwrap();
        assert_eq!(
            output.pages[0].context["modified"],
            "2026-01-04T13:36:24+01:00"
        );
    }

    #[test]
    fn a_page_without_a_layout_or_title_or_markdown_is_not_built_yet() {
        let problems = build(
            &[
                page("pages/a.md", "title: A"),
                page("pages/b.md", "layout: page"),
                page("blog/c.html", "layout: page\ntitle: C"),
                page(
                    "pages/d.md",
                    "layout: page\ntitle: D\nlast_modified_at: 2026-01-01",
                ),
            ],
            &LastCommits::default(),
            &clock(),
        )
        .unwrap_err();
        let messages: Vec<_> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            messages,
            [
                "pages/a.md: a page without a layout is not built yet",
                "pages/b.md: a page without a title is not built yet",
                "blog/c.html: only Markdown pages are built yet",
                "pages/d.md: last_modified_at is not read on pages yet; the last commit dates them",
            ]
        );
    }

    fn route(doc: SourceDoc) -> (String, String) {
        let output = routes(&[doc]).unwrap().remove(0);
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
            routes(&[page("pages/draft.md", "published: false")]).unwrap(),
            []
        );
    }

    #[test]
    fn a_non_html_page_needs_its_extension_in_the_url() {
        let problems = routes(&[page("pages/rss.xml", "permalink: /feed")]).unwrap_err();
        assert!(problems[0].message.contains("must end with .xml"));
    }
}
