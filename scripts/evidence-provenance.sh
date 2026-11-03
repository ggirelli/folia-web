#!/usr/bin/env bash
# Checks that an evidence folder follows the provenance convention (plan v12, §5.4, Q26): its
# builds were made from one committed, clean Folia revision, it keeps their records, and it is
# (or is about to be) committed on its own, as a child of that revision.
#
#   scripts/evidence-provenance.sh EVIDENCE_DIR
#
# EVIDENCE_DIR holds, for each build it reports on, `builds/<name>/build.json` and
# `builds/<name>/sha256.txt`, copied verbatim from the kept build. The check:
#
#   1. every `builds/<name>/` has both files; every record is a JSON object naming a Folia
#      commit with `changes` clean, all the same commit C, and a site commit with `changes`
#      clean or changed (with its hash), never unknown; every manifest is `<sha256>  ./<path>`
#      lines, no path twice, as many as the record's `files`;
#   2. before the evidence is committed (nothing in EVIDENCE_DIR tracked): HEAD is C and nothing
#      outside EVIDENCE_DIR is changed or untracked, so the commit to make is evidence only;
#      once committed: the first commit that touches EVIDENCE_DIR has C as its only parent and
#      changes nothing outside it; a later commit, or the working tree, changes in EVIDENCE_DIR
#      only its README.md (reports and records are never rewritten).
#
# It reads only: git runs with --no-optional-locks, so not even the index is refreshed. It prints
# each build's portable identity, then one `ok:` line (exit 0) or `FAILS:` lines (exit 1); a
# usage error exits 2. Paths in the records (`path` fields, the `status` lists) are diagnostic,
# not identity, and are not compared.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 EVIDENCE_DIR" >&2; exit 2; }
dir=${1%/}
[[ -d $dir ]] || { echo "$dir is not a directory" >&2; exit 2; }
top=$(git rev-parse --show-toplevel)
rel=$(realpath --relative-to="$top" "$dir")
case $rel in ..*) echo "$dir is outside the repository" >&2; exit 2 ;; esac
git() { command git --no-optional-locks -C "$top" "$@"; }
# Prints one FAILS line, $1, and each further argument's lines indented below it; exits 1.
fail() {
  printf 'FAILS: %s\n' "$1" >&2
  shift
  local detail
  for detail in "$@"; do printf '%s\n' "$detail" | sed 's/^/  /' >&2; done
  exit 1
}

# 1. The records.
commit=$(python3 -I - "$dir" <<'EOF'
import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
fail = []
builds = sorted(p for p in (root / "builds").glob("*") if p.is_dir())
if not builds:
    fail.append(f"{root}: no builds/<name>/ with a build.json and a sha256.txt")
commits = set()
for build in builds:
    name = build.name
    record, manifest = build / "build.json", build / "sha256.txt"
    if not record.is_file():
        fail.append(f"{name}: no build.json")
        continue
    try:
        data = json.loads(record.read_text())
    except (ValueError, UnicodeDecodeError) as error:
        fail.append(f"{name}: build.json is not JSON: {error}")
        continue
    if not isinstance(data, dict):
        fail.append(f"{name}: build.json is not an object")
        continue
    side = lambda key: data.get(key) if isinstance(data.get(key), dict) else {}
    state = lambda checkout: (checkout.get("changes") or {}).get("state") \
        if isinstance(checkout.get("changes"), dict) else None
    folia, site = side("folia"), side("site")
    if not isinstance(folia.get("commit"), str) or not folia.get("commit"):
        fail.append(f"{name}: the Folia source has no commit")
    if state(folia) != "clean":
        fail.append(f"{name}: the Folia source is not clean: {json.dumps(folia.get('changes'))}")
    commits.add(folia.get("commit"))
    if not isinstance(site.get("commit"), str) or not site.get("commit"):
        fail.append(f"{name}: the site has no commit")
    if state(site) not in ("clean", "changed"):
        fail.append(f"{name}: the site's identity is not known: {json.dumps(site.get('changes'))}")
    elif state(site) == "changed" and not re.fullmatch(r"[0-9a-f]{64}", str(site["changes"].get("sha256"))):
        fail.append(f"{name}: the site's changes have no SHA-256")
    if not manifest.is_file():
        fail.append(f"{name}: no sha256.txt")
    else:
        lines = manifest.read_text().splitlines()
        paths = [line[68:] for line in lines]
        if not lines or not all(re.fullmatch(r"[0-9a-f]{64}  \./\S.*", line) for line in lines):
            fail.append(f"{name}: sha256.txt is not a manifest of `<sha256>  ./<path>` lines")
        elif len(set(paths)) != len(paths):
            fail.append(f"{name}: sha256.txt lists a path twice")
        elif data.get("files") != len(lines):
            fail.append(f"{name}: build.json counts {data.get('files')} files, sha256.txt lists {len(lines)}")
    changes = site.get("changes") if isinstance(site.get("changes"), dict) else {}
    print(
        f"{name}: folia {folia.get('commit')} {state(folia)}; "
        f"site {site.get('commit')} {state(site)} {changes.get('sha256', '')}; "
        f"config {side('config').get('sha256')}; "
        f"templates {side('templates').get('tree_sha256')} ({side('templates').get('files')} files); "
        f"at {data.get('at')}; {data.get('rustc')}; {data.get('files')} files",
        file=sys.stderr,
    )
if len(commits) > 1:
    fail.append(f"the builds name more than one Folia commit: {sorted(map(str, commits))}")
for line in fail:
    print(f"FAILS: {line}", file=sys.stderr)
if fail:
    sys.exit(1)
print(commits.pop())
EOF
)

# 2. The commit.
if [[ -z $(git ls-files -- "$rel") ]]; then
  head=$(git rev-parse HEAD)
  [[ $head == "$commit" ]] || fail "HEAD is $head, not the builds' Folia commit $commit"
  others=$(git status --porcelain --untracked-files=all -- . ":(exclude)$rel")
  [[ -z $others ]] || fail "changes outside $rel:" "$others"
  echo "ok: $rel is ready to be committed on its own, on $commit"
  exit 0
fi

# Every commit that touches the folder, oldest first, merges included.
mapfile -t touching < <(git log --full-history --reverse --format=%H -- "$rel")
[[ ${#touching[@]} -gt 0 ]] || fail "no commit adds $rel"
added=${touching[0]}
parents=$(git rev-list --parents -n 1 "$added" | cut -s -d' ' -f2-)
[[ $parents == "$commit" ]] || fail "$added, which adds $rel, has parents '$parents', not only the builds' Folia commit $commit"
outside=$(git diff-tree --no-commit-id --name-only -r "$added" -- . ":(exclude)$rel")
[[ -z $outside ]] || fail "$added, which adds $rel, also changes:" "$outside"
for later in "${touching[@]:1}"; do
  inside=$(git diff-tree -m --no-commit-id --name-only -r "$later" -- "$rel" | sort -u |
    grep -vxF "$rel/README.md" || true)
  [[ -z $inside ]] || fail "$later rewrites the evidence, not only its README.md:" "$inside"
done
dirty=$(git status --porcelain --untracked-files=all -- "$rel" | grep -vE "^.. $rel/README\.md$" || true)
[[ -z $dirty ]] || fail "$rel has uncommitted changes besides its README.md:" "$dirty"
echo "ok: $rel was committed on its own in $added, on $commit"
