#!/usr/bin/env python3
"""Training tests for the frozen rational/real FM experiment."""

from __future__ import annotations

import copy
import time
import unittest
from fractions import Fraction

from fm_core import (
    Bounds,
    FmError,
    fm_resolvents,
    normalize_arithmetic,
    saturate,
    simplify_clause,
)
from fm_replay import replay
from run_native import safe_saturate


def arith(
    coefficients: dict[str, str],
    constant: str = "0",
    *,
    strict: bool = False,
    sort: str = "Rat",
) -> dict[str, object]:
    return {
        "kind": "arith",
        "sort": sort,
        "strict": strict,
        "coefficients": coefficients,
        "constant": constant,
    }


def prop(name: str, positive: bool = True) -> dict[str, object]:
    return {"kind": "prop", "name": name, "positive": positive}


def workload(
    identifier: str,
    clauses: list[list[dict[str, object]]],
) -> dict[str, object]:
    return {
        "id": identifier,
        "clauses": [
            {"id": f"{identifier}_c{index}", "literals": literals}
            for index, literals in enumerate(clauses)
        ],
    }


class NormalizationTests(unittest.TestCase):
    def test_exact_positive_primitive_scaling(self) -> None:
        self.assertEqual(
            normalize_arithmetic(arith({"x": "2/3", "z": "-4/9"}, "8/9")),
            arith({"x": "3", "z": "-2"}, "4"),
        )

    def test_constant_truth_and_false(self) -> None:
        self.assertIsNone(simplify_clause([arith({}, "0")]))
        self.assertEqual(simplify_clause([arith({}, "0", strict=True)]), [])

    def test_arithmetic_complement_is_tautological(self) -> None:
        self.assertIsNone(
            simplify_clause(
                [
                    arith({"x": "1"}, "-2", strict=True),
                    arith({"x": "-2"}, "4", strict=False),
                ]
            )
        )


class InferenceTests(unittest.TestCase):
    def test_strictness_is_disjunction_of_premise_strictness(self) -> None:
        left = simplify_clause([arith({"x": "1", "y": "1"}, strict=False)])
        right = simplify_clause([arith({"z": "-1"}, strict=True)])
        assert left is not None and right is not None
        conclusions = list(fm_resolvents(left, right))
        self.assertTrue(conclusions)
        self.assertTrue(
            any(
                literal["kind"] == "arith" and literal["strict"]
                for conclusion, _ in conclusions
                for literal in conclusion
            )
        )

    def test_one_step_fm_closes_while_baseline_does_not(self) -> None:
        case = workload(
            "one_step",
            [
                [arith({"x": "1"}, "-1", strict=True)],
                [arith({"x": "-1"}, "0")],
            ],
        )
        baseline = saturate(case, enable_fm=False)
        native = saturate(case, enable_fm=True)
        self.assertEqual(baseline["outcome"], "unknown")
        self.assertEqual(native["outcome"], "unsat")
        self.assertEqual(replay(case, native)["outcome"], "unsat")

    def test_disjunctive_case_needs_fm_and_resolution(self) -> None:
        case = workload(
            "mixed",
            [
                [prop("p"), arith({"x": "1"}, "-1", strict=True)],
                [prop("q"), arith({"x": "-1"})],
                [prop("p", False)],
                [prop("q", False)],
            ],
        )
        baseline = saturate(case, enable_fm=False)
        native = saturate(case, enable_fm=True)
        self.assertEqual(baseline["outcome"], "unknown")
        self.assertEqual(native["outcome"], "unsat")
        rules = {
            record["derivation"]["rule"]
            for record in native["records"]
        }
        self.assertIn("fourier_motzkin", rules)
        self.assertIn("propositional_resolution", rules)
        replay(case, native)

    def test_neutral_propositional_case_is_not_closed(self) -> None:
        case = workload(
            "neutral",
            [[prop("p"), prop("q")], [prop("p", False), prop("q")]],
        )
        certificate = saturate(case, enable_fm=True)
        self.assertEqual(certificate["outcome"], "unknown")
        replay(case, certificate)

    def test_coefficient_bound_fails_closed(self) -> None:
        case = workload(
            "coefficient_bound",
            [[arith({"x": str((1 << 17) + 1)}, "1")]],
        )
        with self.assertRaisesRegex(FmError, "coefficient_bits"):
            saturate(
                case,
                enable_fm=True,
                bounds=Bounds(max_integer_bits=16),
            )

    def test_attempt_bound_returns_unknown(self) -> None:
        case = workload(
            "attempt_bound",
            [
                [prop("p"), prop("q")],
                [prop("p", False), prop("r")],
                [prop("q", False), prop("r", False)],
            ],
        )
        certificate = saturate(
            case,
            enable_fm=True,
            bounds=Bounds(max_inference_attempts=0),
        )
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(
            certificate["metrics"]["crossed_bound"],
            "inference_attempts",
        )

    def test_retained_bound_precedes_empty_input_trust(self) -> None:
        case = workload("retained_bound", [[]])
        certificate = saturate(
            case,
            enable_fm=True,
            bounds=Bounds(max_retained_clauses=0),
        )
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(
            certificate["metrics"]["crossed_bound"],
            "retained_clauses",
        )
        replay(case, certificate)

    def test_unsupported_workload_returns_unknown(self) -> None:
        case = {
            "id": "unsupported_integer",
            "supported": False,
            "unsupported_reason": "integer_sort",
            "clauses": [{"id": "raw", "literals": [{"kind": "integer"}]}],
        }
        certificate = saturate(case, enable_fm=True)
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(
            certificate["metrics"]["unsupported_reason"],
            "integer_sort",
        )
        replay(case, certificate)

    def test_cancellation_returns_unknown_promptly(self) -> None:
        case = workload(
            "cancelled",
            [[prop("p"), prop("q")], [prop("p", False), prop("r")]],
        )
        started = time.perf_counter()
        certificate = saturate(
            case,
            enable_fm=True,
            cancelled=lambda: True,
        )
        elapsed = time.perf_counter() - started
        self.assertLess(elapsed, 1.0)
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(certificate["metrics"]["crossed_bound"], "cancelled")

    def test_preexisting_cancellation_cannot_trust_empty_input(self) -> None:
        case = workload("cancelled_empty", [[]])
        certificate = saturate(
            case,
            enable_fm=True,
            cancelled=lambda: True,
        )
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(certificate["records"], [])
        replay(case, certificate)

    def test_timeout_returns_unknown(self) -> None:
        case = workload(
            "timeout",
            [[prop("p"), prop("q")], [prop("p", False), prop("r")]],
        )
        certificate = saturate(
            case,
            enable_fm=True,
            bounds=Bounds(max_seconds=-1),
        )
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(certificate["metrics"]["crossed_bound"], "seconds")
        replay(case, certificate)

    def test_malformed_input_wrapper_returns_unknown(self) -> None:
        case = {"id": "malformed", "clauses": [{"id": "bad"}]}
        certificate = safe_saturate(
            case,
            enable_fm=True,
            bounds=Bounds(),
        )
        self.assertEqual(certificate["outcome"], "unknown")
        self.assertEqual(
            certificate["metrics"]["crossed_bound"],
            "malformed_input",
        )
        replay(case, certificate)


class MutationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.case = workload(
            "mutation_base",
            [
                [prop("p"), arith({"x": "1", "y": "1"}, "-2", strict=True)],
                [prop("q"), arith({"z": "-1"}, "0")],
                [prop("p", False)],
                [prop("q", False)],
                [arith({"w": "-1"}, "0")],
            ],
        )
        self.certificate = saturate(self.case, enable_fm=True)
        self.assertEqual(self.certificate["outcome"], "unsat")
        replay(self.case, self.certificate)

    def mutated_fm(self) -> tuple[dict[str, object], dict[str, object]]:
        certificate = copy.deepcopy(self.certificate)
        record = next(
            item
            for item in certificate["records"]
            if item["derivation"]["rule"] == "fourier_motzkin"
            and item["literals"]
        )
        return certificate, record

    def assert_rejected(self, certificate: dict[str, object]) -> None:
        with self.assertRaises(FmError):
            replay(self.case, certificate)

    def test_parent_hash_substitution_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        record["derivation"]["parents"][0] = "c_" + "0" * 20
        self.assert_rejected(certificate)

    def test_wrong_eliminated_variable_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        record["derivation"]["left_variable"] = "not_the_pivot"
        self.assert_rejected(certificate)

    def test_zero_multiplier_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        record["derivation"]["left_multiplier"] = "0"
        self.assert_rejected(certificate)

    def test_altered_multiplier_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        value = Fraction(record["derivation"]["left_multiplier"]) + 1
        record["derivation"]["left_multiplier"] = str(value)
        self.assert_rejected(certificate)

    def test_changed_strictness_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        literal = next(
            item for item in record["literals"] if item["kind"] == "arith"
        )
        literal["strict"] = not literal["strict"]
        self.assert_rejected(certificate)

    def test_changed_conclusion_coefficient_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        literal = next(
            item for item in record["literals"] if item["kind"] == "arith"
        )
        variable = next(iter(literal["coefficients"]))
        literal["coefficients"][variable] = "918273"
        self.assert_rejected(certificate)

    def test_deleted_context_literal_rejected(self) -> None:
        certificate, record = self.mutated_fm()
        context_index = next(
            index
            for index, item in enumerate(record["literals"])
            if item["kind"] == "prop"
        )
        record["literals"].pop(context_index)
        self.assert_rejected(certificate)

    def test_forged_empty_status_rejected(self) -> None:
        certificate = copy.deepcopy(self.certificate)
        certificate["empty_clause_id"] = "c_" + "f" * 20
        self.assert_rejected(certificate)


if __name__ == "__main__":
    unittest.main()
