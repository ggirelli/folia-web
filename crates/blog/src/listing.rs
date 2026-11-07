//! The blog's listings: the paginated index (`/blog/`, `/blog/pageN/`) and the category archives
//! (`/blog/category/:name/`), as the site's `jekyll-paginate` and `jekyll-archives` made them.
//!
//! The index's source (`blog/index.html`) is claimed (plan v12, §3.3–3.4): its front matter
//! gives the pages' frame, its Liquid body is not rendered, and `template` renders every page of
//! it. An archive is rendered with `archive_template`. Each page shows its posts as cards, in
//! `page.items`, each `{url, title, cover_path, box_position, summary, author, date}`:
//!
//! | Key | Value |
//! |-|-|
//! | `cover_path`, `box_position` | its front matter `cover.path` and `cover.box_position`, or none |
//! | `summary` | its excerpt without markup, its first 30 words with `...` when it has more (`strip_html \| truncatewords: 30`) |
//! | `author` | its front matter `author`, a key of the site's authors, or none |
//! | `date` | its date, at midnight in the site's time zone |
//!
//! Posts are listed newest first, those of the same date by source path, descending, as
//! Jekyll's `site.posts`. The index lists the posts not `hidden`, 9 a page, and its first page
//! the `featured` ones too (`page.featured`, hidden or not, as `site.posts` has them); its
//! `page.paginator` is `{page, total_pages, previous_page_path, next_page_path, paths}`, `paths`
//! being every page's URL. An archive lists every post of its category, as
//! `{type: "category", title: name}`, with `use_dark_layout` false: the site's front matter
//! defaults, which make every other page dark, never reached the archives.

use folia_core::content::markdown::{excerpt_html, strip_html, truncate_words};
use folia_core::content::{
    Diagnostic, DocId, FrontMatter, Mode, SiteClock, SlugMode, SourceDoc, cover, hide_website_junk,
    slugify,
};
use folia_core::publish::PageSpec;
use serde_json::{Value, json};

use super::{PER_PAGE, Post, html_route, read_posts};

/// The words of a card's summary.
const SUMMARY_WORDS: usize = 30;

/// The URL of the index's first page.
const INDEX_URL: &str = "/blog/";

/// Builds the pages of the blog's index, from its claimed source rendered with its template
/// (`index`), and of its category archives, rendered with `archive_template`; each only when
/// given.
///
/// # Errors
///
/// Returns every problem found reading the posts or the index's front matter.
pub fn listings(
    docs: &[SourceDoc],
    index: Option<(&SourceDoc, &str)>,
    archive_template: Option<&str>,
    clock: &SiteClock,
) -> Result<Vec<PageSpec>, Vec<Diagnostic>> {
    let (posts, cards) = cards(docs, clock)?;
    let mut pages = Vec::new();
    if let Some((index, template)) = index {
        let title = index
            .front
            .string("title")
            .map_err(|e| vec![Diagnostic::in_doc(&index.id, e)])?
            .unwrap_or_default();
        let frame = Frame::of(index)?;
        pages.extend(index_pages(&posts, &cards, index, template, &title, &frame));
    }
    if let Some(template) = archive_template {
        pages.extend(archives(&posts, &cards, template)?);
    }
    Ok(pages)
}

/// The published posts, newest first and same dates by path descending (`site.posts`), with
/// their cards.
fn cards(
    docs: &[SourceDoc],
    clock: &SiteClock,
) -> Result<(Vec<Post>, Vec<Value>), Vec<Diagnostic>> {
    let mut posts = read_posts(docs, clock)?;
    posts.sort_by(|a, b| (b.date, &b.id).cmp(&(a.date, &a.id)));
    let mut cards = Vec::with_capacity(posts.len());
    let mut problems = Vec::new();
    for post in &posts {
        match card(docs, post, clock) {
            Ok(card) => cards.push(card),
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok((posts, cards))
    } else {
        Err(problems)
    }
}

/// What groups the posts on the blog's categories or tags page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grouping {
    /// By category, as `site.categories`.
    Categories,
    /// By tag, as `site.tags`.
    Tags,
}

/// The posts by category or tag, as the blog's categories and tags pages list them (Liquid's
/// `site.categories | sort`): each group `{name, anchor, count, items}`, by name in byte order,
/// its posts' cards newest first; `anchor` is the name with each space as ` -`, the site's
/// fragment.
///
/// # Errors
///
/// Returns every problem found reading the posts.
pub fn groups(
    docs: &[SourceDoc],
    by: Grouping,
    clock: &SiteClock,
) -> Result<Value, Vec<Diagnostic>> {
    let (posts, cards) = cards(docs, clock)?;
    let names = |post: &Post| match by {
        Grouping::Categories => post.categories.clone(),
        Grouping::Tags => post.tags.clone(),
    };
    let mut all: Vec<String> = posts.iter().flat_map(names).collect();
    all.sort();
    all.dedup();
    Ok(all
        .into_iter()
        .map(|name| {
            let items: Vec<Value> = posts
                .iter()
                .zip(&cards)
                .filter(|(post, _)| names(post).contains(&name))
                .map(|(_, card)| card.clone())
                .collect();
            json!({
                "anchor": name.replace(' ', " -"),
                "count": items.len(),
                "items": items,
                "name": name,
            })
        })
        .collect())
}

