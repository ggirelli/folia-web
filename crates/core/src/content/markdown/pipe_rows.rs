//! kramdown's table rows from unresolved wiki-links.
//!
//! kramdown reads any line that holds a `|` as a row of a table, with no header and no delimiter
//! row needed. Jekyll took excerpts before its plugin replaced wiki-links, so in an excerpt
//! `- [[Title|text]]` was an item holding a one-row table, with the cells `[[Title` and
//! `text]]`, and a page's description was made of that. `CommonMark` needs a delimiter row, so
//! the parse has a paragraph there; this pass turns it into kramdown's table.
//!
//! It is a narrow compatibility rule: it applies only where wiki-links are left as written (no
//! resolver), to a paragraph each of whose lines holds a wiki-link with a `|` and has a `|`
//! outside code (kramdown's condition for a row). Its rows are its lines; their cells are split
//! at every `|` outside code spans and not escaped (`\|`), and trimmed, as kramdown does. A paragraph only some of whose lines are rows is left as it is: kramdown
//! would split it into a table and a paragraph, and no page of the site has one.

use std::sync::LazyLock;

use regex::Regex;

use super::tree::{self, Kind, Node};

/// An aliased wiki-link, `[[Target|text]]`, as the site's plugin matches one.
static ALIASED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[[^\]|]+\|[^\]]+\]\]").expect("the expression is valid"));

/// Turns every paragraph of `node` (list items' included) whose lines are all rows into a table.
pub fn apply(node: &mut Node, src: &str) {
    for child in &mut node.children {
        if matches!(child.kind, Kind::Paragraph)
            && let Some(table) = table(src, child.span.clone())
        {
            *child = table;
        } else {
            apply(child, src);
        }
    }
}

/// The table the paragraph at `span` is, when each of its lines is a row.
fn table(src: &str, span: std::ops::Range<usize>) -> Option<Node> {
    let text = &src[span.clone()];
    let mut rows = Vec::new();
    let mut start = span.start;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches('\n');
        let lead = content.len() - content.trim_start().len();
        let cells = cells(content.trim_start());
        // A row needs a `|` outside code: one cell is a line without.
        if !ALIASED.is_match(content) || cells.len() < 2 {
            return None;
        }
        let cells = cells
            .into_iter()
            .map(|cell| cell_node(src, start + lead + cell.start..start + lead + cell.end))
            .collect();
        rows.push(Node {
            children: cells,
            ..Node::leaf(Kind::TableRow, start..start + content.len())
        });
        start += line.len();
    }
    // kramdown writes a table without a header row in `<tbody>`: no `TableHead` here.
    Some(Node {
        children: rows,
        ..Node::leaf(Kind::Table(Vec::new()), span)
    })
}

/// The ranges of `line`'s cells, trimmed: split at each `|` outside code spans and not escaped,
/// without the leading and trailing `|` of the line.
fn cells(line: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = line.as_bytes();
    let mut bounds = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'`' => {
                let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                let fence = &line[i..i + run];
                // A code span ends at the same run of backticks; without one, they are text.
                if let Some(end) = line[i + run..].find(fence) {
                    i += run + end + run - 1;
                } else {
                    i += run - 1;
                }
            }
            b'|' => bounds.push(i),
            _ => {}
        }
        i += 1;
    }
    let mut ranges = Vec::new();
    let mut from = 0;
    for bound in bounds.into_iter().chain([line.len()]) {
        ranges.push(from..bound);
        from = bound + 1;
    }
    if line.starts_with('|') {
        ranges.remove(0);
    }
    if line.ends_with('|') && line.len() > 1 {
        ranges.pop();
    }
    ranges
        .into_iter()
        .map(|range| {
            let cell = &line[range.clone()];
            let start = range.start + (cell.len() - cell.trim_start().len());
            start..start + cell.trim().len()
        })
        .collect()
}

/// A cell whose Markdown is `src[span]`: its inline content, parsed on its own, with the spans
/// moved to where it is in `src`.
fn cell_node(src: &str, span: std::ops::Range<usize>) -> Node {
    let mut parsed = tree::parse(&src[span.clone()]);
    let mut content = match parsed.children.pop() {
        Some(paragraph)
            if parsed.children.is_empty() && matches!(paragraph.kind, Kind::Paragraph) =>
        {
            paragraph.children
        }
        // Not a lone paragraph: the cell is its text as written.
        _ => vec![Node::leaf(
            Kind::Text {
                text: src[span.clone()].to_owned(),
                plain: true,
            },
            0..span.len(),
        )],
    };
    for node in &mut content {
        shift(node, span.start);
    }
    Node {
        children: content,
        ..Node::leaf(Kind::TableCell, span)
    }
}

