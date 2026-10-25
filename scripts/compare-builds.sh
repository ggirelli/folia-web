#!/usr/bin/env bash
# Compares two baselines made by scripts/baseline.sh (plan v3, §10 spike 0).
#
#   scripts/compare-builds.sh A B [EXPECTED]
#
# - dump/ trees are compared byte for byte, when both baselines have one.
# - site/ trees are compared after one normalization: the feed-level
#   <updated> of each Atom feed, which Jekyll sets to the build time (§2.6).
# - Files listed in UNORDERED hold entries whose order carries no meaning
#   (§2.6). When one differs, parity/same_entries.py compares its entries as a
#   multiset; if they are equal, it is reported as "reordered", which does not
#   fail the comparison. Any changed entry still makes it differ.
#
# EXPECTED lists paths, relative to the baseline root (site/x.html,
# dump/content/y.md.html), that may differ; '#' starts a comment. Any other
# difference fails the comparison. Expected paths that turn out identical are
# reported but do not fail it.
set -euo pipefail

[[ $# -eq 2 || $# -eq 3 ]] || { echo "usage: $0 A B [EXPECTED]" >&2; exit 2; }
a=$(cd "$1" && pwd)
b=$(cd "$2" && pwd)
expected_file=${3:-}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Prints "<sha256>  <path>" for every file below a directory, sorted by path.
hashes() {
  (cd "$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0r sha256sum) | sed 's|  \./|  |'
}

# Prints one line per path that differs: "changed|only-in-A|only-in-B <path>".
differences() {
  awk '
    FILENAME == ARGV[1] { a[$2] = $1; next }
    !($2 in a) { print "only-in-B " $2; next }
    a[$2] != $1 { print "changed " $2 }
    { delete a[$2] }
    END { for (p in a) print "only-in-A " p }
  ' <(hashes "$1") <(hashes "$2") | LC_ALL=C sort -k2
}

# Replaces the feed-level <updated> (the first one before any <entry>) in
# every Atom feed below a directory. Fails if a feed has none.
normalize_feeds() {
  local count=0 feed
  while IFS= read -r -d '' feed; do
    perl -0777 -i -pe '
      s{\A((?:(?!<entry[\s>]).)*?)<updated>[^<]*</updated>}{$1<updated>BUILD-TIME</updated>}s
        or die "no feed-level <updated> in $ARGV\n";
    ' "$feed"
    count=$((count + 1))
  done < <(grep -rlZ --include='*.xml' '<feed xmlns="http://www.w3.org/2005/Atom"' "$1" || true)
  echo "$count"
}

# Outputs whose entries Jekyll lists in filesystem order, and the Lunr page's
# content dump, which holds the same entries.
UNORDERED=(
  site/sitemap.xml
  site/assets/js/lunrsearchengine.js
  dump/content/assets/js/lunrsearchengine.js.html
)

# Turns "changed <path>" into "reordered <path>" for each UNORDERED file whose
# entries are equal as a multiset.
mark_reordered() {
  local path in_a in_b
  for path in "${UNORDERED[@]}"; do
    grep -qx "changed $path" "$found" || continue
    if [[ $path == site/* ]]; then
      in_a="$work/a/${path#site/}" in_b="$work/b/${path#site/}"
    else
      in_a="$a/$path" in_b="$b/$path"
    fi
    if python3 -I "$(dirname "$0")/../parity/same_entries.py" "$in_a" "$in_b" >/dev/null; then
      sed -i "s|^changed $path\$|reordered $path|" "$found"
    fi
  done
}

found="$work/found"
: >"$found"

if [[ -d $a/dump && -d $b/dump ]]; then
  differences "$a/dump" "$b/dump" | sed 's| | dump/|' >>"$found"
  echo "dump: compared byte for byte."
else
  echo "dump: skipped (not present in both baselines)."
fi

cp -r "$a/site" "$work/a"
cp -r "$b/site" "$work/b"
feeds_a=$(normalize_feeds "$work/a")
feeds_b=$(normalize_feeds "$work/b")
differences "$work/a" "$work/b" | sed 's| | site/|' >>"$found"
mark_reordered
echo "site: compared after normalizing $feeds_a / $feeds_b Atom feeds ($(find "$work/a" -type f | wc -l) / $(find "$work/b" -type f | wc -l) files)."

expected="$work/expected"
if [[ -n $expected_file ]]; then
  sed -e 's/#.*//' -e 's/[[:space:]]*$//' -e '/^$/d' "$expected_file" | LC_ALL=C sort -u >"$expected"
else
  : >"$expected"
fi

reordered=$(grep '^reordered ' "$found" || true)
sed -i '/^reordered /d' "$found"
unexpected=$(awk 'FILENAME == ARGV[1] { e[$1]; next } !($2 in e)' "$expected" "$found")
explained=$(awk 'FILENAME == ARGV[1] { e[$1]; next } ($2 in e)' "$expected" "$found")
unused=$(awk 'FILENAME == ARGV[1] { f[$2]; next } !($1 in f)' "$found" "$expected")

# Prints a titled, indented list, unless it is empty.
report() {
  [[ -z $2 ]] && return
  echo "$1 ($(wc -l <<<"$2")):"
  while IFS= read -r line; do echo "  $line"; done <<<"$2"
}

report "Reordered, entries equal (order is not significant)" "$reordered"
report "Expected differences" "$explained"
report "Expected but identical" "$unused"
report "Unexpected differences" "$unexpected"
[[ -z $unexpected ]] || exit 1
echo "No unexpected differences."
