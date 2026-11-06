#!/usr/bin/env bash
# Shows that `parity determinism` (plan v12, §5.4, A2.5) fails when it should, on the real site
# and baseline, which it only reads: a reference changed once per kind of difference (a hash, a
# file it does not list, a file it lists that is not written) fails the run, naming each file,
# while the two builds still agree with each other; a malformed reference, an existing or
# misplaced run directory and other templates are refused, and nothing existing is changed.
# Changes in the builds themselves (a byte, an entry, a link, a result, an input) are shown by
# the unit tests, which build throwaway sites.
#
#   scripts/determinism-negative.sh SITE CONFIG BASELINE REFERENCE TEMPLATES_SHA256 WORKDIR
#
# WORKDIR must not exist; it is kept, with each case's output (<case>.out) and the builds.
set -euo pipefail

[[ $# -eq 6 ]] || { echo "usage: $0 SITE CONFIG BASELINE REFERENCE TEMPLATES_SHA256 WORKDIR" >&2; exit 2; }
site=$1
config=$2
baseline=$3
reference=$4
templates=$5
work=$6
[[ -e $work ]] && { echo "$work already exists" >&2; exit 1; }
mkdir -p "$work"
work=$(realpath "$work")

cases=0
fail() { echo "$1" >&2; exit 1; }

# Runs case $1 with the arguments from $3 on; requires exit status $2 and every line of $LINES
# (one per line) in the output.
check() {
  local name=$1 want=$2 status=0 line
  shift 2
  cargo run --quiet --locked -p parity -- determinism --site "$site" --baseline "$baseline" \
    --config "$config" "$@" >"$work/$name.out" 2>&1 || status=$?
  cases=$((cases + 1))
  [[ $status -eq $want ]] || fail "$name: exit $status, expected $want (see $work/$name.out)"
  while IFS= read -r line; do
    [[ -z $line ]] && continue
    grep -qF -- "$line" "$work/$name.out" || fail "$name: no line with: $line (see $work/$name.out)"
  done <<<"$LINES"
}

snapshot() { (cd "$1" && find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0r sha256sum); }

# A reference with the first file's hash changed, the second file left out, and a file added.
first=$(sed -n 1p "$reference")
second=$(sed -n 2p "$reference")
changed=${first%%  ./*}
first_path=${first#*  ./}
second_path=${second#*  ./}
{
  sed -e "1s/^$changed/$(printf '0%.0s' {1..64})/" -e 2d "$reference"
  echo "$(printf '1%.0s' {1..64})  ./not-written.html"
} >"$work/changed-reference.txt"
[[ $changed != "$(printf '0%.0s' {1..64})" ]] || fail "the reference's first hash is already zeros"

LINES="ok: the two output trees are identical, byte for byte
FAILS: build-1 differs from the reference $work/changed-reference.txt in 3 files
FAILS: build-2 differs from the reference $work/changed-reference.txt in 3 files
$first_path: sha256 $changed, the reference's $(printf '0%.0s' {1..64})
$second_path: written, not in the reference
not-written.html: in the reference, not written
Determinism does not hold" \
  check reference 1 --reference "$work/changed-reference.txt" --templates-sha256 "$templates" \
  --out "$work/reference-run"
for kind in sha256 only-in-build only-in-reference; do
  grep -qF "\"kind\": \"$kind\"" "$work/reference-run/determinism-report.json" ||
    fail "reference: the report has no $kind difference"
done
[[ -d $work/reference-run/build-1/site && -d $work/reference-run/build-2/site ]] ||
  fail "reference: a build was not kept"

# A malformed reference is refused before anything is built or created.
printf 'not a manifest\n' >"$work/malformed.txt"
LINES="malformed.txt line 1: not" check malformed 2 --reference "$work/malformed.txt" --out "$work/malformed-run"
[[ ! -e $work/malformed-run ]] || fail "malformed: created its directory"

# An existing directory is refused and left as it was.
before=$(snapshot "$work/reference-run")
LINES="already exists, and is never overwritten" check existing 2 --out "$work/reference-run"
[[ $(snapshot "$work/reference-run") == "$before" ]] || fail "existing: the directory changed"

# A directory inside the site is refused, and the site is left as it was.
status=$(git -C "$site" status --porcelain --untracked-files=all)
LINES="it is inside the site" check inside 2 --out "$site/determinism-run"
[[ ! -e $site/determinism-run ]] || fail "inside: created a directory in the site"
[[ $(git -C "$site" status --porcelain --untracked-files=all) == "$status" ]] || fail "inside: the site changed"

# Other templates are refused before anything is built; the run's directory stays, empty.
LINES="not the expected $(printf '0%.0s' {1..64}); nothing was built" \
  check templates 2 --templates-sha256 "$(printf '0%.0s' {1..64})" --out "$work/templates-run"
[[ -z $(ls -A "$work/templates-run") ]] || fail "templates: built something"

echo "every case ended as it should ($cases cases); see $work/*.out"
