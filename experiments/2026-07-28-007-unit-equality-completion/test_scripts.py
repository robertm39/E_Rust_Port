#!/usr/bin/env python3
"""Regression tests for the UEQ completion experiment scripts."""

from __future__ import annotations

import importlib.util
import json
import sys
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
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module("ueq_completion_run_test", EXPERIMENT_ROOT / "run.py")
ANALYZE = load_module(
    "ueq_completion_analyze_test", EXPERIMENT_ROOT / "analyze.py"
)
SELECT = load_module(
    "ueq_completion_select_test", EXPERIMENT_ROOT / "select.py"
)
ADAPTER = load_module(
    "ueq_completion_proof_adapter_test",
    EXPERIMENT_ROOT / "proof_adapter.py",
)


def telemetry(cpu: float, generated: int = 10) -> dict:
    return {
        "resources": {
            "total_cpu_seconds": cpu,
            "maximum_resident_pages": 100,
        },
        "search_funnel": {
            "generated": generated,
            "processed": 5,
            "high_water_total": 20,
        },
        "inferences": {"paramodulations": 7},
        "simplification": {"rewrite_steps": 11},
    }


def result(
    problem_id: str,
    strategy: str,
    repetition: int,
    solved: bool,
    cpu: float,
    generated: int = 10,
) -> dict:
    return {
        "problem_id": problem_id,
        "strategy": strategy,
        "budget": "validation",
        "repetition": repetition,
        "expected_status_match": solved,
        "family": "FAM",
        "difficulty_band": "q3",
        "szs_status": "Unsatisfiable" if solved else "ResourceOut",
        "external_timeout": False,
        "_telemetry": telemetry(cpu, generated),
    }


class ExperimentScriptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        _, cls.records = RUN.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )

    def test_family_balanced_selection_is_deterministic_and_complete(self) -> None:
        expected = {
            "train": (28, 9),
            "validation": (20, 5),
            "test": (20, 4),
        }
        family_sets = {}
        for split, (target, family_count) in expected.items():
            first = RUN.select_family_balanced_records(
                self.records, split, target
            )
            second = RUN.select_family_balanced_records(
                self.records, split, target
            )
            self.assertEqual(
                [record["problem_id"] for record in first],
                [record["problem_id"] for record in second],
            )
            self.assertEqual(len(first), target)
            self.assertEqual(len({record["family"] for record in first}), family_count)
            self.assertTrue(
                all(record["holdout_split"] == split for record in first)
            )
            family_sets[split] = {record["family"] for record in first}
        self.assertFalse(family_sets["train"] & family_sets["validation"])
        self.assertFalse(family_sets["train"] & family_sets["test"])
        self.assertFalse(family_sets["validation"] & family_sets["test"])

    def test_phase_coordinate_counts_are_pinned(self) -> None:
        self.assertEqual(len(RUN.STRATEGIES), 9)
        self.assertEqual(len(RUN.SPECIALIST_STRATEGIES), 7)
        calibration = RUN.PHASE_CONFIGS["calibration"]
        validation = RUN.PHASE_CONFIGS["validation"]
        test = RUN.PHASE_CONFIGS["test"]
        self.assertEqual(
            calibration["target_problems"]
            * len(RUN.STRATEGIES)
            * calibration["repetitions"]
            * len(calibration["budgets"]),
            252,
        )
        self.assertEqual(
            validation["target_problems"] * 5 * validation["repetitions"],
            200,
        )
        self.assertEqual(
            test["target_problems"]
            * 3
            * test["repetitions"]
            * len(test["budgets"]),
            240,
        )

    def test_reproducible_coverage_requires_every_repetition(self) -> None:
        results = [
            result("A", "completion_presat", 1, True, 1.0),
            result("A", "completion_presat", 2, True, 1.1),
            result("B", "completion_presat", 1, True, 1.0),
            result("B", "completion_presat", 2, False, 1.0),
        ]
        self.assertEqual(
            ANALYZE.reproducible_coverage(
                results, "completion_presat", "validation", 2
            ),
            {"A"},
        )

    def test_candidate_selection_uses_registered_lexicographic_rule(self) -> None:
        strategies = {
            "completion_queue": RUN.STRATEGIES["completion_queue"],
            "completion_presat": RUN.STRATEGIES["completion_presat"],
            "completion_simul": RUN.STRATEGIES["completion_simul"],
        }
        contract = {
            "phase": "validation",
            "contract_id": "contract",
            "binary_sha256": "binary",
            "budgets": {"validation": {}},
            "repetitions": 2,
            "strategies": strategies,
        }
        results = []
        for repetition in (1, 2):
            results.extend(
                [
                    result(
                        "A",
                        "completion_queue",
                        repetition,
                        True,
                        2.0,
                        20,
                    ),
                    result(
                        "A",
                        "completion_presat",
                        repetition,
                        True,
                        1.0,
                        20,
                    ),
                    result(
                        "A",
                        "completion_simul",
                        repetition,
                        True,
                        1.0,
                        10,
                    ),
                ]
            )
        selection = SELECT.select_candidates(contract, results, 1)
        self.assertEqual(
            selection["selected_strategies"], ["completion_simul"]
        )
        self.assertEqual(
            [row["strategy"] for row in selection["ranking"]],
            [
                "completion_simul",
                "completion_presat",
                "completion_queue",
            ],
        )

    def test_comparison_reports_unique_solves_and_paired_ratios(self) -> None:
        contract = {"repetitions": 2}
        results = []
        for repetition in (1, 2):
            results.extend(
                [
                    result("A", "left", repetition, True, 1.0, 5),
                    result("B", "left", repetition, True, 1.0, 5),
                    result("A", "right", repetition, True, 2.0, 10),
                    result("C", "right", repetition, True, 2.0, 10),
                ]
            )
        comparison = ANALYZE.comparison(
            contract, results, "left", "right", "validation"
        )
        self.assertEqual(comparison["left_only"], ["B"])
        self.assertEqual(comparison["right_only"], ["C"])
        self.assertEqual(comparison["common_ids"], ["A"])
        self.assertEqual(comparison["median_cpu_ratio"], 0.5)
        self.assertEqual(comparison["median_generated_ratio"], 0.5)

    def test_selection_file_is_hash_bound(self) -> None:
        body = {
            "source_phase": "validation",
            "selected_strategies": ["completion_presat"],
        }
        selection = {
            **body,
            "selection_id": __import__("hashlib")
            .sha256(RUN.canonical_json(body))
            .hexdigest(),
        }
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "selection.json"
            path.write_bytes(RUN.canonical_json(selection) + b"\n")
            loaded, digest = RUN.load_selection(
                path, source_phase="validation", count=1
            )
            self.assertEqual(loaded, selection)
            self.assertEqual(digest, RUN.sha256_file(path))

    def test_decision_requires_registered_unique_or_efficiency_gate(self) -> None:
        base = {
            "left_only": [],
            "right_only": [],
            "median_cpu_ratio": 1.0,
            "median_high_water_total_ratio": 1.0,
        }
        rejected = ANALYZE.completion_decision(
            base, contradictory_status_count=0, proof_complete=True
        )
        self.assertEqual(
            rejected["result"], "reject_separate_completion_engine"
        )
        unique = ANALYZE.completion_decision(
            {**base, "left_only": ["A", "B"]},
            contradictory_status_count=0,
            proof_complete=True,
        )
        self.assertEqual(
            unique["result"], "advance_completion_configuration"
        )
        efficient = ANALYZE.completion_decision(
            {
                **base,
                "median_cpu_ratio": 0.90,
                "median_high_water_total_ratio": 1.05,
            },
            contradictory_status_count=0,
            proof_complete=True,
        )
        self.assertEqual(
            efficient["result"], "advance_completion_configuration"
        )
        unverified = ANALYZE.completion_decision(
            {**base, "left_only": ["A", "B"]},
            contradictory_status_count=0,
            proof_complete=False,
        )
        self.assertEqual(
            unverified["result"], "reject_separate_completion_engine"
        )

    def test_strategy_matrix_retains_incremental_feature_names(self) -> None:
        features = Counter(
            feature
            for name in RUN.SPECIALIST_STRATEGIES
            for feature in RUN.STRATEGIES[name]["features"]
        )
        for expected in (
            "presaturation_interreduction",
            "simultaneous_paramodulation",
            "strong_rewrite_instantiation",
            "lpo4",
            "retain_unit_ac_axioms",
            "initial_equations_first",
        ):
            self.assertEqual(features[expected], 1)

    def test_optional_telemetry_distinguishes_absent_and_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "telemetry.json"
            self.assertEqual(
                RUN.load_optional_telemetry(path), (None, None, None)
            )
            path.write_bytes(b"")
            telemetry_value, digest, error = RUN.load_optional_telemetry(path)
            self.assertIsNone(telemetry_value)
            self.assertEqual(digest, RUN.sha256_file(path))
            self.assertTrue(error.startswith("JSONDecodeError:"))

    def test_contract_containers_are_json_stable(self) -> None:
        value = {
            "general": RUN.GENERAL_STRATEGIES,
            "specialists": RUN.SPECIALIST_STRATEGIES,
            "strategies": RUN.STRATEGIES,
            "phases": RUN.PHASE_CONFIGS,
        }
        normalized = json.loads(RUN.canonical_json(value))
        self.assertEqual(normalized, json.loads(json.dumps(normalized)))
        self.assertIsInstance(normalized["general"], list)

    def test_proof_adapter_alpha_equivalence_is_variable_spelling_only(
        self,
    ) -> None:
        self.assertTrue(
            ADAPTER.alpha_equivalent(
                "X = join(meet(X,Y),X)",
                "(X1=join(meet(X1,X2),X1))",
            )
        )
        self.assertFalse(
            ADAPTER.alpha_equivalent(
                "X = join(meet(X,Y),X)",
                "(X1=join(meet(X2,X1),X1))",
            )
        )

    def test_proofcheck_adapter_changes_only_file_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.p"
            source.write_text(
                "cnf(ax,axiom,X=f(X)).\n"
                "cnf(goal,negated_conjecture,p(X)).\n",
                encoding="utf-8",
            )
            controller = root / "controller.p"
            proof = (
                "cnf(ax, axiom, (X1=f(X1)), "
                f"file('{source.as_posix()}',ax)).\n"
                "cnf(goal, negated_conjecture, (p(X1)), "
                f"file('{source.as_posix()}',goal)).\n"
                "cnf(done, plain, ($false), "
                "inference(er,[status(thm)],[goal]), ['proof']).\n"
            )
            prepared, problem, audit = (
                ADAPTER.adapt_proofcheck_sources(
                    proof_text=proof,
                    proof_base=root,
                    controller_path=controller,
                )
            )
            self.assertEqual(audit["input_leaf_count"], 2)
            self.assertEqual(audit["negated_input_count"], 1)
            self.assertTrue(audit["logical_proof_fields_unchanged"])
            self.assertIn("cnf(ax, axiom, (X1=f(X1))", prepared)
            self.assertIn(
                "cnf(goal, negated_conjecture, (p(X1))", prepared
            )
            self.assertIn(
                "inference(er,[status(thm)],[goal])", prepared
            )
            self.assertIn("cnf(ax, axiom, (X1=f(X1))).", problem)
            self.assertIn(
                "cnf(goal, negated_conjecture, (p(X1))).", problem
            )

    def test_proofcheck_adapter_rejects_non_alpha_source_leaf(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.p"
            source.write_text(
                "cnf(ax,axiom,X=f(X)).\n"
                "cnf(goal,negated_conjecture,p(X)).\n",
                encoding="utf-8",
            )
            proof = (
                "cnf(ax, axiom, (X1=g(X1)), "
                f"file('{source.as_posix()}',ax)).\n"
                "cnf(goal, negated_conjecture, (p(X1)), "
                f"file('{source.as_posix()}',goal)).\n"
            )
            with self.assertRaises(ADAPTER.AdapterError):
                ADAPTER.adapt_proofcheck_sources(
                    proof_text=proof,
                    proof_base=root,
                    controller_path=root / "controller.p",
                )


if __name__ == "__main__":
    unittest.main()
