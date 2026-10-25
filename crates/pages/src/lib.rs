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

use folia_core::content::{Diagnostic, Route, SourceDoc};
use folia_core::publish::PlannedOutput;

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
