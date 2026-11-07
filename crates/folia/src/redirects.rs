//! `/redirects.json`: every `redirect_from` URL of the site and the absolute URL it redirects to,
//! as `jekyll-redirect-from` writes it (plan v11, §3.8: structured outputs by Rust serializers).
//!
//! The plugin walks the documents of every collection (`site.docs_to_write`: posts, then the
//! site's collections in their configured order, each sorted) and then the pages, which Jekyll
//! sorts by file name, and records each `redirect_from` in a Ruby hash: a URL listed twice keeps
//! its first place and its last target. The hash is written as compact JSON, UTF-8, with no
//! trailing newline.
//!
//! Folia orders the collections' documents by mode (blog, micro, garden, as the site configures
//! its collections) and, within a mode, by source path; posts' and micro entries' file names
//! begin with their dates, so this is the plugin's order for them. A document's `redirect_to`
//! is not read: the site has none.

use folia_core::content::{DocId, Mode};
use folia_core::publish::PlannedOutput;

use crate::site::Site;

/// The modes whose documents the plugin walks before the pages, in its order.
const COLLECTIONS: [Mode; 3] = [Mode::Blog, Mode::Micro, Mode::Garden];

/// The text of `/redirects.json` for `site`, published at `site_url` (without a trailing `/`).
#[must_use]
pub fn redirects_json(site_url: &str, site: &Site) -> String {
    let mut sources: Vec<&DocId> = Vec::new();
    for mode in COLLECTIONS {
        let mut ids = documents(site, mode);
        ids.sort();
        sources.extend(ids);
    }
    let mut pages = documents(site, Mode::Pages);
    pages.sort_by(|a, b| (file_name(a), *a).cmp(&(file_name(b), *b)));
    sources.extend(pages);

    let outputs = site.routes.outputs();
    let redirects = sources.into_iter().filter_map(|id| page_of(outputs, id));
    serialize(redirects.flat_map(|output| {
        let to = format!("{site_url}{}", output.route.url());
        output
            .redirect_from
            .iter()
            .map(move |from| (from.url().to_owned(), to.clone()))
    }))
}

/// The redirects as a JSON object, in order, a key given twice keeping its first place and its
/// last value.
fn serialize(redirects: impl IntoIterator<Item = (String, String)>) -> String {
    let mut entries: Vec<(String, String)> = Vec::new();
    for (from, to) in redirects {
        match entries.iter_mut().find(|(key, _)| *key == from) {
            Some(entry) => entry.1 = to,
            None => entries.push((from, to)),
        }
    }
    let body: Vec<String> = entries
        .iter()
        .map(|(from, to)| format!("{}:{}", json_string(from), json_string(to)))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// `text` as a JSON string.
fn json_string(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

fn documents(site: &Site, mode: Mode) -> Vec<&DocId> {
    site.documents
        .get(&mode)
        .map(|docs| docs.iter().map(|doc| &doc.id).collect())
        .unwrap_or_default()
}

fn file_name(id: &DocId) -> &str {
    id.as_str().rsplit('/').next().unwrap_or_default()
}

/// The page `id` is published as, if it is.
fn page_of<'a>(outputs: &'a [PlannedOutput], id: &DocId) -> Option<&'a PlannedOutput> {
    outputs
        .iter()
        .find(|o| o.generator.is_none() && o.source.as_ref() == Some(id))
}

#[cfg(test)]
mod tests;
