"""Check that two baselines of the same content differ only by changed last-modified dates.

Committing files moves their git dates, so a baseline of the commit differs from a build of
the uncommitted tree wherever those dates are shown. This checks that it differs nowhere else:

1. the meta dumps differ only in `last_modified_at`, and only between two git dates;
2. every other differing file differs only in fragments (text between tag boundaries) where
   one of those dates changed, in a format the site renders, and nothing else changed.

Dates are rewritten fragment by fragment, never file-wide: posts that were not touched may
share an old date, and must keep it.

    python3 -I parity/check_date_shift.py BEFORE AFTER

Feeds are compared after normalizing their feed-level <updated> (the build time), as in
compare-builds.sh. Nothing else is normalized.
"""

import difflib
import json
import re
import sys
from datetime import datetime
from pathlib import Path

FRAGMENT_END = re.compile(r"(?<=>)|(?<=&gt;)|(?<=\n)")
FEED_UPDATED = re.compile(r"\A((?:(?!<entry[\s>]).)*?)<updated>[^<]*</updated>", re.DOTALL)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before, after = Path(sys.argv[1]), Path(sys.argv[2])
    problems = []
    shifts = date_shifts(before, after, problems)
    if set(files(before)) != set(files(after)):
        problems.append("the baselines hold different files; compare-builds.sh lists them")
    site_files = [
        path for path in files(before) if path.startswith("site/") and (after / path).is_file()
    ]
    differing = [
        path
        for path in site_files
        if differs(before, after, path) and load(before / path) != load(after / path)
    ]

    for path in differing:
        problems += [
            f"{path}: {problem}"
            for problem in fragment_problems(before / path, after / path, shifts)
        ]

    print(f"{len(shifts)} changed dates; {len(differing)} differing site files.")
    for old, new in sorted(shifts.items()):
        print(f"  {old} -> {new}")
    for problem in problems:
        print(f"FAILS: {problem}")
    if not problems:
        print("Every difference is a changed last-modified date.")
    return 1 if problems else 0


def date_shifts(before, after, problems):
    """Old ISO date -> new ISO date, from the meta dumps."""
    shifts = {}
    for path in files(before):
        if not path.startswith("dump/") or not differs(before, after, path):
            continue
        if not path.startswith("dump/meta/"):
            problems.append(f"{path}: a dump other than meta differs")
            continue
        old, new = (
            json.loads((root / path).read_text(encoding="utf-8")) for root in (before, after)
        )
        other_keys = {key for key in old.keys() | new.keys() if old.get(key) != new.get(key)}
        other_keys -= {"last_modified_at"}
        if other_keys or old["last_modified_at_from"] != "git":
            problems.append(
                f"{path}: changes more than a git date ({', '.join(sorted(other_keys)) or 'origin'})"
            )
            continue
        if (
            shifts.setdefault(old["last_modified_at"], new["last_modified_at"])
            != new["last_modified_at"]
        ):
            problems.append(f"{path}: {old['last_modified_at']} moves to two different dates")
    return shifts


def fragment_problems(old_path, new_path, shifts):
    """Each changed fragment must be a changed date, and fragments must pair one for one."""
    old, new = (FRAGMENT_END.split(load(path)) for path in (old_path, new_path))
    problems = []
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(
        None, old, new, autojunk=False
    ).get_opcodes():
        if tag == "equal":
            continue
        if i2 - i1 != j2 - j1:
            problems.append(
                f"fragments added or removed near {old[i1 : i1 + 1] or new[j1 : j1 + 1]}"
            )
            continue
        problems += [
            f"changed beyond a date: {a.strip()!r} -> {b.strip()!r}"
            for a, b in zip(old[i1:i2], new[j1:j2], strict=True)
            if shifted(a, shifts) != b
        ]
    return problems


def shifted(text, shifts):
    """text with every rendering of each old date replaced by the new one."""
    for old, new in shifts.items():
        for render in renderings:
            text = text.replace(render(old), render(new))
    return text


def iso(value):
    return value


def month_day_year(value, month="%b"):
    moment = datetime.fromisoformat(value)
    return moment.strftime(f"{month} {moment.day}, %Y")


def full_month_day_year(value):
    return month_day_year(value, month="%B")


renderings = (iso, month_day_year, full_month_day_year)


def files(root):
    return sorted(
        str(p.relative_to(root))
        for p in root.rglob("*")
        if p.is_file() and p.name != "BASELINE.json"
    )


def differs(before, after, path):
    other = after / path
    return not other.is_file() or (before / path).read_bytes() != other.read_bytes()


def load(path):
    text = path.read_text(encoding="utf-8")
    return (
        FEED_UPDATED.sub(r"\1<updated>BUILD-TIME</updated>", text, count=1)
        if path.suffix == ".xml"
        else text
    )


if __name__ == "__main__":
    sys.exit(main())
