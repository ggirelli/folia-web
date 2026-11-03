#!/usr/bin/env bash
# Shows that `scripts/evidence-provenance.sh` (plan v12, §5.4, Q26) accepts evidence made by the
# convention and refuses each way of breaking it, and that it changes nothing: every case runs in
# a throwaway repository under WORKDIR (none of the user's git configuration applies), must exit
# with the status and print the line stated, and must leave the repository byte for byte as it
# was, `.git` included.
#
#   scripts/evidence-provenance-negative.sh WORKDIR
#
# WORKDIR must not exist.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 WORKDIR" >&2; exit 2; }
work=$1
[[ -e $work ]] && { echo "$work already exists" >&2; exit 1; }
checker=$(realpath "$(dirname "$0")/evidence-provenance.sh")
mkdir -p "$work"
work=$(realpath "$work")

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.org
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.org
export GIT_AUTHOR_DATE=2026-11-03T12:00:00Z GIT_COMMITTER_DATE=2026-11-03T12:00:00Z

cases=0
# A repository whose commit C is the source the builds name; prints its path.
repo() {
  local r=$work/$1
  git init -q -b main "$r"
  printf 'source\n' >"$r/src.txt"
  git -C "$r" add src.txt
  git -C "$r" commit -q -m source
  echo "$r"
}

# Writes builds/$3 into evidence folder $2 of repository $1, naming Folia commit $4 (default:
# HEAD) with Folia changes $5 (default clean) and site changes $6 (default changed).
build() {
  local r=$1 ev=$2 name=$3
  local c=${4:-$(git -C "$r" rev-parse HEAD)}
  local folia=${5:-'{"state":"clean"}'}
  local site=${6:-'{"state":"changed","sha256":"'"$(printf '%064d' 1)"'"}'}
  mkdir -p "$r/$ev/builds/$name"
  printf '%064d  ./a.html\n%064d  ./b.txt\n' 2 3 >"$r/$ev/builds/$name/sha256.txt"
  cat >"$r/$ev/builds/$name/build.json" <<EOF
{"at":"2026-10-25T14:43:51Z","site":{"path":"/x/site","commit":"SITE-COMMIT-B","status":[],"changes":$site},
 "config":{"path":"c.toml","sha256":"$(printf '%064d' 4)"},
 "templates":{"path":"/x/t","files":30,"tree_sha256":"$(printf '%064d' 5)"},
 "folia":{"path":"/x/folia","commit":"$c","status":[],"changes":$folia},
 "rustc":"rustc 1.99.1","result":{"ok":true},"files":2}
EOF
}

# Writes baselines/$3 (a Jekyll baseline's records, A2.7) into evidence folder $2 of repository
# $1, naming Folia commit $4 (default: HEAD) with Folia changes $5 (default clean), site changes
# $6 (default clean) and result $7 (default finished, 2 files).
baseline() {
  local r=$1 ev=$2 name=$3
  local c=${4:-$(git -C "$r" rev-parse HEAD)}
  local folia=${5:-'{"state":"clean"}'}
  local site=${6:-'{"state":"clean"}'}
  local result=${7:-'{"ok":true,"files":2,"site_files":1}'}
  local dir=$r/$ev/baselines/$name
  mkdir -p "$dir"
  printf 'GEM\n  specs:\n    jekyll (4.4.1)\n' >"$dir/Gemfile.lock"
  printf '%064d  ./dump/manifest.jsonl\n%064d  ./site/a.html\n' 7 8 >"$dir/sha256.txt"
  cat >"$dir/BASELINE.json" <<EOF
{"created_at":"2026-11-03T12:00:00Z",
 "site":{"path":"/x/source","commit":"SITE-COMMIT-B","dirty":false,"status":[],"changes":$site},
 "folia":{"path":"/x/folia","commit":"$c","status":[],"changes":$folia},
 "lockfile":{"file":"Gemfile.lock","sha256":"$(sha256sum "$dir/Gemfile.lock" | cut -d' ' -f1)"},
 "tools":{"ruby":"ruby 3.4.5","bundler":"2.6.9"},
 "environment":{"container":{"image":"ruby:3.4.5@sha256:$(printf '%064d' 9)","platform":"linux/amd64"}},
 "build":{"time":"2026-10-25T14:43:51Z"},"result":$result}
EOF
}

snapshot() { (cd "$1" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum); }

# Runs the checker on folder $3 of repository $2 for case $1; requires exit status $4 and a line
# containing $5, and no change to the repository. A folder `-` runs it with no argument.
check() {
  local name=$1 r=$2 ev=$3 want=$4 line=$5 status=0 before after args=()
  [[ $ev == - ]] || args=("$ev")
  before=$(snapshot "$r")
  (cd "$r" && "$checker" "${args[@]}") >"$work/$name.txt" 2>&1 || status=$?
  after=$(snapshot "$r")
  cases=$((cases + 1))
  [[ $status -eq $want ]] || { echo "$name: exit $status, expected $want (see $work/$name.txt)" >&2; exit 1; }
  grep -qF -- "$line" "$work/$name.txt" || { echo "$name: no line with: $line (see $work/$name.txt)" >&2; exit 1; }
  if grep -q Traceback "$work/$name.txt"; then echo "$name: a traceback" >&2; exit 1; fi
  [[ $before == "$after" ]] || { echo "$name: the checker changed the repository" >&2; exit 1; }
}

