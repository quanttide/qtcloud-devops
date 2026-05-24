#!/usr/bin/env bash
set -euo pipefail

# validate-changelog.sh <version>
#   version: e.g. "0.3.0" or "0.3.0-rc.1"
#   Validates CHANGELOG.md exists and contains an entry for the version.
#   Looks for "## [version]" header.

if [ $# -ne 1 ]; then
  echo "usage: validate-changelog.sh <version>" >&2
  exit 1
fi

VERSION="$1"
CHANGELOG="CHANGELOG.md"

if [ ! -f "$CHANGELOG" ]; then
  echo "CHANGELOG.md not found" >&2
  exit 1
fi

MARKER="## [$VERSION]"
if ! grep -Fq "$MARKER" "$CHANGELOG"; then
  echo "CHANGELOG.md missing entry for version $VERSION" >&2
  exit 1
fi
