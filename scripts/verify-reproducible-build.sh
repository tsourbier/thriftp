#!/usr/bin/env bash
set -euo pipefail

ROOT="${CARGO_TARGET_DIR:-target}/reproducible"
rm -rf "$ROOT"
mkdir -p "$ROOT/a" "$ROOT/b"

export CARGO_INCREMENTAL=0

echo "Building first artifact set..."
CARGO_TARGET_DIR="$ROOT/a" cargo build --workspace --release --locked -q
echo "Building second artifact set..."
CARGO_TARGET_DIR="$ROOT/b" cargo build --workspace --release --locked -q

# The dependency graph and SBOM must be identical across clean builds.
cargo metadata --format-version 1 --locked > "$ROOT/a-metadata.json"
cargo metadata --format-version 1 --locked > "$ROOT/b-metadata.json"
shasum -a 256 Cargo.lock > "$ROOT/a-lock.sha"
shasum -a 256 Cargo.lock > "$ROOT/b-lock.sha"

if ! diff -u "$ROOT/a-metadata.json" "$ROOT/b-metadata.json" >/dev/null; then
  echo "Reproducible dependency graph check failed." >&2
  exit 1
fi
if ! diff -u "$ROOT/a-lock.sha" "$ROOT/b-lock.sha" >/dev/null; then
  echo "Lockfile changed between builds." >&2
  exit 1
fi

artifact_manifest() {
  local root="$1"
  (cd "$root" && find release -type f -not -path '*/incremental/*' | sort | while IFS= read -r file; do
    printf '%s  %s\n' "$(shasum -a 256 "$file" | cut -d' ' -f1)" "$file"
  done)
}

artifact_manifest "$ROOT/a" > "$ROOT/a-artifacts.sha"
artifact_manifest "$ROOT/b" > "$ROOT/b-artifacts.sha"
if ! diff -u "$ROOT/a-artifacts.sha" "$ROOT/b-artifacts.sha" >/dev/null; then
  echo "Compiled artifact hashes differ between clean builds." >&2
  diff -u "$ROOT/a-artifacts.sha" "$ROOT/b-artifacts.sha" >&2 || true
  exit 1
fi

scripts/generate-sbom.sh "$ROOT/sbom.json"
shasum -a 256 "$ROOT/sbom.json" > "$ROOT/sbom.sha"
echo "Reproducible build inputs and SBOM match:"
cat "$ROOT/sbom.sha"
