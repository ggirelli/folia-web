//! The site: every output planned in one route table, checked for duplicates and collisions
//! (plan v3, §7.1).
//!
//! Besides the modes' outputs, the table holds what only the whole site can plan:
//!
//! | Output | URL |
//! |-|-|
//! | a static file | its path |
//! | a redirect stub | each `redirect_from` URL, as an HTML page |
//! | a Sass source map | the stylesheet's URL with `.map` appended |
//! | the sitemap, `robots.txt`, `redirects.json` | `/sitemap.xml`, `/robots.txt`, `/redirects.json`, unless a file already takes the URL |

use std::collections::BTreeMap;
use std::path::Path;

use folia_core::content::{Diagnostic, DocId, Mode, Preview, Route, SiteClock, SourceDoc};
use folia_core::publish::PlannedOutput;
use jiff::Timestamp;

use crate::config::Config;
use crate::discover::{Discovery, discover};

/// A site whose outputs are all planned.
#[derive(Clone, Debug)]
pub struct Site {
    /// What discovery found.
    pub discovery: Discovery,
    /// The documents, as read, by mode.
    pub documents: BTreeMap<Mode, Vec<SourceDoc>>,
    /// Every output, by output path.
    pub routes: RouteTable,
}

/// Every output of the site, ordered by output path, with no two sharing a URL or a file.
#[derive(Clone, Debug, Default)]
pub struct RouteTable {
    outputs: Vec<PlannedOutput>,
}

impl RouteTable {
    /// The outputs, ordered by output path.
    #[must_use]
    pub fn outputs(&self) -> &[PlannedOutput] {
        &self.outputs
    }
}

/// Plans every output of the site at `root`, as of the build time `now`: discovers its files,
/// reads its documents, runs each mode, and assembles the route table.
///
/// # Errors
///
/// Returns every problem found, by discovery, by a mode, or in the assembled table.
pub fn plan(root: &Path, config: &Config, now: Timestamp) -> Result<Site, Vec<Diagnostic>> {
    plan_for(root, config, now, Preview::default())
}

