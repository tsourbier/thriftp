#!/usr/bin/env bash
set -euo pipefail

if command -v cargo-license >/dev/null 2>&1; then
  mkdir -p target
  cargo license --avoid-build-deps --avoid-dev-deps --json > target/licenses.json
  python3 - <<'PY'
import json
import re
from pathlib import Path

allowed = {
    "0BSD", "Apache-2.0", "BSD-1-Clause", "BSD-2-Clause", "BSD-3-Clause",
    "CC0-1.0", "ISC", "LGPL-2.1-or-later", "MIT", "MIT-0", "MPL-2.0",
    "Unicode-3.0", "Unlicense", "Zlib", "LLVM-exception",
}
records = json.loads(Path("target/licenses.json").read_text())
bad = []
summary = {}
for record in records:
    license_text = record.get("license") or "UNKNOWN"
    summary[license_text] = summary.get(license_text, 0) + 1
    tokens = set(re.findall(r"[A-Za-z0-9.+-]+", license_text))
    unknown = {token for token in tokens if token not in allowed and token not in {"AND", "OR", "WITH"}}
    if unknown or license_text == "UNKNOWN":
        bad.append((record.get("name"), record.get("version"), license_text, sorted(unknown)))
if bad:
    print("Disallowed or unknown licenses found:")
    for item in bad:
        print(f"  {item[0]} {item[1]}: {item[2]} (unknown: {', '.join(item[3])})")
    raise SystemExit(1)
print(f"License check passed for {len(records)} packages.")
for license_text, count in sorted(summary.items(), key=lambda item: (-item[1], item[0])):
    print(f"  {count:4d}  {license_text}")
PY
else
  echo "cargo-license is not installed."
  echo "Install it with: cargo install cargo-license"
  echo "Dependency metadata preview:"
  cargo metadata --format-version 1 --no-deps >/dev/null
fi
