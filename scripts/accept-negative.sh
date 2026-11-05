#!/usr/bin/env bash
# The Python expressions given to `expect` name assertions in literal backticks, in single quotes.
# shellcheck disable=SC2016
# Shows that the acceptance runner (`scripts/accept.sh`, plan v12, §5.4, A2.10) runs every check
# in order and judges them as it should: a run whose checks pass exits 0 with each criterion not
# built yet `pending`, never `pass`, and `final_acceptance` false; a failing check, a check that
# passes while a criterion it reports on fails, or a failing A2.8 link verdict alone, makes it
# exit 1, and the later checks still run; the determinism reference is A0's for A0's baseline
# only, another baseline's only when configured and declared its own, and with none the
# determinism check is refused and the run fails, never falling back to A0's; a Folia tree with
# changes runs, and is no evidence; each §5.1 assertion is pending only for the outputs of its
# own scope not built yet, passes once they are built, and fails if it did not run on a built
# output; the standalone `just determinism` and `determinism-negative` refuse, before building, a
# reference not declared the baseline's; it keeps both lockfiles, verified, and build records
# `scripts/evidence-provenance.sh` accepts; and it refuses, exit 2 and nothing written, an existing
# or misplaced output directory and a baseline whose lockfile is unknown or differs from its
# record.
#
# Every case runs the runner from a throwaway, committed copy of what it uses (so Folia's identity
# is known), none of the user's git configuration applying, on throwaway baselines, with a
# stand-in for `just` on the PATH that plays the four checks: it needs neither the site, nor a real
# baseline, nor a build, and reads no evidence of the repository's.
#
#   scripts/accept-negative.sh WORKDIR
#
# WORKDIR must not exist; it is kept, with each case's output.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 WORKDIR" >&2; exit 2; }
work=$1
[[ -e $work ]] && { echo "$work already exists" >&2; exit 1; }
repo=$(realpath "$(dirname "$0")/..")
mkdir -p "$work/bin" "$work/out" "$work/site"
work=$(realpath "$work")

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.org
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.org
unset FOLIA_REFERENCE_LABEL

fail() { echo "$1" >&2; exit 1; }

# A committed copy of what the runner uses, so Folia's identity is clean and known.
folia=$work/folia
for file in scripts/accept.sh scripts/evidence-provenance.sh scripts/determinism-reference.sh parity/accept.py \
  parity/filopoe.toml parity/deviations.toml parity/expected-warnings.toml \
  parity/site-exclusions.txt Cargo.lock; do
  mkdir -p "$(dirname "$folia/$file")"
  cp -p "$repo/$file" "$folia/$file"
done
mkdir -p "$folia/parity/toolchain"
printf 'GEM\n  specs:\n    jekyll (4.4.1)\n' >"$folia/parity/toolchain/local-old.Gemfile.lock"
printf 'notes\n' >"$folia/NOTES.md"
# A stand-in for A0's reference, the baseline `old`'s, and another baseline's, outside the copy.
mkdir -p "$folia/parity/evidence/a0-reference" "$work/refs"
printf '%064d  ./a.html\n' 1 >"$folia/parity/evidence/a0-reference/sha256.txt"
printf '%064d  ./a.html\n' 6 >"$work/refs/kept.txt"
git init -q -b main "$folia"
git -C "$folia" add -A
git -C "$folia" commit -q -m folia
commit=$(git -C "$folia" rev-parse HEAD)

