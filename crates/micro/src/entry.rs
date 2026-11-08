//! Micro entries' pages: each entry's body and dates, and the page its kind's template renders.
//!
//! An entry's kind is the directory it sits in, and the site's front matter defaults for that
//! kind (Jekyll's `_config.yml`, scope `type: micro`) give it its layout and category:
//!
//! | Kind | Template | Category | Kind data |
//! |-|-|-|-|
//! | `notes` | `page.html` | `notes` | none |
//! | `likes` | `like.html` | `likes` | `like`: `{url, label}` |
//! | `bookmarks` | `bookmark.html` | `bookmarks` | `bookmark`: `{url, label}` |
//! | `rsvps` | `rsvp.html` | `rsvps` | `rsvp`, the answer (`yes`); `event`: `{date, time, name, organizer, url}`, `date` at midnight in the site's time zone |
//!
//! The template shows the entry as `page`, with the keys a post has (see `folia_blog`'s `post`),
//! set as every micro entry's defaults set them:
//!
//! | Key | Value |
//! |-|-|
//! | `url`, `title`, `layout` | its URL, title, and its kind's layout (`page`, `like`, `bookmark`, `rsvp`) |
//! | `date`, `modified` | its file name's date, at midnight in the site's time zone; its last modification (plan v7, §7.3) |
//! | `description` | its excerpt without markup, cut to 160 characters, as HTML text |
//! | `author` | the configuration's micro author, if any |
//! | `categories`, `tags` | its kind's category, `[{name, anchor}]`; none |
//! | `hide_post_junk` | `{categories: true, tags: true}`: neither is shown |
//! | `pagination`, `previous`, `next` | `false`, none, none: an entry shows no previous and next row |
//! | `hide_website_junk` | the navigation, site title and category menu hidden; the footer shown |
//! | `corner_backlink` | `{show: true, url: "/micro", icon_class: "fa-solid fa-microscope"}` |
//! | `toc`, `cover`, `footer` | none, the default cover, `{extra: none}` |
//! | `syndications`, `interactions`, `narrow_post` | as a post's |
//!
//! Front matter it cannot show, another key or one of another kind, is refused as not built yet,
//! and so is an entry outside the four kinds' directories.

use folia_core::content::markdown::{description, excerpt_html, to_html};
use folia_core::content::{
    Diagnostic, LastCommits, SiteClock, SourceDoc, cover, interactions, modified, syndications,
};
use folia_core::publish::{EntrySummary, FeedItem, ModeOutput, PageSpec};
use serde_json::{Map, Value, json};

use super::{entry_output, kind_of};

/// The front matter keys every entry may have.
const KEYS: [&str; 6] = [
    "title",
    "narrow_post",
    "syndications",
    "redirect_from",
    "published",
    "permalink",
];

/// Each kind: its directory, layout, and the front matter keys of its own.
pub(super) const KINDS: [(&str, &str, &[&str]); 4] = [
    ("notes", "page", &[]),
    ("likes", "like", &["like"]),
    ("bookmarks", "bookmark", &["bookmark"]),
    ("rsvps", "rsvp", &["rsvp", "event"]),
];

