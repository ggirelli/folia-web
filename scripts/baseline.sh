#!/usr/bin/env bash
# Builds a production Jekyll baseline of the reference site (plan v2, §10 spike 0; plan v12, §5.4).
#
#   scripts/baseline.sh [--allow-dirty] [--no-dump] [--at TIME]
#                       [--lockfile FILE --lockfile-sha256 SHA256] [--empty-out] SITE OUT
#
# OUT must not exist (with --empty-out: it must be an empty directory, which the caller made for
# this build, as scripts/baseline-container.sh does). It receives:
#   site/           the production build (JEKYLL_ENV=production)
#   dump/           the folia_dump.rb dumps (absent with --no-dump)
#   BASELINE.json   what was built and with what: the site's and Folia's commits and identities,
#                   the tools, the environment, the lockfile, the build's flags and time, the
#                   result (written before the build, and again with the result)
#   Gemfile.lock    the lockfile the build used, byte for byte
#   sha256.txt      one line per file of site/ and dump/, `<sha256>  ./<path>`, sorted bytewise
#   site.diff       with --allow-dirty on a dirty tree: `git diff --binary HEAD`,
#                   which the parity tools check the working tree against
#   jekyll-time.yml with --at: the configuration that pins Jekyll's time
#   bundle-install.log  with --lockfile: what `bundle install` printed
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
# --lockfile builds with exactly that lockfile, whose SHA-256 must be --lockfile-sha256: it is
# put beside a copy of the site's Gemfile in a new directory outside the site, and its gems are
# installed there with Bundler frozen, so a lockfile that does not fit the Gemfile is refused,
# never resolved again. Without it, the site's own bundle and lockfile are used. Either way the
# lockfile is copied into OUT and recorded, and a build that changes it fails.
#
# --at TIME (`YYYY-MM-DDTHH:MM:SSZ`) pins Jekyll's `site.time`, which dates the feeds and decides
# what is future-dated, with a configuration file added after the site's `_config.yml`.
#
# --no-dump builds without loading the plugin at all, to check that loading
# it changes nothing.
#
# FOLIA_BASELINE_CONTAINER_IMAGE and FOLIA_BASELINE_CONTAINER_PLATFORM, when set (by
# scripts/baseline-container.sh), are recorded as the container the build ran in.
set -euo pipefail
# Git only reads: not even the index is refreshed, in the site or in Folia.
export GIT_OPTIONAL_LOCKS=0

usage() {
  echo "usage: $0 [--allow-dirty] [--no-dump] [--at TIME] [--lockfile FILE --lockfile-sha256 SHA256] [--empty-out] SITE OUT" >&2
  exit 2
}
refuse() { echo "$1" >&2; exit 1; }

