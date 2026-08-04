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
import time
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

    def plan(
        self,
        *,
        hours_ahead: int = 2,
        seconds_ahead: int | None = None,
    ) -> dict:
        now = datetime.now(timezone.utc).replace(microsecond=0)
        if seconds_ahead is None:
            seconds_ahead = random.SystemRandom().randint(1, 50)
        not_before = now + timedelta(
            hours=hours_ahead,
            seconds=seconds_ahead,
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

    def task_observation(self, task_name: str) -> dict:
        quoted_task_name = task_name.replace("'", "''")
        command = (
            f"$task=Get-ScheduledTask -TaskName '{quoted_task_name}'; "
            f"$info=Get-ScheduledTaskInfo -TaskName '{quoted_task_name}'; "
            "[ordered]@{enabled=[bool]$task.Settings.Enabled; "
            "state=[string]$task.State; "
            "last_run_time=$info.LastRunTime.ToUniversalTime().ToString('O'); "
            "last_task_result=[int64]$info.LastTaskResult} | ConvertTo-Json"
        )
        result = self.powershell(command)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_default_is_nonmutating_validated_plan(self) -> None:
        result = self.invoke(self.write_plan(self.plan()))
        evidence = json.loads(result.stdout)

        self.assertEqual(evidence["status"], "ready_to_register")
        self.assertEqual(evidence["release"], "j13")
        self.assertEqual(evidence["checkpoint"]["completed_results"], 1)
        self.assertIn("schedule_casc_resume.ps1", evidence["task"]["arguments"])
        self.assertIn("-Launch", evidence["task"]["arguments"])
        self.assertIn(
            "checkpoint with spaces.tar.gz",
            " ".join(evidence["task"]["controller"]["arguments"]),
        )
        self.assertEqual(evidence["task"]["retry_interval"], "PT5M")
        self.assertEqual(evidence["task"]["retry_duration"], "P1D")
        self.assertTrue(evidence["task"]["disables_before_controller"])
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
            self.assertEqual(audited["task"]["retry_interval"], "PT5M")
            self.assertEqual(audited["task"]["retry_duration"], "P1D")

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

    def test_launch_disables_exact_task_before_controller(self) -> None:
        plan = self.write_plan(
            self.plan(hours_ahead=0, seconds_ahead=30),
            "launch-plan.json",
        )
        existing_launch_logs = set(
            ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")
        )
        created_launch_logs: set[Path] = set()
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        cleanup = (
            f"Stop-ScheduledTask -TaskName '{quoted_task_name}' "
            "-ErrorAction SilentlyContinue; "
            f"Unregister-ScheduledTask -TaskName '{quoted_task_name}' "
            "-Confirm:$false -ErrorAction SilentlyContinue"
        )
        self.powershell(cleanup)
        try:
            self.invoke(plan, "-Register")
            started = self.powershell(
                f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
            )
            self.assertEqual(started.returncode, 0, started.stderr)

            deadline = time.monotonic() + 15
            observation = self.task_observation(task_name)
            while observation["enabled"] and time.monotonic() < deadline:
                time.sleep(0.25)
                observation = self.task_observation(task_name)
            self.assertFalse(observation["enabled"], observation)
            self.assertNotEqual(
                observation["last_run_time"],
                "1999-11-30T00:00:00.0000000Z",
            )
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                created_launch_logs = (
                    set(ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log"))
                    - existing_launch_logs
                )
                if created_launch_logs:
                    break
                time.sleep(0.25)
            self.assertEqual(len(created_launch_logs), 1)
            launch_log = next(iter(created_launch_logs)).read_text(
                encoding="utf-8"
            )
            self.assertIn("task_launch_started", launch_log)
            self.assertIn("task_disabled", launch_log)
            self.assertIn("controller_invocation_started", launch_log)
            self.assertIn("controller_invocation_failed", launch_log)
            self.assertIn("task_launch_failed", launch_log)

            last_run_time = observation["last_run_time"]
            duplicate = self.powershell(
                f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
            )
            self.assertNotEqual(duplicate.returncode, 0)
            time.sleep(1)
            self.assertEqual(
                self.task_observation(task_name)["last_run_time"],
                last_run_time,
            )
        finally:
            cleaned = self.powershell(cleanup)
            self.assertEqual(cleaned.returncode, 0, cleaned.stderr)
            for path in created_launch_logs:
                path.unlink(missing_ok=True)

    def test_logged_controller_records_success_and_failure(self) -> None:
        success_script = self.root / "success-controller.ps1"
        success_script.write_text(
            'param([string]$Value)\nWrite-Output "success-$Value"\n',
            encoding="utf-8",
        )
        failure_script = self.root / "failure-controller.ps1"
        failure_script.write_text(
            'Write-Output "before-failure"\nthrow "synthetic failure"\n',
            encoding="utf-8",
        )

        def invoke_function(controller: Path, log: Path, *arguments: str):
            def quote(value: str) -> str:
                return "'" + value.replace("'", "''") + "'"

            arguments_text = ",".join(quote(value) for value in arguments)
            command = (
                "$ErrorActionPreference='Stop'; $tokens=$null; $errors=$null; "
                f"$ast=[Management.Automation.Language.Parser]::ParseFile({quote(str(SCRIPT))},"
                "[ref]$tokens,[ref]$errors); if($errors.Count){throw $errors[0]}; "
                "$definition=$ast.FindAll({param($node) "
                "$node -is [Management.Automation.Language.FunctionDefinitionAst] "
                "-and $node.Name -eq 'Invoke-LoggedController'},$true); "
                "if($definition.Count -ne 1){throw 'function definition mismatch'}; "
                "Invoke-Expression $definition[0].Extent.Text; "
                f"Invoke-LoggedController -ControllerPath {quote(str(controller))} "
                f"-ControllerArguments @({arguments_text}) -LogPath {quote(str(log))}"
            )
            return self.powershell(command)

        success_log = self.root / "success.log"
        succeeded = invoke_function(success_script, success_log, "exact")
        self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
        success_text = success_log.read_text(encoding="utf-8")
        self.assertIn("controller_invocation_started", success_text)
        self.assertIn("controller_output success-exact", success_text)
        self.assertIn("controller_invocation_completed", success_text)
        self.assertNotIn("controller_invocation_failed", success_text)

        failure_log = self.root / "failure.log"
        failed = invoke_function(failure_script, failure_log, "unused")
        self.assertNotEqual(failed.returncode, 0)
        self.assertTrue(
            failure_log.is_file(),
            f"stdout={failed.stdout!r} stderr={failed.stderr!r}",
        )
        failure_text = failure_log.read_text(encoding="utf-8")
        self.assertIn("controller_output before-failure", failure_text)
        self.assertIn(
            "controller_invocation_failed error=synthetic failure",
            failure_text,
        )
        self.assertNotIn("controller_invocation_completed", failure_text)


if __name__ == "__main__":
    unittest.main()