/// Builds the pages of the published entries among `docs` whose source is in `only`, written by
/// `author` (the configuration's micro author), with their dates in the time zone of `clock`
/// and their last modifications from `commits`.
///
/// # Errors
///
/// Returns every problem found: what [`super::routes`] reports, an entry outside the kinds, and
/// front matter its template cannot show.
pub fn build(
    docs: &[SourceDoc],
    root: &str,
    only: &[&str],
    author: Option<&str>,
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<ModeOutput, Vec<Diagnostic>> {
    let mut output = ModeOutput::default();
    let mut problems = Vec::new();
    for doc in docs.iter().filter(|doc| only.contains(&doc.id.as_str())) {
        match build_entry(doc, root, author, commits, clock) {
            Ok(Some((entry, page, item))) => {
                output.entries.push(entry);
                output.pages.push(page);
                output.feed.push(item);
            }
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok(output)
    } else {
        Err(problems)
    }
}

fn build_entry(
    doc: &SourceDoc,
    root: &str,
    author: Option<&str>,
    commits: &LastCommits,
    clock: &SiteClock,
) -> Result<Option<(EntrySummary, PageSpec, FeedItem)>, Diagnostic> {
    let problem = |message: &dyn std::fmt::Display| Diagnostic::in_doc(&doc.id, message);
    let Some(planned) = entry_output(doc, root, clock)? else {
        return Ok(None);
    };
    let (kind, layout, own) = kind_of(doc, root)
        .and_then(|kind| KINDS.iter().find(|(name, _, _)| *name == kind))
        .copied()
        .ok_or_else(|| {
            problem(&"an entry outside notes, likes, bookmarks and rsvps is not built yet")
        })?;
    for key in doc.front.keys() {
        if !KEYS.contains(&key) && !own.contains(&key) {
            return Err(problem(&format!(
                "the front matter key {key} is not built yet on {kind}"
            )));
        }
    }
    let title = doc
        .front
        .string("title")
        .map_err(|e| problem(&e))?
        .unwrap_or_else(|| folia_core::content::file_title(doc.id.file_name()));
    let narrow_post = match doc.front.json("narrow_post") {
        None => false,
        Some(Value::Bool(narrow)) => narrow,
        Some(_) => return Err(problem(&"narrow_post should be true or false")),
    };
    let (date, _) = folia_core::content::dated_file_name(doc.id.file_name())
        .map_err(|e| problem(&e))?
        .ok_or_else(|| problem(&"an entry without a date in its file name is not built yet"))?;
    let published = clock.midnight(date).map_err(|e| problem(&e))?;
    let modified = modified(doc, commits, Some(published), clock)?;

    let mut context = json!({
        "url": planned.route.url(),
        "title": title,
        "layout": layout,
        "date": clock.xmlschema(published),
        "modified": modified.map(|m| clock.xmlschema(m.at)),
        "description": description(&excerpt_html(&doc.body, None)),
        "author": author,
        "categories": [{"name": kind, "anchor": kind}],
        "tags": [],
        "hide_post_junk": {"categories": true, "tags": true},
        "pagination": false,
        "previous": null,
        "next": null,
        "toc": null,
        "cover": cover(doc)?.to_json(),
        "hide_website_junk": {"main_nav": true, "title": true, "cat_jumbotron": true, "footer": false},
        "corner_backlink": {"show": true, "url": "/micro", "icon_class": "fa-solid fa-microscope"},
        "syndications": syndications(doc),
        "interactions": interactions(doc),
        "narrow_post": narrow_post,
        "footer": {"extra": null},
    });
    for key in own {
        let value = match *key {
            "like" | "bookmark" => link(doc, key),
            "rsvp" => rsvp(doc),
            _ => event(doc, clock),
        }
        .map_err(|message| problem(&message))?;
        context[*key] = value;
    }

    let page = PageSpec {
        route: planned.route.clone(),
        template: format!("{layout}.html"),
        source: Some(doc.id.clone()),
        context,
    };
    let body_html = to_html(&doc.body, None);
    // The feeds file an entry under its kind, the category Jekyll gave it.
    let item = FeedItem {
        collection: "micro".to_owned(),
        url: planned.route.url().to_owned(),
        id: FeedItem::jekyll_id(planned.route.url(), doc),
        title,
        published,
        updated: modified.as_ref().map_or(published, |m| m.at),
        content_html: body_html.clone(),
        excerpt_html: excerpt_html(&doc.body, None),
        author: author.map(str::to_owned),
        categories: vec![kind.to_owned()],
        tags: Vec::new(),
        lang: None,
        search_text: (kind == "rsvps").then(|| rsvp_text(doc)),
    };
    let entry = EntrySummary {
        doc: doc.id.clone(),
        route: planned.route,
        published: Some(published),
        modified,
        body_html,
        redirect_from: planned.redirect_from,
    };
    Ok(Some((entry, page, item)))
}

/// A like's or bookmark's link, its front matter `key`: `{url, label}`, `label` none without one.
pub(super) fn link(doc: &SourceDoc, key: &str) -> Result<Value, String> {
    let Some(Value::Object(link)) = doc.front.json(key) else {
        return Err(format!("{key} should be a mapping with a url"));
    };
    let text = |field: &str| match link.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("{key}: {field} should be text")),
    };
    if let Some(field) = link
        .keys()
        .find(|field| !["url", "label"].contains(&field.as_str()))
    {
        return Err(format!("{key}: the key {field} is not built yet"));
    }
    let url = text("url")?.ok_or_else(|| format!("{key} should have a url"))?;
    Ok(json!({"url": url, "label": text("label")?}))
}

/// An RSVP's answer, as written (`yes`).
pub(super) fn rsvp(doc: &SourceDoc) -> Result<Value, String> {
    match doc.front.json("rsvp") {
        Some(Value::String(answer)) => Ok(answer.into()),
        _ => Err("rsvp should be text, such as yes".to_owned()),
    }
}

/// What search shows of an RSVP, as the site's search script wrote it: its answer and its event's
/// date, time, name and organizer, as written (`RSVPed yes - 2021-08-20, 18:00: Gardens by Anna`).
fn rsvp_text(doc: &SourceDoc) -> String {
    let event = doc.front.json("event");
    let field = |name: &str| {
        event
            .as_ref()
            .and_then(|event| event.get(name)?.as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    let answer = doc.front.string("rsvp").ok().flatten().unwrap_or_default();
    format!(
        "RSVPed {answer} - {}, {}: {} by {}",
        field("date"),
        field("time"),
        field("name"),
        field("organizer")
    )
}

/// The event an RSVP answers: `{date, time, name, organizer, url}`, each text but `date`, a date
/// at midnight in the site's time zone.
pub(super) fn event(doc: &SourceDoc, clock: &SiteClock) -> Result<Value, String> {
    const FIELDS: [&str; 5] = ["date", "time", "name", "organizer", "url"];
    let Some(Value::Object(event)) = doc.front.json("event") else {
        return Err("event should be a mapping".to_owned());
    };
    if let Some(field) = event.keys().find(|field| !FIELDS.contains(&field.as_str())) {
        return Err(format!("event: the key {field} is not built yet"));
    }
    let mut shown = Map::new();
    for field in FIELDS {
        let text = match event.get(field) {
            Some(Value::String(text)) => text.clone(),
            _ => return Err(format!("event: {field} should be text")),
        };
        let value = if field == "date" {
            let at = clock.parse(&text).map_err(|e| format!("event: {e}"))?;
            clock.xmlschema(at)
        } else {
            text
        };
        shown.insert(field.to_owned(), value.into());
    }
    Ok(shown.into())
}

#[cfg(test)]
mod tests;
