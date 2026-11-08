//! The title Jekyll gives a document that has none (`Jekyll::Document#populate_title`, with
//! `Jekyll::Utils.titleize_slug`): posts and collection documents (notes, micro entries) are
//! titled from their file name; pages are not titled at all (their `page.title` is empty).

use super::dated_file_name;

/// A slug as a title, as Jekyll writes it: split at each `-`, each word capitalized as Ruby's
/// `String#capitalize` does (first letter upper case, the rest lower), joined with spaces.
/// `my-first-post` becomes `My First Post`, and `some_note-name` becomes `Some_note Name`.
#[must_use]
pub fn titleize_slug(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The title of a document without one, from its file name (`file_name`, with its extension):
/// for a dated name (`2026-01-12-a-note.md`) the slug after the date, otherwise the name without
/// its last extension (`Mixed-CASE-name.md` is `Mixed Case Name`, `index.md` is `Index`).
#[must_use]
pub fn file_title(file_name: &str) -> String {
    let slug = match dated_file_name(file_name) {
        Ok(Some((_, slug))) => slug,
        // An impossible date is the dated name's own error to report; Jekyll titles it by name.
        _ => file_name
            .rsplit_once('.')
            .map_or(file_name, |(stem, _)| stem)
            .to_owned(),
    };
    titleize_slug(&slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_is_titleized_as_jekyll_does() {
        assert_eq!(titleize_slug("my-first-post"), "My First Post");
        assert_eq!(titleize_slug("some_note-name"), "Some_note Name");
        assert_eq!(titleize_slug("Mixed-CASE-name"), "Mixed Case Name");
        assert_eq!(titleize_slug("with space"), "With space");
        assert_eq!(titleize_slug("é-x"), "É X");
        assert_eq!(titleize_slug(""), "");
    }

    #[test]
    fn a_document_is_titled_from_its_file_name_without_its_date_or_extension() {
        // Each of these was checked against a Jekyll 4.4.1 build.
        assert_eq!(file_title("2026-01-12-a-note_x.md"), "A Note_x");
        assert_eq!(file_title("2026-01-05-dated-note.md"), "Dated Note");
        assert_eq!(file_title("index.md"), "Index");
        assert_eq!(file_title("Mixed-CASE-name.md"), "Mixed Case Name");
        assert_eq!(file_title("with space.md"), "With space");
        assert_eq!(file_title("my.dotted.name.md"), "My.dotted.name");
        assert_eq!(file_title("noextension"), "Noextension");
    }
}
