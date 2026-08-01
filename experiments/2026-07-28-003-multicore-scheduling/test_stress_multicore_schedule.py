#!/usr/bin/env python3
"""Focused tests for lifecycle-result classification."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("stress_multicore_schedule.py")
SPEC = importlib.util.spec_from_file_location("stress_multicore_schedule", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class WorkerCrashContainedTests(unittest.TestCase):
    def test_accepts_checked_proof_after_killed_worker(self) -> None:
        case = {
            "action_pid": 42,
            "exit_code": 0,
            "stdout": (
                "% worker with pid 42 completed with status -1\n"
                "% SZS output start CNFRefutation\n"
                "proof\n"
                "% SZS output end CNFRefutation\n"
            ),
        }
        self.assertTrue(MODULE.worker_crash_contained(case))

    def test_accepts_clean_resource_exhaustion_after_killed_worker(self) -> None:
        case = {
            "action_pid": 42,
            "exit_code": MODULE.RESOURCE_OUT_EXIT,
            "stdout": (
                "% worker with pid 42 completed with status -1\n"
                "% Schedule exhausted\n"
                "% SZS status GaveUp\n"
            ),
        }
        self.assertTrue(MODULE.worker_crash_contained(case))

    def test_rejects_missing_killed_worker_provenance(self) -> None:
        case = {
            "action_pid": 42,
            "exit_code": MODULE.RESOURCE_OUT_EXIT,
            "stdout": "% Schedule exhausted\n% SZS status GaveUp\n",
        }
        self.assertFalse(MODULE.worker_crash_contained(case))

    def test_rejects_signal_terminated_parent(self) -> None:
        case = {
            "action_pid": 42,
            "exit_code": -9,
            "stdout": "% worker with pid 42 completed with status -1\n",
        }
        self.assertFalse(MODULE.worker_crash_contained(case))

    def test_rejects_unfinished_resource_out(self) -> None:
        case = {
            "action_pid": 42,
            "exit_code": MODULE.RESOURCE_OUT_EXIT,
            "stdout": "% worker with pid 42 completed with status -1\n",
        }
        self.assertFalse(MODULE.worker_crash_contained(case))


if __name__ == "__main__":
    unittest.main()
