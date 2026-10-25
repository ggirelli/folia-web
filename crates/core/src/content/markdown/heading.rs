//! Heading IDs as kramdown's GFM parser makes them.
//!
//! The heading's text, with smart typography and entities already applied, is lowercased; every
//! character that is not a letter, mark, decimal digit, connector, `-`, space or tab is dropped;
//! and spaces and tabs become `-`. A repeated ID gets `-1`, `-2`, … in document order. An empty
//! ID stays empty, as kramdown leaves it.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::tree::{Kind, Node};

/// Ruby's `[^\p{Word}\- \t]`.
static NOT_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^\p{Alphabetic}\p{M}\p{Nd}\p{Pc}\- \t]").expect("the expression is valid")
});

/// Gives every heading of the document its ID.
pub fn assign_ids(root: &mut Node) {
    let mut seen = HashMap::new();
    visit(root, &mut seen);
}

fn visit(node: &mut Node, seen: &mut HashMap<String, usize>) {
    if let Kind::Heading(_) = node.kind {
        let mut text = String::new();
        heading_text(node, &mut text);
        let id = id_for(&text, seen);
        node.attrs.0.insert(0, ("id".to_owned(), id));
        return;
    }
    for child in &mut node.children {
        visit(child, seen);
    }
}

/// The text kramdown builds an ID from: text and code, without markup.
fn heading_text(node: &Node, out: &mut String) {
    match &node.kind {
        Kind::Text { text, .. } | Kind::Code(text) => out.push_str(text),
        _ => node.children.iter().for_each(|c| heading_text(c, out)),
    }
}

fn id_for(text: &str, seen: &mut HashMap<String, usize>) -> String {
    let id = NOT_ID
        .replace_all(&text.to_lowercase(), "")
        .replace([' ', '\t'], "-");
    let count = seen.entry(id.clone()).or_insert(0);
    *count += 1;
    if *count > 1 {
        format!("{id}-{}", *count - 1)
    } else {
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(texts: &[&str]) -> Vec<String> {
        let mut seen = HashMap::new();
        texts.iter().map(|text| id_for(text, &mut seen)).collect()
    }

    #[test]
    fn an_id_keeps_letters_and_drops_punctuation() {
        assert_eq!(
            ids(&["Webmentions: a not-so-new recommendation"]),
            ["webmentions-a-not-so-new-recommendation"]
        );
        assert_eq!(ids(&["Cos’è l’autismo?"]), ["cosè-lautismo"]);
    }

    #[test]
    fn an_emoji_is_dropped_but_its_variation_selector_stays() {
        assert_eq!(ids(&["☀️ Summer"]), ["\u{fe0f}-summer"]);
    }

    #[test]
    fn a_repeated_id_is_numbered() {
        assert_eq!(
            ids(&["Notes", "Notes", "Notes"]),
            ["notes", "notes-1", "notes-2"]
        );
    }
}