# The baselines: `old` records its lockfile's hash only (the one in parity/toolchain), `kept`
# keeps its lockfile (A2.7), `mismatch` records another hash, `unknown` has no lockfile at all.
sum() { sha256sum "$1" | cut -d' ' -f1; }
baselines=$work/baselines
record() {
  mkdir -p "$baselines/$1"
  printf '{"created_at":"2026-10-25T14:43:51Z","site":{"commit":"SITE-COMMIT-B","gemfile_lock_sha256":%s},"lockfile":%s,"tools":{"ruby":"ruby stand-in"}}\n' \
    "$2" "$3" >"$baselines/$1/BASELINE.json"
}
record old "\"$(sum "$folia/parity/toolchain/local-old.Gemfile.lock")\"" null
mkdir -p "$baselines/kept"
printf 'GEM\n  specs:\n    jekyll (4.4.0)\n' >"$baselines/kept/Gemfile.lock"
record kept null "{\"sha256\":\"$(sum "$baselines/kept/Gemfile.lock")\"}"
record mismatch "\"$(printf '%064d' 7)\"" null
cp "$folia/parity/toolchain/local-old.Gemfile.lock" "$folia/parity/toolchain/local-mismatch.Gemfile.lock"
record unknown "\"$(printf '%064d' 7)\"" null
git -C "$folia" add -A
git -C "$folia" commit -q -m 'the mismatched lockfile'
commit=$(git -C "$folia" rev-parse HEAD)

# The stand-in for `just`: each recipe prints what the real one prints of its verdicts and writes
# what it keeps, its build records naming the Folia tree's commit and state; STUB_FAIL makes one
# fail: `markdown` (a gate), `pages` (the links), `assertion` (pages exits 0, an assertion in its
# report failing), `links` (pages exits 0, its link verdict failing). Its site plans 5 HTML
# outputs, `a.html` (a page), `s.html` (a stub), `index.html` (`/`), `p.html` and `t.html` (a
# stub), and builds the first two; with STUB_BUILT set, it builds every one, and gives no `rel-me`
# outcome. Each call is logged, with the reference label and the reference, in STUB_LOG.
cat >"$work/bin/just" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ $1 == --version ]] && { echo "just stand-in"; exit 0; }
echo "$1 ${FOLIA_REFERENCE_LABEL:-} ${FOLIA_REFERENCE:-}" >>"$STUB_LOG"
failing=${STUB_FAIL:-}
record() {
  local dir=$1 state
  mkdir -p "$dir"
  if [[ -z $(git status --porcelain --untracked-files=all) ]]; then state='{"state":"clean"}'
  else state="{\"state\":\"changed\",\"sha256\":\"$(printf '%064d' 9)\"}"; fi
  printf '%064d  ./a.html\n%064d  ./b.css\n' 1 2 >"$dir/sha256.txt"
  cat >"$dir/build.json" <<JSON
{"at":"2026-10-25T14:43:51Z","site":{"path":"/x/site","commit":"SITE-COMMIT-B","status":[],"changes":{"state":"changed","sha256":"$(printf '%064d' 3)"}},
 "config":{"path":"parity/filopoe.toml","sha256":"$(printf '%064d' 4)"},
 "templates":{"path":"/x/t","files":30,"tree_sha256":"$(printf '%064d' 5)"},
 "folia":{"path":"$PWD","commit":"$(git rev-parse HEAD)","status":[],"changes":$state},
 "rustc":"rustc stand-in","result":{"ok":true},"files":2}
JSON
}
case $1 in
  routes)
    echo "ok: Authored outputs (source, URL, output): Jekyll 1, Folia 1, identical."
    echo "ok: Redirect stubs (URL, output): Jekyll 2, Folia 2, identical."
    echo "Route parity holds." ;;
  markdown)
    result=pass
    [[ $failing == markdown ]] && result=FAIL
    {
      echo "| Gate | Failing | Unexplained | Explained by the ledger | Result |"
      echo "|-|-:|-:|-|-|"
      for gate in "G1 heading IDs" "G2 link targets" "G3 text" "G4 structure" "G5 excerpts"; do
        echo "| $gate | 0 | 0 |  | pass |"
      done
      echo "| G6 code text | 1 | 1 |  | $result |"
      echo "| **lexed by Rouge** | | 10 | **95.0%** |"
    } | tee "$4"
    [[ $result == pass ]] || exit 1 ;;
  pages)
    out=$3
    record "$out"
    passed=true
    [[ $failing == assertion ]] && passed=false
    later=not-built-yet unbuilt=3 stubs=1
    [[ -n ${STUB_BUILT:-} ]] && later=built-and-checked unbuilt=0 stubs=2
    rows='{"output":"a.html","class":"built-and-checked"},{"output":"s.html","class":"built-and-checked"}'
    for output in index.html p.html t.html; do rows+=",{\"output\":\"$output\",\"class\":\"$later\"}"; done
    echo "{\"counts\":{\"rows\":5,\"not_built_yet\":$unbuilt},\"rows\":[$rows]}" >"$out/coverage.json"
    outcomes=""
    for assertion in robots canonical title lang h-entry; do
      outcomes+="{\"file\":\"a.html\",\"assertion\":\"$assertion\",\"passed\":$passed},"
    done
    [[ -n ${STUB_BUILT:-} ]] && outcomes+='{"file":"index.html","assertion":"h-card","passed":true},'
    outcomes+='{"file":"s.html","assertion":"stub","passed":true}'
    echo "{\"assertions\":[$outcomes],\"ledger\":{\"final_acceptance\":[]}}" >"$out/dom-report.json"
    echo "Folia built 1 pages, $stubs redirect stubs, 0 generated files and 1 static files."
    for file in coverage.md dom-report.md links.json warnings.json; do echo '{}' >"$out/$file"; done
    echo "ok: routes of every output written (2 files)."
    echo "ok: a.html: fingerprint."
    echo "ok: a.md: dates equal the baseline's dump."
    echo "ok: redirect stubs and static files byte-identical (0 stubs, 1 static files)."
    echo "ok: assets/json/notes_graph.json: the nodes and edges are Jekyll's."
    echo "ok: whole-document DOM comparison of 1 pages: 0 differences."
    echo "ok: one warning per broken link of the pages built (0 warnings)."
    echo "ok: expected warnings: 0 warnings, 0 expected warnings."
    echo "ok: coverage of 5 manifest rows: 2 built and checked, 3 not built yet."
    if [[ $failing == pages ]]; then echo "FAILS: links: 1 internal href resolves to nothing planned"; exit 1; fi
    if [[ $failing == links ]]; then echo "FAILS: links: 1 internal href resolves to nothing planned"; exit 0; fi
    echo "ok: links: 1 internal href and src in 1 HTML files." ;;
  determinism)
    record "$2/build-1"
    record "$2/build-2"
    echo '{}' >"$2/determinism-report.json"
    echo '# report' >"$2/determinism-report.md"
    echo "ok: the two builds have the same inputs (build.json)."
    echo "Determinism holds; the builds and the report are kept in $2." ;;
  *) echo "stand-in: no recipe $1" >&2; exit 2 ;;
