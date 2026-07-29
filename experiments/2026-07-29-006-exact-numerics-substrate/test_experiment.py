#!/usr/bin/env python3
"""Focused tests for the exact-numerics experiment oracle and generator."""

from __future__ import annotations

import importlib.util
import pathlib
import tempfile
import unittest
from fractions import Fraction

MODULE_PATH = pathlib.Path(__file__).with_name("run_experiment.py")
SPEC = importlib.util.spec_from_file_location("exact_numerics_experiment", MODULE_PATH)
assert SPEC is not None
assert SPEC.loader is not None
EXPERIMENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPERIMENT)


class ExactNumericsExperimentTests(unittest.TestCase):
    def test_canonical_form_has_positive_reduced_denominator(self) -> None:
        self.assertEqual(EXPERIMENT.canonical(Fraction(-6, -8)), "3/4")
        self.assertEqual(EXPERIMENT.canonical(Fraction(6, -8)), "-3/4")
        self.assertEqual(EXPERIMENT.canonical(Fraction(0, -8)), "0/1")

    def test_digest_covers_order_and_every_operation(self) -> None:
        baseline = [(Fraction(2, 3), Fraction(-5, 7))]
        swapped = [(Fraction(-5, 7), Fraction(2, 3))]
        changed = [(Fraction(2, 3), Fraction(-6, 7))]
        self.assertNotEqual(
            EXPERIMENT.digest_cases(baseline),
            EXPERIMENT.digest_cases(swapped),
        )
        self.assertNotEqual(
            EXPERIMENT.digest_cases(baseline),
            EXPERIMENT.digest_cases(changed),
        )

    def test_generation_is_seeded_and_avoids_zero_divisors(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            docs = pathlib.Path(temporary)
            first, _ = EXPERIMENT.make_workloads(seed=17, viras_docs=docs)
            second, _ = EXPERIMENT.make_workloads(seed=17, viras_docs=docs)
        self.assertEqual(first, second)
        for cases in first.values():
            self.assertTrue(all(right != 0 for _, right in cases))


if __name__ == "__main__":
    unittest.main()
