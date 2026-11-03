#!/usr/bin/env bash
# Shows that `scripts/baseline.sh` and `scripts/verify-baseline.sh` (plan v12, §5.4, A2.6) leave
# out of the site's checkout check exactly the paths of `parity/site-exclusions.txt`, and refuse
# every other change; that `verify-baseline.sh` works with an existing label without touching it,
# passes `--allow-dirty` on, and keeps every build it makes, passing or failing; and (A2.7) that a
# baseline records its Folia and site identities, its lockfile (kept byte for byte) and its pinned
# time, and refuses a missing, mismatched or incompatible lockfile, a destination inside the site
# or Folia, and a build that changes the checkout or its lockfile. Every case runs on a throwaway
# site repository under WORKDIR, with the scripts run from a throwaway, committed copy of Folia's
# (so Folia's identity is known), none of the user's git configuration applying, and a stand-in
# for Bundler and Jekyll on the PATH, so it needs neither the reference site nor its gems; each
# must exit with the status and print the line stated, and leave the site as it was.
#
#   scripts/baseline-negative.sh WORKDIR
#
# WORKDIR must not exist; it is kept, with each case's output.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 WORKDIR" >&2; exit 2; }
work=$1
[[ -e $work ]] && { echo "$work already exists" >&2; exit 1; }
repo=$(realpath "$(dirname "$0")/..")
mkdir -p "$work/bin" "$work/out"
work=$(realpath "$work")

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.org
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.org

# A committed copy of what the scripts use, so Folia's identity is clean and known.
folia=$work/folia
for file in scripts/baseline.sh scripts/verify-baseline.sh scripts/compare-builds.sh \
  parity/check_dump.rb parity/same_entries.py parity/site-exclusions.txt parity/jekyll/folia_dump.rb; do
  mkdir -p "$(dirname "$folia/$file")"
  cp -p "$repo/$file" "$folia/$file"
done
git init -q -b main "$folia"
git -C "$folia" add -A
git -C "$folia" commit -q -m folia
scripts=$folia/scripts

# The stand-in: `bundle exec jekyll build` writes one page and, when FOLIA_DUMP is set, a manifest
# that accounts for it; its arguments go to stub-args.txt beside the destination. STUB_NO_PLUGIN_TEXT
# changes the page of a build without the plugin; STUB_WRITE_SOURCE writes into the source and
# STUB_CHANGE_LOCK into the lockfile; `bundle install` fails on a lockfile holding INCOMPATIBLE.
cat >"$work/bin/bundle" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
lock=${BUNDLE_GEMFILE:-$PWD/Gemfile}.lock
case "$*" in
  -v) echo "Bundler version stand-in" ;;
  "exec ruby -e print RUBY_DESCRIPTION") printf 'ruby stand-in' ;;
  "exec ruby -e print Bundler::VERSION") printf '9.9.9' ;;
  "exec ruby -e print Bundler.default_lockfile") printf '%s' "$lock" ;;
  "exec jekyll -v") echo "jekyll stand-in" ;;
  install)
    if grep -q INCOMPATIBLE "$lock"; then echo "stand-in: the lockfile does not fit the Gemfile"; exit 1; fi
    echo "stand-in: installed" ;;
  "exec jekyll build "*)
    printf '%s\n' "$@" >"${BUNDLE_STUB_ARGS:-/dev/null}"
    shift 3
    while [[ $# -gt 0 ]]; do
      case $1 in --destination) dest=$2; shift 2 ;; --source) src=$2; shift 2 ;; *) shift ;; esac
    done
    [[ -n ${STUB_WRITE_SOURCE:-} ]] && printf 'x\n' >"$src/written-by-the-build.md"
    [[ -n ${STUB_CHANGE_LOCK:-} ]] && printf '# changed\n' >>"$lock"
    text=site
    [[ -z ${FOLIA_DUMP:-} ]] && text=${STUB_NO_PLUGIN_TEXT:-site}
    printf '<p>%s</p>\n' "$text" >"$dest/index.html"
    if [[ -n ${FOLIA_DUMP:-} ]]; then
      printf '%s\n' '{"kind":"page","collection":null,"class":"Jekyll::Page","source":"index.md","url":"/","output":"index.html","written":true,"content":null,"excerpt":null,"meta":null}' \
        >"$FOLIA_DUMP/manifest.jsonl"
    fi
    ;;
  *) echo "bundle stand-in: unexpected arguments: $*" >&2; exit 1 ;;