esac
EOF
chmod +x "$work/bin/just"
real_just=$(command -v just)
export PATH=$work/bin:$PATH

a0=parity/evidence/a0-reference/sha256.txt
# Runs case $1 on baseline $2 (`old` is A0's baseline) into $work/out/$1 (or $3), requiring exit status $4 and each further
# argument as a line of its output.
run_case() {
  local name=$1 label=$2 out=${3:-$work/out/$1} want=$4 status=0 line
  shift 4
  STUB_LOG=$work/$name.log "$folia/scripts/accept.sh" "$work/site" "$baselines" "$label" "$out" \
    old "$a0" >"$work/$name.txt" 2>&1 || status=$?
  [[ $status -eq $want ]] || fail "$name: the runner exited with $status; expected $want (see $work/$name.txt)"
  for line in "$@"; do
    grep -qF -- "$line" "$work/$name.txt" || fail "$name: the output lacks: $line (see $work/$name.txt)"
  done
}
# Requires the Python expression $2, of `d` (accept.json of case $1) and `e` (its evidence
# folder), to be true.
expect() {
  python3 -I - "$work/out/$1/evidence" "$2" <<'EOF' || fail "$1: not true of accept.json: $2"
import json, pathlib, sys
e = pathlib.Path(sys.argv[1])
d = json.loads((e / "accept.json").read_text())
c = {criterion["id"]: criterion for criterion in d["criteria"]}
part = lambda ident, name: next(p for p in c[ident]["parts"] if p["part"].startswith(name))
sys.exit(0 if eval(sys.argv[2]) else 1)
EOF
}

