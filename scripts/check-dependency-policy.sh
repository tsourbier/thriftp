#!/usr/bin/env bash
# Fail when the dependency graph contains an unapproved critical or high advisory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Checking npm advisories at high or critical severity..."
npm --prefix apps/desktop audit --audit-level=high

if command -v cargo-audit >/dev/null 2>&1; then
  echo "Checking Rust advisories..."
  if grep -q 'RUSTSEC-2023-0071' specs/005-dependency-upgrade/exceptions.json; then
    cargo audit --ignore RUSTSEC-2023-0071
  else
    cargo audit
  fi
else
  echo "cargo-audit is not installed; skipping Rust advisory scan (CI installs it explicitly)."
fi

echo "Dependency policy check passed."
