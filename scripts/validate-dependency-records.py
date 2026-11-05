#!/usr/bin/env python3
"""Validate dependency inventory, exception, and evidence records.

This intentionally implements the small JSON Schema subset used by the
contracts in ``specs/005-dependency-upgrade/contracts`` using only the Python
standard library. It does not add a package dependency to the project.
"""

from __future__ import annotations

import json
import re
import sys
from datetime import date
from pathlib import Path
from typing import Any

SCHEMA_NAMES = {
    "inventory": "dependency-inventory.schema.json",
    "exceptions": "upgrade-exception.schema.json",
    "evidence": "verification-evidence.schema.json",
}


class ValidationFailure(Exception):
    pass


def _matches_if(schema: dict[str, Any], value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    properties = schema.get("properties", {})
    for key, expected in properties.items():
        if key not in value:
            return False
        if "const" in expected and value[key] != expected["const"]:
            return False
        if "enum" in expected and value[key] not in expected["enum"]:
            return False
    return True


def _type_matches(expected: str, value: Any) -> bool:
    return {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "boolean": isinstance(value, bool),
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "number": isinstance(value, (int, float)) and not isinstance(value, bool),
    }.get(expected, True)


def validate(schema: dict[str, Any], value: Any, path: str = "$") -> None:
    expected_type = schema.get("type")
    if expected_type and not _type_matches(expected_type, value):
        raise ValidationFailure(f"{path}: expected {expected_type}")

    if "enum" in schema and value not in schema["enum"]:
        raise ValidationFailure(f"{path}: value {value!r} is not in {schema['enum']}")

    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            raise ValidationFailure(f"{path}: shorter than {schema['minLength']}")
        if "pattern" in schema and not re.search(schema["pattern"], value):
            raise ValidationFailure(f"{path}: does not match {schema['pattern']}")

    if isinstance(value, dict):
        for required in schema.get("required", []):
            if required not in value:
                raise ValidationFailure(f"{path}: missing required field {required!r}")
        properties = schema.get("properties", {})
        for key, item in value.items():
            if key in properties:
                validate(properties[key], item, f"{path}.{key}")
            elif schema.get("additionalProperties") is False:
                raise ValidationFailure(f"{path}: unexpected field {key!r}")

    if isinstance(value, list):
        if schema.get("uniqueItems"):
            seen = []
            for item in value:
                if item in seen:
                    raise ValidationFailure(f"{path}: duplicate array item")
                seen.append(item)
        item_schema = schema.get("items")
        if item_schema:
            for index, item in enumerate(value):
                validate(item_schema, item, f"{path}[{index}]")

    for subschema in schema.get("allOf", []):
        if "if" in subschema:
            if _matches_if(subschema["if"], value):
                if "then" in subschema:
                    validate(subschema["then"], value, path)
        else:
            validate(subschema, value, path)


def _parse_date(value: str, path: str) -> date:
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        raise ValidationFailure(f"{path}: expected an ISO date: {error}") from error


def _read_array(path: Path) -> list[Any]:
    if not path.is_file():
        raise ValidationFailure(f"missing required file: {path}")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValidationFailure(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, list):
        raise ValidationFailure(f"{path}: expected a JSON array")
    return value


def validate_feature(feature_dir: Path, release: bool = False) -> list[str]:
    schema_dir = feature_dir / "contracts"
    schemas = {}
    for key, name in SCHEMA_NAMES.items():
        schema_path = schema_dir / name
        if not schema_path.is_file():
            raise ValidationFailure(f"missing schema: {schema_path}")
        schemas[key] = json.loads(schema_path.read_text(encoding="utf-8"))

    records = {
        key: _read_array(feature_dir / f"{key}.json") for key in SCHEMA_NAMES
    }

    errors: list[str] = []
    for key, items in records.items():
        seen = set()
        for index, item in enumerate(items):
            try:
                validate(schemas[key], item, f"{key}[{index}]")
            except ValidationFailure as error:
                errors.append(str(error))
            item_id = item.get("id") if isinstance(item, dict) else None
            if item_id in seen:
                errors.append(f"{key}[{index}]: duplicate id {item_id!r}")
            seen.add(item_id)

    exception_ids = {item.get("id") for item in records["exceptions"]}
    for index, item in enumerate(records["inventory"]):
        if not isinstance(item, dict):
            continue
        exception_id = item.get("exceptionId")
        if exception_id and exception_id not in exception_ids:
            errors.append(f"inventory[{index}]: unknown exceptionId {exception_id!r}")
        target = str(item.get("targetVersion", ""))
        latest = str(item.get("latestStableVersion", ""))
        if target != latest and not exception_id:
            errors.append(
                f"inventory[{index}]: targetVersion differs from latestStableVersion without exceptionId"
            )
        if re.search(r"(alpha|beta|rc|nightly)", target, re.IGNORECASE) and not exception_id:
            errors.append(
                f"inventory[{index}]: pre-release target requires an approved exception"
            )

    for index, item in enumerate(records["exceptions"]):
        if not isinstance(item, dict):
            continue
        try:
            created = _parse_date(str(item.get("createdAt", "")), f"exceptions[{index}].createdAt")
            expires = _parse_date(str(item.get("expiresAt", "")), f"exceptions[{index}].expiresAt")
            review = _parse_date(str(item.get("reviewDate", "")), f"exceptions[{index}].reviewDate")
        except ValidationFailure as error:
            errors.append(str(error))
            continue
        if expires <= created:
            errors.append(f"exceptions[{index}]: expiresAt must be after createdAt")
        if review > expires:
            errors.append(f"exceptions[{index}]: reviewDate must not be after expiresAt")
        if item.get("status") == "proposed" and item.get("approvedBy"):
            errors.append(f"exceptions[{index}]: proposed exception cannot have approvedBy")
        if release and item.get("status") not in {"approved", "closed"}:
            errors.append(f"exceptions[{index}]: release requires approved or closed status")

    if release:
        if not records["inventory"]:
            errors.append("release: inventory.json must not be empty")
        if not records["evidence"]:
            errors.append("release: evidence.json must not be empty")

    return errors


def main(argv: list[str]) -> int:
    args = [arg for arg in argv if not arg.startswith("--")]
    release = "--release" in argv
    if len(args) != 1:
        print(
            "usage: validate-dependency-records.py <feature-dir> [--release]",
            file=sys.stderr,
        )
        return 2
    feature_dir = Path(args[0]).resolve()
    try:
        errors = validate_feature(feature_dir, release=release)
    except ValidationFailure as error:
        print(f"FAIL: {error}")
        return 1
    if errors:
        print(f"FAIL: {len(errors)} validation error(s)")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("PASS: dependency records are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