/// Plans the site as [`plan`] does, with what `preview` adds to the normal build.
///
/// # Errors
///
/// As [`plan`].
pub fn plan_for(
    root: &Path,
    config: &Config,
    now: Timestamp,
    preview: Preview,
) -> Result<Site, Vec<Diagnostic>> {
    let clock = SiteClock::new(&config.time_zone, now)
        .map_err(|e| vec![Diagnostic::site(e)])?
        .with_future(preview.future)
        .with_unpublished(preview.unpublished);
    let discovery = discover(root, config)?;

    let mut docs: BTreeMap<Mode, Vec<SourceDoc>> = BTreeMap::new();
    let mut problems = Vec::new();
    for (mode, id) in &discovery.documents {
        match SourceDoc::read(root, id.clone(), *mode) {
            Ok(doc) => docs.entry(*mode).or_default().push(doc),
            Err(error) => problems.push(Diagnostic::in_doc(id, error)),
        }
    }
    let of = |mode| docs.get(&mode).map_or(&[][..], Vec::as_slice);

    let mut planned = Vec::new();
    for result in [
        folia_blog::routes(of(Mode::Blog), &clock),
        folia_micro::routes(of(Mode::Micro), &config.roots.micro, &clock),
        folia_garden::routes(of(Mode::Garden), &config.roots.garden, &clock),
        folia_pages::routes(of(Mode::Pages), &clock),
    ] {
        match result {
            Ok(outputs) => planned.extend(outputs),
            Err(found) => problems.extend(found),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let routes = assemble(planned, &discovery.statics)?;
    Ok(Site {
        discovery,
        documents: docs,
        routes,
    })
}

/// Adds what only the whole site can plan to the modes' outputs, and checks the result.
///
/// # Errors
///
/// Returns every URL or output file planned twice, and every generated output whose template
/// document is not published.
pub fn assemble(
    mut planned: Vec<PlannedOutput>,
    statics: &[DocId],
) -> Result<RouteTable, Vec<Diagnostic>> {
    let mut problems = Vec::new();
    for id in statics {
        match Route::file(&format!("/{id}")) {
            Ok(route) => planned.push(PlannedOutput::authored(route, id.clone())),
            Err(error) => problems.push(Diagnostic::in_doc(id, error)),
        }
    }
    let stubs: Vec<_> = planned
        .iter()
        .flat_map(|output| output.redirect_from.iter().cloned())
        .map(|from| PlannedOutput::generated(from, "redirect stub"))
        .collect();
    planned.extend(stubs);
    let maps = source_maps(&planned, &mut problems);
    planned.extend(maps);
    for (url, generator) in SITE_FILES {
        if planned.iter().any(|output| output.route.url() == url) {
            continue;
        }
        match Route::file(url) {
            Ok(route) => planned.push(PlannedOutput::generated(route, generator)),
            Err(error) => problems.push(Diagnostic::site(error)),
        }
    }

    problems.extend(missing_templates(&planned));
    problems.extend(clashes(
        &planned,
        |output| output.route.url().to_owned(),
        "URL",
    ));
    problems.extend(clashes(
        &planned,
        |output| output.route.output_path(),
        "output file",
    ));
    if !problems.is_empty() {
        return Err(problems);
    }
    planned.sort_by_key(|output| output.route.output_path());
    Ok(RouteTable { outputs: planned })
}

/// The page `jekyll-redirect-from` writes at an old URL, sending readers to `to`, an absolute
/// URL: a canonical link, a script and a meta refresh. The URL is written as given, unescaped, as
/// the plugin does.
#[must_use]
pub fn redirect_stub(to: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en-US\">\n  \
         <meta charset=\"utf-8\">\n  \
         <title>Redirecting&hellip;</title>\n  \
         <link rel=\"canonical\" href=\"{to}\">\n  \
         <script>location=\"{to}\"</script>\n  \
         <meta http-equiv=\"refresh\" content=\"0; url={to}\">\n  \
         <meta name=\"robots\" content=\"noindex\">\n  \
         <h1>Redirecting&hellip;</h1>\n  \
         <a href=\"{to}\">Click here if you are not redirected.</a>\n\
         </html>\n"
    )
}

/// Files the site gets unless one of its own files takes the URL, and what generates them.
const SITE_FILES: [(&str, &str); 3] = [
    ("/sitemap.xml", "sitemap"),
    ("/robots.txt", "robots"),
    ("/redirects.json", "redirects"),
];

/// A source map next to each stylesheet compiled from Sass.
fn source_maps(planned: &[PlannedOutput], problems: &mut Vec<Diagnostic>) -> Vec<PlannedOutput> {
    planned
        .iter()
        .filter(|output| output.generator.is_none())
        .filter(|output| {
            output
                .source
                .as_ref()
                .is_some_and(|id| matches!(id.extension(), ".scss" | ".sass"))
        })
        .filter_map(
            |output| match Route::file(&format!("{}.map", output.route.url())) {
                Ok(route) => Some(PlannedOutput::generated(route, "sass source map")),
                Err(error) => {
                    problems.push(Diagnostic::site(error));
                    None
                }
            },
        )
        .collect()
}

/// Generated outputs copied from a template document (listing pages) need that document
/// published.
fn missing_templates(planned: &[PlannedOutput]) -> Vec<Diagnostic> {
    let published = |id: &DocId| {
        planned
            .iter()
            .any(|o| o.generator.is_none() && o.source.as_ref() == Some(id))
    };
    planned
        .iter()
        .filter(|output| output.generator.is_some())
        .filter_map(|output| output.source.as_ref().map(|id| (output, id)))
        .filter(|(_, id)| !published(id))
        .map(|(output, id)| {
            Diagnostic::in_doc(
                id,
                format!("is the template of {}, but is not published", output.route),
            )
        })
        .collect()
}

/// A diagnostic for each value of `key` that more than one output has.
fn clashes(
    planned: &[PlannedOutput],
    key: impl Fn(&PlannedOutput) -> String,
    what: &str,
) -> Vec<Diagnostic> {
    let mut by_key: BTreeMap<String, Vec<&PlannedOutput>> = BTreeMap::new();
    for output in planned {
        by_key.entry(key(output)).or_default().push(output);
    }
    by_key
        .into_iter()
        .filter(|(_, outputs)| outputs.len() > 1)
        .map(|(value, outputs)| {
            let origins: Vec<_> = outputs.iter().map(|o| origin(o)).collect();
            Diagnostic::site(format!(
                "the {what} {value} is planned {} times: {}",
                outputs.len(),
                origins.join(", ")
            ))
        })
        .collect()
}

fn origin(output: &PlannedOutput) -> String {
    match (&output.source, output.generator) {
        (Some(id), None) => id.to_string(),
        (Some(id), Some(generator)) => format!("{generator} of {id}"),
        (None, Some(generator)) => generator.to_owned(),
        (None, None) => "an unknown origin".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, source: &str) -> PlannedOutput {
        PlannedOutput::authored(Route::html(url).unwrap(), DocId::new(source))
    }

    fn urls(table: &RouteTable) -> Vec<&str> {
        table.outputs().iter().map(|o| o.route.url()).collect()
    }

    #[test]
    fn a_redirect_becomes_an_html_stub() {
        let about = PlannedOutput {
            redirect_from: vec![Route::html("/blog/about").unwrap()],
            ..page("/about", "pages/profile.md")
        };
        let table = assemble(vec![about], &[]).unwrap();
        let stub = table
            .outputs()
            .iter()
            .find(|o| o.generator == Some("redirect stub"))
            .unwrap();
        assert_eq!(stub.route.output_path(), "blog/about.html");
    }

    #[test]
    fn a_redirect_stub_is_written_as_the_plugin_writes_it() {
        assert_eq!(
            redirect_stub("https://ggirelli.info/about"),
            "<!DOCTYPE html>\n<html lang=\"en-US\">\n  <meta charset=\"utf-8\">\n  <title>Redirecting&hellip;</title>\n  <link rel=\"canonical\" href=\"https://ggirelli.info/about\">\n  <script>location=\"https://ggirelli.info/about\"</script>\n  <meta http-equiv=\"refresh\" content=\"0; url=https://ggirelli.info/about\">\n  <meta name=\"robots\" content=\"noindex\">\n  <h1>Redirecting&hellip;</h1>\n  <a href=\"https://ggirelli.info/about\">Click here if you are not redirected.</a>\n</html>\n"
        );
    }

    #[test]
    fn site_files_are_generated_unless_a_file_takes_their_url() {
        let table = assemble(vec![], &[DocId::new("robots.txt")]).unwrap();
        assert_eq!(
            urls(&table),
            ["/redirects.json", "/robots.txt", "/sitemap.xml"]
        );
        assert_eq!(table.outputs()[1].source, Some(DocId::new("robots.txt")));
    }

    #[test]
    fn a_stylesheet_from_sass_gets_a_source_map() {
        let css = PlannedOutput::authored(
            Route::file("/assets/css/main.css").unwrap(),
            DocId::new("assets/css/main.scss"),
        );
        let table = assemble(vec![css], &[]).unwrap();
        assert!(urls(&table).contains(&"/assets/css/main.css.map"));
    }

    #[test]
    fn two_outputs_on_one_url_or_file_are_reported() {
        let problems = assemble(
            vec![
                page("/about", "pages/profile.md"),
                page("/about", "me/about.md"),
            ],
            &[],
        )
        .unwrap_err();
        assert_eq!(problems.len(), 2);
        assert!(
            problems[0]
                .message
                .contains("pages/profile.md, me/about.md")
        );
    }

    #[test]
    fn different_urls_writing_one_file_are_reported() {
        let problems =
            assemble(vec![page("/x", "a.md"), page("/x.html", "b.md")], &[]).unwrap_err();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("output file x.html"));
    }

    #[test]
    fn a_listing_page_needs_its_template_published() {
        let listing = PlannedOutput {
            source: Some(DocId::new("blog/index.html")),
            ..PlannedOutput::generated(Route::html("/blog/page2/").unwrap(), "pagination")
        };
        let problems = assemble(vec![listing.clone()], &[]).unwrap_err();
        assert!(problems[0].message.contains("not published"));
        assert!(assemble(vec![listing, page("/blog/", "blog/index.html")], &[]).is_ok());
    }
}
