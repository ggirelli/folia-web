//! The blog mode: articles under the blog root, and the outputs generated from them.
//!
//! Routes, as the site has published them under Jekyll:
//!
//! | Output | URL |
//! |-|-|
//! | a post | `/blog/:year/:month/:day/:title` (`:title` from the file name, case kept) |
//! | listing pages 2..N, 9 posts each | `/blog/pageN/`, from the template page `blog/index.html` |
//! | a category archive | `/blog/category/:name/` (`:name` slugified) |
//! | the blog feed | `/blog/feed.xml` |
//! | a tag feed | `/blog/tag/:tag.xml`, for tags of letters, digits and `_` only |

use folia_core::content::{
    Diagnostic, DocId, FrontMatterError, Route, SiteClock, SlugMode, SourceDoc, dated_file_name,
    slugify,
};
use folia_core::publish::PlannedOutput;

/// Posts per listing page.
const PER_PAGE: usize = 9;

/// The page that lists the posts, and is copied for each further listing page.
pub const LISTING_TEMPLATE: &str = "blog/index.html";

/// A published post, with what the generated outputs need to know about it.
struct Post {
    id: DocId,
    route: Route,
    redirect_from: Vec<Route>,
    categories: Vec<String>,
    tags: Vec<String>,
    hidden: bool,
}

/// The outputs of the blog: one page per published post, and the generated pages and feeds.
/// Posts that are unpublished, or dated after the build, are left out.
///
/// # Errors
///
/// Returns every problem found: an undated file name, a date that does not exist, or front
/// matter that cannot be read.
pub fn routes(
    docs: &[SourceDoc],
    clock: &SiteClock,
) -> Result<Vec<PlannedOutput>, Vec<Diagnostic>> {
    let mut posts = Vec::new();
    let mut problems = Vec::new();
    for doc in docs {
        match read_post(doc, clock) {
            Ok(Some(post)) => posts.push(post),
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    let mut outputs: Vec<PlannedOutput> = posts.iter().map(post_output).collect();
    outputs.extend(listing_pages(&posts));
    outputs.extend(category_archives(&posts));
    outputs.push(PlannedOutput::generated(
        file_route("/blog/feed.xml"),
        "blog feed",
    ));
    outputs.extend(tag_feeds(&posts));
    Ok(outputs)
}

/// Reads a post, or `None` when it is not published at this build.
fn read_post(doc: &SourceDoc, clock: &SiteClock) -> Result<Option<Post>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let (date, file_slug) = dated_file_name(doc.id.file_name())
        .map_err(|e| problem(&e))?
        .ok_or_else(|| problem(&"a post's file name must start with its date (YYYY-MM-DD-)"))?;
    if doc.front.get("date").is_some() {
        return Err(problem(
            &"a front matter date on a post is not supported yet; the file name dates it",
        ));
    }
    let published_at = clock.midnight(date).map_err(|e| problem(&e))?;
    if !doc.front.published() || clock.is_future(published_at) {
        return Ok(None);
    }

    let front = |error: FrontMatterError| problem(&error);
    let route = if let Some(permalink) = doc.front.string("permalink").map_err(front)? {
        Route::html(&permalink).map_err(|e| problem(&e))?
    } else {
        let slug = doc
            .front
            .string("slug")
            .map_err(front)?
            .unwrap_or(file_slug);
        let title = slugify(&slug, SlugMode::Pretty, true);
        let url = format!(
            "/blog/{}/{:02}/{:02}/{title}",
            date.year(),
            date.month(),
            date.day()
        );
        Route::html(&url).map_err(|e| problem(&e))?
    };
    Ok(Some(Post {
        id: doc.id.clone(),
        route,
        redirect_from: doc.redirect_from()?,
        categories: categories(doc).map_err(front)?,
        tags: doc.front.pluralized("tag", "tags").map_err(front)?,
        hidden: doc.front.truthy("hidden"),
    }))
}

/// Jekyll's categories: the `categories` list (a string is split on whitespace), then the
/// singular `category`, without repeats.
fn categories(doc: &SourceDoc) -> Result<Vec<String>, FrontMatterError> {
    let mut all = doc.front.words("categories")?;
    if doc.front.get("category").is_some() {
        all.extend(doc.front.strings("category")?);
    }
    let mut unique = Vec::with_capacity(all.len());
    for category in all {
        if !unique.contains(&category) {
            unique.push(category);
        }
    }
    Ok(unique)
}

fn post_output(post: &Post) -> PlannedOutput {
    PlannedOutput {
        redirect_from: post.redirect_from.clone(),
        ..PlannedOutput::authored(post.route.clone(), post.id.clone())
    }
}

/// Listing pages 2..N; the first is the template page itself, which the pages mode publishes.
/// Hidden posts are not listed, so they do not count.
fn listing_pages(posts: &[Post]) -> Vec<PlannedOutput> {
    let listed = posts.iter().filter(|post| !post.hidden).count();
    (2..=listed.div_ceil(PER_PAGE))
        .map(|page| PlannedOutput {
            source: Some(DocId::new(LISTING_TEMPLATE)),
            ..PlannedOutput::generated(html_route(&format!("/blog/page{page}/")), "pagination")
        })
        .collect()
}

fn category_archives(posts: &[Post]) -> Vec<PlannedOutput> {
    unique(posts.iter().flat_map(|post| &post.categories))
        .map(|category| {
            let url = format!(
                "/blog/category/{}/",
                slugify(category, SlugMode::Default, false)
            );
            PlannedOutput::generated(html_route(&url), "category archive")
        })
        .collect()
}

/// A feed per tag; tags with other characters than letters, digits and `_` get none.
fn tag_feeds(posts: &[Post]) -> Vec<PlannedOutput> {
    unique(posts.iter().flat_map(|post| &post.tags))
        .filter(|tag| tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .map(|tag| {
            PlannedOutput::generated(file_route(&format!("/blog/tag/{tag}.xml")), "tag feed")
        })
        .collect()
}

fn unique<'a>(values: impl Iterator<Item = &'a String>) -> impl Iterator<Item = &'a String> {
    let mut seen = std::collections::BTreeSet::new();
    values.filter(move |value| seen.insert(*value))
}

/// A route the blog spells out itself, which is valid by construction.
fn html_route(url: &str) -> Route {
    Route::html(url).expect("the blog's own routes are valid")
}

fn file_route(url: &str) -> Route {
    Route::file(url).expect("the blog's own routes are valid")
}

#[cfg(test)]
mod tests;
