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
import uuid
from collections.abc import Callable
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
        self.scavenge_orphaned_synthetic_tasks()
        self.temporary = tempfile.TemporaryDirectory(dir=ARTIFACT_ROOT)
        self.root = Path(self.temporary.name)
        self.task_names: set[str] = set()
        self.launch_logs: set[Path] = set()
        self.checkpoint = self.root / "checkpoint with spaces.tar.gz"
        self.checkpoint.write_bytes(b"synthetic scheduler checkpoint\n")
        self.checkpoint_sha256 = hashlib.sha256(
            self.checkpoint.read_bytes()
        ).hexdigest()

    def tearDown(self) -> None:
        failures: list[str] = []
        for task_name in sorted(self.task_names):
            result = self.remove_task(task_name)
            if result.returncode != 0:
                failures.append(
                    f"task cleanup failed for {task_name}: {result.stderr}"
                )
        for path in self.launch_logs:
            path.unlink(missing_ok=True)
        self.temporary.cleanup()
        if failures:
            self.fail("\n".join(failures))

    def scavenge_orphaned_synthetic_tasks(self) -> None:
        def quote(value: str) -> str:
            return "'" + value.replace("'", "''") + "'"

        command = (
            "$ErrorActionPreference='Stop'; "
            f"$repo=[IO.Path]::GetFullPath({quote(str(REPO_ROOT))}); "
            f"$artifacts=[IO.Path]::GetFullPath({quote(str(ARTIFACT_ROOT))}); "
            "$artifactPrefix=$artifacts.TrimEnd('\\')+'\\'; "
            f"$script=[IO.Path]::GetFullPath({quote(str(SCRIPT))}); "
            "$removed=@(); "
            "$tasks=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            "$_.TaskName -match '^Umlaut-CASC-(J13|CASC2025)-Resume-"
            "\\d{8}T\\d{6}Z$'}); "
            "foreach($task in $tasks){"
            "if(@($task.Actions).Count -ne 1 -or "
            "@($task.Triggers).Count -ne 1){continue}; "
            "$action=@($task.Actions)[0]; "
            "$trigger=@($task.Triggers)[0]; "
            "if([string]$task.Description -notmatch "
            "'^Guarded immutable (j13|casc2025) checkpoint resume for Umlaut$' "
            "-or [string]$task.Principal.LogonType -ne 'Interactive' "
            "-or [string]$task.Principal.RunLevel -ne 'Limited' "
            "-or [string]$trigger.Repetition.Interval -ne 'PT5M' "
            "-or [string]$trigger.Repetition.Duration -ne 'P1D'){continue}; "
            "if(-not [string]::Equals([string]$action.Execute,'powershell.exe',"
            "[StringComparison]::OrdinalIgnoreCase)){continue}; "
            "if(-not [string]::Equals([IO.Path]::GetFullPath("
            "[string]$action.WorkingDirectory),$repo,"
            "[StringComparison]::OrdinalIgnoreCase)){continue}; "
            "$arguments=[string]$action.Arguments; "
            "$fileNeedle='-File \"'+$script+'\"'; "
            "if(-not $arguments.Contains($fileNeedle) -or "
            "-not $arguments.Contains(' -Launch ')){continue}; "
            "$match=[regex]::Match($arguments,'-Plan \"([^\"]+)\"'); "
            "if(-not $match.Success){continue}; "
            "$plan=[IO.Path]::GetFullPath($match.Groups[1].Value); "
            "if(-not $plan.StartsWith($artifactPrefix,"
            "[StringComparison]::OrdinalIgnoreCase)){continue}; "
            "$relative=$plan.Substring($artifactPrefix.Length); "
            "if($relative -notmatch '^tmp[^\\\\]+\\\\[^\\\\]+\\.json$' "
            "-or (Test-Path -LiteralPath $plan)){continue}; "
            "if([string]$task.State -eq 'Running'){"
            "Stop-ScheduledTask -TaskName $task.TaskName -ErrorAction Stop}; "
            "Unregister-ScheduledTask -TaskName $task.TaskName "
            "-Confirm:$false -ErrorAction Stop; "
            "$removed+=@($task.TaskName)}; "
            "$remaining=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            "$removed -contains $_.TaskName}); "
            "if($remaining.Count){throw 'Synthetic orphan task cleanup failed'}; "
            "$removed | ConvertTo-Json -Compress"
        )
        result = self.powershell(command)
        self.assertEqual(result.returncode, 0, result.stderr)

    def remove_task(self, task_name: str) -> subprocess.CompletedProcess[str]:
        quoted_task_name = task_name.replace("'", "''")
        command = (
            "$ErrorActionPreference='Stop'; "
            "$tasks=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            f"$_.TaskName -eq '{quoted_task_name}'"
            "}); if($tasks.Count -gt 1){throw 'Duplicate exact task identity'}; "
            "if($tasks.Count -eq 1){"
            "if([string]$tasks[0].State -eq 'Running'){"
            f"Stop-ScheduledTask -TaskName '{quoted_task_name}' -ErrorAction Stop"
            "}; "
            f"Unregister-ScheduledTask -TaskName '{quoted_task_name}' "
            "-Confirm:$false -ErrorAction Stop}; "
            "$remaining=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            f"$_.TaskName -eq '{quoted_task_name}'"
            "}); if($remaining.Count){throw 'Exact task remains registered'}"
        )
        return self.powershell(command)

    def track_new_task(self, task_name: str) -> None:
        quoted_task_name = task_name.replace("'", "''")
        result = self.powershell(
            "$ErrorActionPreference='Stop'; "
            "$tasks=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            f"$_.TaskName -eq '{quoted_task_name}'"
            "}); if($tasks.Count){throw 'Refusing to replace an existing task'}"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.task_names.add(task_name)

    def plan(
        self,
        *,
        hours_ahead: int = 2,
        seconds_ahead: int | None = None,
        immediate: bool = False,
    ) -> dict:
        now = datetime.now(timezone.utc).replace(microsecond=0)
        if seconds_ahead is None:
            seconds_ahead = random.SystemRandom().randint(1, 50)
        not_before = now + timedelta(
            hours=hours_ahead,
            seconds=seconds_ahead,
        )
        projected = not_before - timedelta(seconds=10)
        arguments = [
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
        ]
        if not immediate:
            arguments.extend(
                ["-NotBeforeUtc", not_before.isoformat(timespec="seconds")]
            )
        arguments.append("-Execute")
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
                "required_start_available_now": immediate,
                "projected_earliest_required_start_utc": (
                    None
                    if immediate
                    else projected.isoformat(timespec="seconds")
                ),
                "remaining_seconds": 18000 if immediate else 100,
            },
            "controller": {
                "script": str(CONTROLLER.resolve()),
                "arguments": arguments,
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

    def wait_for_disabled(self, task_name: str) -> dict:
        deadline = time.monotonic() + 15
        observation = self.task_observation(task_name)
        while observation["enabled"] and time.monotonic() < deadline:
            time.sleep(0.25)
            observation = self.task_observation(task_name)
        self.assertFalse(observation["enabled"], observation)
        return observation

    def wait_for_launch_log(
        self,
        existing: set[Path],
        marker: str,
    ) -> tuple[Path, str]:
        deadline = time.monotonic() + 30
        created: set[Path] = set()
        text = ""
        while time.monotonic() < deadline:
            created = set(ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")) - existing
            if len(created) == 1:
                path = next(iter(created))
                self.launch_logs.add(path)
                text = path.read_text(encoding="utf-8")
                if marker in text:
                    return path, text
            time.sleep(0.25)
        self.assertEqual(len(created), 1)
        self.fail(f"launch log did not contain {marker!r}: {text}")

    def assert_invalid_plan_disables_before_validation(
        self,
        plan: Path,
        mutate: Callable[[], object],
        expected_error: str,
    ) -> None:
        existing_launch_logs = set(
            ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")
        )
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        self.track_new_task(task_name)
        self.invoke(plan, "-Register")
        mutate()

        started = self.powershell(
            f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
        )
        self.assertEqual(started.returncode, 0, started.stderr)
        observation = self.wait_for_disabled(task_name)
        launch_path, launch_log = self.wait_for_launch_log(
            existing_launch_logs,
            "task_launch_failed",
        )
        self.assertIn("task_disabled", launch_log)
        self.assertIn(expected_error, launch_log)
        self.assertNotIn("plan_validated", launch_log)
        self.assertNotIn("controller_invocation_started", launch_log)

        last_run_time = observation["last_run_time"]
        duplicate = self.powershell(
            f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
        )
        self.assertNotEqual(duplicate.returncode, 0)
        time.sleep(1)
        after = self.powershell(
            "$ErrorActionPreference='Stop'; "
            "$tasks=@(Get-ScheduledTask -ErrorAction Stop | Where-Object {"
            f"$_.TaskName -eq '{quoted_task_name}'"
            "}); if($tasks.Count -gt 1){throw 'Duplicate exact task identity'}; "
            "$lastRun=$null; if($tasks.Count -eq 1){"
            f"$info=Get-ScheduledTaskInfo -TaskName '{quoted_task_name}'; "
            "$lastRun=$info.LastRunTime.ToUniversalTime().ToString('O')}; "
            "[ordered]@{exists=($tasks.Count -eq 1);last_run_time=$lastRun} "
            "| ConvertTo-Json"
        )
        self.assertEqual(after.returncode, 0, after.stderr)
        after_observation = json.loads(after.stdout)
        if after_observation["exists"]:
            self.assertEqual(after_observation["last_run_time"], last_run_time)
        self.assertEqual(
            set(ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")),
            existing_launch_logs | {launch_path},
        )

    def test_default_is_nonmutating_validated_plan(self) -> None:
        result = self.invoke(self.write_plan(self.plan()))
        evidence = json.loads(result.stdout)

        self.assertEqual(evidence["status"], "ready_to_register")
        self.assertEqual(evidence["release"], "j13")
        self.assertEqual(evidence["checkpoint"]["completed_results"], 1)
        self.assertIn("schedule_casc_resume.ps1", evidence["task"]["arguments"])
        self.assertIn("-Launch", evidence["task"]["arguments"])
        self.assertIn("-ScheduledTaskName", evidence["task"]["arguments"])
        self.assertIn("-WindowStyle Hidden", evidence["task"]["arguments"])
        self.assertEqual(evidence["task"]["window_style"], "Hidden")
        self.assertIn(
            "checkpoint with spaces.tar.gz",
            " ".join(evidence["task"]["controller"]["arguments"]),
        )
        self.assertEqual(evidence["task"]["retry_interval"], "PT5M")
        self.assertEqual(evidence["task"]["retry_duration"], "P1D")
        self.assertTrue(evidence["task"]["disables_before_controller"])
        self.assertTrue(evidence["task"]["wake_to_run"])
        self.assertEqual(evidence["task"]["multiple_instances"], "IgnoreNew")

    def test_immediate_plan_has_deterministic_durable_handoff(self) -> None:
        document = self.plan(immediate=True)
        result = self.invoke(self.write_plan(document, "immediate.json"))
        evidence = json.loads(result.stdout)

        observed = datetime.fromisoformat(document["allowance"]["observed_at_utc"])
        trigger = datetime.fromisoformat(evidence["task"]["trigger_utc"])
        self.assertEqual(evidence["task"]["launch_mode"], "immediate_full_fit")
        self.assertEqual(trigger, observed + timedelta(minutes=5))
        self.assertNotIn(
            "-NotBeforeUtc", evidence["task"]["controller"]["arguments"]
        )

    def test_malformed_plans_fail_closed(self) -> None:
        cases: list[tuple[str, dict, str]] = []

        wrong_hash = copy.deepcopy(self.plan())
        wrong_hash["checkpoint"]["sha256"] = "0" * 64
        cases.append(("wrong-hash", wrong_hash, "checkpoint SHA-256 mismatch"))

        extra_argument = copy.deepcopy(self.plan())
        extra_argument["controller"]["arguments"].insert(-1, "--unexpected")
        cases.append(("extra-argument", extra_argument, "6 exact flag/value"))

        inconsistent_immediate = copy.deepcopy(self.plan())
        inconsistent_immediate["allowance"]["required_start_available_now"] = True
        cases.append(
            (
                "inconsistent-immediate",
                inconsistent_immediate,
                "5 exact flag/value pairs",
            )
        )

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

    def test_scavenger_removes_only_missing_synthetic_plan_task(self) -> None:
        plan = self.write_plan(self.plan(immediate=True), "retained-plan.json")
        retained = json.loads(self.invoke(plan).stdout)["task"]["name"]
        self.track_new_task(retained)
        self.invoke(plan, "-Register")

        orphan_time = datetime.now(timezone.utc) + timedelta(minutes=17)
        orphan = "Umlaut-CASC-J13-Resume-" + orphan_time.strftime(
            "%Y%m%dT%H%M%SZ"
        )
        self.track_new_task(orphan)
        missing_root = ARTIFACT_ROOT / f"tmp{uuid.uuid4().hex}"
        missing_plan = missing_root / "missing-plan.json"

        def ps_quote(value: str) -> str:
            return "'" + value.replace("'", "''") + "'"

        action_arguments = (
            "-WindowStyle Hidden -NoProfile -NonInteractive "
            "-ExecutionPolicy Bypass "
            f'-File "{SCRIPT.resolve()}" -Plan "{missing_plan.resolve()}" '
            f'-Launch -ScheduledTaskName "{orphan}" -ExecutionTimeHours 8'
        )
        register = self.powershell(
            "$ErrorActionPreference='Stop'; "
            "$principal=New-ScheduledTaskPrincipal "
            "-UserId ([Security.Principal.WindowsIdentity]::GetCurrent().Name) "
            "-LogonType Interactive -RunLevel Limited; "
            "$action=New-ScheduledTaskAction -Execute 'powershell.exe' "
            f"-Argument {ps_quote(action_arguments)} "
            f"-WorkingDirectory {ps_quote(str(REPO_ROOT.resolve()))}; "
            "$trigger=New-ScheduledTaskTrigger -Once "
            f"-At ([DateTimeOffset]::Parse({ps_quote(orphan_time.isoformat())})"
            ".ToLocalTime().DateTime) -RepetitionInterval "
            "(New-TimeSpan -Minutes 5) -RepetitionDuration "
            "(New-TimeSpan -Days 1); "
            f"Register-ScheduledTask -TaskName {ps_quote(orphan)} "
            "-Description 'Guarded immutable j13 checkpoint resume for Umlaut' "
            "-Action $action -Trigger $trigger -Principal $principal | Out-Null"
        )
        self.assertEqual(register.returncode, 0, register.stderr)

        self.scavenge_orphaned_synthetic_tasks()
        inspect = self.powershell(
            f"$retained=Get-ScheduledTask -TaskName {ps_quote(retained)} "
            "-ErrorAction SilentlyContinue; "
            f"$orphan=Get-ScheduledTask -TaskName {ps_quote(orphan)} "
            "-ErrorAction SilentlyContinue; "
            "[ordered]@{retained=($null -ne $retained);"
            "orphan=($null -ne $orphan)} | ConvertTo-Json"
        )
        self.assertEqual(inspect.returncode, 0, inspect.stderr)
        observation = json.loads(inspect.stdout)
        self.assertTrue(observation["retained"])
        self.assertFalse(observation["orphan"])

    def test_register_audit_and_mismatch_rejection(self) -> None:
        plan = self.write_plan(self.plan(hours_ahead=23))
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        self.track_new_task(task_name)
        registered = json.loads(self.invoke(plan, "-Register").stdout)
        self.assertEqual(registered["status"], "registered")

        audited = json.loads(self.invoke(plan, "-Audit").stdout)
        self.assertEqual(audited["status"], "audit_passed")
        self.assertEqual(audited["task"]["name"], task_name)
        self.assertEqual(audited["task"]["retry_interval"], "PT5M")
        self.assertEqual(audited["task"]["retry_duration"], "P1D")
        self.assertEqual(audited["task"]["window_style"], "Hidden")

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

    def test_launch_disables_exact_task_before_controller(self) -> None:
        plan = self.write_plan(
            self.plan(hours_ahead=0, seconds_ahead=30, immediate=True),
            "launch-plan.json",
        )
        existing_launch_logs = set(
            ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")
        )
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        self.track_new_task(task_name)
        self.invoke(plan, "-Register")
        started = self.powershell(
            f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
        )
        self.assertEqual(started.returncode, 0, started.stderr)

        observation = self.wait_for_disabled(task_name)
        self.assertNotEqual(
            observation["last_run_time"],
            "1999-11-30T00:00:00.0000000Z",
        )
        _, launch_log = self.wait_for_launch_log(
            existing_launch_logs,
            "task_launch_failed",
        )
        self.assertIn("task_launch_started", launch_log)
        self.assertIn("task_disabled", launch_log)
        self.assertIn("plan_validated", launch_log)
        self.assertIn("controller_invocation_started", launch_log)
        self.assertIn("controller_invocation_failed", launch_log)

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

    def test_missing_plan_disables_task_before_validation(self) -> None:
        plan = self.write_plan(
            self.plan(hours_ahead=0, seconds_ahead=35, immediate=True),
            "missing-launch-plan.json",
        )
        self.assert_invalid_plan_disables_before_validation(
            plan,
            plan.unlink,
            "resume plan is missing",
        )

    def test_corrupt_plan_disables_task_before_validation(self) -> None:
        plan = self.write_plan(
            self.plan(hours_ahead=0, seconds_ahead=38, immediate=True),
            "corrupt-launch-plan.json",
        )
        self.assert_invalid_plan_disables_before_validation(
            plan,
            lambda: plan.write_text("{not-json\n", encoding="utf-8"),
            "Resume plan is not valid JSON",
        )

    def test_launch_envelope_drift_does_not_disable_task(self) -> None:
        plan = self.write_plan(
            self.plan(hours_ahead=0, seconds_ahead=40, immediate=True),
            "drift-launch-plan.json",
        )
        existing_launch_logs = set(
            ARTIFACT_ROOT.glob("scheduled-launch-j13-*.log")
        )
        preview = json.loads(self.invoke(plan).stdout)
        task_name = preview["task"]["name"]
        quoted_task_name = task_name.replace("'", "''")
        self.track_new_task(task_name)
        self.invoke(plan, "-Register")
        drift = self.powershell(
            f"$task=Get-ScheduledTask -TaskName '{quoted_task_name}'; "
            "$action=$task.Actions[0]; "
            "$arguments=([string]$action.Arguments).Replace("
            "'-WindowStyle Hidden','-WindowStyle Minimized'); "
            "$replacement=New-ScheduledTaskAction -Execute $action.Execute "
            "-Argument $arguments -WorkingDirectory $action.WorkingDirectory; "
            f"Set-ScheduledTask -TaskName '{quoted_task_name}' "
            "-Action $replacement | Out-Null"
        )
        self.assertEqual(drift.returncode, 0, drift.stderr)

        rejected = self.invoke(
            plan,
            "-Launch",
            "-ScheduledTaskName",
            task_name,
            check=False,
        )
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn("scheduled arguments mismatch", rejected.stderr)
        self.assertTrue(self.task_observation(task_name)["enabled"])
        _, launch_log = self.wait_for_launch_log(
            existing_launch_logs,
            "task_launch_failed",
        )
        self.assertNotIn("task_disabled", launch_log)

    def test_hidden_powershell_task_has_no_visible_window(self) -> None:
        suffix = uuid.uuid4().hex
        task_name = f"Umlaut-CASC-Hidden-Window-Test-{suffix}"
        title = f"UmlautHiddenWindowTest-{suffix}"
        helper = self.root / "hidden-window-test.ps1"
        helper.write_text(
            f"try {{$Host.UI.RawUI.WindowTitle = '{title}'}} catch {{}}\n"
            "Start-Sleep -Seconds 15\n",
            encoding="utf-8",
        )
        self.track_new_task(task_name)
        quoted_task_name = task_name.replace("'", "''")
        quoted_helper = str(helper).replace("'", "''")
        register = self.powershell(
            "$ErrorActionPreference='Stop'; "
            "$principal=New-ScheduledTaskPrincipal "
            "-UserId ([Security.Principal.WindowsIdentity]::GetCurrent().Name) "
            "-LogonType Interactive -RunLevel Limited; "
            "$settings=New-ScheduledTaskSettingsSet "
            "-ExecutionTimeLimit (New-TimeSpan -Minutes 1); "
            "$action=New-ScheduledTaskAction -Execute 'powershell.exe' "
            f"-Argument '-WindowStyle Hidden -NoProfile -NonInteractive -File "
            f"\"{quoted_helper}\"'; "
            f"Register-ScheduledTask -TaskName '{quoted_task_name}' "
            "-Description 'Synthetic hidden-window regression for Umlaut' "
            "-Action $action -Settings $settings -Principal $principal | Out-Null; "
            f"Start-ScheduledTask -TaskName '{quoted_task_name}'"
        )
        self.assertEqual(register.returncode, 0, register.stderr)

        deadline = time.monotonic() + 10
        observation: dict[str, object] = {}
        while time.monotonic() < deadline:
            inspect = self.powershell(
                "$processes=@(Get-CimInstance Win32_Process | Where-Object {"
                "$_.Name -eq 'powershell.exe' -and $_.ProcessId -ne $PID -and "
                f"$_.CommandLine -like '*{helper.name}*'"
                "}); $visible=@(Get-Process | Where-Object {"
                f"$_.MainWindowTitle -eq '{title}' -and $_.MainWindowHandle -ne 0"
                "}); [ordered]@{process_count=$processes.Count;"
                "visible_count=$visible.Count} | ConvertTo-Json"
            )
            self.assertEqual(inspect.returncode, 0, inspect.stderr)
            observation = json.loads(inspect.stdout)
            if observation["process_count"]:
                break
            time.sleep(0.25)
        self.assertGreaterEqual(int(observation.get("process_count", 0)), 1)
        self.assertEqual(observation["visible_count"], 0)

    def test_logged_controller_records_success_and_failure(self) -> None:
        success_script = self.root / "success-controller.ps1"
        success_script.write_text(
            'param([string]$Value)\nWrite-Output "success-$Value"\n',
            encoding="utf-8",
        )
        failure_script = self.root / "failure-controller.ps1"
        failure_script.write_text(
            'param([string]$Unused)\n'
            'Write-Output "before-failure"\nthrow "synthetic failure"\n',
            encoding="utf-8",
        )

        def invoke_function(
            controller: Path,
            log: Path,
            parameters: dict[str, str],
        ):
            def quote(value: str) -> str:
                return "'" + value.replace("'", "''") + "'"

            parameters_text = ";".join(
                f"{name}={quote(value)}" for name, value in parameters.items()
            )
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
                f"-ControllerParameters @{{{parameters_text}}} "
                f"-LogPath {quote(str(log))}"
            )
            return self.powershell(command)

        success_log = self.root / "success.log"
        succeeded = invoke_function(success_script, success_log, {"Value": "exact"})
        self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
        success_text = success_log.read_text(encoding="utf-8")
        self.assertIn("controller_invocation_started", success_text)
        self.assertIn("controller_output success-exact", success_text)
        self.assertIn("controller_invocation_completed", success_text)
        self.assertNotIn("controller_invocation_failed", success_text)

        failure_log = self.root / "failure.log"
        failed = invoke_function(
            failure_script,
            failure_log,
            {"Unused": "unused"},
        )
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
