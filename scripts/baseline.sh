#!/usr/bin/env bash
# Builds a production Jekyll baseline of the reference site (plan v2, §10 spike 0).
#
#   scripts/baseline.sh [--allow-dirty] [--no-dump] SITE OUT
#
# OUT must not exist. It receives:
#   site/           the production build (JEKYLL_ENV=production)
#   dump/           the folia_dump.rb dumps (absent with --no-dump)
#   BASELINE.json   site and submodule commits, tool versions, flags
#   build.log       Jekyll's output
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

status_before=$(git -C "$site" status --porcelain)
if [[ -n $status_before ]] && ! $allow_dirty; then
  echo "$site has uncommitted changes (use --allow-dirty to build them anyway):" >&2
  echo "$status_before" >&2
  exit 1
fi

mkdir -p "$out/site"
out=$(cd "$out" && pwd)

# --- Record what is being built --------------------------------------------

record() {
  SITE="$site" FOLIA="$folia" OUT="$out" DUMP="$dump" STATUS="$status_before" \
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

status_after=$(git -C "$site" status --porcelain)
if [[ $status_after != "$status_before" ]]; then
  echo "The build changed the site's working tree:" >&2
  diff <(echo "$status_before") <(echo "$status_after") >&2 || true
  exit 1
fi

files=$(find "$out/site" -type f | wc -l)
echo "Built $files files." >&2
if $dump; then
  ruby "$folia/parity/check_dump.rb" "$out/dump" "$out/site"
fi