/// What every listing page's frame shows: the parts of the frame its source hides, and its cover.
struct Frame {
    hide_website_junk: Value,
    cover: Value,
}

impl Frame {
    fn of(doc: &SourceDoc) -> Result<Self, Vec<Diagnostic>> {
        Ok(Self {
            hide_website_junk: hide_website_junk(doc),
            cover: cover(doc).map_err(|e| vec![e])?.to_json(),
        })
    }

    /// A page with this frame, at `url`, titled `title`, with the layout `layout`.
    fn page(&self, url: &str, title: &str, layout: &str) -> Value {
        json!({
            "url": url,
            "title": title,
            "layout": layout,
            "hide_website_junk": self.hide_website_junk,
            "cover": self.cover,
            "toc": null,
            "description": null,
            "date": null,
            "author": null,
        })
    }
}

fn index_pages(
    posts: &[Post],
    cards: &[Value],
    index: &SourceDoc,
    template: &str,
    title: &str,
    frame: &Frame,
) -> Vec<PageSpec> {
    let listed: Vec<&Value> = posts
        .iter()
        .zip(cards)
        .filter(|(post, _)| !post.hidden)
        .map(|(_, card)| card)
        .collect();
    let featured: Vec<&Value> = posts
        .iter()
        .zip(cards)
        .filter(|(post, _)| post.featured)
        .map(|(_, card)| card)
        .collect();
    let total = listed.len().div_ceil(PER_PAGE).max(1);
    let path = |page: usize| {
        if page == 1 {
            INDEX_URL.to_owned()
        } else {
            format!("/blog/page{page}/")
        }
    };
    let paths: Vec<String> = (1..=total).map(path).collect();
    (1..=total)
        .map(|page| {
            let url = path(page);
            let mut context = frame.page(&url, title, "default");
            let start = (page - 1) * PER_PAGE;
            let items = &listed[start..(start + PER_PAGE).min(listed.len())];
            context["items"] = items.iter().map(|card| (*card).clone()).collect();
            context["featured"] = if page == 1 {
                featured.iter().map(|card| (*card).clone()).collect()
            } else {
                json!([])
            };
            context["paginator"] = json!({
                "page": page,
                "total_pages": total,
                "previous_page_path": (page > 1).then(|| path(page - 1)),
                "next_page_path": (page < total).then(|| path(page + 1)),
                "paths": paths,
            });
            PageSpec {
                route: html_route(&url),
                template: template.to_owned(),
                source: Some(index.id.clone()),
                context,
            }
        })
        .collect()
}

fn archives(
    posts: &[Post],
    cards: &[Value],
    template: &str,
) -> Result<Vec<PageSpec>, Vec<Diagnostic>> {
    // An archive has no source: its frame is a page's without front matter.
    let blank = SourceDoc {
        id: DocId::new("archive"),
        mode: Mode::Blog,
        front: FrontMatter::parse("").expect("empty front matter parses"),
        body: String::new(),
    };
    let frame = Frame::of(&blank)?;
    let mut names: Vec<&String> = Vec::new();
    for name in posts.iter().flat_map(|post| &post.categories) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    Ok(names
        .into_iter()
        .map(|name| {
            let url = format!(
                "/blog/category/{}/",
                slugify(name, SlugMode::Default, false)
            );
            let mut context = frame.page(&url, name, "archive");
            context["type"] = "category".into();
            // Jekyll's front matter defaults never reached the archives, so they are not dark.
            context["use_dark_layout"] = false.into();
            context["items"] = posts
                .iter()
                .zip(cards)
                .filter(|(post, _)| post.categories.contains(name))
                .map(|(_, card)| card.clone())
                .collect();
            PageSpec {
                route: html_route(&url),
                template: template.to_owned(),
                source: None,
                context,
            }
        })
        .collect())
}

/// A post's card.
fn card(docs: &[SourceDoc], post: &Post, clock: &SiteClock) -> Result<Value, Diagnostic> {
    let doc = docs
        .iter()
        .find(|doc| doc.id == post.id)
        .expect("every post was read from one of `docs`");
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let cover = doc.front.json("cover");
    let text = |key: &str| {
        cover
            .as_ref()
            .and_then(|cover| cover.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    Ok(json!({
        "url": post.route.url(),
        "title": post.title,
        "cover_path": text("path"),
        "box_position": text("box_position"),
        "summary": truncate_words(&strip_html(&excerpt_html(&doc.body, None)), SUMMARY_WORDS),
        "author": doc.front.string("author").map_err(|e| problem(&e))?,
        "date": clock.xmlschema(post.published),
    }))
}

#[cfg(test)]
mod tests;