# 1. The checks pass: exit 0, every check run in order, on the baseline's label; nothing not
# built yet passes; final acceptance not met; eligible; both lockfiles kept, verified.
run_case pass old "" 0 "the checks built so far pass; final acceptance: not met; evidence eligible: yes"
[[ $(cat "$work/pass.log") == "routes old $a0"$'\n'"markdown old $a0"$'\n'"pages old $a0"$'\n'"determinism old $a0" ]] ||
  fail "pass: the checks did not run in order on the baseline's label and A0's reference (see $work/pass.log)"
expect pass "d['reference']['path'] == '$a0' and d['reference']['chosen'].startswith('the A0 reference')"
expect pass "d['reference']['sha256'] == '$(sum "$folia/$a0")'"
expect pass 'd["passed"] and d["final_acceptance"] is False and d["evidence_eligible"]'
expect pass '[x["exit"] for x in d["checks"]] == [0, 0, 0, 0]'
expect pass 'c["F2"]["status"] == "pass" and part("F1", "every row")["status"] == "pass"'
expect pass 'part("F1", "not built yet")["status"] == "pending" and "= 3" in part("F1", "not built yet")["why"]'
expect pass 'all(c[i]["status"] == "pending" for i in ["F1", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15", "F16"])'
expect pass 'part("F3", "the *claimed*")["status"] == part("F8", "Rouge")["status"] == part("F14", "the sample")["status"] == "pending"'
expect pass 'part("F11", "from two fresh")["status"] == part("F15", "F1")["status"] == "pending"'
expect pass 'part("F11", "twice")["status"] == "pass" and len(c["F11"]["parts"]) == 2'
expect pass 'all(part(i, "the outputs not built yet")["status"] == "pending" for i in ["F4", "F5", "F8", "F13"])'
expect pass 'not any(p["part"] == "the outputs not built yet" for i in ["F1", "F2", "F3", "F11", "F12"] for p in c[i]["parts"])'
expect pass 'part("F5", "entries reviewed")["status"] == "pending" and "records no review" in part("F5", "entries reviewed")["why"]'
expect pass 'part("F3", "every *removed*")["status"] == "pending"'
expect pass 'all(x["status"] == "pending" and any(p["status"] == "pass" for p in x["parts"]) for x in (c["F4"], c["F5"], c["F8"], c["F10"], c["F11"], c["F13"]))'
# F6: one part per §5.1 assertion, pending only for the outputs of its own scope not built yet.
expect pass '[p["part"] for p in c["F6"]["parts"]] == ["`robots`", "`canonical`", "`title`", "`description`", "`lang`", "`rel-me`", "`h-card`", "`h-entry`", "`stub`"]'
expect pass 'all(p["status"] == "pending" for p in c["F6"]["parts"]) and c["F6"]["status"] == "pending"'
expect pass '"holds on the 1 built" in part("F6", "`robots`")["why"] and "2 HTML pages other than redirect stubs" in part("F6", "`robots`")["why"]'
expect pass 'part("F6", "`rel-me`")["why"] == part("F6", "`h-card`")["why"] == "has not run; `/` (`index.html`) is not built yet (B2)"'
expect pass '"1 redirect stubs are not built yet" in part("F6", "`stub`")["why"] and part("F6", "`stub`")["plan"] == "§5.1: redirect stubs"'
expect pass '"ambiguity" in part("F6", "`h-entry`") and "ambiguity" not in part("F6", "`robots`")'
# F10: this build's warnings judged; the notes not built yet pending, with the reading open.
expect pass 'part("F10", "every warning of this build")["status"] == "pass" and part("F10", "the warnings of the outputs")["status"] == "pending"'
expect pass '"note.rs" in part("F10", "the warnings of the outputs")["why"] and "ambiguity" in part("F10", "the warnings of the outputs")'
grep -qF "## Readings open for the owner" "$work/out/pass/evidence/accept.md" || fail "pass: accept.md lists no open reading"
expect pass 'd["other_checks"][0]["required"] and d["other_checks"][0]["gates_run"] and d["other_checks"][0]["criterion"] is None'
expect pass 'all(p["status"] != "pass" for x in d["criteria"] for p in x["parts"] if p["source"].startswith("none"))'
expect pass 'd["other_checks"][0]["status"] == "pass" and not any(x["unjudged"] for x in d["checks"])'
expect pass "d['folia']['before']['commit'] == '$commit' and d['folia']['before']['state'] == 'clean'"
expect pass "d['lockfiles'][1]['sha256'] == d['lockfiles'][1]['recorded_sha256'] == '$(sum "$folia/parity/toolchain/local-old.Gemfile.lock")'"
expect pass "d['lockfiles'][0]['sha256'] == '$(sum "$folia/Cargo.lock")'"
cmp -s "$work/out/pass/evidence/lockfiles/Gemfile.lock" "$folia/parity/toolchain/local-old.Gemfile.lock" ||
  fail "pass: the baseline's Gemfile.lock is not kept verbatim"