esac
EOF
chmod +x "$work/bin/bundle"
export PATH="$work/bin:$PATH"

cases=0
# A new site repository with one commit, whose content workflow is tracked, with a Gemfile and
# a lockfile git ignores (as filopoe's); prints its path.
site() {
  local s=$work/$1
  mkdir -p "$s/.github/workflows"
  printf 'About\n' >"$s/about.md"
  printf 'name: Content\n' >"$s/.github/workflows/content.yml"
  printf 'title: Site\n' >"$s/_config.yml"
  printf "source 'https://rubygems.org'\n" >"$s/Gemfile"
  printf 'Gemfile.lock\n' >"$s/.gitignore"
  printf 'GEM\n  specs:\n    jekyll (4.4.1)\n' >"$s/Gemfile.lock"
  git init -q -b main "$s"
  git -C "$s" add -A
  git -C "$s" commit -q -m site
  echo "$s"
}

write() { mkdir -p "$(dirname "$1/$2")" && printf '%s\n' "${3:-x}" >"$1/$2"; }

# The site's files and status, or a directory's files, to show a case changed neither.
snapshot() {
  (cd "$1" && find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0r sha256sum)
  [[ -d $1/.git ]] && git -C "$1" status --porcelain --untracked-files=all
  return 0
}

# Runs case $1 (a command, from $4 on) on site $2; requires exit status $3 and a line containing
# $LINE, and no change to the site.
run() {
  local name=$1 s=$2 want=$3 status=0 before after
  shift 3
  before=$(snapshot "$s")
  "$@" >"$work/$name.txt" 2>&1 || status=$?
  after=$(snapshot "$s")
  cases=$((cases + 1))
  [[ $status -eq $want ]] || { echo "$name: exit $status, expected $want (see $work/$name.txt)" >&2; exit 1; }
  grep -qF -- "$LINE" "$work/$name.txt" || { echo "$name: no line with: $LINE (see $work/$name.txt)" >&2; exit 1; }
  [[ $before == "$after" ]] || { echo "$name: the site changed" >&2; exit 1; }
}

fail() { echo "$1" >&2; exit 1; }
json() { ruby -rjson -e 'puts JSON.parse(File.read(ARGV[0])).dig(*ARGV[1..]).to_json' "$@"; }

# --- baseline.sh: what counts as a change ------------------------------------------------------

s=$(site clean)
LINE="Built 1 files." run clean "$s" 0 "$scripts/baseline.sh" "$s" "$work/out/clean"
[[ $(json "$work/out/clean/BASELINE.json" site excluded) == '["_folia",".github/workflows/content.yml"]' ]] ||
  fail "clean: BASELINE.json does not record the shared list"

# The approved paths, untracked or changed once tracked, do not make the tree dirty.
s=$(site excluded)
write "$s" _folia/templates/page.html
write "$s" .github/workflows/content.yml "name: Changed"
LINE="Built 1 files." run excluded "$s" 0 "$scripts/baseline.sh" "$s" "$work/out/excluded"
[[ $(json "$work/out/excluded/BASELINE.json" site dirty) == false ]] || fail "excluded: recorded as dirty"
[[ ! -e $work/out/excluded/site.diff ]] || fail "excluded: wrote a site.diff"

