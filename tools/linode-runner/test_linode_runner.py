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


class ProvisionApi:
    def __init__(self):
        self.posts = []
        self.last_response_at = datetime(2026, 7, 27, 15, tzinfo=timezone.utc)

    def get(self, path, allow_404=False):
        if path.startswith("/linode/types/"):
            linode_type = path.rsplit("/", 1)[-1]
            spec = runner.PLAN_SPECS[linode_type]
            return {
                "memory": spec.memory,
                "vcpus": spec.vcpus,
                "disk": spec.disk,
                "class": spec.plan_class,
            }
        if path.endswith("/availability"):
            return [
                {"plan": linode_type, "available": True}
                for linode_type in runner.PLAN_SPECS
            ]
        return {}

    def list_all(self, path):
        return []

    def trusted_now(self):
        return self.last_response_at

    def post(self, path, payload):
        self.posts.append((path, payload))
        if path == "/networking/firewalls":
            return {"id": 41}
        if path == "/linode/instances":
            return {
                "id": 7,
                "ipv4": ["192.0.2.8"],
                "created": "2026-07-27T14:59:00",
            }
        raise AssertionError(path)


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


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

    def test_high_memory_linode_request_uses_guarded_plan(self):
        payload = runner.linode_payload(
            "e-rust-codex-260724-a1b2c3",
            41,
            "ssh-ed25519 AAAA test",
            linode_type=runner.HIGH_MEMORY_TYPE,
        )
        self.assertEqual(payload["type"], "g7-highmem-8")

    def test_catalog_accepts_both_supported_plans(self):
        class CatalogApi:
            def get(self, path):
                if path.startswith("/linode/types/"):
                    linode_type = path.rsplit("/", 1)[-1]
                    spec = runner.PLAN_SPECS[linode_type]
                    return {
                        "memory": spec.memory,
                        "vcpus": spec.vcpus,
                        "disk": spec.disk,
                        "class": spec.plan_class,
                    }
                if path.endswith("/availability"):
                    return [
                        {
                            "plan": linode_type,
                            "available": True,
                        }
                        for linode_type in runner.PLAN_SPECS
                    ]
                return {}

        for linode_type in runner.PLAN_SPECS:
            runner.validate_catalog(
                CatalogApi(),
                linode_type,
                "us-ord",
                "linode/ubuntu24.04",
            )

    def test_catalog_rejects_high_memory_resource_drift(self):
        class CatalogApi:
            def get(self, path):
                if path.startswith("/linode/types/"):
                    return {
                        "memory": 153600,
                        "vcpus": 4,
                        "disk": 204800,
                        "class": "highmem",
                    }
                return {}

        with self.assertRaisesRegex(runner.RunnerError, "catalog values"):
            runner.validate_catalog(
                CatalogApi(),
                runner.HIGH_MEMORY_TYPE,
                "us-ord",
                "linode/ubuntu24.04",
            )

    def test_bootstrap_installs_remote_quality_and_cross_compile_toolchain(self):
        script = runner.bootstrap_script()

        self.assertIn("gcc-mingw-w64-x86-64", script)
        self.assertIn("rustup component add rustfmt clippy", script)
        self.assertIn("rustup target add x86_64-pc-windows-gnu", script)
        self.assertIn("x86_64-w64-mingw32-gcc --version", script)

    def test_remote_workload_contains_comprehensive_remote_only_gates(self):
        script = MODULE_PATH.with_name("remote_run.sh").read_text(encoding="utf-8")

        self.assertIn("cargo test --locked --all-targets --all-features", script)
        self.assertIn("cargo clippy --locked --all-targets --all-features", script)
        self.assertIn("canonical_binaries=(", script)
        self.assertIn("umlaut-tsm-classify", script)
        self.assertIn('test -x "$rust_bin_dir/$binary"', script)
        self.assertIn('test -f "$windows_bin_dir/$binary.exe"', script)
        self.assertIn("--target x86_64-pc-windows-gnu", script)
        self.assertIn('"$compat_driver" compare', script)
        self.assertIn('"$compat_driver" compare-tools', script)
        self.assertIn('"$compat_driver" benchmark', script)
        self.assertIn("VALIDATION_COMPLETE", script)
        self.assertIn("SUCCESS", script)
        self.assertIn("no Windows binary was executed", script)