cmp -s "$work/out/pass/evidence/lockfiles/Cargo.lock" "$folia/Cargo.lock" ||
  fail "pass: Folia's Cargo.lock is not kept verbatim"
# Its records are what the provenance check reads: in a clone of the commit, the folder is ready
# to be committed on its own.
git clone -q "$folia" "$work/clone"
cp -r "$work/out/pass/evidence" "$work/clone/accept-evidence"
(cd "$work/clone" && scripts/evidence-provenance.sh accept-evidence) >"$work/provenance.txt" 2>&1 ||
  fail "pass: the provenance check refuses the folder (see $work/provenance.txt)"
grep -qF "ok: accept-evidence is ready to be committed on its own, on $commit" "$work/provenance.txt" ||
  fail "pass: the provenance check says otherwise (see $work/provenance.txt)"
# The report is never written twice.
status=0
python3 -I "$folia/parity/accept.py" report "$work/out/pass/evidence" old "$baselines/old" \
  "$folia/parity/toolchain/local-old.Gemfile.lock" >"$work/rewrite.txt" 2>&1 || status=$?
if [[ $status -ne 2 ]] || ! grep -qF "is never overwritten" "$work/rewrite.txt"; then
  fail "rewrite: the report was not refused (exit $status, see $work/rewrite.txt)"
fi

# 2. Another baseline, which keeps its lockfile (that one is kept), with its reference configured
# and declared its own: that reference is used.
FOLIA_REFERENCE=$work/refs/kept.txt FOLIA_REFERENCE_LABEL=kept run_case kept kept "" 0 "evidence eligible: yes"
expect kept "d['lockfiles'][1]['source'] == '$baselines/kept/Gemfile.lock'"
cmp -s "$work/out/kept/evidence/lockfiles/Gemfile.lock" "$baselines/kept/Gemfile.lock" ||
  fail "kept: the baseline's Gemfile.lock is not kept verbatim"
grep -qxF "determinism kept $work/refs/kept.txt" "$work/kept.log" || fail "kept: determinism did not use kept's reference"
expect kept "d['reference']['path'] == '$work/refs/kept.txt' and d['reference']['chosen'].startswith('configured')"
cmp -s "$work/out/kept/evidence/inputs/reference-sha256.txt" "$work/refs/kept.txt" ||
  fail "kept: the reference is not kept verbatim"

