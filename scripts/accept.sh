#!/usr/bin/env bash
# The acceptance runner (plan v12, §5.4, work item A2.10; F12): runs every check built so far
# against one baseline, in order, and writes one evidence folder with what each found and every
# input it used. It grows with each step, and is completed in D3.
#
#   scripts/accept.sh SITE BASELINES LABEL OUT A0_LABEL A0_REFERENCE
#
# The checks are the `just` recipes, run as they are, with FOLIA_SITE and FOLIA_BASELINES set to
# SITE and BASELINES, and FOLIA_REFERENCE_LABEL to LABEL, so that each reads the same site and
# baseline:
#
#   1. `just routes LABEL`                  (F3)
#   2. `just markdown LABEL --report ...`   (F8)
#   3. `just pages LABEL OUT/work/pages`    (F1, F2, F4, F5, F6, F8, F10, F13)
#   4. `just determinism OUT/work/determinism`  (F11)
#
# Determinism also compares each build with a recorded reference (§4.2), which must be LABEL's,
# chosen fail-closed by `scripts/determinism-reference.sh`: FOLIA_REFERENCE only when
# FOLIA_REFERENCE_LABEL declares it LABEL's, A0_REFERENCE only for A0_LABEL. Without one, the
# determinism check is refused, not run, and the run fails, saying what is missing; A0's
# reference is never used for another baseline. `reference.txt` records the choice.
#
# Every check runs, whether an earlier one failed or not. OUT must not exist, and must lie outside
# Folia's repository and the site. It receives:
#
#   OUT/work/          the kept builds (`pages`, `determinism`), whole: hundreds of MB
#   OUT/evidence/      the evidence folder:
#     accept.json, accept.md       each criterion of plan v12, §8: pass, fail or pending, and why
#     transcripts/<check>.txt      each check's command, output and exit status
#     builds/<name>/               each build's build.json and sha256.txt, verbatim, as
#                                  `just evidence-provenance` reads them
#     reports/                     the checks' reports (coverage, DOM, links, warnings,
#                                  determinism, Markdown)
#     inputs/                      the configuration and the ledgers the checks read
#     lockfiles/                   Folia's Cargo.lock and the baseline's exact Gemfile.lock
#     baseline/BASELINE.json       the baseline's record: its commit, time and toolchain
#     folia-before.txt, folia-after.txt  Folia's commit and `git status`, around the run
#     toolchain.txt                the tools' versions
#     reference.txt, inputs/reference-sha256.txt  the determinism reference chosen, and a copy
#
# The baseline's Gemfile.lock is the one it keeps, or for a baseline that predates kept lockfiles
# `parity/toolchain/local-LABEL.Gemfile.lock`, checked against the SHA-256 its BASELINE.json
# records (`parity/accept.py lockfile`); if neither is known, nothing runs.
#
# A run from a Folia tree with changes runs every check, and records the changed paths; it is
# not evidence (`evidence_eligible: false`). Exit 0: every check ran and passed, and no criterion
# failed (criteria still pending do not fail it; `final_acceptance` says whether any is). Exit 1:
# a check or criterion failed. Exit 2: refused before anything ran.
set -euo pipefail

