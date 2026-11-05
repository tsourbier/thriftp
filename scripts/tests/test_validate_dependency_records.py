import json
import shutil
import tempfile
import unittest
from pathlib import Path

import importlib.util

ROOT = Path(__file__).resolve().parents[2]
FEATURE = ROOT / "specs" / "005-dependency-upgrade"
SCRIPT = ROOT / "scripts" / "validate-dependency-records.py"

spec = importlib.util.spec_from_file_location("dependency_records", SCRIPT)
if spec is None or spec.loader is None:
    raise RuntimeError("could not load validator")
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


def inventory(exception_id=None):
    item = {
        "id": "rust/test",
        "name": "test",
        "category": "library",
        "scope": "runtime",
        "relationship": "direct",
        "currentVersion": "1.0.0",
        "targetVersion": "2.0.0",
        "latestStableVersion": "2.0.0",
        "updateType": "major",
        "source": "https://crates.io/crates/test",
        "reason": "latest stable",
        "migrationNotes": "none",
        "license": "MIT",
        "status": "verified",
        "evidence": ["validation.md"],
    }
    if exception_id:
        item["targetVersion"] = "1.5.0"
        item["exceptionId"] = exception_id
    return item


def exception(**overrides):
    value = {
        "id": "EX-001",
        "component": "test",
        "currentVersion": "1.5.0",
        "latestStableVersion": "2.0.0",
        "reason": "peer-conflict",
        "description": "test",
        "risk": "test",
        "mitigation": "test",
        "owner": "maintainer",
        "approvedBy": "reviewer",
        "createdAt": "2026-01-01",
        "expiresAt": "2027-01-01",
        "reviewDate": "2026-06-01",
        "status": "approved",
        "peerConstraint": "peer <2.0.0",
    }
    value.update(overrides)
    return value


def evidence():
    return {
        "id": "E-001",
        "batchId": "batch-1",
        "kind": "test",
        "command": "cargo test",
        "environment": "test",
        "result": "pass",
        "summary": "test",
        "capturedAt": "2026-11-05T00:00:00Z",
    }


class ValidatorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.feature = Path(self.temp.name)
        shutil.copytree(FEATURE / "contracts", self.feature / "contracts")
        self.write([inventory()], [exception()], [evidence()])

    def tearDown(self):
        self.temp.cleanup()

    def write(self, inv, exc, ev):
        (self.feature / "inventory.json").write_text(json.dumps(inv))
        (self.feature / "exceptions.json").write_text(json.dumps(exc))
        (self.feature / "evidence.json").write_text(json.dumps(ev))

    def test_valid_records_pass(self):
        self.assertEqual(validator.validate_feature(self.feature), [])

    def test_missing_required_field_fails(self):
        item = inventory()
        del item["license"]
        self.write([item], [exception()], [evidence()])
        self.assertTrue(any("license" in e for e in validator.validate_feature(self.feature)))

    def test_unapproved_exception_fails_in_release_mode(self):
        self.write([inventory("EX-001")], [exception(status="proposed", approvedBy="")], [evidence()])
        errors = validator.validate_feature(self.feature, release=True)
        self.assertTrue(any("approved" in e for e in errors))

    def test_expired_exception_fails_date_order(self):
        self.write([inventory("EX-001")], [exception(expiresAt="2025-01-01")], [evidence()])
        errors = validator.validate_feature(self.feature)
        self.assertTrue(any("after createdAt" in e for e in errors))

    def test_duplicate_ids_fail(self):
        self.write([inventory(), inventory()], [exception()], [evidence()])
        errors = validator.validate_feature(self.feature)
        self.assertTrue(any("duplicate id" in e for e in errors))

    def test_pre_release_without_exception_fails(self):
        item = inventory()
        item["targetVersion"] = "2.0.0-beta.1"
        item["latestStableVersion"] = "2.0.0-beta.1"
        self.write([item], [exception()], [evidence()])
        errors = validator.validate_feature(self.feature)
        self.assertTrue(any("pre-release" in e for e in errors))

    def test_target_mismatch_without_exception_fails(self):
        item = inventory()
        item["targetVersion"] = "1.5.0"
        self.write([item], [exception()], [evidence()])
        errors = validator.validate_feature(self.feature)
        self.assertTrue(any("differs from latestStableVersion" in e for e in errors))

    def test_unknown_exception_reference_fails(self):
        self.write([inventory("EX-999")], [exception()], [evidence()])
        errors = validator.validate_feature(self.feature)
        self.assertTrue(any("unknown exceptionId" in e for e in errors))

    def test_release_rejects_empty_registers(self):
        self.write([], [], [])
        errors = validator.validate_feature(self.feature, release=True)
        self.assertTrue(any("inventory.json must not be empty" in e for e in errors))
        self.assertTrue(any("evidence.json must not be empty" in e for e in errors))


if __name__ == "__main__":
    unittest.main()