commit_all() { git -C "$1" add -A && git -C "$1" commit -q -m "$2"; }

# Before the evidence commit.
r=$(repo ready); c=$(git -C "$r" rev-parse HEAD)
build "$r" ev one; build "$r" ev two
check ready "$r" ev 0 "ok: ev is ready to be committed on its own, on $c"
check ready-identity "$r" ev 0 "one: folia $c clean; site SITE-COMMIT-B changed"
check usage "$r" - 2 "usage:"
check not-a-dir "$r" nowhere 2 "nowhere is not a directory"

r=$(repo untracked); build "$r" ev one; printf 'x\n' >"$r/new.txt"
check untracked-outside "$r" ev 1 "FAILS: changes outside ev:"
r=$(repo modified); build "$r" ev one; printf 'changed\n' >"$r/src.txt"
check modified-outside "$r" ev 1 " M src.txt"
grep -qF 'FAILS: changes outside ev:' "$work/modified-outside.txt"
r=$(repo staged); build "$r" ev one; printf 'changed\n' >"$r/src.txt"; git -C "$r" add src.txt
check staged-outside "$r" ev 1 "M  src.txt"
r=$(repo moved); build "$r" ev one; git -C "$r" commit -q --allow-empty -m later
check head-moved "$r" ev 1 "not the builds' Folia commit"

r=$(repo identities)
build "$r" dirty one "" '{"state":"changed","sha256":"'"$(printf '%064d' 6)"'"}'
check folia-dirty "$r" dirty 1 "FAILS: one: the Folia source is not clean"
build "$r" unknown one "" '{"state":"unknown","error":"git failed"}'
check folia-unknown "$r" unknown 1 "FAILS: one: the Folia source is not clean"
build "$r" nocommit one "null"
sed -i 's/"commit":"null"/"commit":null/' "$r/nocommit/builds/one/build.json"
check folia-no-commit "$r" nocommit 1 "FAILS: one: the Folia source has no commit"
build "$r" site one "" "" '{"state":"unknown","error":"unreadable"}'
check site-unknown "$r" site 1 "FAILS: one: the site's identity is not known"
build "$r" nohash one "" "" '{"state":"changed"}'
check site-no-hash "$r" nohash 1 "FAILS: one: the site's changes have no SHA-256"
build "$r" two one; build "$r" two other deadbeef
check two-commits "$r" two 1 "FAILS: the builds and baselines name more than one Folia commit"

r=$(repo files)
mkdir -p "$r/empty/builds"
check no-builds "$r" empty 1 "FAILS: empty: no builds/<name>/"
build "$r" norecord one; rm "$r/norecord/builds/one/build.json"
check no-record "$r" norecord 1 "FAILS: one: no build.json"
build "$r" nomanifest one; rm "$r/nomanifest/builds/one/sha256.txt"
check no-manifest "$r" nomanifest 1 "FAILS: one: no sha256.txt"
build "$r" notjson one; printf '{"folia": ' >"$r/notjson/builds/one/build.json"
check not-json "$r" notjson 1 "FAILS: one: build.json is not JSON"
build "$r" notobject one; printf '[]\n' >"$r/notobject/builds/one/build.json"
check not-object "$r" notobject 1 "FAILS: one: build.json is not an object"
build "$r" badmanifest one; printf 'abc  a.html\n' >"$r/badmanifest/builds/one/sha256.txt"
check bad-manifest "$r" badmanifest 1 "FAILS: one: sha256.txt is not a manifest"
build "$r" twice one; printf '%064d  ./a.html\n%064d  ./a.html\n' 2 3 >"$r/twice/builds/one/sha256.txt"
check manifest-twice "$r" twice 1 "FAILS: one: sha256.txt lists a path twice"
build "$r" count one; printf '%064d  ./a.html\n' 2 >"$r/count/builds/one/sha256.txt"
check file-count "$r" count 1 "FAILS: one: build.json counts 2 files, sha256.txt lists 1"

