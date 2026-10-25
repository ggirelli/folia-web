"""Check that two files hold the same entries, in whatever order.

Jekyll lists pages in filesystem order, which differs between checkouts (plan v3, §2.6). In two
outputs that order carries no meaning:

- sitemap.xml: its <url> blocks;
- the Lunr search index (lunrsearchengine.js, and its content dump): its documents, whose "id"
  is just the position in the array, so it must run 0, 1, 2, … in each file and is left out of
  the comparison.

    python3 -I parity/same_entries.py A B

Exits 0 when the text around the entries is identical and the entries are equal as multisets,
1 when they are not, and 2 for a file it does not know. It never treats a changed entry as a
reordered one: a <lastmod> or a title that differs makes the files differ.
"""

import re
import sys
from collections import Counter
from pathlib import Path

JSON_STRING = r'"(?:[^"\\]|\\.)*"'
SITEMAP_ENTRY = re.compile(r"<url>.*?</url>", re.DOTALL)
LUNR_ENTRY = re.compile(
    rf'\{{\s*"id": (\d+),\s*"url": ({JSON_STRING}),\s*"title": ({JSON_STRING}),\s*"body": ({JSON_STRING})\s*\}}',
    re.DOTALL,
)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    paths = [Path(arg) for arg in sys.argv[1:]]
    name = paths[0].name
    if name == "sitemap.xml":
        split = sitemap_entries
    elif name in ("lunrsearchengine.js", "lunrsearchengine.js.html"):
        split = lunr_entries
    else:
        print(f"{name}: no known entry format", file=sys.stderr)
        return 2

    (a_frame, a_entries), (b_frame, b_entries) = (
        split(p.read_text(encoding="utf-8")) for p in paths
    )
    if a_frame != b_frame:
        print(f"{name}: the text around the entries differs")
        return 1
    if Counter(a_entries) != Counter(b_entries):
        only_a = sum((Counter(a_entries) - Counter(b_entries)).values())
        only_b = sum((Counter(b_entries) - Counter(a_entries)).values())
        print(f"{name}: {only_a} entries only in A, {only_b} only in B")
        return 1
    order = "in the same order" if a_entries == b_entries else "in a different order"
    print(f"{name}: the same {len(a_entries)} entries, {order}")
    return 0


def sitemap_entries(text):
    """The text with each entry replaced by a marker, and the entries."""
    return SITEMAP_ENTRY.sub("<url/>", text), SITEMAP_ENTRY.findall(text)


def lunr_entries(text):
    """The text with each document replaced by a marker, and the documents without their ids.
    Fails if the ids are not the documents' positions."""
    matches = list(LUNR_ENTRY.finditer(text))
    ids = [int(m.group(1)) for m in matches]
    if ids != list(range(len(ids))):
        sys.exit(f"lunrsearchengine.js: the ids are not 0..{len(ids) - 1} in order")
    return LUNR_ENTRY.sub("{doc}", text), [m.groups()[1:] for m in matches]


if __name__ == "__main__":
    sys.exit(main())
