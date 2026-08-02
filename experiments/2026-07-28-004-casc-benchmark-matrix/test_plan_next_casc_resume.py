#!/usr/bin/env python3
"""Focused tests for deterministic CASC successor planning."""

from __future__ import annotations

import importlib.util
import io
import json
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

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
    def validation(completed: int, release: str = "j13") -> dict:
        config = PLANNER.RELEASES[release]
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

    @classmethod
    def combined_validation(cls) -> dict:
        validation = cls.validation(5802, "casc2025")
        validation["combined"] = {
            "summary_sha256": "b" * 64,
            "embedded": True,
            "releases": ["casc2025", "j13"],
            "targeted_problems": 4251,
            "expected_results": 8502,
            "completed_results": 8502,
            "missing_results": 0,
            "official_csv_count": 66,
        }
        return validation

    def run_main(self, responses: list[dict]) -> tuple[int, dict, object, object]:
        output = io.StringIO()
        with (
            patch.object(PLANNER, "run_json", side_effect=responses) as runner,
            patch.object(
                PLANNER.shutil,
                "which",
                return_value="powershell.exe",
            ) as which,
            redirect_stdout(output),
        ):
            status = PLANNER.main(
                [
                    "--checkpoint",
                    str(self.CHECKPOINT),
                    "--checkpoint-sha256",
                    self.SHA256,
                ]
            )
        return status, json.loads(output.getvalue()), runner, which

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

    def test_validated_count_rejects_missing_run_object(self) -> None:
        validation = self.validation(965)
        validation["run"] = None
        with self.assertRaisesRegex(PLANNER.PlanningError, "run object"):
            PLANNER.validated_result_count(
                release="j13",
                checkpoint_sha256=self.SHA256,
                validation=validation,
            )

    def test_campaign_completion_requires_both_exact_boundaries(self) -> None:
        with self.assertRaisesRegex(PLANNER.PlanningError, "every release"):
            PLANNER.build_campaign_complete_plan(
                checkpoint=self.CHECKPOINT,
                checkpoint_sha256=self.SHA256,
                completed_results={"j13": 2700, "casc2025": 5801},
                combined_validation=self.combined_validation(),
            )

    def test_campaign_completion_has_no_allowance_or_controller(self) -> None:
        value = PLANNER.build_campaign_complete_plan(
            checkpoint=self.CHECKPOINT,
            checkpoint_sha256=self.SHA256,
            completed_results={"j13": 2700, "casc2025": 5802},
            combined_validation=self.combined_validation(),
        )
        self.assertEqual(value["status"], "campaign_complete")
        self.assertIsNone(value["release"])
        self.assertIsNone(value["allowance"])
        self.assertIsNone(value["controller"])
        self.assertEqual(value["combined_report"]["completed_results"], 8502)

    def test_campaign_completion_rejects_incomplete_combined_report(self) -> None:
        validation = self.combined_validation()
        validation["combined"]["missing_results"] = 1
        with self.assertRaisesRegex(PLANNER.PlanningError, "missing results"):
            PLANNER.build_campaign_complete_plan(
                checkpoint=self.CHECKPOINT,
                checkpoint_sha256=self.SHA256,
                completed_results={"j13": 2700, "casc2025": 5802},
                combined_validation=validation,
            )

    def test_auto_mode_stops_validation_at_incomplete_j13(self) -> None:
        status, value, runner, which = self.run_main(
            [self.validation(965), self.allowance(available=True)]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "j13")
        self.assertEqual(runner.call_count, 2)
        which.assert_called_once()

    def test_auto_mode_transitions_to_casc2025(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(2700),
                self.validation(0, "casc2025"),
                self.allowance(available=True),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "casc2025")
        self.assertEqual(runner.call_count, 3)
        second_command = runner.call_args_list[1].args[0]
        self.assertTrue(
            any("casc_2025_manifest.jsonl" in str(part) for part in second_command)
        )
        which.assert_called_once()

    def test_auto_mode_ends_without_provider_query(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(2700),
                self.validation(5802, "casc2025"),
                self.combined_validation(),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["status"], "campaign_complete")
        self.assertEqual(runner.call_count, 3)
        combined_command = runner.call_args_list[2].args[0]
        self.assertEqual(combined_command.count("--combined-run"), 2)
        manifest_index = combined_command.index("--manifest") + 1
        self.assertIn("casc_2025_manifest.jsonl", combined_command[manifest_index])
        which.assert_not_called()


if __name__ == "__main__":
    unittest.main()
