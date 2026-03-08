#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:?Usage: ./scripts/release.sh <version>}

# Update workspace version (package + dependency)
sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml
sed -i "/^rk-shared/s/version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml

# Verify it compiles
cargo check --workspace

# Commit, tag
git add Cargo.toml Cargo.lock
git commit -m "Release v$VERSION"
git tag "v$VERSION"

echo ""
echo "Ready. Run: git push origin main --tags"
