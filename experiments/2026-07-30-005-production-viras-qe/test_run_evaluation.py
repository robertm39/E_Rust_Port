"""Focused tests for the production VIRAS evaluation controller."""

from __future__ import annotations

import copy
import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("run_evaluation.py")
SPEC = importlib.util.spec_from_file_location("production_viras_evaluation", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {MODULE_PATH}")
evaluation = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = evaluation
SPEC.loader.exec_module(evaluation)


class EvaluationControllerTests(unittest.TestCase):
    def test_analytic_families_are_deterministic_balanced_and_disjoint(self):
        first = evaluation.analytic_cases(evaluation.SEED, evaluation.CASES_PER_FAMILY)
        second = evaluation.analytic_cases(evaluation.SEED, evaluation.CASES_PER_FAMILY)
        self.assertEqual(first, second)
        self.assertEqual(len(first), 120)
        self.assertEqual(len({case.case_id for case in first}), 120)
        self.assertEqual(len({case.family for case in first}), 6)
        self.assertGreaterEqual(sum(case.expected for case in first), 20)
        self.assertGreaterEqual(sum(not case.expected for case in first), 20)

    def test_exact_canonical_evaluator_handles_negative_floor(self):
        term = ["floor", ["const", "-1/2"]]
        self.assertEqual(evaluation.evaluate_term(term), -1)
        formula = ["atom", ["eq", ["add", term, ["const", "1"]]]]
        self.assertTrue(evaluation.evaluate_formula(formula))

    def test_success_validator_rejects_semantic_and_replay_corruption(self):
        authentic = {
            "schema": "umlaut-viras-qe-v1",
            "status": "success",
            "result_formula": ["bool", True],
            "transformed_tff": "tff(case,conjecture,$true).\n",
            "derivation": {"replay_validated": True},
        }
        self.assertTrue(evaluation.validate_success(authentic, True))

        corrupted = copy.deepcopy(authentic)
        corrupted["result_formula"] = ["bool", False]
        with self.assertRaises(evaluation.ValidationError):
            evaluation.validate_success(corrupted, True)

        corrupted = copy.deepcopy(authentic)
        corrupted["derivation"]["replay_validated"] = False
        with self.assertRaises(evaluation.ValidationError):
            evaluation.validate_success(corrupted, True)

    def test_ast_node_count_includes_formulas_literals_and_terms(self):
        formula = [
            "atom",
            [
                "eq",
                ["add", ["const", "1"], ["scale", "2", ["const", "3"]]],
            ],
        ]
        self.assertEqual(evaluation.ast_nodes(formula), 6)


if __name__ == "__main__":
    unittest.main()
