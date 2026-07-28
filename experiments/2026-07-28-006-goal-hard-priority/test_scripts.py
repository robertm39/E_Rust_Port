#!/usr/bin/env python3
"""Regression tests for the goal-hard-priority experiment scripts."""

from __future__ import annotations

import importlib.util
import json
import unittest
from collections import Counter
from pathlib import Path
from types import ModuleType


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
PRIOR_SELECTION = EXPERIMENT_ROOT / "prior-selection.json"


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUN = load_module("goal_hard_priority_run", EXPERIMENT_ROOT / "run.py")
ANALYZE = load_module(
    "goal_hard_priority_analyze", EXPERIMENT_ROOT / "analyze.py"
)


def telemetry(cpu_seconds: float) -> dict:
    return {
        "resources": {"total_cpu_seconds": cpu_seconds},
        "search_funnel": {"generated": 10, "processed": 5},
        "clause_selection": {"queues": []},
    }


def result(
    problem_id: str,
    strategy: str,
    repetition: int,
    solved: bool,
    cpu_seconds: float,
) -> dict:
    return {
        "problem_id": problem_id,
        "strategy": strategy,
        "budget": "larger",
        "repetition": repetition,
        "expected_status_match": solved,
        "_telemetry": telemetry(cpu_seconds),
    }


class ExperimentScriptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        _, cls.records = RUN.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        cls.prior = json.loads(PRIOR_SELECTION.read_text(encoding="utf-8"))

    def test_selection_excludes_every_prior_family(self) -> None:
        excluded, selected = RUN.select_fresh_records(
            self.records,
            set(self.prior["selected_problem_ids"]),
            4,
        )

        self.assertEqual(len(excluded), 18)
        self.assertEqual(excluded, self.prior["excluded_families"])
        self.assertEqual(len(selected), 23)
        self.assertFalse(
            {record["family"] for record in selected} & set(excluded)
        )
        self.assertEqual(
            Counter(record["category"] for record in selected),
            Counter(
                {
                    "FEQ": 4,
                    "ICU": 4,
                    "SLH": 4,
                    "TEQ": 3,
                    "TFE": 4,
                    "UEQ": 4,
                }
            ),
        )
        self.assertTrue(
            all(record["holdout_split"] == "test" for record in selected)
        )

    def test_full_contract_has_276_coordinates(self) -> None:
        _, selected = RUN.select_fresh_records(
            self.records,
            set(self.prior["selected_problem_ids"]),
            4,
        )
        coordinates = (
            len(selected)
            * len(RUN.STRATEGIES)
            * len(RUN.BUDGETS)
            * 2
        )

        self.assertEqual(coordinates, 276)
        self.assertEqual(
            RUN.BUDGETS["short"],
            {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
        )
        self.assertEqual(
            RUN.BUDGETS["larger"],
            {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        )

    def test_coverage_requires_both_matching_repetitions(self) -> None:
        results = [
            result("A", "goal_hard_priority", 1, True, 1.0),
            result("A", "goal_hard_priority", 2, True, 1.1),
            result("B", "goal_hard_priority", 1, True, 1.0),
            result("B", "goal_hard_priority", 2, False, 1.0),
        ]

        self.assertEqual(
            ANALYZE.reproducible_coverage(
                results, "goal_hard_priority", "larger", 2
            ),
            {"A"},
        )

    def test_comparison_audits_gains_and_losses(self) -> None:
        contract = {"repetitions": 2}
        results = []
        for repetition in (1, 2):
            results.extend(
                [
                    result("A", "goal_hard_priority", repetition, True, 1.0),
                    result("B", "goal_hard_priority", repetition, True, 1.0),
                    result("A", "global_aw", repetition, True, 2.0),
                    result("C", "global_aw", repetition, True, 2.0),
                ]
            )

        comparison = ANALYZE.comparison(
            contract,
            results,
            "goal_hard_priority",
            "global_aw",
            "larger",
        )

        self.assertEqual(comparison["left_only"], ["B"])
        self.assertEqual(comparison["right_only"], ["C"])
        self.assertEqual(comparison["common_ids"], ["A"])

    def test_paired_cpu_ratio_uses_common_solved_coordinates(self) -> None:
        contract = {"repetitions": 2}
        results = [
            result("A", "goal_hard_priority", 1, True, 1.0),
            result("A", "goal_hard_priority", 2, True, 2.0),
            result("A", "global_aw", 1, True, 2.0),
            result("A", "global_aw", 2, True, 4.0),
        ]

        self.assertEqual(
            ANALYZE.paired_cpu_ratio(
                contract,
                results,
                "goal_hard_priority",
                "global_aw",
                "larger",
            ),
            0.5,
        )

    def test_contract_containers_are_json_stable(self) -> None:
        value = {
            "categories": RUN.CATEGORIES,
            "common_args": RUN.COMMON_ARGS,
            "budgets": RUN.BUDGETS,
            "strategies": RUN.STRATEGIES,
        }
        normalized = json.loads(RUN.canonical_json(value))

        self.assertEqual(normalized, json.loads(json.dumps(normalized)))
        self.assertIsInstance(normalized["categories"], list)
        self.assertIsInstance(normalized["common_args"], list)


if __name__ == "__main__":
    unittest.main()
