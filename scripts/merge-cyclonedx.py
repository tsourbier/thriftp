#!/usr/bin/env python3
"""Merge per-crate CycloneDX documents into one workspace SBOM."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: merge-cyclonedx.py <output> <input>...", file=sys.stderr)
        return 2
    output = Path(argv[1])
    inputs = [Path(value) for value in argv[2:]]
    documents = [json.loads(path.read_text(encoding="utf-8")) for path in inputs]
    if not documents:
        print("no input SBOMs", file=sys.stderr)
        return 1

    components = {}
    dependencies = {}
    for document in documents:
        for component in document.get("components", []):
            ref = component.get("bom-ref") or f"{component.get('name')}@{component.get('version')}"
            components[ref] = component
        for dependency in document.get("dependencies", []):
            ref = dependency.get("ref")
            if not ref:
                continue
            existing = dependencies.setdefault(ref, {"ref": ref, "dependsOn": []})
            for child in dependency.get("dependsOn", []):
                if child not in existing["dependsOn"]:
                    existing["dependsOn"].append(child)

    merged = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "thriftp-workspace",
                "name": "thriftp",
                "version": "0.1.0",
            }
        },
        "components": sorted(components.values(), key=lambda item: (item.get("name", ""), item.get("version", ""))),
        "dependencies": sorted(dependencies.values(), key=lambda item: item["ref"]),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(merged, indent=2) + "\n", encoding="utf-8")
    print(f"merged {len(components)} components from {len(documents)} SBOMs into {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
