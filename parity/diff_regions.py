"""Lists where two versions of a text file differ, token by token (plan v12, §5.4, A2.7).

    python3 -I parity/diff_regions.py A B [--limit N]

`diff -u` shows whole lines, which says little about a minified stylesheet or a feed written as
a few very long lines. This splits each file into words, runs of space and single punctuation
marks, aligns the two token lists, and prints one line per region that differs: the kind
(insert, delete, replace), the text before it and both sides, shortened. The first line counts
the regions. A file that is not UTF-8 is reported as binary. Exit 0 always: it describes, it does
not judge.
"""

import argparse
import difflib
import pathlib
import re

TOKEN = re.compile(r"\w+|\s+|[^\w\s]")
CONTEXT = 8
WIDTH = 80


def shorten(text: str) -> str:
    """The text as a Python literal, cut to WIDTH characters."""
    text = text if len(text) <= WIDTH else text[: WIDTH - 1] + "…"
    return repr(text)


def regions(first: str, second: str) -> list[str]:
    """One line per differing region of `first` and `second`."""
    a, b = TOKEN.findall(first), TOKEN.findall(second)
    matcher = difflib.SequenceMatcher(None, a, b, autojunk=False)
    lines = []
    for kind, i1, i2, j1, j2 in matcher.get_opcodes():
        if kind == "equal":
            continue
        before = "".join(a[max(0, i1 - CONTEXT) : i1])
        old, new = "".join(a[i1:i2]), "".join(b[j1:j2])
        lines.append(f"{kind}: after {shorten(before)}: {shorten(old)} -> {shorten(new)}")
    return lines


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("first", type=pathlib.Path)
    parser.add_argument("second", type=pathlib.Path)
    parser.add_argument("--limit", type=int, default=0, help="print at most N regions (0: all)")
    args = parser.parse_args()
    try:
        first = args.first.read_text(encoding="utf-8")
        second = args.second.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        print("binary: not UTF-8, not compared by region")
        return
    found = regions(first, second)
    print(f"{len(found)} regions differ")
    for line in found[: args.limit or None]:
        print(line)


if __name__ == "__main__":
    main()