class PlanSelectionTests(unittest.TestCase):
    def test_default_and_high_memory_cli_selection(self):
        self.assertEqual(
            runner.parser().parse_args(["run"]).linode_type,
            runner.DEFAULT_TYPE,
        )
        self.assertEqual(
            runner.parser().parse_args(["run", "--high-memory"]).linode_type,
            runner.HIGH_MEMORY_TYPE,
        )

    def test_raw_high_memory_type_uses_same_plan(self):
        arguments = runner.parser().parse_args(
            ["up", "--type", runner.HIGH_MEMORY_TYPE]
        )
        self.assertEqual(arguments.linode_type, runner.HIGH_MEMORY_TYPE)

    def test_high_memory_and_raw_type_are_mutually_exclusive(self):
        with (
            mock.patch("sys.stderr"),
            self.assertRaises(SystemExit),
        ):
            runner.parser().parse_args(
                ["check", "--high-memory", "--type", runner.DEFAULT_TYPE]
            )


class TrustedTimeTests(unittest.TestCase):
    def test_http_date_is_parsed_as_utc(self):
        self.assertEqual(
            runner.parse_http_date("Mon, 27 Jul 2026 22:22:15 GMT"),
            datetime(2026, 7, 27, 22, 22, 15, tzinfo=timezone.utc),
        )

    def test_trusted_now_uses_linode_response_header(self):
        response = mock.MagicMock()
        response.read.return_value = b""
        response.headers = {"Date": "Mon, 27 Jul 2026 22:22:15 GMT"}
        response.__enter__.return_value = response
        with mock.patch.object(runner.urllib.request, "urlopen", return_value=response):
            api = runner.LinodeApi(token="test")
            self.assertEqual(
                api.trusted_now(),
                datetime(2026, 7, 27, 22, 22, 15, tzinfo=timezone.utc),
            )

    def test_missing_api_date_fails_closed(self):
        response = mock.MagicMock()
        response.read.return_value = b""
        response.headers = {}
        response.__enter__.return_value = response
        with mock.patch.object(runner.urllib.request, "urlopen", return_value=response):
            api = runner.LinodeApi(token="test")
            with self.assertRaisesRegex(runner.RunnerError, "trusted UTC time"):
                api.trusted_now()


