#!/usr/bin/env python3
"""Focused tests for the experiment-only typed arithmetic adapter."""

from __future__ import annotations

import json
import unittest
from fractions import Fraction
from pathlib import Path

import adapter
import independent_oracle
import run


ROOT = Path(__file__).resolve().parent
CASES = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))


class AdapterTests(unittest.TestCase):
    def test_frozen_acceptance_and_rejection_counts(self) -> None:
        self.assertEqual(len(CASES["accepted"]), 12)
        self.assertEqual(len(CASES["rejected"]), 16)

    def test_every_accepted_case_agrees_in_three_views(self) -> None:
        for case in CASES["accepted"]:
            with self.subTest(case=case["name"]):
                result = adapter.adapt(case["source"])
                views = independent_oracle.verify_views(case["source"], result)
                self.assertEqual(len(set(views.values())), 1)

    def test_every_rejection_has_its_frozen_code(self) -> None:
        for case in CASES["rejected"]:
            with self.subTest(case=case["name"]):
                with self.assertRaises(adapter.AdapterError) as raised:
                    adapter.adapt(case["source"])
                self.assertEqual(raised.exception.code, case["code"])

    def test_negative_to_int_is_floor(self) -> None:
        result = adapter.adapt(
            "tff(case,axiom,$to_int(-1.5) = -2)."
        )
        self.assertTrue(result["lira_formula"]["value"])
        self.assertIn(
            {
                "kind": "coercion",
                "source": "$real->$int",
                "target": "floor",
            },
            result["trace"],
        )

    def test_integer_quantifiers_have_integrality_trace(self) -> None:
        result = adapter.adapt(
            "tff(case,axiom,! [I:$int] : (I = I))."
        )
        binder = next(step for step in result["trace"] if step["kind"] == "binder")
        self.assertEqual(binder["lowering"], "integrality_guard")
        self.assertEqual(result["lira_formula"]["kind"], "forall")

    def test_real_product_must_be_linear(self) -> None:
        with self.assertRaises(adapter.AdapterError) as raised:
            adapter.adapt(
                "tff(case,axiom,! [X:$real,Y:$real] : "
                "($product(X,Y) = 0.0))."
            )
        self.assertEqual(raised.exception.code, "NONLINEAR_PRODUCT")

    def test_duplicate_binder_is_rejected_before_translation(self) -> None:
        with self.assertRaises(adapter.AdapterError) as raised:
            adapter.adapt(
                "tff(case,axiom,! [X:$int,X:$real] : "
                "($to_real(X) = X))."
            )
        self.assertEqual(raised.exception.code, "MALFORMED_INPUT")

    def test_exact_decimal_and_exponent_parsing(self) -> None:
        self.assertEqual(
            adapter.parse_number("1.25E2"),
            ("$real", Fraction(125)),
        )
        self.assertEqual(
            adapter.parse_number("-0.125"),
            ("$real", Fraction(-1, 8)),
        )

    def test_preregistered_mutations_are_detected(self) -> None:
        accepted = {
            case["name"]: (case["source"], adapter.adapt(case["source"]))
            for case in CASES["accepted"]
        }
        records = run.mutation_matrix(accepted)
        self.assertEqual(len(records), 4)
        self.assertTrue(all(record["outcome"] == "detected" for record in records))

    def test_generated_population_is_frozen_and_well_typed(self) -> None:
        sources = run.generated_sources()
        self.assertEqual(len(sources), 500)
        self.assertEqual(sources, run.generated_sources())
        for source in sources[:25]:
            independent_oracle.verify_views(source, adapter.adapt(source))


if __name__ == "__main__":
    unittest.main()
