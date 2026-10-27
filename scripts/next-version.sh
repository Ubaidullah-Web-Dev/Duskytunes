#!/usr/bin/env bash
# Print the next Duskytunes release version, computed purely from the published
# git tags. This is the single source of truth for the version ledger; the
# release workflow calls it to reserve the next version, and it is safe to run
# locally to preview the next number.
#
#   scripts/next-version.sh            # read tags from git
#   DUSKYTUNES_TAGS="v1.0.0" scripts/next-version.sh   # smoke a rollover, no git
#
# The scheme:
# Starts from 1.0.0 (v1), then increments minor version: 1.1.0 (v1.1), 1.2.0 (v1.2), etc.
set -euo pipefail

if [[ -n "${DUSKYTUNES_TAGS+x}" ]]; then
  tags="$DUSKYTUNES_TAGS"
else
  tags="$(git tag -l 'v1.*')"
fi

best_minor=-1
for t in $tags; do
  if [[ "$t" =~ ^v1\.([0-9]+)(\.([0-9]+))?$ ]]; then
    minor="${BASH_REMATCH[1]}"
    if (( minor > best_minor )); then
      best_minor="$minor"
    fi
  fi
done

if (( best_minor < 0 )); then
  echo "1.0.0"
  exit 0
fi

minor=$(( best_minor + 1 ))
echo "1.$minor.0"