# Jekyll baselines (A2.7): their records, alone or with builds, by the same convention.
r=$(repo baseline-only); c=$(git -C "$r" rev-parse HEAD)
baseline "$r" only ruby
check baseline-ready "$r" only 0 "ok: only is ready to be committed on its own, on $c"
check baseline-identity "$r" only 0 "baselines/ruby: folia $c clean; site SITE-COMMIT-B clean"
r=$(repo baselines)
r2=$(repo build-and-baseline); c2=$(git -C "$r2" rev-parse HEAD)
build "$r2" both one; baseline "$r2" both ruby
check build-and-baseline "$r2" both 0 "ok: both is ready to be committed on its own, on $c2"
baseline "$r" bdirty ruby "" '{"state":"changed"}'
check baseline-folia-dirty "$r" bdirty 1 "FAILS: baselines/ruby: the Folia source is not clean"
build "$r" bmixed one; baseline "$r" bmixed ruby deadbeef
check baseline-other-commit "$r" bmixed 1 "FAILS: the builds and baselines name more than one Folia commit"
baseline "$r" bnolock ruby; rm "$r/bnolock/baselines/ruby/Gemfile.lock"
check baseline-no-lockfile "$r" bnolock 1 "FAILS: baselines/ruby: no Gemfile.lock"
baseline "$r" block ruby; printf '# changed\n' >>"$r/block/baselines/ruby/Gemfile.lock"
check baseline-lockfile-changed "$r" block 1 "FAILS: baselines/ruby: Gemfile.lock has the SHA-256"
baseline "$r" bunfinished ruby "" "" "" null
check baseline-unfinished "$r" bunfinished 1 "FAILS: baselines/ruby: the build did not finish"
baseline "$r" bunknown ruby "" "" '{"state":"unknown","error":"git failed"}'
check baseline-site-unknown "$r" bunknown 1 "FAILS: baselines/ruby: the site's identity is not known"
diffsha=$(printf 'diff\n' | sha256sum | cut -d' ' -f1)
baseline "$r" bnodiff ruby "" "" '{"state":"changed","sha256":"'"$diffsha"'"}'
check baseline-no-diff "$r" bnodiff 1 "FAILS: baselines/ruby: the site was changed, and its site.diff is not kept"
baseline "$r" bdiff ruby "" "" '{"state":"changed","sha256":"'"$diffsha"'"}'
printf 'other\n' >"$r/bdiff/baselines/ruby/site.diff"
check baseline-wrong-diff "$r" bdiff 1 "FAILS: baselines/ruby: site.diff is not the diff the record names"
r2=$(repo baseline-diff)
baseline "$r2" bdiff ruby "" "" '{"state":"changed","sha256":"'"$diffsha"'"}'
printf 'diff\n' >"$r2/bdiff/baselines/ruby/site.diff"
check baseline-diff "$r2" bdiff 0 "ok: bdiff is ready to be committed on its own"
baseline "$r" bcount ruby "" "" "" '{"ok":true,"files":3}'
check baseline-file-count "$r" bcount 1 "FAILS: baselines/ruby: the record counts 3 files, sha256.txt lists 2"
r=$(repo baseline-committed); c=$(git -C "$r" rev-parse HEAD)
baseline "$r" ev ruby; commit_all "$r" evidence; e=$(git -C "$r" rev-parse HEAD)
check baseline-committed "$r" ev 0 "ok: ev was committed on its own in $e, on $c"

# After the evidence commit.
r=$(repo committed); c=$(git -C "$r" rev-parse HEAD)
build "$r" ev one; commit_all "$r" evidence; e=$(git -C "$r" rev-parse HEAD)
check committed "$r" ev 0 "ok: ev was committed on its own in $e, on $c"
printf 'note\n' >"$r/ev/README.md"; commit_all "$r" readme
check readme-corrected "$r" ev 0 "ok: ev was committed on its own in $e, on $c"
printf 'note, corrected\n' >"$r/ev/README.md"
check readme-uncommitted "$r" ev 0 "ok: ev was committed on its own"
git -C "$r" checkout -q -- ev/README.md
printf '[]\n' >"$r/ev/builds/one/extra.json"
check record-uncommitted "$r" ev 1 "FAILS: ev has uncommitted changes besides its README.md:"
rm "$r/ev/builds/one/extra.json"
printf '%064d  ./a.html\n%064d  ./c.txt\n' 2 3 >"$r/ev/builds/one/sha256.txt"; commit_all "$r" rewrite
check record-rewritten "$r" ev 1 "rewrites the evidence, not only its README.md:"

r=$(repo mixed); build "$r" ev one; printf 'changed\n' >"$r/src.txt"; commit_all "$r" mixed
check source-in-evidence-commit "$r" ev 1 "which adds ev, also changes:"

r=$(repo parent); build "$r" ev one; git -C "$r" commit -q --allow-empty -m between; commit_all "$r" evidence
check wrong-parent "$r" ev 1 "not only the builds' Folia commit"

r=$(repo second); c=$(git -C "$r" rev-parse HEAD)
build "$r" ev one; commit_all "$r" evidence; build "$r" ev2 one "$c"; commit_all "$r" "more evidence"
check second-folder "$r" ev2 1 "not only the builds' Folia commit"

r=$(repo merge); c=$(git -C "$r" rev-parse HEAD)
git -C "$r" checkout -q -b side; printf 'side\n' >"$r/side.txt"; commit_all "$r" side
git -C "$r" checkout -q main
git -C "$r" merge -q --no-ff --no-commit side >/dev/null 2>&1
build "$r" ev one "$c"; git -C "$r" add ev; git -C "$r" commit -q -m "merge with evidence"
check merge-commit "$r" ev 1 "has parents"

echo "every case ended as it should ($cases cases); see $work/*.txt"
