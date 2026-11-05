import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import dependency_resolution as resolution


class DependencyResolutionTests(unittest.TestCase):
    def test_duplicate_versions_are_consolidated(self):
        self.assertEqual(
            resolution.consolidate_versions(["1.2.0", "1.2.0", "1.3.0", "2.0.0-beta.1"]),
            ["1.3.0", "1.2.0"],
        )

    def test_pre_release_versions_are_rejected(self):
        self.assertTrue(resolution.is_pre_release("2.0.0-rc.1"))
        self.assertEqual(resolution.select_latest_usable(["1.0.0", "2.0.0-beta.1"]), "1.0.0")

    def test_yanked_package_falls_back_to_next_stable(self):
        self.assertEqual(
            resolution.select_latest_usable(["1.0.0", "1.1.0", "1.2.0"], {"1.2.0"}),
            "1.1.0",
        )

    def test_rollback_gating_never_releases_red_checks(self):
        self.assertTrue(resolution.rollback_allowed(True, True))
        self.assertFalse(resolution.rollback_allowed(False, True))
        self.assertTrue(resolution.rollback_allowed(False, False))


if __name__ == "__main__":
    unittest.main()
