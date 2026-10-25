"""Check that two builds differ only by deviation D-001 in the given files.

Plan v2, §10 spike 3. D-001: a `{% highlight %}` block became a fenced code block, so its
wrapper markup changed and kramdown added one trailing newline to the code, as it does for
every fenced block. For each file, this checks that:

1. everything outside the code blocks is byte-identical, and so is every fenced block that
   was already fenced;
2. the blocks pair up, one for one, with the same language;
3. each block's code text is identical, apart from that trailing newline;
4. each block's Rouge tokens (class and text) are identical, apart from that newline.

    python3 -I parity/codemods/check_d001.py BEFORE AFTER PATH...

BEFORE and AFTER are baselines; each PATH is relative to them (site/x.html, dump/content/y.html).
Feeds hold the HTML escaped, so .xml files are unescaped, and their feed-level <updated> (the
build time) is normalized as in compare-builds.sh, before comparing.
"""

import html
import re
import sys
from pathlib import Path

# Group 1-2: a highlight block (language, body); group 3-4: a fenced block (language, body).
BLOCK = re.compile(
    r'<figure class="highlight"><pre><code class="language-(\w+)" data-lang="\1">(.*?)</code></pre></figure>'
    r'|<div class="language-(\w+) highlighter-rouge"><div class="highlight"><pre class="highlight">'
    r"<code>(.*?)</code></pre></div></div>",
    re.DOTALL,
)
TOKEN = re.compile(r'<span class="([^"]+)">(.*?)</span>|([^<]+)', re.DOTALL)
FEED_UPDATED = re.compile(r"\A((?:(?!<entry[\s>]).)*?)<updated>[^<]*</updated>", re.DOTALL)
PLACEHOLDER = "\0block\0"


def main():
    if len(sys.argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    before_dir, after_dir, paths = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
    failures = 0
    for path in paths:
        problems, count = check(load(before_dir / path), load(after_dir / path))
        status = "D-001 only" if not problems else "FAILS"
        print(f"{status}: {path} ({count} blocks)")
        for problem in problems:
            print(f"  {problem}")
        failures += bool(problems)
    print(f"{len(paths) - failures} of {len(paths)} files differ by D-001 only.")
    return 1 if failures else 0


def load(path):
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".xml":
        text = FEED_UPDATED.sub(r"\1<updated>BUILD-TIME</updated>", html.unescape(text), count=1)
    return text


def check(before, after):
    """Returns the problems found, and the number of highlight blocks compared. Blocks pair up
    in order: a highlight block must have become a fenced block, and a fenced block that was
    already there (another post in the same feed) must be unchanged."""
    old_blocks, new_blocks = BLOCK.findall(before), BLOCK.findall(after)
    problems = []
    if BLOCK.sub(PLACEHOLDER, before) != BLOCK.sub(PLACEHOLDER, after):
        problems.append("text outside the code blocks differs")
    if len(old_blocks) != len(new_blocks):
        problems.append(f"{len(old_blocks)} code blocks before, {len(new_blocks)} after")
    converted = 0
    # A different number of blocks is reported above; the blocks that pair up are still compared.
    for number, (old, new) in enumerate(zip(old_blocks, new_blocks, strict=False), start=1):
        old_lang, old_body, kept_lang, _ = old
        _, _, new_lang, new_body = new
        if kept_lang:
            if old != new:
                problems.append(f"block {number}: an existing fenced block changed")
            continue
        converted += 1
        if old_lang != new_lang:
            problems.append(
                f"block {number}: language {old_lang} became {new_lang or 'something else'}"
            )
        if code_text(new_body) != code_text(old_body) + "\n":
            problems.append(f"block {number}: code text differs")
        if without_final_newline(tokens(new_body)) != tokens(old_body):
            problems.append(f"block {number}: Rouge tokens differ")
    return problems, converted


def code_text(body):
    return html.unescape(re.sub(r"<[^>]+>", "", body))


def tokens(body):
    """(class, text) pairs, with plain text as class None."""
    return [(css, text if css else plain) for css, text, plain in TOKEN.findall(body)]


def without_final_newline(pairs):
    """The tokens without the newline kramdown appends, which Rouge puts in the last token."""
    if not pairs or not pairs[-1][1].endswith("\n"):
        return pairs
    css, text = pairs[-1]
    return pairs[:-1] + ([(css, text[:-1])] if text[:-1] else [])


if __name__ == "__main__":
    sys.exit(main())