[[ $# -eq 6 ]] || { echo "usage: $0 SITE BASELINES LABEL OUT A0_LABEL A0_REFERENCE" >&2; exit 2; }
site=$1
baselines=$2
label=$3
out=$4
a0_label=$5
a0_reference=$6
refuse() { echo "$1" >&2; exit 2; }

top=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$top"
baseline=$baselines/$label
[[ -f $baseline/BASELINE.json ]] || refuse "$baseline is not a baseline (no BASELINE.json)"
[[ -e $out ]] && refuse "$out already exists; a run never writes into an existing directory"
abs=$(realpath -m "$out")
for inside in "$top" "$(realpath -m "$site")"; do
  [[ $abs == "$inside" || $abs == "$inside"/* ]] && refuse "$out is inside $inside"
done
# The determinism reference (above, `scripts/determinism-reference.sh`): `reference` and how it
# was chosen, or why there is none.
reference=
if found=$(scripts/determinism-reference.sh "$label" "$a0_label" "$a0_reference" 2>&1); then
  reference=${found%%$'\n'*}
  chosen=${found#*$'\n'}
else
  missing=$found
fi

gemfile_lock=$(python3 -I parity/accept.py lockfile "$baseline" "$label" parity/toolchain) ||
  refuse "the baseline's lockfile is not known, so the run would not be reproducible (F12)"

# Folia's commit, then `git status` (ignored files aside): the identity Q26 asks of the source.
folia_state() {
  git --no-optional-locks rev-parse HEAD
  git --no-optional-locks status --porcelain --untracked-files=all
}

evidence=$abs/evidence
work=$abs/work
mkdir -p "$evidence"/{transcripts,builds,reports/pages,reports/determinism,inputs,lockfiles,baseline} "$work"
folia_state >"$evidence/folia-before.txt"
cp -p parity/filopoe.toml parity/deviations.toml parity/expected-warnings.toml \
  parity/site-exclusions.txt "$evidence/inputs/"
cp -p Cargo.lock "$evidence/lockfiles/Cargo.lock"
cp -p "$gemfile_lock" "$evidence/lockfiles/Gemfile.lock"
cp -p "$baseline/BASELINE.json" "$evidence/baseline/BASELINE.json"
{
  echo "rustc: $(rustc --version)"
  echo "cargo: $(cargo --version)"
  echo "just: $(just --version)"
  echo "python3: $(python3 --version)"
  echo "git: $(git --version)"
} >"$evidence/toolchain.txt"
if [[ -n $reference ]]; then
  cp -p "$reference" "$evidence/inputs/reference-sha256.txt"
  printf 'path: %s\nchosen: %s\n' "$reference" "$chosen" >"$evidence/reference.txt"
else
  printf 'refused: %s\n' "$missing" >"$evidence/reference.txt"
fi

export FOLIA_SITE=$site FOLIA_BASELINES=$baselines FOLIA_REFERENCE=$reference FOLIA_REFERENCE_LABEL=$label
# Runs `just $2...` as check $1, its command, output and exit status in its transcript.
check() {
  local name=$1 status=0
  shift
  {
    echo "\$ just $*"
    echo "# Folia $(git rev-parse HEAD), $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  } >"$evidence/transcripts/$name.txt"
  just "$@" >>"$evidence/transcripts/$name.txt" 2>&1 || status=$?
  echo "# exit $status" >>"$evidence/transcripts/$name.txt"
  echo "$name: exit $status"
}
check routes routes "$label"
check markdown markdown "$label" --report "$evidence/reports/markdown.md"
check pages pages "$label" "$work/pages"
if [[ -n $reference ]]; then
  check determinism determinism "$work/determinism"
else
  {
    echo "\$ just determinism $work/determinism"
    echo "refused: $missing"
    echo "# exit 2"
  } >"$evidence/transcripts/determinism.txt"
  echo "determinism: refused: $missing" >&2
fi

# The records and reports the checks kept, verbatim; what a failed check did not write is missing.
keep() {
  local from=$1 to=$2 file
  shift 2
  for file in "$@"; do
    if [[ -f $from/$file ]]; then mkdir -p "$to" && cp -p "$from/$file" "$to/"; fi
  done
}
keep "$work/pages" "$evidence/builds/pages" build.json sha256.txt
keep "$work/determinism/build-1" "$evidence/builds/determinism-1" build.json sha256.txt
keep "$work/determinism/build-2" "$evidence/builds/determinism-2" build.json sha256.txt
keep "$work/pages" "$evidence/reports/pages" coverage.json coverage.md dom-report.json \
  dom-report.md links.json warnings.json
keep "$work/determinism" "$evidence/reports/determinism" determinism-report.json \
  determinism-report.md
folia_state >"$evidence/folia-after.txt"

python3 -I parity/accept.py report "$evidence" "$label" "$baseline" "$gemfile_lock"
