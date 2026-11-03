#!/usr/bin/env bash
# Runs the spike 0 acceptance checks (plan v2, §10; plan v12, §5.4, A2.6) against the reference
# site.
#
#   scripts/verify-baseline.sh [--allow-dirty] SITE BASELINES LABEL
#
# Checks, on fresh builds of SITE:
#   1. a second run gives byte-identical dumps and the same normalized site;
#   2. a build without the plugin gives the same normalized site;
#   3. the manifest accounts for every built file (checked by baseline.sh).
#
# Every build goes to a new directory, and none is ever removed or overwritten:
#   - if BASELINES/LABEL does not exist, the first build is made there and kept as the baseline;
#   - if it exists, it is neither read nor written: the first build is made in the verification
#     directory instead, as run1/, so the documented command works with an existing label.
# The verification directory, BASELINES/LABEL.verify-XXXXXX (new on every run), keeps the other
# builds (run2/, no-plugin/) with their records and logs, and each comparison's output
# (check1.txt, check2.txt), whether the checks pass or fail; its path is printed.
#
# --allow-dirty is passed to every build (see baseline.sh); the paths of
# parity/site-exclusions.txt never count as changes.
set -euo pipefail

usage() {
  echo "usage: $0 [--allow-dirty] SITE BASELINES LABEL" >&2
  exit 2
}

flags=()
while [[ $# -gt 0 && $1 == --* ]]; do
  case $1 in
    --allow-dirty) flags+=("$1") ;;
    *) usage ;;
  esac
  shift
done
[[ $# -eq 3 ]] || usage
site=$1
baselines=$2
label=$3
scripts=$(dirname "$0")

[[ -d $baselines ]] || { echo "$baselines is not a directory" >&2; exit 1; }
verify=$(mktemp -d "$baselines/$label.verify-XXXXXX")
trap 'echo "Verification builds and reports kept in $verify" >&2' EXIT

if [[ -e $baselines/$label ]]; then
  echo "== $baselines/$label exists: it is left as it is, and the checks build into $verify"
  first=$verify/run1
else
  first=$baselines/$label
fi

echo "== Check 3: the manifest accounts for every built file"
"$scripts/baseline.sh" "${flags[@]}" "$site" "$first"

echo "== Check 1: two runs give identical dumps and normalized sites"
"$scripts/baseline.sh" "${flags[@]}" "$site" "$verify/run2"
"$scripts/compare-builds.sh" "$first" "$verify/run2" 2>&1 | tee "$verify/check1.txt"

echo "== Check 2: the plugin does not change the site"
"$scripts/baseline.sh" "${flags[@]}" --no-dump "$site" "$verify/no-plugin"
"$scripts/compare-builds.sh" "$first" "$verify/no-plugin" 2>&1 | tee "$verify/check2.txt"

echo "== All checks passed on $first"