# Anything else does, alongside the approved paths, tracked or untracked: other workflows, other
# files of that name, look-alikes of either path.
n=0
for path in about.md new.md .github/workflows/jekyll.yml .github/workflows/content.yml.bak \
  .github/content.yml content.yml _foliage/x.md _folia.md docs/_folia/x.md; do
  n=$((n + 1))
  s=$(site "dirty-$n")
  write "$s" _folia/templates/page.html
  write "$s" .github/workflows/content.yml "name: Changed"
  write "$s" "$path" changed
  LINE="has uncommitted changes" run "dirty-$n" "$s" 1 "$scripts/baseline.sh" "$s" "$work/out/dirty-$n"
  grep -qF -- "$path" "$work/dirty-$n.txt" || fail "dirty-$n: $path is not named"
  [[ ! -e $work/out/dirty-$n ]] || fail "dirty-$n: wrote $work/out/dirty-$n"
done

# With --allow-dirty, the changes are recorded, and the approved paths are not among them.
s=$(site allow-dirty)
write "$s" _folia/templates/page.html
write "$s" .github/workflows/content.yml "name: Changed"
write "$s" about.md Edited
LINE="Built 1 files." run allow-dirty "$s" 0 "$scripts/baseline.sh" --allow-dirty "$s" "$work/out/allow-dirty"
[[ $(json "$work/out/allow-dirty/BASELINE.json" site status) == '[" M about.md"]' ]] ||
  fail "allow-dirty: the recorded status is not about.md alone"
grep -q '^diff --git a/about.md' "$work/out/allow-dirty/site.diff" || fail "allow-dirty: about.md is not in site.diff"
! grep -q 'content.yml\|_folia' "$work/out/allow-dirty/site.diff" || fail "allow-dirty: an approved path is in site.diff"

# --allow-dirty still refuses untracked files outside the approved paths, which site.diff cannot
# record and the parity tools refuse; staged, they are in site.diff and the build goes ahead.
n=0
for path in new.md _foliage/x.md .github/workflows/jekyll.yml; do
  n=$((n + 1))
  s=$(site "allow-dirty-untracked-$n")
  write "$s" _folia/templates/page.html
  write "$s" about.md Edited
  write "$s" "$path" new
  LINE="has untracked files, which site.diff cannot record" \
    run "allow-dirty-untracked-$n" "$s" 1 "$scripts/baseline.sh" --allow-dirty "$s" "$work/out/allow-dirty-untracked-$n"
  grep -qF -- "?? $path" "$work/allow-dirty-untracked-$n.txt" || fail "allow-dirty-untracked-$n: $path is not named"
  ! grep -q "^?? _folia/" "$work/allow-dirty-untracked-$n.txt" || fail "allow-dirty-untracked-$n: _folia is named"
  [[ ! -e $work/out/allow-dirty-untracked-$n ]] || fail "allow-dirty-untracked-$n: wrote a baseline"
done
s=$(site allow-dirty-staged)
write "$s" _folia/templates/page.html
write "$s" new.md new
git -C "$s" add new.md
LINE="Built 1 files." run allow-dirty-staged "$s" 0 "$scripts/baseline.sh" --allow-dirty "$s" "$work/out/allow-dirty-staged"
grep -q '^diff --git a/new.md' "$work/out/allow-dirty-staged/site.diff" || fail "allow-dirty-staged: new.md is not in site.diff"

# An existing baseline is refused, and left as it is.
s=$(site existing)
kept=$(snapshot "$work/out/clean")
LINE="already exists; pick a new label" run existing "$s" 1 "$scripts/baseline.sh" "$s" "$work/out/clean"
[[ $(snapshot "$work/out/clean") == "$kept" ]] || fail "existing: the baseline changed"

# --- verify-baseline.sh ------------------------------------------------------------------------

