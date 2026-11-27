"""Deterministic guard tests; these do not simulate a successful deployment."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("update_dev", Path(__file__).with_name("update-dev.py"))
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


class DeploymentGuards(unittest.TestCase):
    def test_matching_baseline_is_accepted(self):
        updater.verify_baseline({"01.sql": "a"}, {"01.sql": "a"})

    def test_another_branch_database_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "baseline mismatch"):
            updater.verify_baseline({"01.sql": "a"}, {"01.sql": "a", "other.sql": "x"})

    def test_rewritten_applied_migration_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "baseline mismatch"):
            updater.verify_baseline({"01.sql": "b"}, {"01.sql": "a"})

    def image(self):
        return {"Config": {"Labels": {"io.qvis.staging.ci": "passed",
                "org.opencontainers.image.revision": "a" * 40}},
                "RepoDigests": ["ghcr.io/example/backend@sha256:" + "b" * 64]}

    def test_immutable_image_is_selected(self):
        self.assertEqual(updater.image_reference(self.image(), "ghcr.io/example/backend"),
                         "ghcr.io/example/backend@sha256:" + "b" * 64)

    def test_missing_ci_marker_is_rejected(self):
        image = self.image()
        del image["Config"]["Labels"]["io.qvis.staging.ci"]
        with self.assertRaises(KeyError):
            updater.image_reference(image, "ghcr.io/example/backend")

    def test_mutable_revision_is_rejected(self):
        image = self.image()
        image["Config"]["Labels"]["org.opencontainers.image.revision"] = "dev"
        with self.assertRaises(ValueError):
            updater.image_reference(image, "ghcr.io/example/backend")

    def test_wrong_registry_is_rejected(self):
        with self.assertRaises(ValueError):
            updater.image_reference(self.image(), "ghcr.io/other/backend")

    def test_malformed_digest_is_rejected(self):
        image = self.image()
        image["RepoDigests"] = ["ghcr.io/example/backend@sha256:short"]
        with self.assertRaises(ValueError):
            updater.image_reference(image, "ghcr.io/example/backend")
