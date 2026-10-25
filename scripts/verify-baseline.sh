#!/usr/bin/env bash
# Runs the spike 0 acceptance checks (plan v2, §10) against the reference site.
#
#   scripts/verify-baseline.sh SITE BASELINES LABEL
#
# Builds BASELINES/LABEL, then two throwaway builds next to it, and checks:
#   1. a second run gives byte-identical dumps and the same normalized site;
#   2. a build without the plugin gives the same normalized site;
#   3. the manifest accounts for every built file (checked by baseline.sh).
# The throwaway builds are removed afterwards; BASELINES/LABEL is kept.
set -euo pipefail

[[ $# -eq 3 ]] || { echo "usage: $0 SITE BASELINES LABEL" >&2; exit 2; }
site=$1
keep="$2/$3"
scripts=$(dirname "$0")

scratch=$(mktemp -d "$2/.verify-XXXXXX")
trap 'rm -rf "$scratch"' EXIT

echo "== Check 3: the manifest accounts for every built file"
"$scripts/baseline.sh" "$site" "$keep"

echo "== Check 1: two runs give identical dumps and normalized sites"
"$scripts/baseline.sh" "$site" "$scratch/run2"
"$scripts/compare-builds.sh" "$keep" "$scratch/run2"

echo "== Check 2: the plugin does not change the site"
"$scripts/baseline.sh" --no-dump "$site" "$scratch/no-plugin"
"$scripts/compare-builds.sh" "$keep" "$scratch/no-plugin"

echo "== All checks passed; baseline kept in $keep"
