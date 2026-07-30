#!/usr/bin/env python3
"""Regression tests for the online-stagnation experiment scripts."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]
sys.path.insert(0, str(ROOT))


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


COMMON = load_module("online_adaptation_common_test", ROOT / "common.py")
SELECT = load_module("online_adaptation_select_test", ROOT / "select_corpus.py")
RUN = load_module("online_adaptation_run_test", ROOT / "run.py")
ANALYZE = load_module(
    "online_adaptation_analyze_test", ROOT / "analyze.py"
)


def telemetry(
    *,
    processed: int = 100,
    generated: int = 1_000,
    passive: int = 800,
    cpu: float = 1.0,
) -> dict[str, object]:
    return {
        "schema": "umlaut.search-telemetry",
        "schema_version": 1,
        "search_funnel": {
            "processed_non_trivial": processed,
            "generated_non_trivial": generated,
            "high_water_unprocessed": passive,
        },
        "resources": {
            "maximum_resident_pages": 100,
            "total_cpu_seconds": cpu,
        },
    }


def primitive(
    policy: str,
    *,
    problem: str = "P",
    repetition: int = 1,
    status: str = "ResourceOut",
    cpu: float = 1.0,
    probe_telemetry: dict[str, object] | None = None,
) -> dict[str, object]:
    return {
        "policy": policy,
        "problem_id": problem,
        "repetition": repetition,
        "szs_status": status,
        "proof_steps": 2 if status == "Unsatisfiable" else 0,
        "telemetry_cpu_seconds": cpu,
        "policy_wall_seconds": cpu,
        "configured_cpu_seconds": (
            1
            if policy == "probe"
            else 4
            if policy.startswith("continuation")
            else 5
        ),
        "_phase_telemetry": [
            (
                probe_telemetry
                if policy == "probe"
                else telemetry(cpu=cpu)
            )
        ],
    }


class SignalTests(unittest.TestCase):
    def test_threshold_switch_and_keep_are_exact(self) -> None:
        record = telemetry(processed=100, generated=1_000)
        self.assertEqual(COMMON.choose_branch(record, 8.0)["branch"], "goal")
        self.assertEqual(
            COMMON.choose_branch(record, 16.0)["branch"], "global"
        )

    def test_missing_and_small_probes_take_goal_fallback(self) -> None:
        missing = COMMON.choose_branch(None, 16.0)
        self.assertEqual(missing["branch"], "goal")
        self.assertEqual(missing["fallback_reason"], "missing_telemetry")
        small = COMMON.choose_branch(
            telemetry(processed=63, generated=0), 16.0
        )
        self.assertEqual(small["branch"], "goal")
        self.assertEqual(
            small["fallback_reason"], "insufficient_processed_clauses"
        )

    def test_unregistered_threshold_is_rejected(self) -> None:
        with self.assertRaises(COMMON.ExperimentError):
            COMMON.choose_branch(telemetry(), 10.0)


class CorpusTests(unittest.TestCase):
    def test_selector_uses_disjoint_families_and_frozen_quotas(self) -> None:
        manifest = (
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        rows = COMMON.read_jsonl(manifest)
        selected = SELECT.select_records(rows[1:])
        self.assertEqual(len(selected), 24)
        counts = {
            split: {
                category: sum(
                    record["experiment_split"] == split
                    and record["category"] == category
                    for record in selected
                )
                for category in ("EPU", "UEQ")
            }
            for split in ("calibration", "validation", "test")
        }
        self.assertEqual(counts, SELECT.QUOTAS)
        families = {
            split: {
                record["family"]
                for record in selected
                if record["experiment_split"] == split
            }
            for split in counts
        }
        self.assertFalse(families["calibration"] & families["validation"])
        self.assertFalse(families["calibration"] & families["test"])
        self.assertFalse(families["validation"] & families["test"])

    def test_selector_does_not_read_outcome_fields(self) -> None:
        record = {
            "record_type": "problem",
            "category": "EPU",
            "problem_id": "HWV001+1",
            "sha256": "a" * 64,
            "expected_class": "unsatisfiable",
            "size_bytes": 300,
            "invented_outcome": "Theorem",
        }
        rank = SELECT.selection_rank("calibration", record)
        changed = dict(record)
        changed["invented_outcome"] = "ResourceOut"
        self.assertEqual(
            rank, SELECT.selection_rank("calibration", changed)
        )


class CalibrationTests(unittest.TestCase):
    @staticmethod
    def matrix_results(
        *,
        ratio: float,
        goal_solves: bool,
        global_solves: bool,
    ) -> list[dict[str, object]]:
        results = []
        for repetition in (1, 2):
            probe_record = telemetry(
                processed=100, generated=int(100 * ratio)
            )
            results.extend(
                [
                    primitive(
                        "global_full", repetition=repetition, cpu=5.0
                    ),
                    primitive(
                        "goal_full", repetition=repetition, cpu=5.0
                    ),
                    primitive(
                        "probe",
                        repetition=repetition,
                        cpu=1.0,
                        probe_telemetry=probe_record,
                    ),
                    primitive(
                        "continuation_global",
                        repetition=repetition,
                        status=(
                            "Unsatisfiable"
                            if global_solves
                            else "ResourceOut"
                        ),
                        cpu=4.0,
                    ),
                    primitive(
                        "continuation_goal",
                        repetition=repetition,
                        status=(
                            "Unsatisfiable"
                            if goal_solves
                            else "ResourceOut"
                        ),
                        cpu=4.0,
                    ),
                ]
            )
        return results

    def test_calibration_selects_highest_solve_preserving_threshold(
        self,
    ) -> None:
        results = self.matrix_results(
            ratio=10.0, goal_solves=True, global_solves=False
        )
        selection, traces = ANALYZE.calibration_selection(
            results, {"repetitions": 2}
        )
        self.assertEqual(selection["selected_threshold"], 8.0)
        self.assertEqual({trace["branch"] for trace in traces}, {"goal"})

    def test_exact_no_solve_tie_prefers_fewer_interventions(self) -> None:
        results = self.matrix_results(
            ratio=10.0, goal_solves=False, global_solves=False
        )
        selection, traces = ANALYZE.calibration_selection(
            results, {"repetitions": 2}
        )
        self.assertEqual(selection["selected_threshold"], 64.0)
        self.assertEqual(
            {trace["branch"] for trace in traces}, {"global"}
        )

    def test_combined_policy_sums_cpu_and_stops_on_probe_proof(self) -> None:
        probe = primitive(
            "probe", status="Unsatisfiable", cpu=0.25
        )
        continuation = primitive(
            "continuation_goal", status="ResourceOut", cpu=4.0
        )
        combined = ANALYZE.combine_primitives(
            probe, continuation, "adaptive", {"branch": "probe_solved"}
        )
        self.assertEqual(combined["szs_status"], "Unsatisfiable")
        self.assertEqual(combined["telemetry_cpu_seconds"], 0.25)
        self.assertEqual(combined["configured_cpu_seconds"], 1)


class IntegrityTests(unittest.TestCase):
    def test_load_selection_verifies_identifier(self) -> None:
        body = {
            "schema_version": 1,
            "source_revision": COMMON.SOURCE_REVISION,
            "selected_threshold": 16.0,
        }
        selection = {
            **body,
            "selection_id": COMMON.sha256_bytes(
                COMMON.canonical_json(body)
            ),
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "selection.json"
            path.write_text(json.dumps(selection), encoding="utf-8")
            loaded = RUN.load_selection(path)
            self.assertEqual(loaded["selected_threshold"], 16.0)
            selection["selected_threshold"] = 8.0
            path.write_text(json.dumps(selection), encoding="utf-8")
            with self.assertRaises(RUN.common.ExperimentError):
                RUN.load_selection(path)

    def test_raw_phase_hashes_are_verified(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            phase_dir = root / "phase-01-probe-global"
            phase_dir.mkdir()
            stdout = phase_dir / "stdout.pcl"
            stderr = phase_dir / "stderr.txt"
            telemetry_path = phase_dir / "telemetry.json"
            stdout.write_text("% SZS status ResourceOut\n", encoding="utf-8")
            stderr.write_text("", encoding="utf-8")
            telemetry_path.write_text(
                json.dumps(telemetry()), encoding="utf-8"
            )
            phase = {
                "artifact_directory": phase_dir.name,
                "stdout_sha256": COMMON.sha256_file(stdout),
                "stderr_sha256": COMMON.sha256_file(stderr),
                "telemetry_sha256": COMMON.sha256_file(telemetry_path),
                "proof_steps": 0,
            }
            result_path = root / "result.json"
            result_path.write_text("{}", encoding="utf-8")
            loaded = ANALYZE.load_telemetry(result_path, phase)
            self.assertEqual(
                loaded["schema"], "umlaut.search-telemetry"
            )
            stdout.write_text("changed\n", encoding="utf-8")
            with self.assertRaises(ANALYZE.common.ExperimentError):
                ANALYZE.load_telemetry(result_path, phase)


class ContractTests(unittest.TestCase):
    def test_frozen_source_and_policy_shape(self) -> None:
        self.assertEqual(
            COMMON.SOURCE_REVISION,
            "42bfa440729dfe214042020898f7ba87fed7ab4f",
        )
        self.assertEqual(
            set(RUN.POLICIES["validation"]),
            ANALYZE.EVALUATION_POLICIES,
        )
        self.assertEqual(
            sum(
                RUN.DEFAULT_BUDGETS[kind]["soft_cpu_seconds"]
                for kind in ("probe", "continuation")
            ),
            RUN.DEFAULT_BUDGETS["full"]["soft_cpu_seconds"],
        )


if __name__ == "__main__":
    unittest.main()
