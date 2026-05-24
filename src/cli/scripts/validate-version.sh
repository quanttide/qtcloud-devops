#!/usr/bin/env bash
set -euo pipefail

# validate-version.sh <tag-ref>
#   tag-ref: e.g. "cli/v0.3.0" (from GITHUB_REF_NAME)
#   Validates tag version matches Cargo.toml and pyproject.toml.
#   Prints the validated version (without v prefix) on success.

if [ $# -ne 1 ]; then
  echo "usage: validate-version.sh <tag-ref>" >&2
  exit 1
fi

TAG_REF="$1"

# Strip optional prefix (e.g. "cli/v0.3.0" → "v0.3.0")
TAG_VERSION="${TAG_REF#*/}"
[ -z "$TAG_VERSION" ] && { echo "cannot extract version from tag: $TAG_REF" >&2; exit 1; }

# Strip v prefix for comparison
EXPECTED="${TAG_VERSION#v}"

# Validate Cargo.toml
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
[ -z "$CARGO_VERSION" ] && { echo "cannot read version from Cargo.toml" >&2; exit 1; }
if [ "$CARGO_VERSION" != "$EXPECTED" ]; then
  echo "version mismatch: tag=$TAG_VERSION cargo=$CARGO_VERSION" >&2
  exit 1
fi

# Validate pyproject.toml
PYPROJECT="pyproject.toml"
if [ -f "$PYPROJECT" ]; then
  PY_VERSION=$(grep '^version = ' "$PYPROJECT" | head -1 | sed 's/version = "\(.*\)"/\1/')
  if [ -n "$PY_VERSION" ] && [ "$PY_VERSION" != "$EXPECTED" ]; then
    echo "version mismatch: tag=$TAG_VERSION pyproject=$PY_VERSION" >&2
    exit 1
  fi
fi

echo "$EXPECTED"
