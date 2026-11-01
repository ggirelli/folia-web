#!/usr/bin/env bash
# Checks that a step of the vertical slice keeps out of the other step's mode (plan v7, §12.2):
# step 2 (a post) must not change crates/garden, step 3 (garden notes) must not change
# crates/blog. It looks at every change from BASE to the working tree, untracked files included.
#
#   scripts/slice-boundary.sh STEP [BASE]
#
# BASE defaults to main.
set -euo pipefail

[[ $# -ge 1 && $# -le 2 ]] || { echo "usage: $0 STEP [BASE]" >&2; exit 2; }
step=$1
base=${2:-main}
case $step in
  2) forbidden=crates/garden ;;
  3) forbidden=crates/blog ;;
  *) echo "step must be 2 or 3, not $step" >&2; exit 2 ;;
esac

changed=$(
  {
    git diff --name-only "$base" --
    git ls-files --others --exclude-standard
  } | sort -u
)
touched=$(grep -E "^$forbidden/" <<<"$changed" || true)
if [[ -n $touched ]]; then
  echo "Step $step changes $forbidden, which it must not:" >&2
  echo "$touched" >&2
  exit 1
fi
echo "Step $step keeps out of $forbidden ($(grep -c . <<<"$changed" || true) files changed since $base)."