class HighMemoryUsageTests(unittest.TestCase):
    NOW = datetime(2026, 7, 27, 15, tzinfo=timezone.utc)

    def usage(self, history: Path, current: Path | None = None, active=()):
        return runner.high_memory_usage(
            self.NOW,
            history_root=history,
            current_state_path=current or history.parent / "current.json",
            active_linodes=active,
        )

    def test_summer_accounting_day_still_starts_at_0500_utc(self):
        day_start, next_reset = runner.fixed_est_day_bounds(self.NOW)
        self.assertEqual(
            day_start,
            datetime(2026, 7, 27, 5, tzinfo=timezone.utc),
        )
        self.assertEqual(
            next_reset,
            datetime(2026, 7, 28, 5, tzinfo=timezone.utc),
        )

    def test_usage_counts_only_current_day_high_memory_overlap(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            write_json(
                history / "cross-midnight.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T04:30:00+00:00",
                    "linode_deleted_at": "2026-07-27T05:30:00+00:00",
                },
            )
            write_json(
                history / "prior-day.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 2,
                    "linode_created_at": "2026-07-27T03:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T05:00:00+00:00",
                },
            )
            write_json(
                history / "normal.json",
                {
                    "type": runner.DEFAULT_TYPE,
                    "linode_id": 3,
                    "created_at": "broken but irrelevant",
                    "deleted_at": "also irrelevant",
                },
            )
            usage = self.usage(history)
            self.assertEqual(usage.actual, timedelta(minutes=30))
            self.assertEqual(usage.carried_in, timedelta(minutes=30))
            self.assertEqual(usage.used, timedelta(hours=1))
            self.assertFalse(usage.exhausted)

    def test_exactly_two_hours_blocks_new_start(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            write_json(
                history / "two-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T08:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T10:00:00+00:00",
                },
            )
            usage = self.usage(history)
            self.assertEqual(usage.used, timedelta(hours=2))
            self.assertTrue(usage.exhausted)
            with self.assertRaisesRegex(runner.RunnerError, "two-hour daily limit"):
                runner.require_high_memory_allowance(usage)

    def test_prior_day_overflow_counts_as_current_day_usage(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            write_json(
                history / "prior-overflow.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-26T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-26T09:00:00+00:00",
                },
            )
            usage = self.usage(history)
            self.assertEqual(usage.actual, timedelta())
            self.assertEqual(usage.carried_in, timedelta(hours=1))
            self.assertEqual(usage.used, timedelta(hours=1))
            self.assertEqual(usage.remaining, timedelta(hours=1))

    def test_large_overflow_rolls_forward_until_daily_allowances_absorb_it(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            write_json(
                history / "large-overflow.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-26T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-26T11:00:00+00:00",
                },
            )
            usage = runner.high_memory_usage(
                datetime(2026, 7, 28, 15, tzinfo=timezone.utc),
                history_root=history,
                current_state_path=Path(temporary) / "current.json",
            )
            self.assertEqual(usage.actual, timedelta())
            self.assertEqual(usage.carried_in, timedelta(hours=1))
            self.assertEqual(usage.used, timedelta(hours=1))

    def test_projected_eligibility_accounts_for_carried_overflow(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            write_json(
                history / "large-overflow.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-26T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-26T11:00:00+00:00",
                },
            )
            usage = self.usage(history)
            self.assertEqual(usage.used, timedelta(hours=3))
            self.assertTrue(usage.exhausted)
            self.assertEqual(
                usage.projected_eligible_at,
                datetime(2026, 7, 28, 5, tzinfo=timezone.utc),
            )

    def test_live_managed_high_memory_is_counted(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            usage = self.usage(
                history,
                active=[
                    {
                        "id": 7,
                        "label": "e-rust-codex-260727-a1b2c3",
                        "type": runner.HIGH_MEMORY_TYPE,
                        "created": "2026-07-27T14:00:00",
                    }
                ],
            )
            self.assertEqual(usage.used, timedelta(hours=1))

    def test_malformed_history_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            history.mkdir()
            (history / "broken.json").write_text("{", encoding="utf-8")
            with self.assertRaisesRegex(
                runner.RunnerError,
                "Could not verify high-memory usage",
            ):
                self.usage(history)


class ProvisionGuardTests(unittest.TestCase):
    def provision_patches(self, temporary: str):
        root = Path(temporary)
        return (
            mock.patch.object(runner, "RUN_HISTORY", root / "runs"),
            mock.patch.object(runner, "CURRENT_STATE", root / "current.json"),
            mock.patch.object(runner, "command_path", return_value="tool"),
            mock.patch.object(runner, "ensure_ssh_key"),
            mock.patch.object(runner, "read_public_key", return_value="ssh-ed25519 test"),
            mock.patch.object(runner, "wait_for_linode"),
            mock.patch.object(runner, "wait_for_ssh"),
            mock.patch.object(runner, "bootstrap"),
        )

    def test_exhausted_high_memory_guard_precedes_resource_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "two-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T08:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T10:00:00+00:00",
                },
            )
            api = ProvisionApi()
            patches = self.provision_patches(temporary)
            with patches[0], patches[1]:
                with self.assertRaisesRegex(
                    runner.RunnerError,
                    "two-hour daily limit",
                ):
                    runner.provision(
                        api,
                        allow_ip="192.0.2.10",
                        linode_type=runner.HIGH_MEMORY_TYPE,
                    )
            self.assertEqual(api.posts, [])

    def test_high_memory_check_returns_nonzero_at_daily_limit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "two-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T08:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T10:00:00+00:00",
                },
            )
            api = ProvisionApi()
            patches = self.provision_patches(temporary)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                mock.patch.object(runner, "LinodeApi", return_value=api),
                mock.patch("sys.stderr"),
            ):
                exit_code = runner.main(
                    [
                        "check",
                        "--high-memory",
                        "--allow-ip",
                        "192.0.2.10",
                    ]
                )
            self.assertEqual(exit_code, 1)
            self.assertEqual(api.posts, [])

    def test_high_memory_history_does_not_block_normal_profile(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "two-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T08:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T10:00:00+00:00",
                },
            )
            api = ProvisionApi()
            patches = self.provision_patches(temporary)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                patches[5] as wait_for_linode,
                patches[6],
                patches[7],
            ):
                wait_for_linode.return_value = {
                    "id": 7,
                    "ipv4": ["192.0.2.8"],
                }
                state = runner.provision(
                    api,
                    allow_ip="192.0.2.10",
                    linode_type=runner.DEFAULT_TYPE,
                )
            self.assertEqual(state["type"], runner.DEFAULT_TYPE)
            self.assertEqual(
                [path for path, _payload in api.posts],
                ["/networking/firewalls", "/linode/instances"],
            )

    def test_high_memory_provision_records_trusted_creation_time(self):
        with tempfile.TemporaryDirectory() as temporary:
            api = ProvisionApi()
            patches = self.provision_patches(temporary)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                patches[5] as wait_for_linode,
                patches[6],
                patches[7],
            ):
                wait_for_linode.return_value = {
                    "id": 7,
                    "ipv4": ["192.0.2.8"],
                }
                state = runner.provision(
                    api,
                    allow_ip="192.0.2.10",
                    linode_type=runner.HIGH_MEMORY_TYPE,
                )
            self.assertEqual(
                state["linode_created_at"],
                "2026-07-27T14:59:00+00:00",
            )


