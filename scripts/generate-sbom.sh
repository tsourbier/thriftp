#!/usr/bin/env bash
set -euo pipefail

OUTPUT="${1:-target/sbom.json}"
mkdir -p "$(dirname "$OUTPUT")"

if command -v cargo-cyclonedx >/dev/null 2>&1; then
  cargo cyclonedx --format json --output-file "$OUTPUT"
else
  echo "cargo-cyclonedx is not installed; writing a dependency inventory fallback." >&2
  cargo metadata --format-version 1 --locked > "$OUTPUT"
fi
echo "SBOM written to $OUTPUT"