# 3. to 7. Another baseline without its own reference, or A0's baseline declared another's: the
# determinism check is refused, the others run, the run fails; A0's reference is never used.
ref_case() {
  local name=$1 label=$2 line=$3
  run_case "$name" "$label" "" 1 "determinism: refused: " "$line" "F11 Determinism: fail"
  [[ $(cut -d' ' -f1 "$work/$name.log" | tr '\n' ' ') == "routes markdown pages " ]] ||
    fail "$name: determinism ran, or another check did not (see $work/$name.log)"
  ! grep -qF "$a0" "$work/$name.log" || fail "$name: A0's reference was used"
  expect "$name" "'refused' in d['reference'] and 'path' not in d['reference'] and d['checks'][3]['exit'] == 2"
  expect "$name" "part('F11', 'twice')['status'] == 'fail' and 'was not run' in part('F11', 'twice')['why']"
  expect "$name" "d['checks'][0]['passed'] and d['checks'][2]['passed'] and not d['passed']"
}
ref_case no-reference kept "no reference is configured for kept (A0's is old's only)"
FOLIA_REFERENCE=$work/refs/kept.txt ref_case undeclared kept "does not declare it kept's"
FOLIA_REFERENCE=$work/refs/kept.txt FOLIA_REFERENCE_LABEL=other ref_case declared-other kept "FOLIA_REFERENCE_LABEL (other) does not declare it kept's"
FOLIA_REFERENCE_LABEL=kept ref_case label-only kept "no reference is configured for kept"
FOLIA_REFERENCE=$work/refs/none.txt FOLIA_REFERENCE_LABEL=kept ref_case not-a-file kept "is not a file"
FOLIA_REFERENCE_LABEL=other ref_case a0-declared-other old "FOLIA_REFERENCE_LABEL is other, not old"

# 8. to 11. A failing check, criterion or A2.8 link verdict: exit 1, and every check still runs.
STUB_FAIL=markdown run_case markdown old "" 1 "F8 Markdown and content: fail" "FAILS; final acceptance: not met"
expect markdown 'not d["passed"] and [x["exit"] for x in d["checks"]] == [0, 1, 0, 0]'
expect markdown 'part("F8", "G1")["status"] == "fail" and "G6 code text: FAIL" in part("F8", "G1")["why"]'
STUB_FAIL=pages run_case pages old "" 1 "FAILS; final acceptance: not met"
expect pages 'not d["passed"] and d["other_checks"][0]["status"] == "fail" and d["checks"][3]["exit"] == 0'
STUB_FAIL=assertion run_case assertion old "" 1 "F6 Document assertions: fail"
expect assertion 'not d["passed"] and all(x["passed"] for x in d["checks"])'
STUB_FAIL=links run_case links old "" 1 "FAILS; final acceptance: not met"
expect links 'not d["passed"] and all(x["passed"] for x in d["checks"]) and all(x["status"] != "fail" for x in d["criteria"])'
expect links 'd["other_checks"][0]["status"] == "fail" and d["other_checks"][0]["gates_run"]'
for name in markdown pages assertion links; do
  [[ $(wc -l <"$work/$name.log") -eq 4 ]] || fail "$name: not every check ran (see $work/$name.log)"
done

# 12. to 16. Refused before anything runs: exit 2, nothing written, no check run.
mkdir -p "$work/out/exists"
printf 'kept\n' >"$work/out/exists/sentinel"
run_case exists old "" 2 "already exists; a run never writes into an existing directory"
[[ $(ls -A "$work/out/exists") == sentinel && $(cat "$work/out/exists/sentinel") == kept ]] ||
  fail "exists: the existing directory was changed"
run_case inside old "$folia/out" 2 "is inside $folia"
run_case in-site old "$work/site/out" 2 "is inside $work/site"
run_case mismatch mismatch "" 2 "has the SHA-256" "the baseline's lockfile is not known"
run_case unknown unknown "" 2 "the lockfile the baseline was built with is not known"
for name in inside in-site mismatch unknown; do
  [[ ! -e $work/$name.log ]] || fail "$name: a check ran"
done
[[ ! -e $folia/out && ! -e $work/site/out && ! -e $work/out/mismatch && ! -e $work/out/unknown ]] ||
  fail "a refused run wrote its directory"

