#!/usr/bin/env python3
"""Focused tests for deterministic CASC successor planning."""

from __future__ import annotations

import importlib.util
import io
import json
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("plan_next_casc_resume.py")
SPEC = importlib.util.spec_from_file_location("plan_next_casc_resume", SCRIPT)
if SPEC is None or SPEC.loader is None:  # pragma: no cover
    raise RuntimeError(f"cannot load {SCRIPT}")
PLANNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLANNER)
CONTROLLER = SCRIPT.with_name("resume_j13_checkpoint.ps1")


class ResumeControllerSourceTests(unittest.TestCase):
    def test_native_output_logging_ignores_empty_records(self) -> None:
        source = CONTROLLER.read_text(encoding="utf-8")

        self.assertEqual(
            source.count('if (-not [string]::IsNullOrEmpty($text))'),
            2,
        )

    def test_frozen_contract_identity_is_explicit_in_both_batch_calls(self) -> None:
        source = CONTROLLER.read_text(encoding="utf-8")

        self.assertEqual(source.count('"--expected-contract-id"'), 1)
        self.assertEqual(
            source.count("--expected-contract-id '$expectedContract'"),
            1,
        )

    def test_restore_inventory_and_batch_verification_are_separate(self) -> None:
        source = CONTROLLER.read_text(encoding="utf-8")

        self.assertIn("checkpoint_restore_verified", source)
        self.assertIn(
            '$verifyCommand = "cd /opt/e-rust-port/source && "',
            source,
        )
        self.assertIn('"python3 tools/casc_benchmark/batch.py "', source)
        preflight = source.split('$preflightCommand = @"', 1)[1].split('"@', 1)[0]
        self.assertNotIn("grep -Fq", preflight)
        self.assertIn("summary contract mismatch", preflight)


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
    def combined_validation(
        cls,
        *,
        selected_release: str = "casc2025",
        j13: int = 2700,
        casc2025: int = 5802,
    ) -> dict:
        selected_count = j13 if selected_release == "j13" else casc2025
        validation = cls.validation(selected_count, selected_release)
        completed = j13 + casc2025
        validation["combined"] = {
            "summary_sha256": "b" * 64,
            "embedded": True,
            "releases": ["CASC-2025", "CASC-J13"],
            "release_completed_results": {
                "CASC-2025": casc2025,
                "CASC-J13": j13,
            },
            "targeted_problems": 4251,
            "expected_results": 8502,
            "completed_results": completed,
            "missing_results": 8502 - completed,
            "official_csv_count": 66,
        }
        return validation

    def run_main(
        self,
        responses: list[object],
        extra_arguments: list[str] | None = None,
    ) -> tuple[int, dict, object, object]:
        output = io.StringIO()
        arguments = [
            "--checkpoint",
            str(self.CHECKPOINT),
            "--checkpoint-sha256",
            self.SHA256,
        ]
        if extra_arguments is not None:
            arguments.extend(extra_arguments)
        with (
            patch.object(PLANNER, "run_json", side_effect=responses) as runner,
            patch.object(
                PLANNER.shutil,
                "which",
                return_value="powershell.exe",
            ) as which,
            redirect_stdout(output),
        ):
            status = PLANNER.main(arguments)
        return status, json.loads(output.getvalue()), runner, which

    def run_main_error(
        self, responses: list[object]
    ) -> tuple[int, str, object, object]:
        output = io.StringIO()
        error_output = io.StringIO()
        with (
            patch.object(PLANNER, "run_json", side_effect=responses) as runner,
            patch.object(
                PLANNER.shutil,
                "which",
                return_value="powershell.exe",
            ) as which,
            redirect_stdout(output),
            redirect_stderr(error_output),
        ):
            status = PLANNER.main(
                [
                    "--checkpoint",
                    str(self.CHECKPOINT),
                    "--checkpoint-sha256",
                    self.SHA256,
                ]
            )
        self.assertEqual(output.getvalue(), "")
        return status, error_output.getvalue(), runner, which

    def test_plans_guarded_boundary_from_validated_inputs(self) -> None:
        value = PLANNER.build_resume_plan(
            release="j13",
            checkpoint=self.CHECKPOINT,
            checkpoint_sha256=self.SHA256,
            completed_results=965,
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
            completed_results=2700,
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
                completed_results=965,
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
                completed_results=965,
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
                selected_release="casc2025",
                combined_validation=self.combined_validation(),
            )

    def test_campaign_completion_has_no_allowance_or_controller(self) -> None:
        value = PLANNER.build_campaign_complete_plan(
            checkpoint=self.CHECKPOINT,
            checkpoint_sha256=self.SHA256,
            completed_results={"j13": 2700, "casc2025": 5802},
            selected_release="casc2025",
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
                selected_release="casc2025",
                combined_validation=validation,
            )

    def test_combined_counts_reject_selected_count_disagreement(self) -> None:
        validation = self.combined_validation(casc2025=50)
        validation["combined"]["release_completed_results"]["CASC-2025"] = 49
        with self.assertRaisesRegex(PLANNER.PlanningError, "selected and combined"):
            PLANNER.validated_combined_result_counts(
                selected_release="casc2025",
                checkpoint_sha256=self.SHA256,
                validation=validation,
            )

    def test_auto_mode_stops_validation_at_incomplete_j13(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(965),
                PLANNER.PlanningError("not the outer release"),
                self.allowance(available=True),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "j13")
        self.assertEqual(runner.call_count, 3)
        which.assert_called_once()

    def test_inspect_only_reports_j13_without_allowance(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(965),
                PLANNER.PlanningError("not the outer release"),
            ],
            ["--inspect-only"],
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["kind"], "umlaut-casc-checkpoint-state")
        self.assertEqual(value["release"], "j13")
        self.assertEqual(value["outer_release"], "j13")
        self.assertEqual(value["checkpoint"]["completed_results"], 965)
        self.assertEqual(runner.call_count, 2)
        which.assert_not_called()

    def test_auto_mode_transitions_to_casc2025(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(2700),
                PLANNER.PlanningError("not the outer release"),
                self.combined_validation(
                    selected_release="j13",
                    j13=2700,
                    casc2025=0,
                ),
                self.allowance(available=True),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "casc2025")
        self.assertEqual(runner.call_count, 4)
        combined_command = runner.call_args_list[2].args[0]
        manifest_index = combined_command.index("--manifest") + 1
        self.assertIn("casc_2026_manifest.jsonl", combined_command[manifest_index])
        which.assert_called_once()

    def test_inspect_only_reports_cross_release_transition(self) -> None:
        status, value, runner, which = self.run_main(
            [
                self.validation(2700),
                PLANNER.PlanningError("not the outer release"),
                self.combined_validation(
                    selected_release="j13",
                    j13=2700,
                    casc2025=0,
                ),
            ],
            ["--inspect-only"],
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "casc2025")
        self.assertEqual(value["outer_release"], "j13")
        self.assertEqual(value["checkpoint"]["completed_results"], 0)
        self.assertEqual(
            value["campaign_completed_results"],
            {"j13": 2700, "casc2025": 0},
        )
        self.assertEqual(runner.call_count, 3)
        which.assert_not_called()

    def test_auto_mode_continues_casc2025_from_its_outer_inventory(self) -> None:
        status, value, runner, which = self.run_main(
            [
                PLANNER.PlanningError("not the outer release"),
                self.validation(50, "casc2025"),
                self.combined_validation(casc2025=50),
                self.allowance(available=True),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["release"], "casc2025")
        self.assertEqual(value["checkpoint"]["completed_results"], 50)
        self.assertEqual(runner.call_count, 4)
        which.assert_called_once()

    def test_auto_mode_rejects_ambiguous_outer_inventory(self) -> None:
        status, error, runner, which = self.run_main_error(
            [self.validation(965), self.validation(50, "casc2025")]
        )
        self.assertEqual(status, 2)
        self.assertIn("did not select exactly one release", error)
        self.assertEqual(runner.call_count, 2)
        which.assert_not_called()

    def test_auto_mode_rejects_advancing_past_incomplete_j13(self) -> None:
        status, error, runner, which = self.run_main_error(
            [
                PLANNER.PlanningError("not the outer release"),
                self.validation(50, "casc2025"),
                self.combined_validation(j13=2699, casc2025=50),
            ]
        )
        self.assertEqual(status, 2)
        self.assertIn("advanced past an incomplete release", error)
        self.assertEqual(runner.call_count, 3)
        which.assert_not_called()

    def test_auto_mode_ends_without_provider_query(self) -> None:
        status, value, runner, which = self.run_main(
            [
                PLANNER.PlanningError("not the outer release"),
                self.validation(5802, "casc2025"),
                self.combined_validation(),
            ]
        )
        self.assertEqual(status, 0)
        self.assertEqual(value["status"], "campaign_complete")
        self.assertEqual(runner.call_count, 3)
        combined_command = runner.call_args_list[2].args[0]
        self.assertEqual(combined_command.count("--combined-run"), 2)
        combined_indexes = [
            index
            for index, argument in enumerate(combined_command)
            if argument == "--combined-run"
        ]
        combined_labels = [
            combined_command[index + 1] for index in combined_indexes
        ]
        self.assertEqual(combined_labels, ["CASC-2025", "CASC-J13"])
        manifest_index = combined_command.index("--manifest") + 1
        self.assertIn("casc_2025_manifest.jsonl", combined_command[manifest_index])
        which.assert_not_called()


if __name__ == "__main__":
    unittest.main()
