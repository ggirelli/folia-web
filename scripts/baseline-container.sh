#!/usr/bin/env bash
# Builds a Jekyll baseline with `scripts/baseline.sh` inside a Docker image pinned by digest, with
# exactly the lockfile given (plan v12, §5.4, A2.7; Q17), so that a toolchain other than the
# host's can be built, recorded and rebuilt.
#
#   scripts/baseline-container.sh [--no-dump] [--at TIME] IMAGE LOCKFILE LOCKFILE_SHA256 SITE OUT
#
# IMAGE is `name:tag@sha256:<digest>` or `name@sha256:<digest>`: a tag alone is refused, since it
# can move. The image is pulled for linux/amd64 if it is not present, and its digest checked.
# LOCKFILE must have the SHA-256 LOCKFILE_SHA256; baseline.sh installs its gems, frozen, beside a
# copy of the site's Gemfile, in the container (they go with it). OUT must not exist; it is made
# here, and is the only directory the container can write: the site, Folia and the lockfile are
# mounted read-only, each at its own path, so the build sees the paths a host build would. The
# container runs as the calling user, so OUT is theirs. OUT receives what baseline.sh writes,
# with the image and its platform in BASELINE.json's `environment.container`.
set -euo pipefail

usage() {
  echo "usage: $0 [--no-dump] [--at TIME] IMAGE LOCKFILE LOCKFILE_SHA256 SITE OUT" >&2
  exit 2
}
refuse() { echo "$1" >&2; exit 1; }

flags=()
while [[ $# -gt 0 && $1 == --* ]]; do
  case $1 in
    --no-dump) flags+=("$1") ;;
    --at) [[ $# -ge 2 ]] || usage; flags+=("$1" "$2"); shift ;;
    *) usage ;;
  esac
  shift
done
[[ $# -eq 5 ]] || usage
image=$1
lockfile=$2
lockfile_sha256=$3
site=$(cd "$4" && pwd)
out=$5
folia=$(cd "$(dirname "$0")/.." && pwd)

[[ $image =~ ^[^@[:space:]]+@sha256:[0-9a-f]{64}$ ]] ||
  refuse "the image $image is not pinned by digest (name[:tag]@sha256:<digest>)."
[[ -f $lockfile ]] || refuse "the lockfile $lockfile does not exist."
lockfile=$(realpath "$lockfile")
actual=$(sha256sum "$lockfile" | cut -d' ' -f1)
[[ $actual == "$lockfile_sha256" ]] ||
  refuse "the lockfile $lockfile has the SHA-256 $actual, not the expected $lockfile_sha256."
[[ -e $out ]] && refuse "$out already exists; pick a new label."
out_abs=$(realpath -m "$out")
case $out_abs/ in
  "$site"/*) refuse "$out is inside the site; a baseline is kept apart from it." ;;
  "$folia"/*) refuse "$out is inside the Folia repository, whose identity the baseline records." ;;
esac

digest=${image##*@}
if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "Pulling $image" >&2
  docker pull --quiet --platform linux/amd64 "$image" >&2
fi
docker image inspect --format '{{range .RepoDigests}}{{println .}}{{end}}' "$image" |
  grep -qF "@$digest" || refuse "the image $image does not have the digest $digest."
platform=$(docker image inspect --format '{{.Os}}/{{.Architecture}}{{with .Variant}}/{{.}}{{end}}' "$image")

# Every git the build runs is the image's: it must read the site's repository (a newer
# repository format, such as `extensions.relativeworktrees`, needs a newer git). A fresh clone
# can be read; scripts/toolchain-compare.sh builds from one.
if ! check=$(docker run --rm --user "$(id -u):$(id -g)" --env HOME=/tmp \
  --env GIT_CONFIG_COUNT=1 --env GIT_CONFIG_KEY_0=safe.directory --env GIT_CONFIG_VALUE_0='*' \
  --volume "$site:$site:ro" "$image" git -C "$site" rev-parse HEAD 2>&1); then
  refuse "the image's git cannot read $site's repository (build from a fresh clone): $check"
fi

mkdir -p "$(dirname "$out_abs")"
mkdir "$out_abs" || refuse "cannot create $out."

echo "Building $site in $image ($platform) into $out_abs" >&2
docker run --rm --user "$(id -u):$(id -g)" \
  --env HOME=/tmp --env LANG=C.UTF-8 \
  --env FOLIA_BASELINE_CONTAINER_IMAGE="$image" --env FOLIA_BASELINE_CONTAINER_PLATFORM="$platform" \
  --env GIT_CONFIG_COUNT=1 --env GIT_CONFIG_KEY_0=safe.directory --env GIT_CONFIG_VALUE_0='*' \
  --volume "$site:$site:ro" --volume "$folia:$folia:ro" --volume "$lockfile:$lockfile:ro" \
  --volume "$out_abs:$out_abs" \
  "$image" \
  bash "$folia/scripts/baseline.sh" --empty-out "${flags[@]}" \
  --lockfile "$lockfile" --lockfile-sha256 "$lockfile_sha256" "$site" "$out_abs"
