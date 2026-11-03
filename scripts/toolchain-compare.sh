#!/usr/bin/env bash
# Builds one commit of the reference site with each Jekyll toolchain of ENVIRONMENTS, as of one
# pinned time, and compares the baselines with `scripts/compare-builds.sh` (plan v12, §5.4, A2.7;
# Q17): each environment with the next, and the first with the last.
#
#   scripts/toolchain-compare.sh SITE ENVIRONMENTS TIME OUT [HOST_BASELINE]
#
# ENVIRONMENTS (parity/toolchain/environments.txt) has one line per environment: a name, a Docker
# image pinned by digest, a lockfile and its SHA-256. TIME (`YYYY-MM-DDTHH:MM:SSZ`) pins Jekyll's
# time in every build. OUT must not exist. It receives:
#
#   source/          a fresh clone of SITE at its HEAD, every tracked file dated the commit's
#                    committer time, so no date comes from when the clone was made; mounted
#                    read-only in every build (SITE is only read, by the host's git)
#   <name>/          each environment's baseline (scripts/baseline-container.sh)
#   compare/<a>--<b>.txt   compare-builds.sh's output for each pair
#   compare/<a>--<b>/<path>.diff   `diff -u` of each file that differs
#   compare/<a>--<b>/<path>.regions.txt   where it differs, token by token (parity/diff_regions.py)
#   toolchain-report.md    the environments, the lockfiles' differences and each comparison
#
# HOST_BASELINE, when given (`BASELINE-B`), is compared with the first environment too, for
# information: it was built on the host, from SITE itself, unpinned.
#
# A difference between environments is a finding, not a failure: the run exits 0 when every build
# succeeded and every comparison ran, and 1 otherwise; nothing it wrote is ever removed.
set -euo pipefail