fn shift(node: &mut Node, by: usize) {
    node.span = node.span.start + by..node.span.end + by;
    for child in &mut node.children {
        shift(child, by);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{description, excerpt_html, to_html};

    /// The description of a garden note whose body starts with `body`, as its head shows it.
    fn described(body: &str) -> String {
        description(&excerpt_html(body, None))
    }

    // The six garden notes whose descriptions showed kramdown's table rows (owner decision of
    // 2026-11-07): the start of each body, enough for its description, and the description in
    // Jekyll's page (baseline d332352).

    #[test]
    fn the_2014_books_list() {
        let body = "\nThis is an incomplete list.\n\n- [[Fourth Kingdom Trilogy - by J12H|J12H - Fourth Kingdom Trilogy]]\n  - The Traveller.\n  - The Dark River.\n  - The Golden City.\n- [[Renaissance - by Raymond F. Jones|Raymond F. Jones - Renaissance]]\n";
        assert_eq!(
            described(body),
            "This is an incomplete list.                              [[Fourth Kingdom Trilogy - by J12H          J12H - Fourth Kingdom Trilogy]]                         ..."
        );
    }

    #[test]
    fn the_2016_books_list() {
        let body = "\nThis is an incomplete list.\n\n- [[Dune - by Frank Herbert|Frank Herbert - Dune]]\n- [[Doctor Sleep - by Stephen King|Stephen King - Doctor Sleep]]\n";
        assert_eq!(
            described(body),
            "This is an incomplete list.                              [[Dune - by Frank Herbert          Frank Herbert - Dune]]                                           ..."
        );
    }

    #[test]
    fn the_2025_books_list() {
        let body = "\n- [[The Wisdom of Insecurity - A message for an age of anxiety by Alan Watts|Alan Watts - The Wisdom of Insecurity: a message for an age of anxiety]].\n- [[Buchi Bianchi - Dentro l'orizzonte by Carlo Rovelli|Carlo Rovelli - Buchi Bianchi: dentro l'orizzonte]].\n";
        assert_eq!(
            described(body),
            "                              [[The Wisdom of Insecurity - A message for an age of anxiety by Alan Watts          Alan Watts - The Wisdom of Insecurity: a me..."
        );
    }

    #[test]
    fn the_2026_books_list() {
        let body = "\n- I. Asimov - Le Grandi Storie della Fantascienza 3.\n- F. Dürrenmatt\n  - Greco cerca greca.\n  - [[La Morte della Pizia - Friedrich Dürrenmatt|La Morte della Pizia]].\n  - [[La guerra invernale nel Tibet - Friedrich Dürrenmatt|La guerra invernale nel Tibet]]\n  - Minotauro.\n";
        assert_eq!(
            described(body),
            "  I. Asimov - Le Grandi Storie della Fantascienza 3.  F. Dürrenmatt          Greco cerca greca.                                                  [[La Morte d..."
        );
    }

    #[test]
    fn the_book_quotes() {
        let body = "\nA collection of quotes from a bunch of different books I read or skimmed through.\n\n\n- [[The Wisdom of Insecurity - A message for an age of anxiety by Alan Watts|Alan Watts - The Wisdom of Insecurity: a message for an age of anxiety]].\n- [[Camus' The myth of Sisyphus|Albert Camus - The Myth of Sisyphus]].\n";
        assert_eq!(
            described(body),
            "A collection of quotes from a bunch of different books I read or skimmed through.                              [[The Wisdom of Insecurity - A message for an ..."
        );
    }

    #[test]
    fn the_python_index() {
        let body = "\nFor some instructions on how to setup `python` on your system for the first\ntime, check out the page: [[First setup: python]].\n\n- [[Don't forget your SPF! 🌞😎|`uv`]]: Python package manager by Astral!\n- How to implement [[CLI dev in Python|CLIs]] in Python.\n- [[Python: type checking|Type checking]]\n";
        assert_eq!(
            described(body),
            "For some instructions on how to setup python on your system for the firsttime, check out the page: [[First setup: python]].                              [[Do..."
        );
    }

    #[test]
    fn a_row_is_split_outside_code_and_its_cells_are_markdown() {
        assert_eq!(
            excerpt_html("- [[Don't|`a|b`]]: *x* \\| y\n- Plain.\n", None),
            "<ul>\n  <li>\n    <table>\n      <tbody>\n        <tr>\n          <td>[[Don’t</td>\n          <td><code class=\"language-plaintext highlighter-rouge\">a|b</code>]]: <em>x</em> | y</td>\n        </tr>\n      </tbody>\n    </table>\n  </li>\n  <li>Plain.</li>\n</ul>\n"
        );
    }

    #[test]
    fn consecutive_lines_are_rows_of_one_table() {
        assert_eq!(
            excerpt_html("[[A|a]]\n[[B|b]] and [[C|c]]\n", None),
            "<table>\n  <tbody>\n    <tr>\n      <td>[[A</td>\n      <td>a]]</td>\n    </tr>\n    <tr>\n      <td>[[B</td>\n      <td>b]] and [[C</td>\n      <td>c]]</td>\n    </tr>\n  </tbody>\n</table>\n"
        );
    }

    #[test]
    fn other_text_is_left_as_it_was() {
        for (src, html) in [
            // A wiki-link without `|`.
            ("See [[Rust]].\n", "<p>See [[Rust]].</p>\n"),
            // A `|` outside a wiki-link.
            ("Yes | no.\n", "<p>Yes | no.</p>\n"),
            // A paragraph only some of whose lines hold one.
            ("First,\n[[A|a]].\n", "<p>First,\n[[A|a]].</p>\n"),
            // In code.
            (
                "`[[A|a]]`\n",
                "<p><code class=\"language-plaintext highlighter-rouge\">[[A|a]]</code></p>\n",
            ),
        ] {
            assert_eq!(excerpt_html(src, None), html, "{src:?}");
        }
    }

    /// With a resolver, wiki-links are links before parsing, so there is no row.
    #[test]
    fn resolved_wiki_links_are_not_rows() {
        struct Nowhere;
        impl super::super::LinkResolver for Nowhere {
            fn resolve(&self, _: &str) -> Option<String> {
                None
            }
        }
        assert_eq!(
            to_html("- [[A|a]]\n", Some(&Nowhere)),
            "<ul>\n  <li><span class=\"broken-link\">a</span></li>\n</ul>\n"
        );
    }
}
