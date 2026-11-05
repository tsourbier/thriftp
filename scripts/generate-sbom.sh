#!/usr/bin/env bash
set -euo pipefail

OUTPUT="${1:-target/sbom.json}"
mkdir -p "$(dirname "$OUTPUT")"
OUTPUT_DIR="$(cd "$(dirname "$OUTPUT")" && pwd)"
NPM_OUTPUT="$OUTPUT_DIR/sbom-npm.json"

if command -v cargo-cyclonedx >/dev/null 2>&1; then
  find crates tests/integration -name 'sbom.json' -type f -delete 2>/dev/null || true
  cargo cyclonedx --format json --spec-version 1.5 --override-filename sbom >/dev/null
  SBOMS=()
  while IFS= read -r file; do
    SBOMS+=("$file")
  done < <(find crates tests/integration -name 'sbom.json' -type f | sort)
  if [ "${#SBOMS[@]}" -eq 0 ]; then
    echo "cargo-cyclonedx produced no SBOM files" >&2
    exit 1
  fi
  python3 scripts/merge-cyclonedx.py "$OUTPUT" "${SBOMS[@]}"
  find crates tests/integration -name 'sbom.json' -type f -delete 2>/dev/null || true
else
  echo "cargo-cyclonedx is not installed; writing a dependency inventory fallback." >&2
  cargo metadata --format-version 1 --locked > "$OUTPUT"
fi

if command -v npm >/dev/null 2>&1 && [ -f apps/desktop/package-lock.json ]; then
  (cd apps/desktop && npm sbom --package-lock-only --sbom-format cyclonedx > "$NPM_OUTPUT")
  echo "npm SBOM written to $NPM_OUTPUT"
fi

echo "SBOM written to $OUTPUT"
