//! Which note a wiki-link names, by the rules of the site's Jekyll wiki-link
//! plugin: its title first, then its file name with `-` read as a
//! space, both ignoring case and surrounding whitespace. When two notes share a key, the later
//! one wins.

use std::collections::HashMap;

use folia_core::content::DocId;
use folia_core::content::markdown::LinkResolver;

/// The garden's notes, by the keys a wiki-link may name them with.
#[derive(Clone, Debug, Default)]
pub struct WikiLinks {
    by_title: HashMap<String, String>,
    by_file_name: HashMap<String, String>,
}

impl WikiLinks {
    /// Indexes `notes`, each given as its title, its source and its URL, in the order the
    /// plugin met them.
    pub fn new<'a>(notes: impl IntoIterator<Item = (&'a str, &'a DocId, &'a str)>) -> Self {
        let mut links = Self::default();
        for (title, id, url) in notes {
            links.by_title.insert(key(title), url.to_owned());
            links
                .by_file_name
                .insert(key(&id.stem().replace('-', " ")), url.to_owned());
        }
        links
    }
}

impl LinkResolver for WikiLinks {
    fn resolve(&self, target: &str) -> Option<String> {
        let key = key(target);
        self.by_title
            .get(&key)
            .or_else(|| self.by_file_name.get(&key))
            .cloned()
    }
}

fn key(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links() -> WikiLinks {
        let rust = DocId::new("_digital_garden/topic/note.md");
        let chat = DocId::new("_digital_garden/tracking/chat-control.md");
        WikiLinks::new([
            ("Rust", &rust, "/dg/coding/rust"),
            ("Chat Control 2.0", &chat, "/dg/tracking/chat-control"),
        ])
    }

    #[test]
    fn a_wiki_link_names_a_note_by_title_ignoring_case() {
        assert_eq!(
            links().resolve(" rust "),
            Some("/dg/coding/rust".to_owned())
        );
    }

    #[test]
    fn a_wiki_link_falls_back_to_the_file_name() {
        assert_eq!(
            links().resolve("Chat Control"),
            Some("/dg/tracking/chat-control".to_owned())
        );
        assert_eq!(links().resolve("Go"), None);
    }
}
