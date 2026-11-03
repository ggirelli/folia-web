#!/usr/bin/env bash
# Shows that the container baselines of the toolchain comparison (plan v12, §5.4, A2.7) refuse
# what they must, in the real images: an image not pinned by digest, a lockfile that is missing or
# not the one named, a lockfile the site's Gemfile does not take (refused by Bundler, frozen, in
# the container, with its log and record kept), a destination that exists or lies inside the site,
# a site whose repository the image's git cannot read, and an environments file whose lockfile is
# not the one it names. SITE is only read: by the host's git, to clone it, and by the images' git
# (which cannot read filopoe's repository format, the last case but one); nothing is ever written
# into it. Needs Docker, and the network for the gems.
#
#   scripts/toolchain-negative.sh SITE ENVIRONMENTS WORKDIR
#
# WORKDIR must not exist; it is kept, with each case's output (<case>.out).
set -euo pipefail

[[ $# -eq 3 ]] || { echo "usage: $0 SITE ENVIRONMENTS WORKDIR" >&2; exit 2; }
site=$(cd "$1" && pwd)
environments=$2
work=$3
[[ -e $work ]] && { echo "$work already exists" >&2; exit 1; }
scripts=$(cd "$(dirname "$0")" && pwd)
folia=$(dirname "$scripts")
mkdir -p "$work"
work=$(cd "$work" && pwd)

cases=0
fail() { echo "$1" >&2; exit 1; }
# Runs case $1, a command from $3 on; requires exit status $2 and a line containing $LINE.
check() {
  local name=$1 want=$2 status=0
  shift 2
  "$@" >"$work/$name.out" 2>&1 || status=$?
  cases=$((cases + 1))
  [[ $status -eq $want ]] || fail "$name: exit $status, expected $want (see $work/$name.out)"
  grep -qF -- "$LINE" "$work/$name.out" || fail "$name: no line with: $LINE (see $work/$name.out)"
}
snapshot() { (cd "$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0r sha256sum); }
site_state() { git -C "$site" --no-optional-locks status --porcelain --untracked-files=all; git -C "$site" rev-parse HEAD; }
before=$(site_state)

# The production environment's line of ENVIRONMENTS: name, image, lockfile, SHA-256.
read -r _ image lockfile sha256 < <(sed -e 's/#.*//' -e '/^[[:space:]]*$/d' "$environments" | tail -n 1)
lockfile=$folia/$lockfile

# A clone the images can read, as toolchain-compare.sh makes.
git clone --quiet --no-checkout --no-hardlinks "$site" "$work/source"
git -C "$work/source" checkout --quiet --detach "$(git -C "$site" rev-parse HEAD)"
source=$work/source

LINE="is not pinned by digest" check unpinned 1 \
  "$scripts/baseline-container.sh" "${image%@*}" "$lockfile" "$sha256" "$source" "$work/unpinned"
LINE="does not exist" check lock-missing 1 \
  "$scripts/baseline-container.sh" "$image" "$work/none.lock" "$sha256" "$source" "$work/lock-missing"
LINE="not the expected $(printf '0%.0s' {1..64})" check lock-sha256 1 \
  "$scripts/baseline-container.sh" "$image" "$lockfile" "$(printf '0%.0s' {1..64})" "$source" "$work/lock-sha256"
for name in unpinned lock-missing lock-sha256; do [[ ! -e $work/$name ]] || fail "$name: wrote $work/$name"; done

mkdir "$work/existing" && printf 'kept\n' >"$work/existing/kept.txt"
kept=$(snapshot "$work/existing")
LINE="already exists" check existing 1 \
  "$scripts/baseline-container.sh" "$image" "$lockfile" "$sha256" "$source" "$work/existing"
[[ $(snapshot "$work/existing") == "$kept" ]] || fail "existing: the directory changed"
LINE="is inside the site" check inside-site 1 \
  "$scripts/baseline-container.sh" "$image" "$lockfile" "$sha256" "$source" "$source/out"
[[ ! -e $source/out ]] || fail "inside-site: wrote into the site"

# The image's git cannot read SITE's repository format: refused before anything is created.
LINE="the image's git cannot read" check unreadable 1 \
  "$scripts/baseline-container.sh" "$image" "$lockfile" "$sha256" "$site" "$work/unreadable"
[[ ! -e $work/unreadable ]] || fail "unreadable: created its directory"

# A lockfile without one of the Gemfile's dependencies: Bundler, frozen, refuses it in the image,
# and the record and the install log are kept.
sed '/^DEPENDENCIES$/,/^$/{/^  rouge$/d}' "$lockfile" >"$work/incompatible.lock"
! cmp -s "$lockfile" "$work/incompatible.lock" || fail "incompatible: the lockfile was not changed"
LINE="The lockfile does not install for the site's Gemfile" check incompatible 1 \
  "$scripts/baseline-container.sh" "$image" "$work/incompatible.lock" \
  "$(sha256sum "$work/incompatible.lock" | cut -d' ' -f1)" "$source" "$work/incompatible"
grep -q '"stage": "bundle install"' "$work/incompatible/BASELINE.json" || fail "incompatible: the record does not say where it stopped"
grep -qi "rouge" "$work/incompatible/bundle-install.log" || fail "incompatible: Bundler's log does not name the dependency"

# An environments file whose lockfile is not the one it names: the comparison stops before building.
sed "\$s/$sha256/$(printf '1%.0s' {1..64})/" "$environments" >"$work/environments.txt"
LINE="not the expected $(printf '1%.0s' {1..64})" check environments 1 \
  "$scripts/toolchain-compare.sh" "$site" "$work/environments.txt" 2026-10-25T14:43:51Z "$work/environments-run"
[[ ! -e $work/environments-run ]] || fail "environments: created its directory"
LINE="already exists" check compare-existing 1 \
  "$scripts/toolchain-compare.sh" "$site" "$environments" 2026-10-25T14:43:51Z "$work/existing"

[[ $(site_state) == "$before" ]] || fail "the site changed"
echo "every case ended as it should ($cases cases); see $work/*.out"
