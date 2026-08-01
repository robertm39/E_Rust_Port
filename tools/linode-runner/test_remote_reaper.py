import importlib.util
import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("remote_reaper.py")
SPEC = importlib.util.spec_from_file_location("remote_reaper", MODULE_PATH)
reaper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(reaper)


class FakeApi:
    def __init__(self, resources):
        self.resources = resources
        self.puts = []
        self.deleted = []

    def get(self, path, allow_404=False):
        value = self.resources.get(path)
        if value is None and not allow_404:
            raise reaper.ReaperError(f"missing {path}")
        return value

    def put(self, path, payload):
        self.puts.append((path, payload))
        resource = self.resources[path]
        resource.update(payload)
        return resource

    def delete(self, path, allow_404=False):
        self.deleted.append(path)
        self.resources.pop(path, None)

    def trusted_now(self):
        return datetime(2026, 8, 1, 12, 58, tzinfo=timezone.utc)


class RemoteReaperTests(unittest.TestCase):
    STATE = {
        "linode_id": 7,
        "firewall_id": 9,
        "label": "e-rust-codex-260801-a1b2",
        "lease_id": "0123456789abcdef0123456789abcdef",
        "delete_at": "2026-08-01T12:58:00+00:00",
    }

    def api(self):
        label = self.STATE["label"]
        return FakeApi(
            {
                "/linode/instances/7": {"id": 7, "label": label},
                "/networking/firewalls/9": {
                    "id": 9,
                    "label": label,
                    "tags": ["preserve-me"],
                },
            }
        )

    def test_reap_marks_firewall_and_deletes_only_assigned_linode(self):
        api = self.api()
        self.assertTrue(reaper.reap(api, dict(self.STATE)))
        self.assertEqual(api.deleted, ["/linode/instances/7"])
        self.assertEqual(len(api.puts), 2)
        for path, payload in api.puts:
            self.assertEqual(path, "/networking/firewalls/9")
            self.assertIn("preserve-me", payload["tags"])
        self.assertIn(
            reaper.marker(self.STATE["lease_id"], "accepted"),
            api.puts[-1][1]["tags"],
        )

    def test_absent_linode_is_an_idempotent_noop(self):
        api = self.api()
        api.resources.pop("/linode/instances/7")
        self.assertFalse(reaper.reap(api, dict(self.STATE)))
        self.assertEqual(api.puts, [])
        self.assertEqual(api.deleted, [])

    def test_wrong_live_label_prevents_every_mutation(self):
        api = self.api()
        api.resources["/linode/instances/7"]["label"] = "production"
        with self.assertRaisesRegex(reaper.ReaperError, "does not match"):
            reaper.reap(api, dict(self.STATE))
        self.assertEqual(api.puts, [])
        self.assertEqual(api.deleted, [])

    def test_early_invocation_is_fail_closed(self):
        api = self.api()
        api.trusted_now = lambda: datetime(
            2026,
            8,
            1,
            12,
            57,
            59,
            tzinfo=timezone.utc,
        )
        with self.assertRaisesRegex(reaper.ReaperError, "before"):
            reaper.reap(api, dict(self.STATE))
        self.assertEqual(api.puts, [])
        self.assertEqual(api.deleted, [])

    def test_invalid_state_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "state.json"
            path.write_text(json.dumps({**self.STATE, "lease_id": "unsafe"}))
            with self.assertRaisesRegex(reaper.ReaperError, "lease ID"):
                reaper.read_state(path)

    def test_nonpositive_resource_id_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "state.json"
            path.write_text(
                json.dumps({**self.STATE, "linode_id": 0}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(reaper.ReaperError, "linode_id"):
                reaper.read_state(path)

    @unittest.skipIf(__import__("os").name == "nt", "POSIX mode check")
    def test_token_file_must_be_private(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "token"
            path.write_text("secret", encoding="utf-8")
            path.chmod(0o644)
            with self.assertRaisesRegex(reaper.ReaperError, "not private"):
                reaper.read_secret(path)


if __name__ == "__main__":
    unittest.main()
