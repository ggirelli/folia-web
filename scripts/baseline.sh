#!/usr/bin/env bash
# Builds a production Jekyll baseline of the reference site (plan v2, §10 spike 0).
#
#   scripts/baseline.sh [--allow-dirty] [--no-dump] SITE OUT
#
# OUT must not exist. It receives:
#   site/           the production build (JEKYLL_ENV=production)
#   dump/           the folia_dump.rb dumps (absent with --no-dump)
#   BASELINE.json   site and submodule commits, tool versions, flags
#   site.diff       with --allow-dirty on a dirty tree: `git diff --binary HEAD`,
#                   which the parity tools check the working tree against
#   build.log       Jekyll's output
#
# The paths of parity/site-exclusions.txt, which the parity tools leave out of their checkout
# check, are left out here too: a change there, tracked or untracked, does not make the tree
# dirty and is not in site.diff. Any other change, untracked files included, does.
#
# --allow-dirty builds uncommitted changes to tracked (or staged) files, such as a source fix
# not yet committed, and records them in site.diff. It still refuses untracked files: site.diff
# cannot record them, and the parity tools refuse a checkout that has any, so the baseline could
# not be used.
#
# --no-dump builds without loading the plugin at all, to check that loading
# it changes nothing.
set -euo pipefail

usage() {
  echo "usage: $0 [--allow-dirty] [--no-dump] SITE OUT" >&2
  exit 2
}

allow_dirty=false
dump=true
while [[ $# -gt 0 && $1 == --* ]]; do
  case $1 in
    --allow-dirty) allow_dirty=true ;;
    --no-dump) dump=false ;;
    *) usage ;;
  esac
  shift
done
[[ $# -eq 2 ]] || usage

folia=$(cd "$(dirname "$0")/.." && pwd)
site=$(cd "$1" && pwd)
out=$2

[[ -e $out ]] && { echo "$out already exists; pick a new label." >&2; exit 1; }
git -C "$site" rev-parse --git-dir >/dev/null

# The approved paths, and the pathspec that leaves them out of the site's status and diff.
mapfile -t excluded < <(sed -e 's/#.*//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e '/^$/d' \
  "$folia/parity/site-exclusions.txt")
counted=(-- .)
for path in "${excluded[@]}"; do counted+=(":(exclude,literal)$path"); done

# What counts as a change, and the whole tree, excluded paths and untracked files included, to
# check that the build leaves it as it found it.
status_before=$(git -C "$site" status --porcelain --untracked-files=all "${counted[@]}")
tree_before=$(git -C "$site" status --porcelain --untracked-files=all)
if [[ -n $status_before ]] && ! $allow_dirty; then
  echo "$site has uncommitted changes (use --allow-dirty to build them anyway):" >&2
  echo "$status_before" >&2
  exit 1
fi
untracked=$(grep '^?? ' <<<"$status_before" || true)
if [[ -n $untracked ]]; then
  echo "$site has untracked files, which site.diff cannot record (commit, stage or remove them):" >&2
  echo "$untracked" >&2
  exit 1
fi

mkdir -p "$out/site"
out=$(cd "$out" && pwd)
if [[ -n $status_before ]]; then
  git -C "$site" diff --binary HEAD "${counted[@]}" >"$out/site.diff"
fi

# --- Record what is being built --------------------------------------------

record() {
  SITE="$site" FOLIA="$folia" OUT="$out" DUMP="$dump" STATUS="$status_before" \
    EXCLUDED="$(printf '%s\n' "${excluded[@]}")" \
    ruby -rjson -rdigest -rtime -e '
      def sh(*cmd, dir: ENV["SITE"]) = IO.popen(cmd, chdir: dir, err: File::NULL, &:read).strip
      def sha256(path) = File.file?(path) ? Digest::SHA256.file(path).hexdigest : nil

      site = ENV["SITE"]
      declared = sh("git", "config", "--file", ".gitmodules", "--get-regexp", "path").lines.map { _1.split[1] }
      record = {
        "created_at" => Time.now.utc.iso8601,
        "site" => {
          "path" => site,
          "commit" => sh("git", "rev-parse", "HEAD"),
          "dirty" => !ENV["STATUS"].empty?,
          "status" => ENV["STATUS"].lines.map(&:chomp),
          "diff" => ENV["STATUS"].empty? ? nil : "site.diff",
          "excluded" => ENV["EXCLUDED"].lines.map(&:chomp).reject(&:empty?),
          "submodules" => sh("git", "submodule", "status", "--recursive").lines.map(&:strip),
          "submodules_declared" => declared,
          "gemfile_lock_sha256" => sha256(File.join(site, "Gemfile.lock")),
        },
        "folia" => {
          "commit" => sh("git", "rev-parse", "HEAD", dir: ENV["FOLIA"]),
          "plugin_sha256" => ENV["DUMP"] == "true" ? sha256(File.join(ENV["FOLIA"], "parity/jekyll/folia_dump.rb")) : nil,
        },
        "tools" => {
          "ruby" => sh("bundle", "exec", "ruby", "-v"),
          "bundler" => sh("bundle", "-v"),
          "jekyll" => sh("bundle", "exec", "jekyll", "-v"),
          "git" => sh("git", "--version"),
        },
        "build" => {
          "jekyll_env" => "production",
          "dump" => ENV["DUMP"] == "true",
          "tz" => ENV["TZ"],
          "lang" => ENV["LANG"],
        },
      }
      File.write(File.join(ENV["OUT"], "BASELINE.json"), JSON.pretty_generate(record) + "\n")
    '
}

record

# --- Build -------------------------------------------------------------------

vars=(JEKYLL_ENV=production)
args=(build --source "$site" --destination "$out/site" --disable-disk-cache)
if $dump; then
  mkdir "$out/dump"
  vars+=("FOLIA_DUMP=$out/dump")
  args+=(--plugins "$site/_plugins,$folia/parity/jekyll")
fi

echo "Building $site into $out" >&2
if ! (cd "$site" && env -u FOLIA_DUMP "${vars[@]}" bundle exec jekyll "${args[@]}") >"$out/build.log" 2>&1; then
  tail -n 30 "$out/build.log" >&2
  echo "Jekyll build failed; see $out/build.log" >&2
  exit 1
fi

# --- Checks ------------------------------------------------------------------

tree_after=$(git -C "$site" status --porcelain --untracked-files=all)
if [[ $tree_after != "$tree_before" ]]; then
  echo "The build changed the site's working tree:" >&2
  diff <(echo "$tree_before") <(echo "$tree_after") >&2 || true
  exit 1
fi

files=$(find "$out/site" -type f | wc -l)
echo "Built $files files." >&2
if $dump; then
  ruby "$folia/parity/check_dump.rb" "$out/dump" "$out/site"
fi
