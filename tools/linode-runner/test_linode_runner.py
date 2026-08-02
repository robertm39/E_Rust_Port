from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from io import StringIO
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


def package_maintenance_record() -> dict:
    return {
        "schema_version": 1,
        "captured_at": "2026-08-01T08:00:00+00:00",
        "cloud_init_wait_completed": True,
        "units": {
            unit: {
                "active_state": "inactive",
                "unit_file_state": "masked",
            }
            for unit in runner.PACKAGE_MAINTENANCE_UNITS
        },
    }


def package_maintenance_state() -> dict:
    return {
        "path": str(runner.PACKAGE_MAINTENANCE_RECORD),
        "sha256": "a" * 64,
        "record": package_maintenance_record(),
    }


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


class ExplicitTransferTests(unittest.TestCase):
    def test_transfer_commands_parse_explicit_file_paths(self):
        upload = runner.parser().parse_args(
            ["upload", "local.bin", "/root/reference.bin"]
        )
        self.assertEqual(upload.local_path, Path("local.bin"))
        self.assertEqual(upload.remote_path, "/root/reference.bin")
        download = runner.parser().parse_args(
            ["download", "/root/results.tar.gz", "results.tar.gz", "--overwrite"]
        )
        self.assertTrue(download.overwrite)

    def test_remote_transfer_path_is_conservative_and_absolute(self):
        self.assertEqual(
            runner.validate_remote_file_path("/root/casc-30/results.tar.gz"),
            "/root/casc-30/results.tar.gz",
        )
        for invalid in [
            "relative/file",
            "/root/../etc/passwd",
            "/root/a file",
            "/root/file;touch-x",
        ]:
            with self.subTest(invalid=invalid):
                with self.assertRaises(runner.RunnerError):
                    runner.validate_remote_file_path(invalid)

    def test_upload_destination_must_be_disposable_root_content(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "reference.bin"
            source.write_bytes(b"reference")
            for destination in ("/etc/reference.bin", "/root/.ssh/key", "/root"):
                with self.subTest(destination=destination), self.assertRaisesRegex(
                    runner.RunnerError,
                    "beneath /root",
                ):
                    runner.upload_file(
                        {"ipv4": "192.0.2.1"},
                        source,
                        destination,
                    )

    def test_upload_requires_file_and_uses_scp_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "reference.bin"
            source.write_bytes(b"reference")
            with mock.patch.object(runner, "scp_to") as scp, mock.patch.object(
                runner,
                "save_current",
            ):
                state = {"ipv4": "192.0.2.1"}
                runner.upload_file(state, source, "/root/reference.bin")
            self.assertEqual(
                scp.call_args.args,
                (state, source.resolve(), "/root/reference.bin"),
            )
            self.assertEqual(state["uploaded_files"], ["/root/reference.bin"])

    def test_download_refuses_implicit_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "results.tar.gz"
            destination.write_bytes(b"existing")
            with (
                mock.patch.object(runner, "scp_from"),
                self.assertRaisesRegex(runner.RunnerError, "already exists"),
            ):
                runner.download_file(
                    {"ipv4": "192.0.2.1"},
                    "/root/results.tar.gz",
                    destination,
                    overwrite=False,
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

    @unittest.skipUnless(os.name == "nt", "requires Windows PowerShell")
    def test_powershell_wrapper_preserves_successful_native_stderr(self):
        wrapper = MODULE_PATH.parents[2] / "linode-runner.ps1"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            copied_wrapper = root / "linode-runner.ps1"
            copied_wrapper.write_bytes(wrapper.read_bytes())
            controller = root / "tools" / "linode-runner" / "linode_runner.py"
            controller.parent.mkdir(parents=True)
            controller.write_text(
                "import sys\n"
                "print('controller stdout')\n"
                "print('Created symlink test', file=sys.stderr)\n",
                encoding="utf-8",
            )
            local_app_data = root / "local-app-data"
            secret = local_app_data / "E-Rust-Port" / "linode-token.dpapi"
            secret.parent.mkdir(parents=True)
            setup = (
                "$secure=ConvertTo-SecureString 'test-token' "
                "-AsPlainText -Force; $secure | ConvertFrom-SecureString | "
                f"Set-Content -LiteralPath '{secret}'; "
                f"& '{copied_wrapper}' status; exit $LASTEXITCODE"
            )
            environment = os.environ.copy()
            environment["LOCALAPPDATA"] = str(local_app_data)
            result = subprocess.run(
                [
                    "powershell.exe",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    setup,
                ],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=environment,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("controller stdout", result.stdout)
        self.assertIn("Created symlink test", result.stderr)


class InterruptedBootstrapRecoveryTests(unittest.TestCase):
    def state(self) -> dict:
        return {
            "run_id": "260802-050016-76d3",
            "label": "e-rust-codex-260802-050016-76d3",
            "lifecycle": "active",
            "phase": "bootstrapping",
            "linode_id": 102066534,
            "firewall_id": 100863634,
            "ipv4": "192.0.2.8",
            "type": runner.HIGH_MEMORY_TYPE,
            "region": "us-ord",
            "image": "linode/ubuntu24.04",
        }

    def api(self, state: dict) -> FakeApi:
        return FakeApi(
            {
                f"/linode/instances/{state['linode_id']}": {
                    "label": state["label"],
                    "status": "running",
                    "ipv4": [state["ipv4"]],
                    "type": state["type"],
                    "region": state["region"],
                    "image": state["image"],
                },
                f"/networking/firewalls/{state['firewall_id']}": {
                    "label": state["label"],
                    "status": "enabled",
                },
            }
        )

    def test_recovery_revalidates_live_and_remote_state_before_ready(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            current = root / "current.json"
            lock = root / "lifecycle.lock"
            state = self.state()
            write_json(current, state)
            maintenance = package_maintenance_state()
            with (
                mock.patch.object(runner, "CURRENT_STATE", current),
                mock.patch.object(runner, "LIFECYCLE_LOCK", lock),
                mock.patch.object(runner, "wait_for_ssh") as wait_for_ssh,
                mock.patch.object(
                    runner,
                    "read_package_maintenance_state",
                    return_value=maintenance,
                ) as read_maintenance,
            ):
                recovered = runner.recover_interrupted_bootstrap(
                    self.api(state), state
                )

            saved = json.loads(current.read_text(encoding="utf-8"))
        self.assertEqual(recovered["phase"], "ready")
        self.assertEqual(saved, recovered)
        self.assertEqual(saved["package_maintenance"], maintenance)
        wait_for_ssh.assert_called_once_with(state)
        read_maintenance.assert_called_once_with(state)

    def test_recovery_rejects_live_identity_mismatch_without_mutation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            current = root / "current.json"
            state = self.state()
            write_json(current, state)
            api = self.api(state)
            api.resources[f"/linode/instances/{state['linode_id']}"][
                "label"
            ] = "somebody-elses-runner"
            with (
                mock.patch.object(runner, "CURRENT_STATE", current),
                mock.patch.object(runner, "LIFECYCLE_LOCK", root / "lock"),
                mock.patch.object(runner, "wait_for_ssh") as wait_for_ssh,
                mock.patch.object(
                    runner, "read_package_maintenance_state"
                ) as read_maintenance,
                self.assertRaisesRegex(runner.RunnerError, "live label"),
            ):
                runner.recover_interrupted_bootstrap(api, state)

            saved = json.loads(current.read_text(encoding="utf-8"))
        self.assertEqual(saved, state)
        wait_for_ssh.assert_not_called()
        read_maintenance.assert_not_called()

    def test_recovery_rechecks_saved_identity_before_committing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            current = root / "current.json"
            state = self.state()
            write_json(current, state)

            def replace_active(_state: dict) -> dict:
                changed = dict(state)
                changed["linode_id"] += 1
                write_json(current, changed)
                return package_maintenance_state()

            with (
                mock.patch.object(runner, "CURRENT_STATE", current),
                mock.patch.object(runner, "LIFECYCLE_LOCK", root / "lock"),
                mock.patch.object(runner, "wait_for_ssh"),
                mock.patch.object(
                    runner,
                    "read_package_maintenance_state",
                    side_effect=replace_active,
                ),
                self.assertRaisesRegex(runner.RunnerError, "identity changed"),
            ):
                runner.recover_interrupted_bootstrap(self.api(state), state)

            saved = json.loads(current.read_text(encoding="utf-8"))
        self.assertEqual(saved["linode_id"], state["linode_id"] + 1)
        self.assertNotIn("package_maintenance", saved)

    def test_recover_command_is_explicit(self):
        self.assertEqual(runner.parser().parse_args(["recover"]).command, "recover")

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
        self.assertIn("g++-mingw-w64-x86-64", script)
        self.assertIn(
            "cadical_commit=c60730422e758ef1cebe7aeddf2dda31c996bf04",
            script,
        )
        self.assertIn('git -C "$candidate" rev-parse HEAD', script)
        self.assertIn('git -C "$candidate" diff --quiet HEAD --', script)
        self.assertIn('git -C "$candidate" fsck --strict', script)
        self.assertIn(
            'cadical_staging="$cadical_root/.cadical-$cadical_version.incoming"',
            script,
        )
        self.assertIn("Refusing to replace unmanaged CaDiCaL path", script)
        self.assertNotIn("cadical-3.0.1.tar.gz", script)
        self.assertIn("rustup component add rustfmt clippy", script)
        self.assertIn("rustup target add x86_64-pc-windows-gnu", script)
        self.assertIn("x86_64-w64-mingw32-gcc --version", script)
        self.assertIn("x86_64-w64-mingw32-g++-posix --version", script)

    def test_bootstrap_quiesces_package_maintenance_before_apt(self):
        script = runner.bootstrap_script()

        self.assertLess(script.index("cloud-init status --wait"), script.index("apt-get update"))
        self.assertLess(
            script.index('systemctl stop "${maintenance_units[@]}"'),
            script.index("apt-get update"),
        )
        self.assertLess(
            script.index('systemctl mask "${maintenance_units[@]}"'),
            script.index("apt-get update"),
        )
        self.assertLess(
            script.index('systemctl reset-failed "$unit"'),
            script.index("apt-get update"),
        )
        for unit in runner.PACKAGE_MAINTENANCE_UNITS:
            self.assertIn(unit, script)
        self.assertIn("package-maintenance-quiescence.json", script)
        self.assertIn('property_value(unit, "ActiveState")', script)
        self.assertIn('property_value(unit, "UnitFileState")', script)

    def test_package_maintenance_record_requires_every_unit_quiesced(self):
        record = package_maintenance_record()
        self.assertIs(runner.validate_package_maintenance_record(record), record)

        for property_name, invalid_value, message in [
            ("active_state", "active", "not inactive"),
            ("unit_file_state", "enabled", "not masked"),
        ]:
            with self.subTest(property_name=property_name):
                invalid = package_maintenance_record()
                invalid["units"][runner.PACKAGE_MAINTENANCE_UNITS[0]][
                    property_name
                ] = invalid_value
                with self.assertRaisesRegex(runner.RunnerError, message):
                    runner.validate_package_maintenance_record(invalid)

        missing = package_maintenance_record()
        del missing["units"][runner.PACKAGE_MAINTENANCE_UNITS[-1]]
        with self.assertRaisesRegex(runner.RunnerError, "unexpected unit set"):
            runner.validate_package_maintenance_record(missing)

    def test_bootstrap_records_remote_quiescence_hash(self):
        serialized = json.dumps(package_maintenance_record(), sort_keys=True) + "\n"
        with mock.patch.object(
            runner,
            "ssh_command",
            side_effect=[mock.Mock(), mock.Mock(stdout=serialized)],
        ) as ssh:
            state = runner.bootstrap({"ipv4": "192.0.2.8"})

        self.assertEqual(ssh.call_count, 2)
        self.assertEqual(state["path"], str(runner.PACKAGE_MAINTENANCE_RECORD))
        self.assertEqual(
            state["sha256"],
            runner.hashlib.sha256(serialized.encode("utf-8")).hexdigest(),
        )
        self.assertEqual(state["record"], package_maintenance_record())

    def test_remote_workload_contains_comprehensive_remote_only_gates(self):
        script = MODULE_PATH.with_name("remote_run.sh").read_text(encoding="utf-8")
        lifecycle = MODULE_PATH.with_name("maintenance_lifecycle_test.sh").read_text(
            encoding="utf-8"
        )

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
        self.assertIn("maintenance_lifecycle_test.sh", script)
        self.assertIn("systemctl daemon-reexec", lifecycle)
        self.assertIn("pid_after", lifecycle)
        self.assertIn("invocation_after", lifecycle)
        self.assertIn("cgroup.procs", lifecycle)
        self.assertIn("pgrep -f", lifecycle)
        self.assertIn("sha256sum -c results.sha256", lifecycle)
        self.assertIn("resume-verified", lifecycle)
        self.assertIn("PACKAGE_MAINTENANCE_LIFECYCLE_COMPLETE", lifecycle)


@unittest.skipUnless(
    os.name == "posix" and shutil.which("bash") and shutil.which("git"),
    "requires POSIX bash and Git",
)
class CadicalBootstrapRetryTests(unittest.TestCase):
    def command(
        self, arguments: list[str], *, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            arguments,
            check=check,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def create_source(self, root: Path) -> tuple[Path, str, str]:
        source = root / "cadical-source"
        source.mkdir()
        self.command(
            ["git", "init", "--quiet", "--initial-branch=rel-3.0.1", str(source)]
        )
        self.command(["git", "-C", str(source), "config", "user.name", "Test"])
        self.command(
            ["git", "-C", str(source), "config", "user.email", "test@example.com"]
        )
        (source / "VERSION").write_text("3.0.1\n", encoding="utf-8")
        (source / "tracked.txt").write_text("pinned\n", encoding="utf-8")
        self.command(["git", "-C", str(source), "add", "."])
        self.command(["git", "-C", str(source), "commit", "--quiet", "-m", "pinned"])
        pinned = self.command(
            ["git", "-C", str(source), "rev-parse", "HEAD"]
        ).stdout.strip()

        self.command(["git", "-C", str(source), "switch", "--quiet", "-c", "wrong"])
        (source / "tracked.txt").write_text("wrong\n", encoding="utf-8")
        self.command(["git", "-C", str(source), "commit", "--quiet", "-am", "wrong"])
        wrong = self.command(
            ["git", "-C", str(source), "rev-parse", "HEAD"]
        ).stdout.strip()
        self.command(
            ["git", "-C", str(source), "switch", "--quiet", "rel-3.0.1"]
        )
        return source, pinned, wrong

    def clone(self, source: Path, target: Path) -> None:
        self.command(["git", "clone", "--quiet", str(source), str(target)])

    def run_checkout(
        self,
        install_root: Path,
        source: Path,
        pinned: str,
        *,
        check: bool = True,
    ) -> subprocess.CompletedProcess[str]:
        script = runner.cadical_checkout_script(
            root=runner.PurePosixPath(install_root.as_posix()),
            source_url=source.as_posix(),
            revision=pinned,
        )
        return self.command(
            ["bash", "-E", "-e", "-u", "-o", "pipefail", "-c", script],
            check=check,
        )

    def test_complete_exact_checkout_is_validated_and_reused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, pinned, _ = self.create_source(root)
            install_root = root / "install"
            install_root.mkdir()
            target = install_root / "cadical-3.0.1"
            self.clone(source, target)
            sentinel = target / "reuse-sentinel"
            sentinel.write_text("preserve\n", encoding="utf-8")

            result = self.run_checkout(install_root, source, pinned)

            self.assertIn("Reusing validated CaDiCaL", result.stdout)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve\n")
            self.assertEqual(
                self.command(["git", "-C", str(target), "rev-parse", "HEAD"])
                .stdout.strip(),
                pinned,
            )

    def test_incomplete_managed_checkout_is_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, pinned, _ = self.create_source(root)
            install_root = root / "install"
            install_root.mkdir()
            target = install_root / "cadical-3.0.1"
            self.clone(source, target)
            (target / "tracked.txt").unlink()
            sentinel = target / "incomplete-sentinel"
            sentinel.write_text("remove\n", encoding="utf-8")

            result = self.run_checkout(install_root, source, pinned)

            self.assertIn("Installed validated CaDiCaL", result.stdout)
            self.assertFalse(sentinel.exists())
            self.assertEqual(
                (target / "VERSION").read_text(encoding="utf-8"), "3.0.1\n"
            )
            self.assertEqual(
                (target / "tracked.txt").read_text(encoding="utf-8"), "pinned\n"
            )

    def test_wrong_revision_managed_checkout_is_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, pinned, wrong = self.create_source(root)
            install_root = root / "install"
            install_root.mkdir()
            target = install_root / "cadical-3.0.1"
            self.clone(source, target)
            self.command(["git", "-C", str(target), "checkout", "--quiet", wrong])

            self.run_checkout(install_root, source, pinned)

            self.assertEqual(
                self.command(["git", "-C", str(target), "rev-parse", "HEAD"])
                .stdout.strip(),
                pinned,
            )
            self.assertEqual(
                (target / "tracked.txt").read_text(encoding="utf-8"), "pinned\n"
            )

    def test_interrupted_staging_checkout_is_rebuilt_from_owned_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, pinned, _ = self.create_source(root)
            install_root = root / "install"
            install_root.mkdir()
            staging = install_root / ".cadical-3.0.1.incoming"
            self.clone(source, staging)
            (staging / "VERSION").unlink()
            claim = install_root / ".cadical-3.0.1.bootstrap-claim"
            claim.write_text(
                "schema=1\n"
                f"source={source.as_posix()}\n"
                f"revision={pinned}\n"
                "version=3.0.1\n",
                encoding="utf-8",
            )

            self.run_checkout(install_root, source, pinned)

            target = install_root / "cadical-3.0.1"
            self.assertEqual(
                self.command(["git", "-C", str(target), "rev-parse", "HEAD"])
                .stdout.strip(),
                pinned,
            )
            self.assertFalse(staging.exists())
            self.assertFalse(claim.exists())

    def test_unmanaged_target_is_preserved_and_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, pinned, _ = self.create_source(root)
            install_root = root / "install"
            install_root.mkdir()
            target = install_root / "cadical-3.0.1"
            target.mkdir()
            sentinel = target / "unrelated.txt"
            sentinel.write_text("preserve\n", encoding="utf-8")

            result = self.run_checkout(
                install_root,
                source,
                pinned,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Refusing to replace unmanaged", result.stderr)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve\n")
            self.assertFalse((install_root / ".cadical-3.0.1.incoming").exists())
            self.assertFalse(
                (install_root / ".cadical-3.0.1.bootstrap-claim").exists()
            )


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
        conflicting_arguments = [
            ["check", "--high-memory", "--type", runner.DEFAULT_TYPE],
            ["check", "--type", runner.DEFAULT_TYPE, "--high-memory"],
            ["check", "--high-memory", "--type", runner.HIGH_MEMORY_TYPE],
        ]
        for arguments in conflicting_arguments:
            with (
                self.subTest(arguments=arguments),
                mock.patch("sys.stderr"),
                self.assertRaises(SystemExit),
            ):
                runner.parser().parse_args(arguments)


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

    def test_response_metadata_captures_exact_oauth_scopes(self):
        response = mock.MagicMock()
        response.read.return_value = b"{}"
        response.headers = {
            "Date": "Mon, 27 Jul 2026 22:22:15 GMT",
            "X-OAuth-Scopes": (
                "account:read_write, firewall:read_write linodes:read_write"
            ),
        }
        response.__enter__.return_value = response
        with mock.patch.object(runner.urllib.request, "urlopen", return_value=response):
            api = runner.LinodeApi(token="test")
            api.get("/test")
        self.assertEqual(api.last_oauth_scopes, runner.MAIN_REAPER_SCOPES)


class HighMemoryUsageTests(unittest.TestCase):
    NOW = datetime(2026, 7, 27, 15, tzinfo=timezone.utc)

    @staticmethod
    def write_run(
        history: Path,
        name: str,
        linode_id: int,
        created: str,
        deleted: str,
    ) -> None:
        write_json(
            history / f"{name}.json",
            {
                "type": runner.HIGH_MEMORY_TYPE,
                "linode_id": linode_id,
                "linode_created_at": created,
                "linode_deleted_at": deleted,
            },
        )

    def usage(self, history: Path, current: Path | None = None, active=()):
        return runner.high_memory_usage(
            self.NOW,
            history_root=history,
            current_state_path=current or history.parent / "current.json",
            parked_root=history.parent / "parked",
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

    def test_no_history_starts_with_full_bank(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            usage = self.usage(history)
            self.assertEqual(usage.actual, timedelta())
            self.assertEqual(usage.balance_at_start, timedelta(hours=4))
            self.assertEqual(usage.banked_at_start, timedelta(hours=4))
            self.assertEqual(usage.debt_at_start, timedelta())
            self.assertEqual(usage.capacity, timedelta(hours=8))
            self.assertEqual(usage.remaining, timedelta(hours=8))
            self.assertEqual(usage.next_balance, timedelta(hours=4))

    def test_machine_allowance_record_exposes_exact_boundaries(self):
        with tempfile.TemporaryDirectory() as temporary:
            usage = self.usage(Path(temporary) / "runs")
            value = runner.high_memory_usage_record(
                usage,
                observed_at=self.NOW,
                active_managed_high_memory=0,
            )
        self.assertEqual(
            value["kind"], "umlaut-linode-high-memory-allowance"
        )
        self.assertEqual(value["observed_at_utc"], "2026-07-27T15:00:00+00:00")
        self.assertEqual(value["next_boundary_utc"], "2026-07-28T05:00:00+00:00")
        self.assertEqual(
            value["earliest_new_start_utc"], "2026-07-27T15:00:00+00:00"
        )
        self.assertTrue(value["new_starts_allowed"])
        self.assertEqual(value["capacity_seconds"], 8 * 60 * 60)
        self.assertEqual(value["remaining_seconds"], 8 * 60 * 60)
        self.assertEqual(
            value["projected_capacity_at_next_boundary_seconds"],
            8 * 60 * 60,
        )

    def test_machine_allowance_projects_required_slice_boundary(self):
        usage = runner.HighMemoryUsage(
            actual=timedelta(seconds=27_551),
            balance_at_start=timedelta(hours=4),
            day_start=datetime(2026, 8, 1, 5, tzinfo=timezone.utc),
            next_boundary=datetime(2026, 8, 2, 5, tzinfo=timezone.utc),
        )
        value = runner.high_memory_usage_record(
            usage,
            observed_at=datetime(2026, 8, 2, 2, tzinfo=timezone.utc),
            active_managed_high_memory=0,
            required_seconds=14_700,
        )
        self.assertEqual(value["remaining_seconds"], 1_249)
        self.assertEqual(
            value["projected_capacity_at_next_boundary_seconds"], 15_649
        )
        self.assertFalse(value["required_start_available_now"])
        self.assertEqual(
            value["projected_earliest_required_start_utc"],
            "2026-08-02T05:00:00+00:00",
        )

    def test_allowance_command_emits_read_only_json(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            api = ProvisionApi()
            with (
                mock.patch.object(runner, "RUN_HISTORY", root / "runs"),
                mock.patch.object(runner, "CURRENT_STATE", root / "current.json"),
                mock.patch.object(runner, "PARKED_ROOT", root / "parked"),
                mock.patch.object(runner, "LinodeApi", return_value=api),
                mock.patch("sys.stdout", new_callable=StringIO) as stdout,
            ):
                exit_code = runner.main(
                    ["allowance", "--required-seconds", "14700"]
                )
            value = json.loads(stdout.getvalue())
        self.assertEqual(exit_code, 0)
        self.assertEqual(value["actual_seconds"], 0)
        self.assertEqual(value["active_managed_high_memory"], 0)
        self.assertTrue(value["required_start_available_now"])
        self.assertEqual(api.posts, [])

    def test_usage_counts_only_current_day_high_memory_overlap(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "cross-midnight",
                1,
                "2026-07-27T04:30:00+00:00",
                "2026-07-27T05:30:00+00:00",
            )
            self.write_run(
                history,
                "prior-day",
                2,
                "2026-07-27T03:00:00+00:00",
                "2026-07-27T05:00:00+00:00",
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
            self.assertEqual(usage.banked_at_start, timedelta(hours=4))
            self.assertEqual(usage.capacity, timedelta(hours=8))
            self.assertEqual(
                usage.remaining,
                timedelta(hours=7, minutes=30),
            )
            self.assertEqual(usage.next_balance, timedelta(hours=4))
            self.assertFalse(usage.exhausted)

    def test_usage_immediately_below_capacity_allows_new_start(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "below-capacity",
                1,
                "2026-07-27T06:00:00+00:00",
                "2026-07-27T13:59:59+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(
                usage.actual,
                timedelta(hours=7, minutes=59, seconds=59),
            )
            self.assertEqual(usage.capacity, timedelta(hours=8))
            self.assertEqual(usage.remaining, timedelta(seconds=1))
            self.assertFalse(usage.exhausted)
            runner.require_high_memory_allowance(usage)

    def test_exactly_full_bank_capacity_blocks_new_start(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "eight-hours",
                1,
                "2026-07-27T06:00:00+00:00",
                "2026-07-27T14:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.actual, timedelta(hours=8))
            self.assertEqual(usage.capacity, timedelta(hours=8))
            self.assertEqual(usage.next_balance, timedelta())
            self.assertTrue(usage.exhausted)
            with self.assertRaisesRegex(
                runner.RunnerError,
                "bank-adjusted capacity of 08:00:00",
            ):
                runner.require_high_memory_allowance(usage)

    def test_partial_daily_usage_accrues_into_existing_bank(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-six-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T12:00:00+00:00",
            )
            self.write_run(
                history,
                "current-three-hours",
                2,
                "2026-07-27T06:00:00+00:00",
                "2026-07-27T09:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.banked_at_start, timedelta(hours=2))
            self.assertEqual(usage.capacity, timedelta(hours=6))
            self.assertEqual(usage.actual, timedelta(hours=3))
            self.assertEqual(usage.remaining, timedelta(hours=3))
            self.assertEqual(usage.next_balance, timedelta(hours=3))

    def test_unused_day_fills_partial_bank_to_cap(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-six-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T12:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.banked_at_start, timedelta(hours=2))
            self.assertEqual(usage.actual, timedelta())
            self.assertEqual(usage.next_balance, timedelta(hours=4))

    def test_usage_above_daily_base_consumes_bank(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-six-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T12:00:00+00:00",
            )
            self.write_run(
                history,
                "current-five-hours",
                2,
                "2026-07-27T06:00:00+00:00",
                "2026-07-27T11:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.banked_at_start, timedelta(hours=2))
            self.assertEqual(usage.actual, timedelta(hours=5))
            self.assertEqual(usage.next_balance, timedelta(hours=1))

    def test_overshoot_beyond_bank_becomes_debt(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-six-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T12:00:00+00:00",
            )
            self.write_run(
                history,
                "current-seven-hours",
                2,
                "2026-07-27T06:00:00+00:00",
                "2026-07-27T13:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.capacity, timedelta(hours=6))
            self.assertEqual(usage.actual, timedelta(hours=7))
            self.assertEqual(usage.next_balance, -timedelta(hours=1))
            self.assertEqual(
                usage.debt_at_next_boundary,
                timedelta(hours=1),
            )
            self.assertTrue(usage.exhausted)

    def test_carried_debt_reduces_next_days_capacity(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-nine-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T15:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.balance_at_start, -timedelta(hours=1))
            self.assertEqual(usage.banked_at_start, timedelta())
            self.assertEqual(usage.debt_at_start, timedelta(hours=1))
            self.assertEqual(usage.capacity, timedelta(hours=3))
            self.assertEqual(usage.remaining, timedelta(hours=3))
            self.assertEqual(usage.next_balance, timedelta(hours=3))

    def test_usage_report_shows_debt_and_projected_repayment(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-nine-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T15:00:00+00:00",
            )
            usage = self.usage(history)
            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                runner.report_high_memory_usage(usage)
            output = stdout.getvalue()
            self.assertIn("Banked usage at start of day: 00:00:00", output)
            self.assertIn("Usage debt at start of day: 01:00:00", output)
            self.assertIn("Adjusted daily capacity: 03:00:00", output)
            self.assertIn(
                "Projected bank at next boundary if no further usage accrues: "
                "03:00:00",
                output,
            )
            self.assertIn(
                "Projected debt at next boundary if no further usage accrues: "
                "00:00:00",
                output,
            )

    def test_debt_can_eliminate_daily_capacity(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-twelve-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T18:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.balance_at_start, -timedelta(hours=4))
            self.assertEqual(usage.capacity, timedelta())
            self.assertEqual(usage.remaining, timedelta())
            self.assertTrue(usage.exhausted)
            self.assertEqual(usage.next_balance, timedelta())
            self.assertEqual(
                usage.projected_eligible_at,
                datetime(2026, 7, 28, 5, tzinfo=timezone.utc),
            )

    def test_large_debt_delays_projected_eligibility_across_empty_days(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "prior-seventeen-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T23:00:00+00:00",
            )
            usage = self.usage(history)
            self.assertEqual(usage.balance_at_start, -timedelta(hours=9))
            self.assertEqual(usage.capacity, timedelta())
            self.assertEqual(usage.next_balance, -timedelta(hours=5))
            self.assertEqual(
                usage.projected_eligible_at,
                datetime(2026, 7, 29, 5, tzinfo=timezone.utc),
            )

    def test_empty_days_repay_debt_then_fill_bank(self):
        with tempfile.TemporaryDirectory() as temporary:
            history = Path(temporary) / "runs"
            self.write_run(
                history,
                "twelve-hours",
                1,
                "2026-07-26T06:00:00+00:00",
                "2026-07-26T18:00:00+00:00",
            )
            usage = runner.high_memory_usage(
                datetime(2026, 7, 29, 15, tzinfo=timezone.utc),
                history_root=history,
                current_state_path=Path(temporary) / "current.json",
            )
            self.assertEqual(usage.balance_at_start, timedelta(hours=4))
            self.assertEqual(usage.banked_at_start, timedelta(hours=4))
            self.assertEqual(usage.debt_at_start, timedelta())
            self.assertEqual(usage.capacity, timedelta(hours=8))

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
            self.assertEqual(usage.actual, timedelta(hours=1))
            self.assertEqual(usage.banked_at_start, timedelta(hours=4))
            self.assertEqual(usage.capacity, timedelta(hours=8))

    def test_parked_high_memory_state_remains_billable_usage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            parked = root / "parked"
            write_json(
                parked / "7.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 7,
                    "linode_created_at": "2026-07-27T14:00:00+00:00",
                    "lifecycle": "parked",
                },
            )
            usage = runner.high_memory_usage(
                self.NOW,
                history_root=root / "runs",
                current_state_path=root / "current.json",
                parked_root=parked,
            )
            self.assertEqual(usage.actual, timedelta(hours=1))

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
            mock.patch.object(
                runner,
                "bootstrap",
                return_value=package_maintenance_state(),
            ),
        )

    def test_exhausted_high_memory_guard_precedes_resource_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "eight-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T14:00:00+00:00",
                },
            )
            api = ProvisionApi()
            patches = self.provision_patches(temporary)
            with patches[0], patches[1]:
                with self.assertRaisesRegex(
                    runner.RunnerError,
                    "bank-adjusted capacity of 08:00:00",
                ):
                    runner.provision(
                        api,
                        allow_ip="192.0.2.10",
                        linode_type=runner.HIGH_MEMORY_TYPE,
                    )
            self.assertEqual(api.posts, [])

    def test_high_memory_check_reports_bank_and_returns_nonzero_at_capacity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "eight-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T14:00:00+00:00",
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
                mock.patch("sys.stdout", new_callable=StringIO) as stdout,
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
            output = stdout.getvalue()
            self.assertIn("Daily base allowance: 04:00:00", output)
            self.assertIn("Banked usage at start of day: 04:00:00", output)
            self.assertIn("Usage debt at start of day: 00:00:00", output)
            self.assertIn("Adjusted daily capacity: 08:00:00", output)
            self.assertIn("Actual Linode lifetime today: 08:00:00", output)
            self.assertIn("Projected bank at next boundary", output)
            self.assertIn("Projected debt at next boundary", output)
            self.assertEqual(api.posts, [])

    def test_high_memory_history_does_not_block_normal_profile(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(
                root / "runs" / "eight-hours.json",
                {
                    "type": runner.HIGH_MEMORY_TYPE,
                    "linode_id": 1,
                    "linode_created_at": "2026-07-27T06:00:00+00:00",
                    "linode_deleted_at": "2026-07-27T14:00:00+00:00",
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
                state["package_maintenance"], package_maintenance_state()
            )
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


class BillingLifecycleTests(unittest.TestCase):
    NOW = datetime(2026, 8, 1, 12, 10, tzinfo=timezone.utc)

    @staticmethod
    def state(
        linode_id: int,
        *,
        delete_at: datetime,
        linode_type: str = runner.DEFAULT_TYPE,
    ) -> dict:
        return {
            "run_id": f"260801-120000-{linode_id:04x}",
            "label": f"e-rust-codex-260801-{linode_id:04x}",
            "linode_id": linode_id,
            "firewall_id": linode_id + 100,
            "linode_created_at": "2026-08-01T12:00:00+00:00",
            "type": linode_type,
            "region": runner.DEFAULT_REGION,
            "image": runner.DEFAULT_IMAGE,
            "ipv4": f"192.0.2.{linode_id}",
            "lifecycle": "parked",
            "phase": "parked",
            "lease_id": f"{linode_id:032x}",
            "delete_at": runner.format_utc(delete_at),
        }

    @staticmethod
    def path_patches(root: Path):
        return (
            mock.patch.object(runner, "CURRENT_STATE", root / "current.json"),
            mock.patch.object(runner, "PARKED_ROOT", root / "parked"),
            mock.patch.object(runner, "RUN_HISTORY", root / "runs"),
            mock.patch.object(runner, "LIFECYCLE_LOCK", root / "lifecycle.lock"),
            mock.patch.object(
                runner,
                "PROVISION_CLAIM",
                root / "provision-claim.json",
            ),
        )

    def test_billing_deadline_uses_current_hour_and_two_minute_margin(self):
        created = datetime(2026, 8, 1, 12, tzinfo=timezone.utc)
        self.assertEqual(
            runner.billing_delete_at(created, created + timedelta(minutes=10)),
            created + timedelta(minutes=58),
        )
        self.assertEqual(
            runner.billing_delete_at(created, created + timedelta(hours=1)),
            created + timedelta(hours=1, minutes=58),
        )
        self.assertEqual(
            runner.billing_delete_at(
                created,
                created + timedelta(hours=2, minutes=45),
            ),
            created + timedelta(hours=2, minutes=58),
        )

    def test_cli_exposes_default_park_and_explicit_immediate_teardown(self):
        default = runner.parser().parse_args(["down"])
        self.assertFalse(default.now)
        self.assertFalse(default.all)
        self.assertTrue(runner.parser().parse_args(["down", "--now"]).now)
        self.assertTrue(runner.parser().parse_args(["down", "--all"]).all)

    def test_exact_configuration_match_is_required(self):
        state = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
        self.assertTrue(
            runner.compatible_runner(
                state,
                linode_type=runner.DEFAULT_TYPE,
                region=runner.DEFAULT_REGION,
                image=runner.DEFAULT_IMAGE,
            )
        )
        self.assertFalse(
            runner.compatible_runner(
                state,
                linode_type=runner.HIGH_MEMORY_TYPE,
                region=runner.DEFAULT_REGION,
                image=runner.DEFAULT_IMAGE,
            )
        )

    def test_acquire_selects_earliest_compatible_deadline(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            early = self.state(7, delete_at=self.NOW + timedelta(minutes=10))
            late = self.state(8, delete_at=self.NOW + timedelta(minutes=20))
            patches = self.path_patches(root)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch.object(runner, "validate_catalog"),
                mock.patch.object(
                    runner,
                    "activate_parked_runner",
                    side_effect=lambda _api, _path, value, **_kwargs: value,
                ) as activate,
            ):
                write_json(runner.parked_state_path(8), late)
                write_json(runner.parked_state_path(7), early)
                api = mock.Mock()
                api.trusted_now.return_value = self.NOW
                acquired, reused = runner.acquire_runner(api)
            self.assertTrue(reused)
            self.assertEqual(acquired["linode_id"], 7)
            self.assertEqual(activate.call_args.args[2]["linode_id"], 7)

    def test_mismatched_parked_runner_stays_parked_while_new_one_is_created(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            parked = self.state(
                7,
                delete_at=self.NOW + timedelta(minutes=20),
                linode_type=runner.HIGH_MEMORY_TYPE,
            )
            fresh = {"linode_id": 9, "label": "e-rust-codex-fresh"}
            patches = self.path_patches(root)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch.object(runner, "validate_catalog"),
                mock.patch.object(runner, "provision", return_value=fresh) as provision,
            ):
                parked_path = runner.parked_state_path(7)
                write_json(parked_path, parked)
                api = mock.Mock()
                api.trusted_now.return_value = self.NOW
                acquired, reused = runner.acquire_runner(api)
                self.assertTrue(parked_path.is_file())
            self.assertFalse(reused)
            self.assertIs(acquired, fresh)
            self.assertTrue(provision.call_args.kwargs["prevalidated"])

    def test_activation_opens_current_firewall_before_disarming_remote_timer(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            parked = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
            patches = self.path_patches(root)
            events = []
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch.object(
                    runner,
                    "replace_firewall_rules",
                    side_effect=lambda *_args: events.append("firewall")
                    or "198.51.100.4/32",
                ),
                mock.patch.object(
                    runner,
                    "wait_for_ssh",
                    side_effect=lambda *_args: events.append("ssh"),
                ),
                mock.patch.object(
                    runner,
                    "disarm_remote_reaper",
                    side_effect=lambda *_args: events.append("disarm"),
                ),
                mock.patch.object(
                    runner,
                    "sync_reaper_access",
                    side_effect=lambda *_args, **_kwargs: events.append("access"),
                ),
            ):
                path = runner.parked_state_path(7)
                write_json(path, parked)
                api = mock.Mock()
                api.get.side_effect = [
                    {
                        "id": 7,
                        "label": parked["label"],
                        "status": "running",
                        "ipv4": ["192.0.2.7"],
                    },
                    {
                        "id": 107,
                        "label": parked["label"],
                        "status": "enabled",
                    },
                ]
                activated = runner.activate_parked_runner(
                    api,
                    path,
                    parked,
                    allow_ip="198.51.100.4",
                )
            self.assertIsNotNone(activated)
            self.assertEqual(events, ["firewall", "ssh", "disarm", "access"])
            self.assertEqual(activated["allow_cidr"], "198.51.100.4/32")

    def test_park_moves_active_state_only_after_both_reapers_are_armed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            patches = self.path_patches(root)
            active = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
            active["lifecycle"] = "active"
            active["phase"] = "ready"
            active.pop("lease_id")
            active.pop("delete_at")
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch.object(runner, "cleanup_remote_workspace"),
                mock.patch.object(runner, "load_reaper_config", return_value={"username": "reaper-user"}),
                mock.patch.object(runner, "reaper_token", return_value="secret"),
                mock.patch.object(runner, "sync_reaper_access") as sync_access,
                mock.patch.object(runner, "arm_remote_reaper") as arm,
            ):
                write_json(runner.CURRENT_STATE, active)
                api = mock.Mock()
                api.get.return_value = {
                    "id": 7,
                    "label": active["label"],
                    "created": "2026-08-01T12:00:00+00:00",
                }
                api.trusted_now.return_value = self.NOW
                self.assertTrue(runner.park_runner(api, active))
                parked_path = runner.parked_state_path(7)
                saved = runner.read_state_file(parked_path)
                self.assertFalse(runner.CURRENT_STATE.exists())
            self.assertEqual(saved["lifecycle"], "parked")
            self.assertEqual(saved["delete_at"], "2026-08-01T12:58:00+00:00")
            sync_access.assert_called_once()
            arm.assert_called_once()

    def test_missing_reaper_setup_falls_back_to_immediate_deletion(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            patches = self.path_patches(root)
            active = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
            active["lifecycle"] = "active"
            active.pop("lease_id")
            active.pop("delete_at")
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch.object(runner, "cleanup_remote_workspace"),
                mock.patch.object(runner, "load_reaper_config", return_value=None),
                mock.patch.object(runner, "delete_state_resources") as delete,
            ):
                write_json(runner.CURRENT_STATE, active)
                api = mock.Mock()
                api.get.return_value = {
                    "id": 7,
                    "label": active["label"],
                    "created": "2026-08-01T12:00:00+00:00",
                }
                api.trusted_now.return_value = self.NOW
                self.assertFalse(runner.park_runner(api, active))
            delete.assert_called_once()

    def test_remote_timer_is_persistent_and_uses_exact_runner_state(self):
        state = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
        service, timer = runner.remote_reaper_unit_files(state)
        self.assertIn("remote_reaper.py --state", service)
        self.assertIn("/7/state.json", service)
        self.assertIn("Restart=on-failure", service)
        self.assertIn("OnCalendar=2026-08-01 12:30:00 UTC", timer)
        self.assertIn("Persistent=true", timer)
        self.assertIn("AccuracySec=1s", timer)

    def test_status_reports_active_and_parked_resources_separately(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            parked = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
            patches = self.path_patches(root)
            with (
                patches[0],
                patches[1],
                patches[2],
                patches[3],
                patches[4],
                mock.patch("sys.stdout", new_callable=StringIO) as stdout,
            ):
                write_json(runner.parked_state_path(7), parked)
                api = FakeApi(
                    {
                        "/linode/instances/7": {"status": "running"},
                        "/networking/firewalls/107": {"status": "enabled"},
                    }
                )
                runner.status(api)
            value = json.loads(stdout.getvalue())
            self.assertIsNone(value["active"])
            self.assertEqual(value["parked"][0]["linode_id"], 7)
            self.assertEqual(value["parked"][0]["live_linode_status"], "running")

    def test_reaper_access_rejects_every_unexpected_entity(self):
        state = self.state(7, delete_at=self.NOW + timedelta(minutes=20))
        current = {
            "account_access": ["account_event_viewer"],
            "entity_access": [
                {"id": 999, "type": "linode", "roles": ["linode_admin"]}
            ],
        }
        with self.assertRaisesRegex(runner.RunnerError, "unexpected entity access"):
            runner.validate_reaper_access(current, allowed_states=[state])

    def test_reaper_setup_validates_scope_headers_without_profile_access(self):
        class MainApi:
            last_oauth_scopes = runner.MAIN_REAPER_SCOPES

            def get(self, _path):
                return {
                    "account_access": ["account_event_viewer"],
                    "entity_access": [],
                }

        class RestrictedApi:
            def __init__(self):
                self.last_oauth_scopes = None
                self.requests = []

            def request(self, method, path):
                self.requests.append((method, path))
                self.last_oauth_scopes = runner.RESTRICTED_REAPER_SCOPES

            def get(self, path):
                raise AssertionError(f"unexpected restricted GET {path}")

        restricted = RestrictedApi()
        with (
            mock.patch.object(
                runner,
                "load_reaper_config",
                return_value={"username": "umlaut-reaper"},
            ),
            mock.patch.object(runner, "reaper_token", return_value="secret"),
            mock.patch.object(runner, "list_parked_states", return_value=[]),
            mock.patch.object(runner, "LinodeApi", return_value=restricted),
            mock.patch("sys.stdout", new_callable=StringIO),
        ):
            runner.validate_reaper_setup(MainApi())
        self.assertEqual(restricted.requests, [("HEAD", "/regions")])

    def test_reaper_setup_rejects_extra_restricted_scope(self):
        class MainApi:
            last_oauth_scopes = runner.MAIN_REAPER_SCOPES

            def get(self, _path):
                return {"account_access": [], "entity_access": []}

        restricted = mock.Mock()
        restricted.last_oauth_scopes = None

        def capture_scopes(_method, _path):
            restricted.last_oauth_scopes = {
                *runner.RESTRICTED_REAPER_SCOPES,
                "account:read_only",
            }

        restricted.request.side_effect = capture_scopes
        with (
            mock.patch.object(
                runner,
                "load_reaper_config",
                return_value={"username": "umlaut-reaper"},
            ),
            mock.patch.object(runner, "reaper_token", return_value="secret"),
            mock.patch.object(runner, "list_parked_states", return_value=[]),
            mock.patch.object(runner, "LinodeApi", return_value=restricted),
            self.assertRaisesRegex(runner.RunnerError, "exactly firewall"),
        ):
            runner.validate_reaper_setup(MainApi())

    def test_begin_workload_uses_a_new_artifact_identity_each_time(self):
        state = {}
        with mock.patch.object(runner, "save_current"), mock.patch.object(
            runner,
            "run_id",
            side_effect=["260801-120000-a001", "260801-120100-a002"],
        ):
            first = runner.begin_workload(state)
            state["remote_artifact_path"] = "/old"
            second = runner.begin_workload(state)
        self.assertNotEqual(first, second)
        self.assertNotIn("remote_artifact_path", state)


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
            self.assertIn("bank", document.lower())
            self.assertIn("debt", document.lower())
        self.assertIn("fixed UTC-05:00", runbook)
        self.assertIn(
            "daily capacity = max(0, 4 hours + starting balance)",
            runbook,
        )
        self.assertIn(
            "next balance = min(4 hours, starting balance + 4 hours - actual usage)",
            runbook,
        )
        self.assertIn("cloud-init", runbook)
        self.assertIn("apt-daily-upgrade.timer", runbook)
        self.assertIn("package-maintenance-lifecycle.json", runbook)
        self.assertIn("daemon-reexec", runbook)
        for required in (
            "init-reaper",
            "two minutes",
            "Windows Scheduled Task",
            "persistent systemd timer",
            "down --now",
            "down --all",
            "linode_admin",
            "firewall_admin",
        ):
            self.assertIn(required, runbook)

    def test_wrapper_keeps_tokens_out_of_scheduled_task_arguments(self):
        repo_root = MODULE_PATH.parents[2]
        wrapper = (repo_root / "linode-runner.ps1").read_text(encoding="utf-8")
        self.assertIn('"init-reaper"', wrapper)
        self.assertIn("Register-ScheduledTask", wrapper)
        self.assertIn("-WakeToRun", wrapper)
        self.assertIn("-StartWhenAvailable", wrapper)
        self.assertIn("-RestartCount 10", wrapper)
        action_start = wrapper.index("$actionArguments")
        action_end = wrapper.index("$action =", action_start)
        self.assertNotIn("TOKEN", wrapper[action_start:action_end].upper())
        self.assertIn("ZeroFreeBSTR($reaperTokenPointer)", wrapper)


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
            for reference_root in (
                "cadical",
                "gmp-6.3.0",
                "minisat",
                "problems",
                "vampire",
                "z3",
            ):
                path = root / reference_root
                path.mkdir()
                (path / "reference-source").write_text(
                    "not needed by routine validation\n",
                    encoding="utf-8",
                )
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
            for reference_root in (
                "cadical",
                "gmp-6.3.0",
                "minisat",
                "problems",
                "vampire",
                "z3",
            ):
                self.assertNotIn(f"{reference_root}/reference-source", names)
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
