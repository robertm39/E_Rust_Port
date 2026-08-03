#!/usr/bin/env python3
"""Focused tests for durable guarded CASC resume scheduling."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import random
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

SCRIPT = Path(__file__).with_name("schedule_casc_resume.ps1")
REPO_ROOT = SCRIPT.parents[2]
CONTROLLER = SCRIPT.with_name("resume_j13_checkpoint.ps1")
ARTIFACT_ROOT = REPO_ROOT / ".artifacts" / "casc-benchmark"
POWERSHELL = "powershell.exe"


@unittest.skipUnless(os.name == "nt", "requires Windows Task Scheduler")
class CascResumeSchedulerTests(unittest.TestCase):
    def setUp(self) -> None:
        ARTIFACT_ROOT.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=ARTIFACT_ROOT)
        self.root = Path(self.temporary.name)
        self.checkpoint = self.root / "checkpoint with spaces.tar.gz"
        self.checkpoint.write_bytes(b"synthetic scheduler checkpoint\n")
        self.checkpoint_sha256 = hashlib.sha256(
            self.checkpoint.read_bytes()
        ).hexdigest()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def plan(self, *, hours_ahead: int = 2) -> dict:
        now = datetime.now(timezone.utc).replace(microsecond=0)
        not_before = now + timedelta(
            hours=hours_ahead,
            seconds=random.SystemRandom().randint(1, 50),
        )
        projected = not_before - timedelta(seconds=10)
        return {
            "schema_version": 1,
            "kind": "umlaut-casc-next-resume-plan",
            "status": "ready_to_arm",
            "release": "j13",
            "checkpoint": {
                "path": str(self.checkpoint.resolve()),
                "sha256": self.checkpoint_sha256,
                "completed_results": 1,
                "expected_results": 2700,
            },
            "allowance": {
                "observed_at_utc": now.isoformat(timespec="seconds"),
                "required_seconds": 14700,
                "required_start_available_now": False,
                "projected_earliest_required_start_utc": projected.isoformat(
                    timespec="seconds"
                ),
                "remaining_seconds": 100,
            },
            "controller": {
                "script": str(CONTROLLER.resolve()),
                "arguments": [
                    "-Release",
                    "j13",
                    "-CheckpointArchive",
                    str(self.checkpoint.resolve()),
                    "-CheckpointSha256",
                    self.checkpoint_sha256,
                    "-ExpectedInitialResults",
                    "1",
                    "-MaxSessionWallSeconds",
                    "14400",
                    "-NotBeforeUtc",
                    not_before.isoformat(timespec="seconds"),
                    "-Execute",
                ],
            },
        }

    def write_plan(self, document: dict, name: str = "plan.json") -> Path:
        path = self.root / name
        path.write_text(
            json.dumps(document, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return path

    def invoke(
        self,
        plan: Path,
        *arguments: str,
        check: bool = True,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                POWERSHELL,
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(SCRIPT),
                "-Plan",
                str(plan),
                *arguments,
            ],
            cwd=REPO_ROOT,
            check=check,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def powershell(self, command: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                POWERSHELL,
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                command,
            ],
            cwd=REPO_ROOT,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def test_default_is_nonmutating_validated_plan(self) -> None:
        result = self.invoke(self.write_plan(self.plan()))
        evidence = json.loads(result.stdout)

        self.assertEqual(evidence["status"], "ready_to_register")
        self.assertEqual(evidence["release"], "j13")
        self.assertEqual(evidence["checkpoint"]["completed_results"], 1)
        self.assertIn('checkpoint with spaces.tar.gz"', evidence["task"]["arguments"])
        self.assertTrue(evidence["task"]["wake_to_run"])
        self.assertEqual(evidence["task"]["multiple_instances"], "IgnoreNew")

    def test_malformed_plans_fail_closed(self) -> None:
        cases: list[tuple[str, dict, str]] = []

        wrong_hash = copy.deepcopy(self.plan())
        wrong_hash["checkpoint"]["sha256"] = "0" * 64
        cases.append(("wrong-hash", wrong_hash, "checkpoint SHA-256 mismatch"))

        extra_argument = copy.deepcopy(self.plan())
        extra_argument["controller"]["arguments"].insert(-1, "--unexpected")
        cases.append(("extra-argument", extra_argument, "six exact flag/value"))

        immediate = copy.deepcopy(self.plan())
        immediate["allowance"]["required_start_available_now"] = True
        cases.append(("immediate", immediate, "executed directly"))

        outside = copy.deepcopy(self.plan())
        outside_path = Path(os.environ["WINDIR"]) / "System32" / "notepad.exe"
        outside["checkpoint"]["path"] = str(outside_path)
        cases.append(("outside", outside, "inside the repository"))

        for name, document, message in cases:
            with self.subTest(name=name):
                result = self.invoke(
                    self.write_plan(document, f"{name}.json"),
                    check=False,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

    def test_register_audit_and_mismatch_rejection(self) -> None:
        plan = self.write_plan(self.plan(hours_ahead=23))
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        cleanup = (
            f"Unregister-ScheduledTask -TaskName '{quoted_task_name}' "
            "-Confirm:$false -ErrorAction SilentlyContinue"
        )
        self.powershell(cleanup)
        try:
            registered = json.loads(self.invoke(plan, "-Register").stdout)
            self.assertEqual(registered["status"], "registered")

            audited = json.loads(self.invoke(plan, "-Audit").stdout)
            self.assertEqual(audited["status"], "audit_passed")
            self.assertEqual(audited["task"]["name"], task_name)

            weaken = (
                "$settings=New-ScheduledTaskSettingsSet "
                "-ExecutionTimeLimit (New-TimeSpan -Hours 8) "
                "-StartWhenAvailable -RunOnlyIfNetworkAvailable "
                "-AllowStartIfOnBatteries -DontStopIfGoingOnBatteries "
                "-MultipleInstances IgnoreNew; "
                f"Set-ScheduledTask -TaskName '{quoted_task_name}' "
                "-Settings $settings | Out-Null"
            )
            weakened = self.powershell(weaken)
            self.assertEqual(weakened.returncode, 0, weakened.stderr)

            rejected = self.invoke(plan, "-Audit", check=False)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("guarded policy", rejected.stderr)
        finally:
            cleaned = self.powershell(cleanup)
            self.assertEqual(cleaned.returncode, 0, cleaned.stderr)


if __name__ == "__main__":
    unittest.main()
