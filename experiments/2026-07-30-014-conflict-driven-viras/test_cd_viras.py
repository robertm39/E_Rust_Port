#!/usr/bin/env python3
"""Focused tests for the Experiment 014 prototype."""

from __future__ import annotations

import unittest
from fractions import Fraction

import cd_viras
import corpus

base = cd_viras.base


class AffineCheckerTests(unittest.TestCase):
    def test_affine_extraction_is_exact(self) -> None:
        x = base.variable("x")
        term = base.add(
            base.scale(Fraction(3, 2), x),
            base.constant(Fraction(-7, 3)),
        )
        self.assertEqual(
            cd_viras.affine_from_term(term),
            cd_viras.Affine.create(
                {"x": Fraction(3, 2)}, Fraction(-7, 3)
            ),
        )

    def test_floor_is_rejected(self) -> None:
        with self.assertRaises(cd_viras.UnsupportedSlice):
            cd_viras.affine_from_term(
                base.floor_term(base.variable("x"))
            )

    def test_fourier_motzkin_feasible_boundary(self) -> None:
        x = base.variable("x")
        literals = (
            corpus.ge(x, base.constant(0)),
            corpus.ge(base.constant(0), x),
        )
        self.assertTrue(cd_viras.affine_feasible(literals).feasible)

    def test_fourier_motzkin_strict_conflict(self) -> None:
        x = base.variable("x")
        literals = (
            corpus.gt(x, base.constant(0)),
            corpus.ge(base.constant(0), x),
        )
        self.assertFalse(cd_viras.affine_feasible(literals).feasible)

    def test_clause_soundness_accepts_only_implication(self) -> None:
        x = base.variable("x")
        original = (corpus.equality(x, base.constant(0)),)
        sound = (
            cd_viras.ClauseComponent("x", base.constant(1)),
        )
        unsound = (
            cd_viras.ClauseComponent("x", base.constant(0)),
        )
        self.assertTrue(cd_viras.clause_soundness(original, sound).feasible)
        self.assertFalse(cd_viras.clause_soundness(original, unsound).feasible)

    def test_progress_requires_current_assignment_to_be_blocked(self) -> None:
        prefix = (
            cd_viras.Decision("x", base.constant(1), 0, "linear_zero", 0),
        )
        self.assertTrue(
            cd_viras.clause_progress(
                (cd_viras.ClauseComponent("x", base.constant(1)),),
                prefix,
            )
        )
        self.assertFalse(
            cd_viras.clause_progress(
                (cd_viras.ClauseComponent("x", base.constant(0)),),
                prefix,
            )
        )


class SearchTests(unittest.TestCase):
    def test_first_variable_exhaustion_reaches_unsat(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "hand-first-variable-unsat"
        )
        for treatment in cd_viras.Treatment:
            outcome = cd_viras.run_search(case.literals, treatment)
            self.assertTrue(outcome.supported)
            self.assertFalse(outcome.decision)
        basic = cd_viras.run_search(case.literals, cd_viras.Treatment.BASIC)
        self.assertTrue(any(not clause.components for clause in basic.clauses))

    def test_early_sat_avoids_eager_substitutions(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "hand-early-sat"
        )
        eager = cd_viras.run_search(case.literals, "eager")
        basic = cd_viras.run_search(case.literals, "basic")
        self.assertTrue(eager.decision)
        self.assertTrue(basic.decision)
        self.assertLess(
            basic.metrics.virtual_substitutions,
            eager.metrics.virtual_substitutions,
        )

    def test_cross_variable_candidate_keeps_generation_context(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "hand-cross-variable-candidate"
        )
        outcome = cd_viras.run_search(case.literals, "basic")
        decide = next(event for event in outcome.trace if event["rule"] == "Decide")
        self.assertIn("residual", decide)
        self.assertIn("prefix", decide)
        self.assertTrue(outcome.decision)

    def test_focused_conflict_can_learn_smaller_clause(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "hand-irrelevant-prefix-conflict"
        )
        basic = cd_viras.run_search(case.literals, "basic")
        focused = cd_viras.run_search(case.literals, "focused")
        self.assertFalse(basic.decision)
        self.assertFalse(focused.decision)
        self.assertLess(
            focused.metrics.virtual_substitutions,
            basic.metrics.virtual_substitutions,
        )
        self.assertTrue(all(clause.describe()["soundness"] for clause in focused.clauses))

    def test_missing_equality_guard_is_unsupported(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "boundary-epsilon"
        )
        outcome = cd_viras.run_search(case.literals, "focused")
        self.assertFalse(outcome.supported)
        self.assertIsNone(outcome.decision)

    def test_periodic_candidate_is_unsupported(self) -> None:
        case = next(
            item
            for item in corpus.hand_cases()
            if item.case_id == "boundary-periodic-grid"
        )
        outcome = cd_viras.run_search(case.literals, "basic")
        self.assertFalse(outcome.supported)

    def test_semantic_trace_is_deterministic(self) -> None:
        case = corpus.generated_cases(count=3)[0]
        first = cd_viras.run_search(case.literals, "focused")
        second = cd_viras.run_search(case.literals, "focused")
        self.assertEqual(
            first.semantic_trace_sha256, second.semantic_trace_sha256
        )
        self.assertEqual(
            first.metrics.semantic_description(),
            second.metrics.semantic_description(),
        )

    def test_seeded_family_decisions_match_affine_oracle(self) -> None:
        for case in corpus.generated_cases(count=12):
            exact = cd_viras.affine_feasible(case.literals).feasible
            self.assertEqual(exact, case.expected_decision, case.case_id)
            for treatment in cd_viras.Treatment:
                outcome = cd_viras.run_search(case.literals, treatment)
                self.assertTrue(outcome.supported, case.case_id)
                self.assertEqual(outcome.decision, exact, case.case_id)


if __name__ == "__main__":
    unittest.main()
