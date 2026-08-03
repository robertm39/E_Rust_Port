#!/usr/bin/env python3
"""Focused structural tests for guarded CASC controller probe deadlines."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("resume_j13_checkpoint.ps1")


class CascResumeControllerProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")

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
            6,
        )
        self.assertIn(
            'Invoke-Runner @("exec", "--", $captureCommand)',
            self.source,
        )

    def test_adoption_requires_complete_exact_identity(self) -> None:
        self.assertIn("[switch]$AdoptExistingService", self.source)
        self.assertIn("[int64]$ExpectedServiceMainPid", self.source)
        self.assertIn("[string]$ExpectedServiceInvocationId", self.source)
        self.assertIn(
            '"AdoptExistingService requires Execute, ExistingRunnerRunId, "',
            self.source,
        )
        self.assertIn('forbids NotBeforeUtc"', self.source)
        self.assertIn('$expectedRunnerPhase = if ($adoptingExistingService)', self.source)
        self.assertIn('"synced"', self.source)
        self.assertIn('[string]$candidate.phase -ne $expectedRunnerPhase', self.source)

    def test_adoption_fails_closed_before_skipping_restore_and_launch(self) -> None:
        runner_ready = self.source.index("runner_ready run_id=")
        branch_start = self.source.index(
            "if ($adoptingExistingService) {",
            runner_ready,
        )
        branch_end = self.source.index("\n    else {", branch_start)
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


if __name__ == "__main__":
    unittest.main()
