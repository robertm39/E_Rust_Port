#!/usr/bin/env python3
"""Focused tests for deterministic CASC successor planning."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("plan_next_casc_resume.py")
SPEC = importlib.util.spec_from_file_location("plan_next_casc_resume", SCRIPT)
if SPEC is None or SPEC.loader is None:  # pragma: no cover
    raise RuntimeError(f"cannot load {SCRIPT}")
PLANNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLANNER)


class ResumePlanningTests(unittest.TestCase):
    CHECKPOINT = Path("checkpoint.tar.gz")
    SHA256 = "a" * 64

    @staticmethod
    def validation(completed: int) -> dict:
        config = PLANNER.RELEASES["j13"]
        return {
            "schema_version": 1,
            "kind": "umlaut-casc-checkpoint-validation",
            "archive": {"sha256": "a" * 64},
            "run": {
                "run_name": config["run_name"],
                "contract_id": config["contract_id"],
                "expected_results": config["expected_results"],
                "completed_results": completed,
            },
        }

    @staticmethod
    def allowance(*, available: bool) -> dict:
        return {
            "schema_version": 1,
            "kind": "umlaut-linode-high-memory-allowance",
            "required_seconds": 14700,
            "active_managed_high_memory": 0,
            "observed_at_utc": "2026-08-02T02:00:00+00:00",
            "required_start_available_now": available,
            "projected_earliest_required_start_utc": (
                "2026-08-02T05:00:00+00:00" if not available else None
            ),
            "remaining_seconds": 16000 if available else 1249,
        }

    def test_plans_guarded_boundary_from_validated_inputs(self) -> None:
        value = PLANNER.build_resume_plan(
            release="j13",
            checkpoint=self.CHECKPOINT,
            checkpoint_sha256=self.SHA256,
            validation=self.validation(965),
            allowance=self.allowance(available=False),
            max_session_wall_seconds=14400,
            boundary_guard_seconds=10,
        )
        self.assertEqual(value["status"], "ready_to_arm")
        arguments = value["controller"]["arguments"]
        self.assertIn("965", arguments)
        self.assertIn("2026-08-02T05:00:10+00:00", arguments)

    def test_complete_release_has_no_controller(self) -> None:
        value = PLANNER.build_resume_plan(
            release="j13",
            checkpoint=self.CHECKPOINT,
            checkpoint_sha256=self.SHA256,
            validation=self.validation(2700),
            allowance=self.allowance(available=True),
            max_session_wall_seconds=14400,
            boundary_guard_seconds=10,
        )
        self.assertEqual(value["status"], "release_complete")
        self.assertIsNone(value["controller"])

    def test_rejects_allowance_for_wrong_duration(self) -> None:
        allowance = self.allowance(available=True)
        allowance["required_seconds"] = 14699
        with self.assertRaisesRegex(PLANNER.PlanningError, "required seconds"):
            PLANNER.build_resume_plan(
                release="j13",
                checkpoint=self.CHECKPOINT,
                checkpoint_sha256=self.SHA256,
                validation=self.validation(965),
                allowance=allowance,
                max_session_wall_seconds=14400,
                boundary_guard_seconds=10,
            )

    def test_rejects_inconsistent_allowance_decision(self) -> None:
        allowance = self.allowance(available=True)
        allowance["remaining_seconds"] = 1
        with self.assertRaisesRegex(PLANNER.PlanningError, "inconsistent"):
            PLANNER.build_resume_plan(
                release="j13",
                checkpoint=self.CHECKPOINT,
                checkpoint_sha256=self.SHA256,
                validation=self.validation(965),
                allowance=allowance,
                max_session_wall_seconds=14400,
                boundary_guard_seconds=10,
            )


if __name__ == "__main__":
    unittest.main()
