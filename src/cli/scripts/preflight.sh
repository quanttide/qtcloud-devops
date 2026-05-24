#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }

VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "preflight for v$VERSION"
echo ""

echo "--- cargo build ---"
cargo build --release 2>&1 | tail -1 && pass "build" || fail "build"

echo ""
echo "--- cargo test ---"
cargo test 2>&1 | grep "test result" && pass "test" || fail "test"

echo ""
echo "--- cargo publish --dry-run ---"
cargo publish --dry-run --registry crates-io 2>&1 | tail -3 && pass "cargo publish dry-run" || fail "cargo publish dry-run"

echo ""
echo "--- cargo build (python feature) ---"
cargo build --features python --release 2>&1 | tail -1 && pass "cargo build (python)" || fail "cargo build (python)"

echo ""
echo "--- maturin build ---"
pip install maturin -q 2>/dev/null
maturin build --release --out dist --auditwheel skip 2>&1 | tail -1 && pass "maturin build" || fail "maturin build"

echo ""
echo "preflight passed"
