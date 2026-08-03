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
            4,
        )
        self.assertIn(
            'Invoke-Runner @("exec", "--", $captureCommand)',
            self.source,
        )
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
