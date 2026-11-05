#!/usr/bin/env bash
set -euo pipefail

echo "Building the Rust workspace..."
cargo build --workspace

if command -v node >/dev/null 2>&1 && [ -f apps/desktop/package.json ]; then
  echo "Installing frontend dependencies..."
  (cd apps/desktop && (npm ci --no-audit --no-fund 2>/dev/null || npm install --no-audit --no-fund))
  echo "Building the frontend..."
  (cd apps/desktop && npm run build)
fi

echo "Contributor build validation passed."
