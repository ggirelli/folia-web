#!/usr/bin/env bash
# Chooses the recorded reference the determinism check compares a baseline's builds with (plan
# v12, §4.2, A2.5), fail-closed, for `just determinism`, `just determinism-negative` and
# `just accept` alike: the reference must be the baseline's own.
#
#   scripts/determinism-reference.sh LABEL A0_LABEL A0_REFERENCE
#
# With FOLIA_REFERENCE set, that reference, only when FOLIA_REFERENCE_LABEL declares it LABEL's
# (a reference does not record its baseline, so the declaration is the mapping). Without it,
# A0_REFERENCE, only when LABEL is A0_LABEL (and FOLIA_REFERENCE_LABEL, if set, says LABEL).
# A0's reference is never chosen for another baseline. Prints the reference's path, then how it
# was chosen, and exits 0; otherwise prints why there is none and exits 2. The reference must be
# a file.
set -euo pipefail

[[ $# -eq 3 ]] || { echo "usage: $0 LABEL A0_LABEL A0_REFERENCE" >&2; exit 2; }
label=$1
a0_label=$2
a0_reference=$3
declared=${FOLIA_REFERENCE_LABEL:-}
none() { echo "$1" >&2; exit 2; }

if [[ -n ${FOLIA_REFERENCE:-} ]]; then
  [[ $declared == "$label" ]] ||
    none "FOLIA_REFERENCE is $FOLIA_REFERENCE, but FOLIA_REFERENCE_LABEL (${declared:-unset}) does not declare it $label's; set FOLIA_REFERENCE_LABEL=$label if it is"
  reference=$FOLIA_REFERENCE
  chosen="configured: FOLIA_REFERENCE, declared $label's by FOLIA_REFERENCE_LABEL"
elif [[ $label == "$a0_label" ]]; then
  [[ -z $declared || $declared == "$label" ]] ||
    none "FOLIA_REFERENCE_LABEL is $declared, not $label, and no FOLIA_REFERENCE is set"
  reference=$a0_reference
  chosen="the A0 reference, $a0_label's by default"
else
  none "no reference is configured for $label (A0's is $a0_label's only): set FOLIA_REFERENCE to $label's reference and FOLIA_REFERENCE_LABEL=$label"
fi
[[ -f $reference ]] || none "the reference $reference ($chosen) is not a file"
printf '%s\n%s\n' "$reference" "$chosen"
