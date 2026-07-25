from __future__ import annotations

import importlib.util
import json
import tarfile
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("linode_runner.py")
SPEC = importlib.util.spec_from_file_location("linode_runner", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class FakeApi:
    def __init__(self, resources):
        self.resources = resources
        self.deleted = []

    def get(self, path, allow_404=False):
        return self.resources.get(path)

    def delete(self, path, allow_404=False):
        self.deleted.append(path)
        self.resources.pop(path, None)


class PayloadTests(unittest.TestCase):
    def test_generated_resource_label_fits_firewall_limit(self):
        label = runner.resource_label(runner.run_id())
        self.assertTrue(label.startswith("e-rust-codex-"))
        self.assertLessEqual(len(label), 32)

    def test_firewall_allows_only_ssh_from_controller(self):
        payload = runner.firewall_payload("e-rust-codex-260724-a1b2c3", "192.0.2.7/32")
        self.assertEqual(payload["rules"]["inbound_policy"], "DROP")
        self.assertEqual(payload["rules"]["outbound_policy"], "ACCEPT")
        self.assertEqual(payload["rules"]["outbound"], [])
        self.assertEqual(payload["rules"]["inbound"][0]["ports"], "22")
        self.assertEqual(
            payload["rules"]["inbound"][0]["addresses"]["ipv4"],
            ["192.0.2.7/32"],
        )

    def test_linode_request_disables_extras_and_attaches_firewall(self):
        payload = runner.linode_payload(
            "e-rust-codex-260724-a1b2c3", 41, "ssh-ed25519 AAAA test"
        )
        self.assertEqual(payload["type"], "g8-dedicated-8-4")
        self.assertEqual(payload["region"], "us-ord")
        self.assertEqual(payload["image"], "linode/ubuntu24.04")
        self.assertEqual(payload["firewall_id"], 41)
        self.assertFalse(payload["backups_enabled"])
        self.assertEqual(payload["interface_generation"], "legacy_config")
        self.assertEqual(payload["disk_encryption"], "enabled")

    def test_catalog_accepts_current_direct_availability_list(self):
        class CatalogApi:
            def get(self, path):
                if path.startswith("/linode/types/"):
                    return {"memory": 8192, "vcpus": 4, "disk": 83968}
                if path.endswith("/availability"):
                    return [
                        {
                            "plan": "g8-dedicated-8-4",
                            "available": True,
                        }
                    ]
                return {}

        runner.validate_catalog(
            CatalogApi(),
            "g8-dedicated-8-4",
            "us-ord",
            "linode/ubuntu24.04",
        )


class SafetyTests(unittest.TestCase):
    def test_rejects_live_resource_with_different_label(self):
        state = {
            "run_id": "260724-120000-a1b2c3",
            "label": "e-rust-codex-260724-a1b2c3",
            "linode_id": 7,
        }
        api = FakeApi(
            {
                "/linode/instances/7": {
                    "id": 7,
                    "label": "production-do-not-delete",
                }
            }
        )
        with self.assertRaisesRegex(runner.RunnerError, "Refusing to delete"):
            runner.delete_state_resources(api, state)
        self.assertEqual(api.deleted, [])

    def test_rejects_unmanaged_saved_label(self):
        api = FakeApi({})
        with self.assertRaisesRegex(runner.RunnerError, "unmanaged label"):
            runner.delete_state_resources(
                api,
                {"run_id": "x", "label": "production"},
            )

    def test_stale_filter_requires_prefix_and_age(self):
        old = (datetime.now(timezone.utc) - timedelta(hours=8)).isoformat()
        new = datetime.now(timezone.utc).isoformat()
        values = [
            {"id": 1, "label": "e-rust-codex-old", "created": old},
            {"id": 2, "label": "production", "created": old},
            {"id": 3, "label": "e-rust-codex-new", "created": new},
        ]
        cutoff = datetime.now(timezone.utc) - timedelta(hours=6)
        self.assertEqual(
            [item["id"] for item in runner.managed_older_than(values, cutoff)],
            [1],
        )


class SnapshotTests(unittest.TestCase):
    def test_snapshot_includes_sources_and_excludes_generated_and_secrets(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            (root / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
            (root / "src").mkdir()
            (root / "src" / "lib.rs").write_text("pub fn x() {}\n", encoding="utf-8")
            (root / "eprover").mkdir()
            (root / "eprover" / "main.c").write_text("int main() {}\n", encoding="utf-8")
            (root / "target").mkdir()
            (root / "target" / "binary").write_bytes(b"large")
            (root / ".git").mkdir()
            (root / ".git" / "config").write_text("secret", encoding="utf-8")
            (root / ".claude").mkdir()
            (root / ".claude" / "settings.json").write_text(
                "local state", encoding="utf-8"
            )
            (root / ".beads-credential-key").write_text("secret", encoding="utf-8")
            archive_path = Path(temporary) / "source.tar.gz"
            with mock.patch.object(runner, "snapshot_metadata", return_value={}):
                metadata = runner.create_snapshot(root, archive_path)
            with tarfile.open(archive_path, "r:gz") as archive:
                names = {name.removeprefix("./") for name in archive.getnames()}
            self.assertIn("Cargo.toml", names)
            self.assertIn("src/lib.rs", names)
            self.assertIn("eprover/main.c", names)
            self.assertIn(".linode-snapshot.json", names)
            self.assertNotIn("target/binary", names)
            self.assertNotIn(".git/config", names)
            self.assertNotIn(".claude/settings.json", names)
            self.assertNotIn(".beads-credential-key", names)
            self.assertEqual(metadata["file_count"], 3)

    def test_safe_extract_rejects_parent_traversal(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "bad.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                info = tarfile.TarInfo("../escape")
                info.size = 0
                archive.addfile(info)
            with self.assertRaisesRegex(runner.RunnerError, "unsafe"):
                runner.safe_extract(archive_path, Path(temporary) / "out")


if __name__ == "__main__":
    unittest.main()