usage() { echo "usage: $0 SITE ENVIRONMENTS TIME OUT [HOST_BASELINE]" >&2; exit 2; }
[[ $# -eq 4 || $# -eq 5 ]] || usage
site=$(cd "$1" && pwd)
environments=$2
at=$3
out=$4
host=${5:-}
scripts=$(cd "$(dirname "$0")" && pwd)
folia=$(dirname "$scripts")

[[ -f $environments ]] || { echo "$environments does not exist" >&2; exit 2; }
[[ -e $out ]] && { echo "$out already exists; pick a new directory." >&2; exit 1; }

# The environments: name, image, lockfile, SHA-256; each checked before anything is built.
mapfile -t lines < <(sed -e 's/#.*//' -e '/^[[:space:]]*$/d' "$environments")
names=()
for line in "${lines[@]}"; do
  read -r name image lockfile sha256 extra <<<"$line"
  [[ -n $sha256 && -z ${extra:-} ]] || { echo "$environments: not 'name image lockfile sha256': $line" >&2; exit 2; }
  [[ $image =~ @sha256:[0-9a-f]{64}$ ]] || { echo "$name: the image $image is not pinned by digest." >&2; exit 1; }
  [[ -f $folia/$lockfile ]] || { echo "$name: the lockfile $lockfile does not exist." >&2; exit 1; }
  actual=$(sha256sum "$folia/$lockfile" | cut -d' ' -f1)
  [[ $actual == "$sha256" ]] ||
    { echo "$name: the lockfile $lockfile has the SHA-256 $actual, not the expected $sha256." >&2; exit 1; }
  names+=("$name")
done
[[ ${#names[@]} -ge 2 ]] || { echo "$environments names fewer than two environments" >&2; exit 2; }

mkdir -p "$(dirname "$out")"
mkdir "$out"
out=$(cd "$out" && pwd)
trap 'echo "Everything is kept in $out" >&2' EXIT

# --- The source: one clone, dated by its commit ----------------------------------------------

commit=$(git -C "$site" rev-parse HEAD)
git clone --quiet --no-checkout --no-hardlinks "$site" "$out/source"
git -C "$out/source" checkout --quiet --detach "$commit"
committed=$(git -C "$out/source" log -1 --format=%ct "$commit")
(cd "$out/source" && git ls-files -z | xargs -0 touch --no-dereference --date="@$committed")
echo "== Source: $commit, cloned into $out/source, files dated $(date -u -d "@$committed" +%FT%TZ)"

# --- The builds --------------------------------------------------------------------------------

for line in "${lines[@]}"; do
  read -r name image lockfile sha256 <<<"$line"
  echo "== $name: $image, $lockfile"
  "$scripts/baseline-container.sh" --at "$at" "$image" "$folia/$lockfile" "$sha256" \
    "$out/source" "$out/$name"
done

# Every build is of the same source, as of the same time, with the same Folia.
ruby -rjson -e '
  records = ARGV.map { |dir| [File.basename(dir), JSON.parse(File.read(File.join(dir, "BASELINE.json")))] }
  same = ->(label, &key) {
    values = records.map { |name, record| [name, key.(record)] }.uniq { _2 }
    abort "The builds differ in #{label}: #{values.inspect}" if values.size > 1
  }
  same.("site commit") { _1.dig("site", "commit") }
  same.("Folia commit") { _1.dig("folia", "commit") }
  same.("Folia identity") { _1.dig("folia", "changes") }
  same.("pinned time") { _1.dig("build", "time") }
  same.("plugin") { _1.dig("folia", "plugin_sha256") }
  records.each { |name, record| abort "#{name} did not finish" unless record.dig("result", "ok") }
' "${names[@]/#/$out/}"

# --- The comparisons ---------------------------------------------------------------------------

mkdir "$out/compare"
pairs=()
for ((i = 0; i + 1 < ${#names[@]}; i++)); do pairs+=("${names[i]} ${names[i + 1]}"); done
if [[ ${#names[@]} -gt 2 ]]; then pairs+=("${names[0]} ${names[-1]}"); fi

# Compares baselines $1 and $2 (directories), named $3 and $4; writes the output and the diffs.
compare() {
  local a=$1 b=$2 label="$3--$4" status=0 kind path
  "$scripts/compare-builds.sh" "$a" "$b" >"$out/compare/$label.txt" 2>&1 || status=$?
  [[ $status -le 1 ]] || { cat "$out/compare/$label.txt" >&2; exit 1; }
  while read -r kind path; do
    [[ $kind == changed && -f $a/$path && -f $b/$path ]] || continue
    mkdir -p "$(dirname "$out/compare/$label/$path")"
    diff -u --label "$3/$path" --label "$4/$path" "$a/$path" "$b/$path" \
      >"$out/compare/$label/$path.diff" || true
    python3 -I "$folia/parity/diff_regions.py" "$a/$path" "$b/$path" \
      >"$out/compare/$label/$path.regions.txt"
  done < <(sed -n '/^Unexpected differences/,$p' "$out/compare/$label.txt" | grep '^  ' || true)
  echo "$label: $(tail -n 1 "$out/compare/$label.txt")"
}
for pair in "${pairs[@]}"; do
  read -r a b <<<"$pair"
  compare "$out/$a" "$out/$b" "$a" "$b"
done
if [[ -n $host ]]; then
  compare "$host" "$out/${names[0]}" "host-$(basename "$host")" "${names[0]}"
fi

# --- The report --------------------------------------------------------------------------------

ruby -rjson - "$out" "$at" "$commit" "${names[@]}" >"$out/toolchain-report.md" <<'EOF'
out, at, commit, *names = ARGV
records = names.to_h { [_1, JSON.parse(File.read(File.join(out, _1, "BASELINE.json")))] }
# Each gem's version, without the platform suffix of a precompiled variant (`1.19.3-x86_64-linux-gnu`
# is 1.19.3), with the platforms the lockfile lists it for.
gems = ->(lock) {
  found = Hash.new { |hash, name| hash[name] = {versions: [], platforms: []} }
  File.read(lock).scan(/^    (\S+) \(([^)]+)\)$/) do |name, spec|
    version, platform = spec.split("-", 2)
    found[name][:versions] |= [version]
    found[name][:platforms] |= [platform || "ruby"]
  end
  found.transform_values { {version: _1[:versions].sort.join(" / "), platforms: _1[:platforms].sort} }
}
puts "# Toolchain comparison", ""
puts "Plan v12, §5.4, A2.7 (Q17). Source `#{commit}` (`source/`, a fresh clone, files dated the"
puts "commit's time), built with each toolchain below as of `#{at}`, by `scripts/toolchain-compare.sh`.", ""
puts "## Environments", ""
puts "| Name | Image | Platform | OS | Ruby | Bundler (loaded / command) | Jekyll | Lockfile SHA-256 |"
puts "|-|-|-|-|-|-|-|-|"
records.each do |name, r|
  c = r.dig("environment", "container") || {}
  puts "| #{name} | `#{c["image"]}` | #{c["platform"]} | #{r.dig("environment", "os")} | " \
       "#{r.dig("tools", "ruby")} | #{r.dig("tools", "bundler")} / #{r.dig("tools", "bundler_command")} | " \
       "#{r.dig("tools", "jekyll")} | `#{r.dig("lockfile", "sha256")}` |"
end
puts "", "## Lockfiles", ""
names.each_cons(2).to_a.push([names.first, names.last]).uniq.each do |a, b|
  ga, gb = gems.(File.join(out, a, "Gemfile.lock")), gems.(File.join(out, b, "Gemfile.lock"))
  changed = (ga.keys | gb.keys).sort.reject { ga.dig(_1, :version) == gb.dig(_1, :version) }
  platforms = (ga.keys & gb.keys).sort.reject { ga[_1][:platforms] == gb[_1][:platforms] }
  puts "- #{a} → #{b}: " + (changed.empty? ? "the same gem versions" :
    changed.map { "`#{_1}` #{ga.dig(_1, :version) || "—"} → #{gb.dig(_1, :version) || "—"}" }.join(", "))
  platforms.each { puts "  - `#{_1}` platforms: #{ga[_1][:platforms].join(", ")} → #{gb[_1][:platforms].join(", ")}" }
end
identity = ->(r) {
  {
    "image" => r.dig("environment", "container", "image"), "OS" => r.dig("environment", "os"),
    "Ruby" => r.dig("tools", "ruby"), "Bundler" => r.dig("tools", "bundler"),
    "lockfile" => r.dig("lockfile", "sha256"), "Jekyll" => r.dig("tools", "jekyll"),
  }
}
puts "", "## Comparisons", ""
puts "Each comparison lists the recorded fields that differ between its two builds, then the files"
puts "that differ. The fields change together: a difference in output is observed with all of them,"
puts "and is not attributed to any one by this report.", ""
Dir.glob(File.join(out, "compare", "*.txt")).sort.each do |file|
  text = File.read(file)
  a, b = File.basename(file, ".txt").split("--", 2)
  varies = if records[a] && records[b]
    ia, ib = identity.(records[a]), identity.(records[b])
    changed = ia.keys.reject { ia[_1] == ib[_1] }
    same = ia.keys - changed
    "Varies: #{changed.empty? ? "nothing recorded" : changed.join(", ")}. Held: #{same.join(", ")}."
  else
    "Varies: the build itself (#{a} was built on the host, from the site's checkout, unpinned)."
  end
  unexpected = text[/^Unexpected differences \((\d+)\)/, 1].to_i
  reordered = text[/^Reordered.*\((\d+)\):$/, 1].to_i
  verdict = unexpected.zero? ? "no difference" : "#{unexpected} files differ"
  verdict += ", #{reordered} reordered only" if reordered.positive?
  puts "### #{File.basename(file, ".txt")}: #{verdict}", "", varies, ""
  entries = text.lines.drop_while { !_1.start_with?("Unexpected differences") }.drop(1).map(&:strip)
  entries.each do |entry|
    regions = File.join(file.delete_suffix(".txt"), entry.split(" ", 2).last + ".regions.txt")
    if File.file?(regions)
      count, first = File.readlines(regions, chomp: true)
      puts "- `#{entry}`: #{count}" + (first ? "; the first: `#{first.gsub("`", "'")}`" : "")
    else
      puts "- `#{entry}`"
    end
  end
  puts "" unless entries.empty?
end
EOF
echo "== Report: $out/toolchain-report.md"
