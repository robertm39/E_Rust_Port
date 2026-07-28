#!/usr/bin/env python3
"""Regression tests for the layered clause-selection experiment scripts."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from types import ModuleType


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUN = load_module("layered_clause_selection_run", EXPERIMENT_ROOT / "run.py")
ANALYZE = load_module(
    "layered_clause_selection_analyze", EXPERIMENT_ROOT / "analyze.py"
)


class ExperimentScriptTests(unittest.TestCase):
    def test_expected_status_classes_and_contradictions(self) -> None:
        matching = (
            ("theorem", "Theorem"),
            ("unsatisfiable", "Unsatisfiable"),
            ("non_theorem", "CounterSatisfiable"),
            ("satisfiable", "Satisfiable"),
        )
        for expected_class, status in matching:
            with self.subTest(expected_class=expected_class, status=status):
                self.assertTrue(RUN.expected_status_match(expected_class, status))
                self.assertFalse(
                    ANALYZE.status_contradicts_expected(
                        {"expected_class": expected_class, "szs_status": status}
                    )
                )

        self.assertFalse(RUN.expected_status_match("theorem", "ResourceOut"))
        self.assertFalse(
            ANALYZE.status_contradicts_expected(
                {"expected_class": "theorem", "szs_status": "ResourceOut"}
            )
        )
        self.assertTrue(
            ANALYZE.status_contradicts_expected(
                {"expected_class": "theorem", "szs_status": "Satisfiable"}
            )
        )
        self.assertTrue(
            ANALYZE.status_contradicts_expected(
                {"expected_class": "satisfiable", "szs_status": "Theorem"}
            )
        )

    def test_selected_manifest_strata_are_deterministic(self) -> None:
        _, records = RUN.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        first = RUN.select_records(records, 6)
        second = RUN.select_records(records, 6)

        self.assertEqual(
            [record["problem_id"] for record in first],
            [record["problem_id"] for record in second],
        )
        self.assertEqual(len(first), 44)
        self.assertEqual(
            Counter((record["holdout_split"], record["category"]) for record in first),
            Counter(
                {
                    ("validation", "FNE"): 6,
                    ("validation", "FEQ"): 6,
                    ("validation", "EPS"): 6,
                    ("validation", "SLH"): 6,
                    ("test", "FNE"): 6,
                    ("test", "FEQ"): 6,
                    ("test", "EPS"): 2,
                    ("test", "SLH"): 6,
                }
            ),
        )

    def test_optional_telemetry_preserves_empty_file_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "telemetry.json"
            self.assertEqual(RUN.load_optional_telemetry(path), (None, None, None))

            path.write_bytes(b"")
            telemetry, digest, error = RUN.load_optional_telemetry(path)
            self.assertIsNone(telemetry)
            self.assertEqual(digest, RUN.sha256_file(path))
            self.assertTrue(error.startswith("JSONDecodeError:"))

            path.write_text(
                '{"schema":"umlaut.search-telemetry"}', encoding="utf-8"
            )
            telemetry, digest, error = RUN.load_optional_telemetry(path)
            self.assertEqual(telemetry["schema"], "umlaut.search-telemetry")
            self.assertEqual(digest, RUN.sha256_file(path))
            self.assertIsNone(error)

    def test_resume_requires_matching_artifact_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary)
            stdout = run_dir / "stdout.txt"
            stderr = run_dir / "stderr.txt"
            result_path = run_dir / "result.json"
            stdout.write_text("proof\n", encoding="utf-8")
            stderr.write_text("", encoding="utf-8")
            result_path.write_text(
                json.dumps(
                    {
                        "contract_id": "contract",
                        "problem_sha256": "problem",
                        "binary_sha256": "binary",
                        "stdout_sha256": RUN.sha256_file(stdout),
                        "stderr_sha256": RUN.sha256_file(stderr),
                        "telemetry_sha256": None,
                    }
                ),
                encoding="utf-8",
            )

            self.assertTrue(
                RUN.result_is_resumable(
                    result_path,
                    contract_id="contract",
                    problem_sha256="problem",
                    binary_sha256="binary",
                )
            )
            stdout.write_text("changed\n", encoding="utf-8")
            self.assertFalse(
                RUN.result_is_resumable(
                    result_path,
                    contract_id="contract",
                    problem_sha256="problem",
                    binary_sha256="binary",
                )
            )

    def test_contract_containers_are_json_normalized(self) -> None:
        value = {
            "categories": RUN.CATEGORIES,
            "splits": RUN.SPLITS,
            "common_args": RUN.COMMON_ARGS,
            "strategies": RUN.STRATEGIES,
        }
        normalized = json.loads(RUN.canonical_json(value))

        self.assertEqual(normalized, json.loads(json.dumps(normalized)))
        self.assertIsInstance(normalized["categories"], list)
        self.assertIsInstance(normalized["common_args"], list)

    def test_candidate_order_uses_solved_case_efficiency(self) -> None:
        faster = {
            "reproducible_solved": 7,
            "median_solved_cpu_seconds": 0.75,
            "median_solved_generated_per_processed": 6.0,
        }
        slower = {
            "reproducible_solved": 7,
            "median_solved_cpu_seconds": 1.0,
            "median_solved_generated_per_processed": 5.0,
        }

        self.assertGreater(
            ANALYZE.candidate_order_key(faster),
            ANALYZE.candidate_order_key(slower),
        )


if __name__ == "__main__":
    unittest.main()
