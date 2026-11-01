//! Posts' pages (plan v8, §12.2 step 2): each post's entry, with its body and dates, and the page
//! [`POST_TEMPLATE`] renders.
//!
//! The template shows the post as `page`:
//!
//! | Key | Value |
//! |-|-|
//! | `url` | the post's URL (`/blog/2021/08/08/webmentions`) |
//! | `title` | its title |
//! | `layout` | `post` |
//! | `date` | its date, midnight in the site's time zone (`2021-08-08T00:00:00+02:00`) |
//! | `modified` | its last modification (plan v7, §7.3), in the same form |
//! | `description` | its excerpt without markup, cut to 160 characters, as HTML text |
//! | `author` | its front matter `author`, a key of the site's authors, or none |
//! | `categories`, `tags` | `{name, anchor}` for each, sorted by name; `anchor` is the fragment of the categories or tags page |
//! | `previous`, `next` | `{url, title}` of the posts before and after it by date, or none |
//! | `toc` | its table of contents, `{placement, title, entries}`, or none (`folia_core`'s `Toc`) |
//! | `cover` | its cover, `{path, position, show, unsplash}` (`folia_core`'s `Cover`) |
//! | `hide_website_junk` | the parts of the frame it hides, each `true` or `false` |
//! | `syndications` | `{name, url}` for each of its front matter `syndications`, in order |
//!
//! Every post shows its author box and its previous and next posts, so `show_author` and
//! `pagination`, which the site's posts never set otherwise, are not read.
//!
//! So far the template has only what the slice's post uses. A post whose front matter it could
//! not show (another key, a cover not shown behind the page, a table of contents not on the
//! left, or a syndication that offers to pull interactions) is reported as not built yet.

use folia_core::content::markdown::{description, excerpt_html, to_html};
use folia_core::content::{
    Cover, Diagnostic, LastCommits, SiteClock, SourceDoc, Toc, TocPlacement, cover,
    hide_website_junk, modified, table_of_contents,
};
use folia_core::publish::{EntrySummary, ModeOutput, PageSpec};
use serde_json::{Value, json};

use super::{Post, read_posts};

/// The template of a post's page.
pub const POST_TEMPLATE: &str = "post.html";

/// The front matter keys the post template can show, besides the presentation keys.
const KEYS: [&str; 7] = [
    "layout",
    "title",
    "author",
    "categories",
    "tags",
    "lang",
    "last_modified_at",
];

/// The presentation keys the post template can show.
const PRESENTATION_KEYS: [&str; 4] = ["cover", "toc", "hide_website_junk", "syndications"];

/// Syndications whose links the site's template turns into an interactions crawler, which the
/// post template does not have yet.
const INTERACTIONS: [&str; 3] = ["hacker-news", "mastodon", "bluesky"];

/// Builds the pages of the published posts among `docs` whose source is in `only`. Every
/// published post counts for the previous and next links. A post's `modified` time comes from
/// its front matter, then its last commit (`commits`), then its date; times are shown in the time
/// zone of `clock`.
///
/// # Errors
///
/// Returns every problem found: what [`super::routes`] reports, a `last_modified_at` Folia does
/// not read, presentation front matter Folia cannot resolve, and front matter the template
/// cannot show yet.
pub fn build(
    docs: &[SourceDoc],
    only: &[&str],
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let mut posts = read_posts(docs, clock)?;
    posts.sort_by(|a, b| (a.date, &a.id).cmp(&(b.date, &b.id)));
    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    for (i, post) in posts.iter().enumerate() {
        // Every post was read from one of `docs`.
        let Some(doc) = docs.iter().find(|doc| doc.id == post.id) else {
            continue;
        };
        if !only.contains(&post.id.as_str()) {
            continue;
        }
        let neighbours = Neighbours {
            previous: i.checked_sub(1).map(|i| &posts[i]),
            next: posts.get(i + 1),
        };
        match build_post(doc, post, &neighbours, commits, clock) {
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

/// The posts before and after a post, by date.
struct Neighbours<'a> {
    previous: Option<&'a Post>,
    next: Option<&'a Post>,
}

fn build_post(
    doc: &SourceDoc,
    post: &Post,
    neighbours: &Neighbours,
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<(EntrySummary, PageSpec), Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let toc = table_of_contents(doc)?;
    let cover = cover(doc)?;
    unsupported(doc, toc.as_ref(), &cover).map_err(|message| problem(&message))?;
    let modified = modified(doc, commits, Some(post.published), clock)?;
    let author = doc.front.string("author").map_err(|e| problem(&e))?;

    let context = json!({
        "url": post.route.url(),
        "title": post.title,
        "layout": "post",
        "date": clock.xmlschema(post.published),
        "modified": modified.map(|m| clock.xmlschema(m.at)),
        "description": description(&excerpt_html(&doc.body, None)),
        "author": author,
        "categories": named(&post.categories),
        "tags": named(&post.tags),
        "previous": neighbours.previous.map(link),
        "next": neighbours.next.map(link),
        "toc": toc.as_ref().map(Toc::to_json),
        "cover": cover.to_json(),
        "hide_website_junk": hide_website_junk(doc),
        "syndications": syndications(doc),
    });

    let page = PageSpec {
        route: post.route.clone(),
        template: POST_TEMPLATE.to_owned(),
        source: Some(doc.id.clone()),
        context,
    };
    let entry = EntrySummary {
        doc: doc.id.clone(),
        route: post.route.clone(),
        published: Some(post.published),
        modified,
        body_html: to_html(&doc.body, None),
        redirect_from: post.redirect_from.clone(),
    };
    Ok((entry, page))
}

/// Why the post template cannot show `doc`, with its table of contents `toc` and its `cover`,
/// yet, if it cannot.
fn unsupported(doc: &SourceDoc, toc: Option<&Toc>, cover: &Cover) -> Result<(), String> {
    for key in doc.front.keys() {
        if !KEYS.contains(&key) && !PRESENTATION_KEYS.contains(&key) {
            return Err(format!("the front matter key {key} is not built yet"));
        }
    }
    if let Some(layout) = doc.front.string("layout").map_err(|e| e.to_string())?
        && layout != "post"
    {
        return Err(format!("a post with the layout {layout} is not built yet"));
    }
    if cover.path.is_some() && cover.position != "behind" {
        return Err("a cover not shown behind the post is not built yet".to_owned());
    }
    if toc.is_some_and(|toc| toc.placement != TocPlacement::Left) {
        return Err("a table of contents not on the left is not built yet".to_owned());
    }
    let syndications = doc.front.entries("syndications").unwrap_or_default();
    if let Some((name, _)) = syndications
        .iter()
        .find(|(name, _)| INTERACTIONS.contains(&name.as_str()))
    {
        return Err(format!("a syndication to {name} is not built yet"));
    }
    Ok(())
}

/// The syndications as `{name, url}`, in their front matter's order: a JSON mapping would sort
/// them. Without a mapping of its own, a post has its default's, which is none.
fn syndications(doc: &SourceDoc) -> Value {
    doc.front
        .entries("syndications")
        .unwrap_or_default()
        .into_iter()
        .map(|(name, url)| json!({"name": name, "url": url}))
        .collect()
}

/// Categories or tags sorted by name, each with its fragment on the page listing them, as the
/// site's templates made it with Liquid (`replace: " ", " -"`).
fn named(names: &[String]) -> Value {
    let mut sorted = names.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(|name| json!({"name": name, "anchor": name.replace(' ', " -")}))
        .collect()
}

fn link(post: &Post) -> Value {
    json!({"url": post.route.url(), "title": post.title})
}