# A new label becomes the baseline; the other builds are kept beside it.
s=$(site verify)
LINE="All checks passed on $work/out/new" run verify-new "$s" 0 "$scripts/verify-baseline.sh" "$s" "$work/out" new
[[ -f $work/out/new/BASELINE.json ]] || fail "verify-new: no baseline"
mapfile -t dirs < <(find "$work/out" -maxdepth 1 -name 'new.verify-*')
[[ ${#dirs[@]} -eq 1 && -d ${dirs[0]}/run2/site && -d ${dirs[0]}/no-plugin/site && ! -e ${dirs[0]}/run1 ]] ||
  fail "verify-new: the verification builds are not kept"
grep -qF "No unexpected differences." "${dirs[0]}/check1.txt" || fail "verify-new: no check1.txt"

# The documented command with an existing label: it is left as it is, and run twice, each run
# builds into a directory of its own, with nothing removed.
kept=$(snapshot "$work/out/new")
for i in 1 2; do
  LINE="exists: it is left as it is" run "verify-existing-$i" "$s" 0 "$scripts/verify-baseline.sh" "$s" "$work/out" new
done
[[ $(snapshot "$work/out/new") == "$kept" ]] || fail "verify-existing: the baseline changed"
mapfile -t dirs < <(find "$work/out" -maxdepth 1 -name 'new.verify-*')
[[ ${#dirs[@]} -eq 3 ]] || fail "verify-existing: expected 3 verification directories, found ${#dirs[@]}"
for d in "${dirs[@]}"; do [[ -d $d/run2/site && -d $d/no-plugin/site ]] || fail "verify-existing: $d lost a build"; done

# A dirty site is refused without --allow-dirty, which verify-baseline.sh passes to every build.
s=$(site verify-dirty)
write "$s" _folia/templates/page.html
write "$s" about.md Edited
LINE="has uncommitted changes" run verify-dirty "$s" 1 "$scripts/verify-baseline.sh" "$s" "$work/out" dirty
[[ ! -e $work/out/dirty ]] || fail "verify-dirty: wrote a baseline"
LINE="All checks passed" run verify-allow-dirty "$s" 0 "$scripts/verify-baseline.sh" --allow-dirty "$s" "$work/out" dirty
d=$(find "$work/out" -maxdepth 1 -name 'dirty.verify-*' -exec test -d '{}/run2' ';' -print)
for b in "$work/out/dirty" "$d/run2" "$d/no-plugin"; do
  [[ -f $b/site.diff ]] || fail "verify-allow-dirty: $b has no site.diff"
done
write "$s" new.md new
LINE="has untracked files" run verify-allow-dirty-untracked "$s" 1 "$scripts/verify-baseline.sh" --allow-dirty "$s" "$work/out" dirty-untracked
[[ ! -e $work/out/dirty-untracked ]] || fail "verify-allow-dirty-untracked: wrote a baseline"
rm "$s/new.md"
LINE="usage:" run verify-flag "$s" 2 "$scripts/verify-baseline.sh" --force "$s" "$work/out" dirty

# A failing check keeps every build and the comparison that failed.
s=$(site verify-fails)
export STUB_NO_PLUGIN_TEXT=changed
LINE="Verification builds and reports kept in" run verify-fails "$s" 1 "$scripts/verify-baseline.sh" "$s" "$work/out" fails
unset STUB_NO_PLUGIN_TEXT
d=$(find "$work/out" -maxdepth 1 -name 'fails.verify-*')
[[ -d $work/out/fails/site && -d $d/run2/site && -d $d/no-plugin/site ]] || fail "verify-fails: a build was removed"
grep -qF "changed site/index.html" "$d/check2.txt" || fail "verify-fails: check2.txt does not name the difference"

# --- baseline.sh: provenance, lockfiles, pinned time (A2.7) ------------------------------------

sha() { sha256sum "$1" | cut -d' ' -f1; }
want() { [[ $(json "$1" "${@:2:$#-2}") == "${!#}" ]] || fail "$1: ${*:2:$#-2} is $(json "$1" "${@:2:$#-2}"), not ${!#}"; }

# The record names Folia's commit and identity, the site's, the lockfile kept byte for byte, the
# Bundler loaded, and a manifest of what was built.
r=$work/out/clean/BASELINE.json
want "$r" folia commit "\"$(git -C "$folia" rev-parse HEAD)\""
want "$r" folia changes '{"state":"clean"}'
want "$r" site changes '{"state":"clean"}'
want "$r" lockfile sha256 "\"$(sha "$work/clean/Gemfile.lock")\""
want "$r" lockfile installed false
want "$r" tools bundler '"9.9.9"'
want "$r" result ok true
want "$r" result files 2
want "$r" build time null
cmp -s "$work/clean/Gemfile.lock" "$work/out/clean/Gemfile.lock" || fail "clean: the lockfile was not kept byte for byte"
grep -qx "[0-9a-f]\{64\}  ./site/index.html" "$work/out/clean/sha256.txt" || fail "clean: sha256.txt does not list site/index.html"
grep -qx "[0-9a-f]\{64\}  ./dump/manifest.jsonl" "$work/out/clean/sha256.txt" || fail "clean: sha256.txt does not list the dump"

# A site change made with --allow-dirty is identified by its diff.
want "$work/out/allow-dirty/BASELINE.json" site changes "{\"state\":\"changed\",\"sha256\":\"$(sha "$work/out/allow-dirty/site.diff")\"}"

# Folia's own changes are recorded, never hidden.
s=$(site folia-dirty)
write "$folia" scratch.txt
LINE="Built 1 files." run folia-dirty "$s" 0 "$scripts/baseline.sh" "$s" "$work/out/folia-dirty"
rm "$folia/scratch.txt"
want "$work/out/folia-dirty/BASELINE.json" folia changes '{"state":"changed"}'
want "$work/out/folia-dirty/BASELINE.json" folia status '["?? scratch.txt"]'

# No destination inside the site, or inside Folia, whose identity the record names.
s=$(site places)
LINE="is inside the site" run inside-site "$s" 1 "$scripts/baseline.sh" "$s" "$s/out"
[[ ! -e $s/out ]] || fail "inside-site: wrote into the site"
LINE="is inside the Folia repository" run inside-folia "$s" 1 "$scripts/baseline.sh" "$s" "$folia/out"
[[ ! -e $folia/out ]] || fail "inside-folia: wrote into Folia"

# A build whose bundle has no lockfile cannot record its gems: refused, its record kept.
s=$(site no-lock)
rm "$s/Gemfile.lock"
LINE="No lockfile for the build's bundle" run no-lock "$s" 1 "$scripts/baseline.sh" "$s" "$work/out/no-lock"
want "$work/out/no-lock/BASELINE.json" result '{"ok":false,"stage":"lockfile","files":null}'

# --lockfile: exactly the lockfile given, by its SHA-256, installed and kept; anything else refused
# before anything is written.
s=$(site lockfile)
printf 'GEM\n  specs:\n    jekyll (4.4.0)\n' >"$work/given.lock"
given=$(sha "$work/given.lock")
LINE="go together" run lock-alone "$s" 1 "$scripts/baseline.sh" --lockfile "$work/given.lock" "$s" "$work/out/lock-alone"
LINE="does not exist" run lock-missing "$s" 1 "$scripts/baseline.sh" --lockfile "$work/none.lock" --lockfile-sha256 "$given" "$s" "$work/out/lock-missing"
LINE="not the expected $(printf '0%.0s' {1..64})" run lock-sha256 "$s" 1 \
  "$scripts/baseline.sh" --lockfile "$work/given.lock" --lockfile-sha256 "$(printf '0%.0s' {1..64})" "$s" "$work/out/lock-sha256"
for name in lock-alone lock-missing lock-sha256; do [[ ! -e $work/out/$name ]] || fail "$name: wrote $work/out/$name"; done
LINE="Built 1 files." run lock-given "$s" 0 "$scripts/baseline.sh" --lockfile "$work/given.lock" --lockfile-sha256 "$given" "$s" "$work/out/lock-given"
r=$work/out/lock-given/BASELINE.json
want "$r" lockfile sha256 "\"$given\""
want "$r" lockfile installed true
want "$r" lockfile source "\"$work/given.lock\""
cmp -s "$work/given.lock" "$work/out/lock-given/Gemfile.lock" || fail "lock-given: the lockfile was not kept byte for byte"
grep -q "stand-in: installed" "$work/out/lock-given/bundle-install.log" || fail "lock-given: no bundle-install.log"
# One that does not fit the Gemfile is refused by the frozen install; the record says where.
printf 'INCOMPATIBLE\n' >"$work/incompatible.lock"
LINE="The lockfile does not install for the site's Gemfile" run lock-incompatible "$s" 1 \
  "$scripts/baseline.sh" --lockfile "$work/incompatible.lock" --lockfile-sha256 "$(sha "$work/incompatible.lock")" "$s" "$work/out/lock-incompatible"
want "$work/out/lock-incompatible/BASELINE.json" result stage '"bundle install"'
grep -q "does not fit the Gemfile" "$work/out/lock-incompatible/bundle-install.log" || fail "lock-incompatible: the log was not kept"

# --at pins Jekyll's time with a configuration after the site's, and records it.
s=$(site pinned)
LINE="Built 1 files." BUNDLE_STUB_ARGS=$work/pinned-args.txt \
  run pinned "$s" 0 "$scripts/baseline.sh" --at 2026-10-25T14:43:51Z "$s" "$work/out/pinned"
want "$work/out/pinned/BASELINE.json" build time '"2026-10-25T14:43:51Z"'
[[ $(cat "$work/out/pinned/jekyll-time.yml") == "time: 2026-10-25T14:43:51Z" ]] || fail "pinned: jekyll-time.yml"
grep -qxF "$s/_config.yml,$work/out/pinned/jekyll-time.yml" "$work/pinned-args.txt" || fail "pinned: Jekyll was not given both configurations"
LINE="is not YYYY-MM-DDTHH:MM:SSZ" run pinned-bad "$s" 1 "$scripts/baseline.sh" --at 2026-10-25 "$s" "$work/out/pinned-bad"

# A build that changes the checkout, or its lockfile, fails; the record says where. (These change
# the site on purpose, so they are run without the snapshot check.)
s=$(site writes)
status=0; STUB_WRITE_SOURCE=1 "$scripts/baseline.sh" "$s" "$work/out/writes" >"$work/writes.txt" 2>&1 || status=$?
cases=$((cases + 1))
if [[ $status -ne 1 ]] || ! grep -qF "The build changed the site's working tree." "$work/writes.txt"; then
  fail "writes: not refused (see $work/writes.txt)"
fi
want "$work/out/writes/BASELINE.json" result stage '"checkout"'
s=$(site changes-lock)
status=0; STUB_CHANGE_LOCK=1 "$scripts/baseline.sh" "$s" "$work/out/changes-lock" >"$work/changes-lock.txt" 2>&1 || status=$?
cases=$((cases + 1))
if [[ $status -ne 1 ]] || ! grep -qF "The build changed its lockfile" "$work/changes-lock.txt"; then
  fail "changes-lock: not refused (see $work/changes-lock.txt)"
fi
want "$work/out/changes-lock/BASELINE.json" result stage '"lockfile"'

# --empty-out takes only an empty directory.
s=$(site empty-out)
mkdir -p "$work/out/empty" "$work/out/not-empty"
write "$work/out" not-empty/kept.txt
kept=$(snapshot "$work/out/not-empty")
LINE="is not an empty directory" run not-empty "$s" 1 "$scripts/baseline.sh" --empty-out "$s" "$work/out/not-empty"
[[ $(snapshot "$work/out/not-empty") == "$kept" ]] || fail "not-empty: the directory changed"
LINE="Built 1 files." run empty-out "$s" 0 "$scripts/baseline.sh" --empty-out "$s" "$work/out/empty"

[[ -z $(git -C "$folia" status --porcelain --untracked-files=all) ]] || fail "the cases left Folia's copy changed"
echo "ok: $cases cases; outputs kept in $work"
