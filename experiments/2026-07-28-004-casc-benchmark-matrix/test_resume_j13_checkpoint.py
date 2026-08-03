#!/usr/bin/env python3
"""Focused structural tests for guarded CASC controller probe deadlines."""

from __future__ import annotations

import base64
import copy
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("resume_j13_checkpoint.ps1")


class CascResumeControllerProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")
        match = re.search(
            r"\$terminalJournalVerifier = @'\n(?P<code>.*?)\n'@",
            cls.source,
            re.DOTALL,
        )
        if match is None:
            raise AssertionError("terminal journal verifier is missing")
        cls.terminal_verifier = match.group("code")

    def test_probe_deadline_is_bounded_inside_monitor_slack(self) -> None:
        match = re.search(
            r"\$runnerProbeTimeoutSeconds\s*=\s*(\d+)",
            self.source,
        )
        self.assertIsNotNone(match)
        timeout_seconds = int(match.group(1))
        self.assertGreater(timeout_seconds, 0)
        self.assertLess(timeout_seconds, 300)

    def test_probe_helper_passes_timeout_before_remote_remainder(self) -> None:
        helper_start = self.source.index("function Invoke-RunnerProbe")
        helper_end = self.source.index(
            "function Invoke-CheckpointValidator",
            helper_start,
        )
        helper = self.source[helper_start:helper_end]

        self.assertIn('"exec",', helper)
        self.assertIn('"--timeout-seconds",', helper)
        self.assertIn("[string]$runnerProbeTimeoutSeconds", helper)
        self.assertLess(
            helper.index('"--timeout-seconds",'),
            helper.index('"--",'),
        )

    def test_all_short_service_and_inventory_probes_are_bounded(self) -> None:
        self.assertEqual(
            self.source.count("Invoke-RunnerProbe -RemoteCommand"),
            7,
        )
        self.assertIn(
            'Invoke-Runner @("exec", "--", $captureCommand)',
            self.source,
        )

    def test_adoption_requires_complete_exact_identity(self) -> None:
        self.assertIn("[switch]$AdoptExistingService", self.source)
        self.assertIn("[switch]$AdoptCompletedService", self.source)
        self.assertIn("[int64]$ExpectedServiceMainPid", self.source)
        self.assertIn("[string]$ExpectedServiceInvocationId", self.source)
        self.assertIn(
            '"Service adoption requires Execute, ExistingRunnerRunId, "',
            self.source,
        )
        self.assertIn('forbids NotBeforeUtc"', self.source)
        self.assertIn('$expectedRunnerPhase = if ($adoptingAnyService)', self.source)
        self.assertIn('"synced"', self.source)
        self.assertIn('[string]$candidate.phase -ne $expectedRunnerPhase', self.source)
        self.assertIn("$completedCaptureAllowanceSeconds = 1800", self.source)

    def test_adoption_fails_closed_before_skipping_restore_and_launch(self) -> None:
        runner_ready = self.source.index("runner_ready run_id=")
        branch_start = self.source.index(
            "if ($adoptingAnyService) {",
            runner_ready,
        )
        branch_end = self.source.index(
            '    Invoke-Runner @("sync")',
            branch_start,
        )
        branch = self.source[branch_start:branch_end]

        self.assertIn("$launchAttempted = $true", branch)
        self.assertIn("$ExpectedServiceMainPid", branch)
        self.assertIn("$ExpectedServiceInvocationId", branch)
        self.assertIn("$expectedExecStart", branch)
        self.assertIn("$batchCommand", branch)
        self.assertIn("$corpusSha256", branch)
        self.assertIn("$CheckpointSha256", branch)
        self.assertIn("$umlautSha256", branch)
        self.assertIn("$vampireSha256", branch)
        self.assertIn("$expectedContractFile", branch)
        self.assertIn("test ! -e '$remoteCheckpointRoot'", branch)
        self.assertIn("result count outside recovery range", branch)
        self.assertIn("existing_service_adopted", branch)
        self.assertIn("completed_service_adopted", branch)
        self.assertIn("Get-VerifiedCompletedServiceEvidence", branch)
        self.assertNotIn('Invoke-Runner @("sync")', branch)
        self.assertIn(
            'Invoke-Runner @("exec", "--", $launchCommand)',
            self.source,
        )

    def test_probe_failure_retains_launched_runner_for_recovery(self) -> None:
        self.assertIn('Write-ResumeLog "controller_failed error=', self.source)
        self.assertIn(
            'Write-ResumeLog "runner_retained_for_recovery"',
            self.source,
        )

    def run_terminal_verifier(
        self,
        records: list[dict[str, object]],
        *,
        command: str = "/usr/bin/python3 guarded-batch.py --exact",
    ) -> subprocess.CompletedProcess[str]:
        unit = "casc-j13-v2-resume-260803-135624-bc09.service"
        invocation = "a" * 32
        contract = "b" * 64
        return subprocess.run(
            [
                sys.executable,
                "-c",
                self.terminal_verifier,
                unit,
                invocation,
                "3995",
                base64.b64encode(command.encode("utf-8")).decode("ascii"),
                contract,
            ],
            input="".join(json.dumps(record) + "\n" for record in records),
            capture_output=True,
            text=True,
            check=False,
        )

    def terminal_records(self) -> list[dict[str, object]]:
        unit = "casc-j13-v2-resume-260803-135624-bc09.service"
        invocation = "a" * 32
        boot = "c" * 32
        command = "/usr/bin/python3 guarded-batch.py --exact"
        process = {
            "_SYSTEMD_UNIT": unit,
            "_SYSTEMD_INVOCATION_ID": invocation,
            "_PID": "3995",
            "_CMDLINE": command,
            "_BOOT_ID": boot,
            "__SEQNUM": "1",
            "MESSAGE": "solver progress",
        }
        summary = {
            **process,
            "__SEQNUM": "2",
            "MESSAGE": f"OK: contract {'b' * 64}; new=492, resumed=965",
        }
        success = {
            "UNIT": unit,
            "INVOCATION_ID": invocation,
            "_SYSTEMD_UNIT": "init.scope",
            "_BOOT_ID": boot,
            "__SEQNUM": "3",
            "MESSAGE_ID": "7ad2d189f7e94e70a38c781354912448",
            "MESSAGE": f"{unit}: Deactivated successfully.",
        }
        resource = {
            "UNIT": unit,
            "INVOCATION_ID": invocation,
            "_SYSTEMD_UNIT": "init.scope",
            "_BOOT_ID": boot,
            "__SEQNUM": "4",
            "MESSAGE": f"{unit}: Consumed resources.",
        }
        return [process, summary, success, resource]

    def test_terminal_journal_verifier_accepts_exact_completed_identity(self) -> None:
        result = self.run_terminal_verifier(self.terminal_records())
        self.assertEqual(result.returncode, 0, result.stderr)
        evidence = json.loads(result.stdout)
        self.assertEqual(evidence["unit"], self.terminal_records()[0]["_SYSTEMD_UNIT"])
        self.assertEqual(evidence["invocation_id"], "a" * 32)
        self.assertEqual(evidence["main_pid"], 3995)
        self.assertEqual(evidence["reported_results"], 1457)
        self.assertEqual(evidence["boot_id"], "c" * 32)

    def test_terminal_journal_verifier_rejects_identity_ambiguity(self) -> None:
        base = self.terminal_records()
        cases: dict[str, list[dict[str, object]]] = {}

        mixed_invocation = copy.deepcopy(base)
        mixed_invocation.append(
            {
                **copy.deepcopy(base[0]),
                "_SYSTEMD_INVOCATION_ID": "d" * 32,
                "__SEQNUM": "5",
            }
        )
        cases["mixed invocation"] = mixed_invocation

        wrong_command = copy.deepcopy(base)
        wrong_command[0]["_CMDLINE"] = "/usr/bin/python3 replacement.py"
        wrong_command[1]["_CMDLINE"] = "/usr/bin/python3 replacement.py"
        cases["wrong command"] = wrong_command

        cases["missing terminal success"] = copy.deepcopy(base[:2] + base[3:])

        duplicate_success = copy.deepcopy(base)
        duplicate = copy.deepcopy(duplicate_success[2])
        duplicate["__SEQNUM"] = "5"
        duplicate_success.append(duplicate)
        cases["duplicate terminal success"] = duplicate_success

        terminal_before_summary = copy.deepcopy(base)
        terminal_before_summary[2]["__SEQNUM"] = "1"
        terminal_before_summary[1]["__SEQNUM"] = "3"
        cases["terminal before summary"] = terminal_before_summary

        mixed_boot = copy.deepcopy(base)
        mixed_boot[3]["_BOOT_ID"] = "e" * 32
        cases["mixed boot"] = mixed_boot

        for name, records in cases.items():
            with self.subTest(name=name):
                result = self.run_terminal_verifier(records)
                self.assertNotEqual(result.returncode, 0, result.stdout)


if __name__ == "__main__":
    unittest.main()
