//! When a list item's first paragraph gets `<p>`. `CommonMark` decides for the whole list (tight
//! or loose); kramdown decides per item: the paragraph is wrapped when a blank line follows it
//! inside the item. The last item of a list of several is wrapped too, unless an earlier item
//! was not.

use super::tree::{Kind, Node};

/// Marks the list items' first paragraphs that kramdown writes without `<p>`.
pub fn mark_transparent(node: &mut Node, src: &str) {
    if let Kind::List(_) = node.kind {
        mark_items(&mut node.children, src);
    }
    for child in &mut node.children {
        mark_transparent(child, src);
    }
}

fn mark_items(items: &mut [Node], src: &str) {
    let count = items.len();
    let mut earlier_unwrapped = false;
    for (i, item) in items.iter_mut().enumerate() {
        let last = i + 1 == count;
        let Some(first) = item
            .children
            .first()
            .filter(|child| matches!(child.kind, Kind::Paragraph))
        else {
            earlier_unwrapped = true;
            continue;
        };
        let blank_after = is_blank_line_after(src, first.span.end);
        let alone = item.children.len() == 1;
        let transparent =
            (!blank_after || (last && alone)) && (!last || count == 1 || earlier_unwrapped);
        if transparent {
            item.children[0].kind = Kind::Transparent;
            earlier_unwrapped = true;
        }
    }
}

/// Whether the line after the one that ends at `end` is blank.
fn is_blank_line_after(src: &str, end: usize) -> bool {
    let line_start = if src[..end].ends_with('\n') {
        end
    } else {
        match src[end..].find('\n') {
            Some(i) => end + i + 1,
            None => return false,
        }
    };
    let next = &src[line_start..];
    let line = next.split_once('\n').map_or(next, |(line, _)| line);
    !next.is_empty() && line.trim().is_empty()
}