allow_dirty=false
dump=true
empty_out=false
at=
lockfile=
lockfile_sha256=
while [[ $# -gt 0 && $1 == --* ]]; do
  case $1 in
    --allow-dirty) allow_dirty=true ;;
    --no-dump) dump=false ;;
    --empty-out) empty_out=true ;;
    --at) [[ $# -ge 2 ]] || usage; at=$2; shift ;;
    --lockfile) [[ $# -ge 2 ]] || usage; lockfile=$2; shift ;;
    --lockfile-sha256) [[ $# -ge 2 ]] || usage; lockfile_sha256=$2; shift ;;
    *) usage ;;
  esac
  shift
done
[[ $# -eq 2 ]] || usage
[[ -n $lockfile && -n $lockfile_sha256 || -z $lockfile && -z $lockfile_sha256 ]] ||
  refuse "--lockfile and --lockfile-sha256 go together."

folia=$(cd "$(dirname "$0")/.." && pwd)
site=$(cd "$1" && pwd)
out=$2

# --- What may be built, and where --------------------------------------------

if $empty_out; then
  [[ -d $out && -z $(ls -A "$out") ]] || refuse "$out is not an empty directory (--empty-out)."
else
  [[ -e $out ]] && refuse "$out already exists; pick a new label."
fi
out_abs=$(realpath -m "$out")
case $out_abs/ in
  "$site"/*) refuse "$out is inside the site; a baseline is kept apart from it." ;;
  "$folia"/*) refuse "$out is inside the Folia repository, whose identity the baseline records." ;;
esac
if [[ -n $at ]]; then
  [[ $at =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] ||
    refuse "--at $at is not YYYY-MM-DDTHH:MM:SSZ."
  [[ -f $site/_config.yml ]] || refuse "$site has no _config.yml to pin the time after."
fi
if [[ -n $lockfile ]]; then
  [[ -f $lockfile ]] || refuse "the lockfile $lockfile does not exist."
  lockfile=$(realpath "$lockfile")
  actual=$(sha256sum "$lockfile" | cut -d' ' -f1)
  [[ $actual == "$lockfile_sha256" ]] ||
    refuse "the lockfile $lockfile has the SHA-256 $actual, not the expected $lockfile_sha256."
  [[ -f $site/Gemfile ]] || refuse "$site has no Gemfile for the lockfile."
fi
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

# Folia's identity, as the parity tools' records name it: clean, changed, or unknown.
if folia_status=$(git -C "$folia" status --porcelain --untracked-files=all 2>&1); then
  folia_state=$([[ -z $folia_status ]] && echo clean || echo changed)
else
  folia_state=unknown
fi

$empty_out || mkdir -p "$(dirname "$out_abs")"
$empty_out || mkdir "$out" || refuse "cannot create $out."
out=$(cd "$out" && pwd)
mkdir "$out/site"
site_diff_sha256=
if [[ -n $status_before ]]; then
  git -C "$site" diff --binary HEAD "${counted[@]}" >"$out/site.diff"
  site_diff_sha256=$(sha256sum "$out/site.diff" | cut -d' ' -f1)
fi

# --- The bundle, and the lockfile it uses --------------------------------------

if [[ -n $lockfile ]]; then
  # A new directory outside the site, gone with the container when run in one.
  work=$(mktemp -d)
  cp "$site/Gemfile" "$work/Gemfile"
  cp "$lockfile" "$work/Gemfile.lock"
  export BUNDLE_GEMFILE=$work/Gemfile BUNDLE_PATH=$work/bundle BUNDLE_APP_CONFIG=$work/.bundle
  export BUNDLE_FROZEN=true
fi

# --- Record what is being built --------------------------------------------

# Writes BASELINE.json; $1 is the result as JSON (`null` before the build finishes).
record() {
  SITE="$site" FOLIA="$folia" OUT="$out" DUMP="$dump" STATUS="$status_before" AT="$at" \
    EXCLUDED="$(printf '%s\n' "${excluded[@]}")" SITE_DIFF_SHA256="$site_diff_sha256" \
    FOLIA_STATE="$folia_state" FOLIA_STATUS="$folia_status" RESULT="$1" \
    LOCKFILE_SOURCE="${lockfile:-}" CREATED_AT="${created_at:-}" \
    ruby -rjson -rdigest -rtime -e '
      def sh(*cmd, dir: ENV["SITE"])
        out = IO.popen(cmd, chdir: dir, err: File::NULL, &:read).strip
        $?.success? ? out : nil
      rescue SystemCallError
        nil
      end
      def sha256(path) = File.file?(path) ? Digest::SHA256.file(path).hexdigest : nil

      site, out = ENV["SITE"], ENV["OUT"]
      dirty = !ENV["STATUS"].empty?
      declared = (sh("git", "config", "--file", ".gitmodules", "--get-regexp", "path") || "").lines.map { _1.split[1] }
      os = File.file?("/etc/os-release") ? File.read("/etc/os-release")[/^PRETTY_NAME="?([^"\n]*)/, 1] : nil
      image = ENV["FOLIA_BASELINE_CONTAINER_IMAGE"]
      configs = [File.join(site, "_config.yml")]
      configs << File.join(out, "jekyll-time.yml") unless ENV["AT"].empty?
      record = {
        "created_at" => ENV["CREATED_AT"],
        "site" => {
          "path" => site,
          "commit" => sh("git", "rev-parse", "HEAD"),
          "dirty" => dirty,
          "status" => ENV["STATUS"].lines.map(&:chomp),
          "diff" => dirty ? "site.diff" : nil,
          "changes" => dirty ? {"state" => "changed", "sha256" => ENV["SITE_DIFF_SHA256"]} : {"state" => "clean"},
          "excluded" => ENV["EXCLUDED"].lines.map(&:chomp).reject(&:empty?),
          "submodules" => (sh("git", "submodule", "status", "--recursive") || "").lines.map(&:strip),
          "submodules_declared" => declared,
          "gemfile_lock_sha256" => sha256(File.join(out, "Gemfile.lock")),
        },
        "folia" => {
          "path" => ENV["FOLIA"],
          "commit" => sh("git", "rev-parse", "HEAD", dir: ENV["FOLIA"]),
          "status" => ENV["FOLIA_STATE"] == "unknown" ? nil : ENV["FOLIA_STATUS"].lines.map(&:chomp),
          "changes" => ENV["FOLIA_STATE"] == "unknown" ? {"state" => "unknown", "error" => ENV["FOLIA_STATUS"]} : {"state" => ENV["FOLIA_STATE"]},
          "plugin_sha256" => ENV["DUMP"] == "true" ? sha256(File.join(ENV["FOLIA"], "parity/jekyll/folia_dump.rb")) : nil,
        },
        "lockfile" => {
          "file" => "Gemfile.lock",
          "sha256" => sha256(File.join(out, "Gemfile.lock")),
          "source" => ENV["LOCKFILE_SOURCE"].empty? ? "the site bundle" : ENV["LOCKFILE_SOURCE"],
          "installed" => !ENV["LOCKFILE_SOURCE"].empty?,
        },
        "tools" => {
          "ruby" => sh("bundle", "exec", "ruby", "-e", "print RUBY_DESCRIPTION"),
          "bundler" => sh("bundle", "exec", "ruby", "-e", "print Bundler::VERSION"),
          "bundler_command" => sh("bundle", "-v"),
          "jekyll" => sh("bundle", "exec", "jekyll", "-v"),
          "git" => sh("git", "--version"),
        },
        "environment" => {
          "container" => image ? {"image" => image, "platform" => ENV["FOLIA_BASELINE_CONTAINER_PLATFORM"]} : nil,
          "os" => os,
          "machine" => sh("uname", "-m"),
        },
        "build" => {
          "jekyll_env" => "production",
          "dump" => ENV["DUMP"] == "true",
          "time" => ENV["AT"].empty? ? nil : ENV["AT"],
          "config" => configs.map { {"path" => _1, "sha256" => sha256(_1)} },
          "tz" => ENV["TZ"],
          "lang" => ENV["LANG"],
        },
        "result" => JSON.parse(ENV["RESULT"]),
      }
      File.write(File.join(out, "BASELINE.json"), JSON.pretty_generate(record) + "\n")
    '
}

# Records the failure of stage $1 and stops, with message $2.
stop() {
  record "{\"ok\": false, \"stage\": \"$1\", \"files\": null}"
  echo "$2" >&2
  exit 1
}

created_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
if [[ -n $lockfile ]]; then
  echo "Installing the gems of $lockfile" >&2
  if ! (cd "$work" && bundle install) >"$out/bundle-install.log" 2>&1; then
    tail -n 20 "$out/bundle-install.log" >&2
    stop "bundle install" "The lockfile does not install for the site's Gemfile (Bundler is frozen, so it is never resolved again); see $out/bundle-install.log"
  fi
fi
used=$(cd "$site" && bundle exec ruby -e 'print Bundler.default_lockfile' 2>/dev/null) || used=
[[ -n $used && -f $used ]] || stop "lockfile" "No lockfile for the build's bundle: its gems could not be recorded."
cp "$used" "$out/Gemfile.lock"
lock_before=$(sha256sum "$used" | cut -d' ' -f1)
if [[ -n $at ]]; then
  printf 'time: %s\n' "$at" >"$out/jekyll-time.yml"
fi
record null

# --- Build -------------------------------------------------------------------

vars=(JEKYLL_ENV=production)
args=(build --source "$site" --destination "$out/site" --disable-disk-cache)
[[ -n $at ]] && args+=(--config "$site/_config.yml,$out/jekyll-time.yml")
if $dump; then
  mkdir "$out/dump"
  vars+=("FOLIA_DUMP=$out/dump")
  args+=(--plugins "$site/_plugins,$folia/parity/jekyll")
fi

echo "Building $site into $out" >&2
if ! (cd "$site" && env -u FOLIA_DUMP "${vars[@]}" bundle exec jekyll "${args[@]}") >"$out/build.log" 2>&1; then
  tail -n 30 "$out/build.log" >&2
  stop "jekyll build" "Jekyll build failed; see $out/build.log"
fi

# --- Checks ------------------------------------------------------------------

tree_after=$(git -C "$site" status --porcelain --untracked-files=all)
if [[ $tree_after != "$tree_before" ]]; then
  diff <(echo "$tree_before") <(echo "$tree_after") >&2 || true
  stop "checkout" "The build changed the site's working tree."
fi
[[ $(sha256sum "$used" | cut -d' ' -f1) == "$lock_before" ]] ||
  stop "lockfile" "The build changed its lockfile $used."

files=$(find "$out/site" -type f | wc -l)
echo "Built $files files." >&2
if $dump && ! ruby "$folia/parity/check_dump.rb" "$out/dump" "$out/site"; then
  stop "dump" "The dump does not account for the build; see above."
fi
parts=(site)
$dump && parts+=(dump)
(cd "$out" && find "${parts[@]}" -type f -print0 | LC_ALL=C sort -z | xargs -0r sha256sum |
  sed 's|  |  ./|') >"$out/sha256.txt"
record "{\"ok\": true, \"files\": $(wc -l <"$out/sha256.txt"), \"site_files\": $files}"