class DocumentationTests(unittest.TestCase):
    def test_runbook_contains_high_memory_cost_and_casc_policy(self):
        repo_root = MODULE_PATH.parents[2]
        runbook = (repo_root / "docs" / "linode-runner.md").read_text(
            encoding="utf-8"
        )
        agent_docs = (repo_root / "DOCS.md").read_text(encoding="utf-8")
        claude = (repo_root / "CLAUDE.md").read_text(encoding="utf-8")
        for document in (runbook, agent_docs, claude):
            self.assertIn("$0.14 an hour", document)
            self.assertIn("$0.74 an hour", document)
            self.assertIn("--high-memory", document)
            self.assertIn("CASC", document)
            self.assertIn("--memory-limit=131072", document)
        self.assertIn("fixed UTC-05:00", runbook)
        self.assertIn("at least two hours", runbook)
        self.assertIn("overflow", runbook.lower())


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

    def test_high_memory_cleanup_archives_trusted_deletion_time(self):
        class CleanupApi(FakeApi):
            def __init__(self, resources):
                super().__init__(resources)
                self.last_response_at = datetime(
                    2026,
                    7,
                    27,
                    16,
                    tzinfo=timezone.utc,
                )

            def get(self, path, allow_404=False):
                return self.resources.get(path)

            def delete(self, path, allow_404=False):
                super().delete(path, allow_404=allow_404)
                self.last_response_at = datetime(
                    2026,
                    7,
                    27,
                    16,
                    1,
                    tzinfo=timezone.utc,
                )

        state = {
            "run_id": "260727-120000-a1b2c3",
            "label": "e-rust-codex-260727-a1b2c3",
            "linode_id": 7,
            "type": runner.HIGH_MEMORY_TYPE,
            "linode_created_at": "2026-07-27T14:00:00+00:00",
        }
        api = CleanupApi(
            {
                "/linode/instances/7": {
                    "id": 7,
                    "label": state["label"],
                }
            }
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with (
                mock.patch.object(runner, "RUN_HISTORY", root / "runs"),
                mock.patch.object(runner, "CURRENT_STATE", root / "current.json"),
            ):
                runner.delete_state_resources(api, state)
                archived = json.loads(
                    (root / "runs" / f"{state['run_id']}.json").read_text(
                        encoding="utf-8"
                    )
                )
        self.assertEqual(
            archived["linode_deleted_at"],
            "2026-07-27T16:01:00+00:00",
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
