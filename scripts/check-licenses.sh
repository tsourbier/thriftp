#!/usr/bin/env bash
set -euo pipefail

if command -v cargo-license >/dev/null 2>&1; then
  cargo license --workspace --avoid-build-dependencies --avoid-dev-dependencies
else
  echo "cargo-license is not installed."
  echo "Install it with: cargo install cargo-license"
  echo "Dependency metadata preview:"
  cargo metadata --format-version 1 --no-deps >/dev/null
fi