# 17. Every output built: a part with nothing left to build passes, and an assertion that should
# have run on a built output and did not (`rel-me` on `/`) fails, rather than passing or pending.
STUB_BUILT=1 run_case built old "" 1 "F6 Document assertions: fail"
expect built 'part("F6", "`rel-me`")["status"] == "fail" and "did not run" in part("F6", "`rel-me`")["why"]'
expect built 'all(part("F6", n)["status"] == "pass" for n in ["`robots`", "`h-card`", "`stub`"])'
expect built 'part("F6", "`description`")["status"] == "pass" and part("F1", "not built yet")["status"] == "pass"'
expect built 'part("F10", "the warnings of the outputs")["status"] == "pass"'

# 18. to 23. The standalone recipes (the repository's justfile, run by the real `just` in the
# copy, with a stand-in for cargo): A0's reference for BASELINE-B by default; another baseline's
# only when configured and declared its own; otherwise refused before anything is built.
mkdir -p "$work/recipe-bin"
printf '#!/usr/bin/env bash\necho "cargo $*" >>"$CARGO_LOG"\n' >"$work/recipe-bin/cargo"
chmod +x "$work/recipe-bin/cargo"
recipe() {
  local name=$1 want=$2 status=0 line
  shift 2
  CARGO_LOG=$work/$name.cargo PATH=$work/recipe-bin:$PATH "$real_just" --justfile "$repo/justfile" \
    --working-directory "$folia" "${recipe_name:-determinism}" >"$work/$name.txt" 2>&1 || status=$?
  [[ $status -eq $want ]] || fail "$name: the recipe exited with $status; expected $want (see $work/$name.txt)"
  for line in "$@"; do
    grep -qF -- "$line" "$work/$name.txt" "$work/$name.cargo" 2>/dev/null ||
      fail "$name: lacks: $line (see $work/$name.txt, $work/$name.cargo)"
  done
  if [[ $want -ne 0 && -e $work/$name.cargo ]]; then fail "$name: refused, yet cargo ran"; fi
}
(
  unset FOLIA_REFERENCE FOLIA_REFERENCE_LABEL
  recipe recipe-a0 0 "/BASELINE-B " "--reference $a0 "
  FOLIA_REFERENCE_LABEL=other recipe recipe-label-only 2 "no reference is configured for other"
  FOLIA_REFERENCE=$work/refs/kept.txt recipe recipe-undeclared 2 "FOLIA_REFERENCE_LABEL (unset) does not declare it BASELINE-B's"
  FOLIA_REFERENCE=$work/refs/kept.txt FOLIA_REFERENCE_LABEL=other recipe recipe-declared 0 "/other " "--reference $work/refs/kept.txt "
  FOLIA_REFERENCE=$work/refs/none.txt FOLIA_REFERENCE_LABEL=other recipe recipe-not-a-file 2 "is not a file"
  FOLIA_REFERENCE_LABEL=other recipe_name=determinism-negative recipe recipe-negative 2 "no reference is configured for other"
)
! grep -qF "$a0" "$work/recipe-declared.cargo" || fail "recipe-declared: A0's reference was used"

# 24. A Folia tree with changes: every check runs and passes, and the run is no evidence.
printf 'changed\n' >>"$folia/NOTES.md"
printf 'new\n' >"$folia/scratch.txt"
run_case dirty old "" 0 "the checks built so far pass; final acceptance: not met; evidence eligible: no"
expect dirty 'd["passed"] and d["evidence_eligible"] is False and d["folia"]["before"]["state"] == "changed"'
expect dirty 'd["folia"]["before"]["changes"] == [" M NOTES.md", "?? scratch.txt"]'
expect dirty 'any("builds/pages: Folia not clean" in r for r in d["not_eligible_because"])'
grep -qF '**Not evidence**' "$work/out/dirty/evidence/accept.md" || fail "dirty: accept.md does not say it is no evidence"

echo "every case ended as it should; see $work/*.txt and $work/out/"
