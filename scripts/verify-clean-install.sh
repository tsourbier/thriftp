#!/usr/bin/env bash
# Verify a clean, cache-independent install/build/test sequence.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --workspace --locked
cargo test --workspace --locked

cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build
