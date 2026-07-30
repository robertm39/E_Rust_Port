#!/usr/bin/env python3
"""Focused and differential tests for the clean-room base VIRAS prototype."""

from __future__ import annotations

import random
import sys
import unittest
from fractions import Fraction
from pathlib import Path

EXPERIMENT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(EXPERIMENT_DIR))

import prototype as p  # noqa: E402
import support  # noqa: E402


def remainder(term: p.Term, period: Fraction | int) -> p.Term:
    exact = p.frac(period)
    return p.add(
        term,
        p.scale(-exact, p.floor_term(p.scale(Fraction(1, 1) / exact, term))),
    )


class ExactArithmeticTests(unittest.TestCase):
    def test_documented_rational_lcm_vectors(self) -> None:
        self.assertEqual(
            p.rational_lcm((Fraction(1, 3), Fraction(1, 2))), Fraction(1)
        )
        self.assertEqual(
            p.rational_lcm((Fraction(2, 3), Fraction(4, 5))), Fraction(4)
        )
        self.assertEqual(
            p.rational_lcm((Fraction(3, 10), Fraction(9, 14))),
            Fraction(9, 2),
        )

    def test_documented_quotient_remainder_vectors(self) -> None:
        vectors = (
            (Fraction(7), Fraction(3), 2, Fraction(1)),
            (Fraction(-1), Fraction(3), -1, Fraction(2)),
            (Fraction(-1, 2), Fraction(2, 3), -1, Fraction(1, 6)),
        )
        for value, period, quotient, expected_remainder in vectors:
            actual_quotient = p.floor_fraction(value / period)
            actual_remainder = value - period * actual_quotient
            self.assertEqual(actual_quotient, quotient)
            self.assertEqual(actual_remainder, expected_remainder)

    def test_negative_floor_is_mathematical(self) -> None:
        self.assertEqual(
            p.evaluate_term(p.floor_term(p.constant(Fraction(-1, 2))), {}),
            -1,
        )


class GridTests(unittest.TestCase):
    def test_documented_grid_intersection(self) -> None:
        kernel = p.Kernel()
        a = p.variable("a")
        grid = p.Grid(p.constant(1), Fraction(2))
        result = kernel.grid_intersection(
            grid,
            a,
            Fraction(4),
            lower_closed=True,
            upper_closed=False,
        )
        self.assertEqual(len(result), 2)
        for value in (Fraction(-5, 2), Fraction(0), Fraction(7, 3)):
            evaluated = {p.evaluate_term(term, {"a": value}) for term in result}
            actual = {
                Fraction(1 + 2 * integer)
                for integer in range(-20, 21)
                if value <= 1 + 2 * integer < value + 4
            }
            self.assertTrue(actual <= evaluated)

    def test_zero_width_boundary_extension(self) -> None:
        kernel = p.Kernel()
        grid = p.Grid(p.constant(0), Fraction(1))
        closed = kernel.grid_intersection(
            grid,
            p.constant(Fraction(1, 2)),
            Fraction(0),
            lower_closed=True,
            upper_closed=True,
        )
        opened = kernel.grid_intersection(
            grid,
            p.constant(Fraction(1, 2)),
            Fraction(0),
            lower_closed=False,
            upper_closed=False,
        )
        self.assertEqual(len(closed), 1)
        self.assertEqual(opened, ())

    def test_generated_intersections_cover_concrete_grid_points(self) -> None:
        rng = random.Random(0x641D)
        for _ in range(200):
            period = rng.choice(
                (Fraction(1, 3), Fraction(1, 2), Fraction(1), Fraction(3, 2), Fraction(2))
            )
            base = Fraction(rng.randint(-4, 4), rng.choice((1, 2, 3)))
            lower = Fraction(rng.randint(-8, 8), rng.choice((1, 2, 3)))
            width = Fraction(rng.randint(0, 10), rng.choice((1, 2, 3)))
            lower_closed = bool(rng.getrandbits(1))
            upper_closed = bool(rng.getrandbits(1))
            kernel = p.Kernel()
            result = kernel.grid_intersection(
                p.Grid(p.constant(base), period),
                p.constant(lower),
                width,
                lower_closed=lower_closed,
                upper_closed=upper_closed,
            )
            evaluated = {p.evaluate_term(term, {}) for term in result}
            actual = set()
            for integer in range(-100, 101):
                point = base + period * integer
                lower_ok = point >= lower if lower_closed else point > lower
                upper = lower + width
                upper_ok = point <= upper if upper_closed else point < upper
                if lower_ok and upper_ok:
                    actual.add(point)
            self.assertTrue(actual <= evaluated)


class ProfileTests(unittest.TestCase):
    def setUp(self) -> None:
        self.x = p.variable("x")
        self.z = p.variable("z")
        self.c = p.variable("c")

    def assert_profile(
        self,
        term: p.Term,
        outer: Fraction | int,
        segment: Fraction | int,
        period: Fraction | int,
        delta: Fraction | int,
    ) -> p.Profile:
        result = p.Kernel().profile(term, "x")
        self.assertEqual(result.outer_slope, outer)
        self.assertEqual(result.segment_slope, segment)
        self.assertEqual(result.period, period)
        self.assertEqual(result.delta_y, delta)
        return result

    def test_documented_profile_vectors(self) -> None:
        self.assert_profile(self.x, 1, 1, 0, 0)
        self.assert_profile(self.z, 0, 0, 0, 0)

        floor_three_x = p.floor_term(p.scale(3, self.x))
        profile = self.assert_profile(floor_three_x, 3, 0, Fraction(1, 3), 1)
        self.assertEqual(profile.dist_y_minus, p.constant(-1))
        breaks = p.Kernel().breaks(floor_three_x, "x")
        self.assertEqual(breaks, (p.Grid(p.constant(0), Fraction(1, 3)),))

        mixed = p.add(
            p.negate(p.floor_term(p.add(p.scale(-3, self.x), self.z))),
            p.negate(self.x),
        )
        kernel = p.Kernel()
        profile = kernel.profile(mixed, "x")
        self.assertEqual(
            (
                profile.outer_slope,
                profile.segment_slope,
                profile.period,
                profile.delta_y,
                profile.dist_y_minus,
            ),
            (2, -1, Fraction(1, 3), 1, p.negate(self.z)),
        )
        self.assertEqual(
            kernel.breaks(mixed, "x"),
            (p.Grid(p.scale(Fraction(1, 3), self.z), Fraction(1, 3)),),
        )
        for x_value in (Fraction(-3, 2), Fraction(-1), Fraction(0), Fraction(5, 3)):
            for z_value in (Fraction(-2), Fraction(1, 2), Fraction(3)):
                environment = {"x": x_value, "z": z_value}
                expected_limit = p.add(
                    p.floor_term(p.add(p.scale(3, self.x), p.negate(self.z))),
                    p.constant(1),
                    p.negate(self.x),
                )
                self.assertEqual(
                    p.evaluate_term(profile.right_limit, environment),
                    p.evaluate_term(expected_limit, environment),
                )

        periodic = p.add(
            p.ceil_term(self.x), p.negate(self.x), p.negate(self.c)
        )
        kernel = p.Kernel()
        profile = kernel.profile(periodic, "x")
        self.assertEqual(
            (
                profile.outer_slope,
                profile.segment_slope,
                profile.period,
                profile.delta_y,
                profile.dist_y_minus,
            ),
            (0, -1, 1, 1, p.negate(self.c)),
        )
        self.assertEqual(kernel.breaks(periodic, "x"), (p.Grid(p.constant(0), 1),))

    def test_profile_bounds_and_periodic_shift(self) -> None:
        rng = random.Random(0xB0A5)
        for _ in range(200):
            term = support.generated_term(rng, depth=3)
            kernel = p.Kernel()
            profile = kernel.profile(term, "x")
            for x_value in (Fraction(-7, 3), Fraction(-1), Fraction(0), Fraction(5, 2)):
                value = p.evaluate_term(term, {"x": x_value})
                lower = profile.outer_slope * x_value + p.evaluate_term(
                    profile.dist_y_minus, {}
                )
                upper = profile.outer_slope * x_value + p.evaluate_term(
                    profile.dist_y_plus, {}
                )
                self.assertLessEqual(lower, value)
                self.assertLessEqual(value, upper)
                if profile.period:
                    shift_count = -2
                    shifted = x_value + profile.period * shift_count
                    shifted_value = p.evaluate_term(term, {"x": shifted})
                    self.assertEqual(
                        shifted_value,
                        value
                        + profile.outer_slope * profile.period * shift_count,
                    )


class CandidateAndSubstitutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.x = p.variable("x")
        self.a = p.variable("a")
        self.c = p.variable("c")
        self.z = p.variable("z")

    def candidates(self, literal: p.Literal) -> list[p.VirtualTerm]:
        return [
            candidate.virtual
            for candidate in p.Kernel().literal_candidates(literal, "x", 0)
        ]

    def test_no_break_candidate_vectors(self) -> None:
        vectors = (
            (
                p.Literal(self.x, p.Relation.GE),
                {p.VirtualTerm(p.constant(0))},
            ),
            (
                p.Literal(self.x, p.Relation.GT),
                {p.VirtualTerm(p.constant(0), epsilon=True)},
            ),
            (
                p.Literal(p.negate(self.x), p.Relation.GE),
                {p.VirtualTerm(infinity=p.InfinitySign.NEGATIVE)},
            ),
            (
                p.Literal(self.x, p.Relation.EQ),
                {p.VirtualTerm(p.constant(0))},
            ),
            (
                p.Literal(self.x, p.Relation.NE),
                {
                    p.VirtualTerm(infinity=p.InfinitySign.NEGATIVE),
                    p.VirtualTerm(p.constant(0), epsilon=True),
                },
            ),
        )
        for literal, expected in vectors:
            self.assertEqual(set(self.candidates(literal)), expected)

    def test_periodic_candidate_vector(self) -> None:
        literal = p.Literal(
            p.add(p.ceil_term(self.x), p.negate(self.x), p.negate(self.c)),
            p.Relation.GE,
        )
        candidates = set(self.candidates(literal))
        self.assertIn(p.VirtualTerm(p.constant(0), grid_period=1), candidates)
        self.assertIn(
            p.VirtualTerm(p.constant(0), epsilon=True, grid_period=1),
            candidates,
        )

    def test_periodic_zero_segment_slope_with_breaks_is_total(self) -> None:
        term = p.add(
            p.floor_term(self.x),
            p.floor_term(p.negate(self.x)),
        )
        profile = p.Kernel().profile(term, "x")
        self.assertEqual(profile.outer_slope, 0)
        self.assertEqual(profile.segment_slope, 0)
        self.assertTrue(p.Kernel().breaks(term, "x"))
        expected = {
            p.Relation.EQ: True,
            p.Relation.NE: True,
            p.Relation.GT: False,
            p.Relation.GE: True,
        }
        for relation, decision in expected.items():
            outcome = p.eliminate_exists(
                "x", [p.Literal(term, relation)]
            )
            self.assertEqual(outcome.status, p.QEStatus.SUCCESS)
            assert outcome.formula is not None
            self.assertEqual(outcome.formula.evaluate({}), decision)

    def test_epsilon_substitution_vectors(self) -> None:
        vectors = (
            (p.Literal(self.x, p.Relation.EQ), False),
            (p.Literal(self.x, p.Relation.NE), True),
            (p.Literal(self.x, p.Relation.GE), True),
            (p.Literal(self.x, p.Relation.GT), True),
            (p.Literal(p.negate(self.x), p.Relation.GE), False),
            (p.Literal(p.negate(self.x), p.Relation.GT), False),
            (p.Literal(p.floor_term(self.x), p.Relation.EQ), True),
        )
        for literal, expected in vectors:
            result = p.Kernel().virtual_substitute(
                [literal],
                "x",
                p.VirtualTerm(p.constant(0), epsilon=True),
            )
            self.assertEqual(result.evaluate({}), expected, literal.render())

    def test_infinity_substitution_preserves_periodic_residue(self) -> None:
        periodic = p.Literal(
            subtract_remainders(self.x, self.c, 2),
            p.Relation.EQ,
        )
        kernel = p.Kernel()
        for sign in (p.InfinitySign.NEGATIVE, p.InfinitySign.POSITIVE):
            result = kernel.virtual_substitute(
                [periodic],
                "x",
                p.VirtualTerm(self.c, infinity=sign),
            )
            self.assertTrue(result.evaluate({"c": Fraction(3, 2)}))

    def test_grid_flattening_v1_v2_v3(self) -> None:
        residue_two = p.Literal(
            subtract_remainders(self.x, self.c, 2),
            p.Relation.EQ,
        )
        v1_literals = [p.Literal(self.x, p.Relation.GE), residue_two]
        kernel = p.Kernel()
        v1 = kernel.flatten_grid(
            v1_literals,
            "x",
            p.VirtualTerm(self.c, grid_period=2),
        )
        self.assertEqual(len(v1), 1)
        self.assertEqual(v1[0].infinity, p.InfinitySign.POSITIVE)
        self.assertEqual(kernel.flatten_records[-1]["case"], "V1")

        v2_literals = [
            p.Literal(p.add(self.x, p.negate(self.a)), p.Relation.EQ),
            residue_two,
        ]
        kernel = p.Kernel()
        v2 = kernel.flatten_grid(
            v2_literals,
            "x",
            p.VirtualTerm(self.c, grid_period=2),
        )
        self.assertEqual(len(v2), 1)
        self.assertEqual(kernel.flatten_records[-1]["case"], "V2")

        mixed = p.add(
            p.negate(
                p.floor_term(p.add(p.scale(-3, self.x), self.z))
            ),
            p.negate(self.x),
        )
        v3_literals = [
            p.Literal(mixed, p.Relation.GT),
            p.Literal(p.negate(self.x), p.Relation.GT),
            p.Literal(
                subtract_remainders(self.x, self.c, 3),
                p.Relation.EQ,
            ),
            p.Literal(
                p.add(remainder(self.x, 2), p.constant(-1)),
                p.Relation.NE,
            ),
        ]
        kernel = p.Kernel()
        v3 = kernel.flatten_grid(
            v3_literals,
            "x",
            p.VirtualTerm(self.c, grid_period=3),
        )
        self.assertEqual(len(v3), 3)
        self.assertEqual(kernel.flatten_records[-1]["case"], "V3")
        self.assertEqual(kernel.flatten_records[-1]["common_period"], "6")


def subtract_remainders(
    left: p.Term, right: p.Term, period: Fraction | int
) -> p.Term:
    return p.subtract(remainder(left, period), remainder(right, period))


class EndToEndTests(unittest.TestCase):
    def test_motivating_example_matches_c_le_two_thirds(self) -> None:
        x = p.variable("x")
        a = p.variable("a")
        c = p.variable("c")
        literals = [
            p.Literal(
                p.add(
                    x,
                    p.negate(p.floor_term(a)),
                    p.constant(Fraction(-1, 3)),
                ),
                p.Relation.GE,
            ),
            p.Literal(
                p.add(
                    p.floor_term(a),
                    p.constant(Fraction(2, 3)),
                    p.negate(x),
                ),
                p.Relation.GE,
            ),
            p.Literal(
                p.add(p.ceil_term(x), p.negate(x), p.negate(c)),
                p.Relation.GE,
            ),
        ]
        outcome = p.eliminate_exists("x", literals)
        self.assertEqual(outcome.status, p.QEStatus.SUCCESS)
        assert outcome.formula is not None
        candidates = [item["virtual"] for item in outcome.derivation["candidates"]]
        self.assertEqual(len(candidates), 4)
        self.assertIn(
            p.VirtualTerm(
                p.add(p.floor_term(a), p.constant(Fraction(1, 3)))
            ).describe(),
            candidates,
        )
        self.assertIn(
            p.VirtualTerm(infinity=p.InfinitySign.NEGATIVE).describe(),
            candidates,
        )
        self.assertIn(p.VirtualTerm(grid_period=1).describe(), candidates)
        self.assertIn(
            p.VirtualTerm(epsilon=True, grid_period=1).describe(),
            candidates,
        )
        for a_value in (
            Fraction(-7, 3),
            Fraction(-2),
            Fraction(-1, 2),
            Fraction(0),
            Fraction(2, 3),
            Fraction(1),
            Fraction(5, 2),
        ):
            for c_value in (
                Fraction(-2),
                Fraction(-1, 3),
                Fraction(0),
                Fraction(1, 2),
                Fraction(2, 3),
                Fraction(3, 4),
                Fraction(2),
            ):
                self.assertEqual(
                    outcome.formula.evaluate({"a": a_value, "c": c_value}),
                    c_value <= Fraction(2, 3),
                )

    def test_pure_linear_open_closed_matrix(self) -> None:
        x = p.variable("x")
        a = p.variable("a")
        b = p.variable("b")
        for lower_closed in (False, True):
            for upper_closed in (False, True):
                literals = [
                    p.Literal(
                        p.add(x, p.negate(a)),
                        p.Relation.GE if lower_closed else p.Relation.GT,
                    ),
                    p.Literal(
                        p.add(b, p.negate(x)),
                        p.Relation.GE if upper_closed else p.Relation.GT,
                    ),
                ]
                outcome = p.eliminate_exists("x", literals)
                self.assertEqual(outcome.status, p.QEStatus.SUCCESS)
                assert outcome.formula is not None
                for a_value in (Fraction(-1), Fraction(0), Fraction(2)):
                    for b_value in (Fraction(-1), Fraction(0), Fraction(2)):
                        expected = a_value < b_value or (
                            a_value == b_value
                            and lower_closed
                            and upper_closed
                        )
                        self.assertEqual(
                            outcome.formula.evaluate(
                                {"a": a_value, "b": b_value}
                            ),
                            expected,
                        )

    def test_seeded_cases_agree_with_independent_exact_oracle(self) -> None:
        decisions = {True: 0, False: 0}
        for case in support.generate_cases(0xB451E, 150):
            expected = support.exact_oracle_decision(case)
            outcome = p.eliminate_exists("x", case.literals)
            self.assertEqual(outcome.status, p.QEStatus.SUCCESS, case.case_id)
            assert outcome.formula is not None
            actual = outcome.formula.evaluate({})
            self.assertEqual(actual, expected, case.case_id)
            decisions[actual] += 1
        self.assertGreater(decisions[True], 0)
        self.assertGreater(decisions[False], 0)

    def test_metamorphic_literal_order_and_duplicates(self) -> None:
        for case in support.generate_cases(0xA17A, 50):
            baseline = p.eliminate_exists("x", case.literals)
            reversed_case = p.eliminate_exists("x", tuple(reversed(case.literals)))
            duplicate = p.eliminate_exists(
                "x", (*case.literals, case.literals[-1])
            )
            self.assertEqual(baseline.status, p.QEStatus.SUCCESS)
            self.assertEqual(reversed_case.status, p.QEStatus.SUCCESS)
            self.assertEqual(duplicate.status, p.QEStatus.SUCCESS)
            assert baseline.formula is not None
            assert reversed_case.formula is not None
            assert duplicate.formula is not None
            expected = baseline.formula.evaluate({})
            self.assertEqual(reversed_case.formula.evaluate({}), expected)
            self.assertEqual(duplicate.formula.evaluate({}), expected)

    def test_mutations_are_detected(self) -> None:
        negative_half = p.Term(
            p.TermOp.FLOOR,
            args=(p.constant(Fraction(-1, 2)),),
        )
        self.assertNotEqual(
            p.evaluate_term(negative_half, {}),
            p.evaluate_term(
                negative_half, {}, truncate_negative_floor=True
            ),
        )

        x = p.variable("x")
        c = p.constant(1)
        residue = p.Literal(
            subtract_remainders(x, c, 2), p.Relation.EQ
        )
        v1 = [p.Literal(x, p.Relation.GE), residue]
        baseline = p.eliminate_exists("x", v1)
        reversed_infinity = p.eliminate_exists(
            "x",
            v1,
            mutations=p.Mutations(reverse_infinity_periodicity=True),
        )
        assert baseline.formula is not None
        assert reversed_infinity.formula is not None
        self.assertTrue(baseline.formula.evaluate({}))
        self.assertFalse(reversed_infinity.formula.evaluate({}))

        strict = [p.Literal(x, p.Relation.GT)]
        baseline = p.eliminate_exists("x", strict)
        weakened = p.eliminate_exists(
            "x",
            strict,
            mutations=p.Mutations(drop_epsilon_strictness=True),
        )
        assert baseline.formula is not None
        assert weakened.formula is not None
        self.assertTrue(baseline.formula.evaluate({}))
        self.assertFalse(weakened.formula.evaluate({}))

        singleton = [
            p.Literal(x, p.Relation.EQ),
            p.Literal(x, p.Relation.GE),
        ]
        baseline = p.eliminate_exists("x", singleton)
        omitted = p.eliminate_exists(
            "x",
            singleton,
            mutations=p.Mutations(omit_last_candidate=True),
        )
        assert baseline.formula is not None
        assert omitted.formula is not None
        self.assertTrue(baseline.formula.evaluate({}))
        self.assertFalse(omitted.formula.evaluate({}))

    def test_resource_and_unsupported_outcomes_fail_closed(self) -> None:
        x = p.variable("x")
        simple = [p.Literal(x, p.Relation.GE)]
        floor_literal = [p.Literal(p.floor_term(x), p.Relation.EQ)]
        cases = (
            p.eliminate_exists("x", simple, limits=p.Limits(max_steps=0)),
            p.eliminate_exists(
                "x", simple, limits=p.Limits(max_candidates=0)
            ),
            p.eliminate_exists(
                "x", floor_literal, limits=p.Limits(max_grids=0)
            ),
            p.eliminate_exists(
                "x",
                floor_literal,
                limits=p.Limits(max_grid_points=0),
            ),
            p.eliminate_exists(
                "x", simple, limits=p.Limits(max_formula_nodes=0)
            ),
            p.eliminate_exists(
                "x",
                [p.Literal(p.scale(2**100, x), p.Relation.GE)],
                limits=p.Limits(max_rational_bits=8),
            ),
        )
        for outcome in cases:
            self.assertEqual(outcome.status, p.QEStatus.UNKNOWN)
            self.assertEqual(
                outcome.unknown_kind, p.UnknownKind.RESOURCE_LIMIT
            )
            self.assertIsNone(outcome.formula)

        unsupported = p.eliminate_exists("x", p.boolean(True))
        self.assertEqual(unsupported.status, p.QEStatus.UNKNOWN)
        self.assertEqual(
            unsupported.unknown_kind, p.UnknownKind.UNSUPPORTED_FRAGMENT
        )
        self.assertIsNone(unsupported.formula)

    def test_success_has_no_virtual_marker_or_eliminated_variable(self) -> None:
        case = support.generate_cases(0x515A, 1)[0]
        outcome = p.eliminate_exists("x", case.literals)
        self.assertEqual(outcome.status, p.QEStatus.SUCCESS)
        assert outcome.formula is not None
        self.assertNotIn("x", outcome.formula.variables())
        self.assertGreater(len(outcome.derivation["candidates"]), 0)
        rendered = outcome.formula.render()
        self.assertNotIn("epsilon", rendered)
        self.assertNotIn("infinity", rendered)
        self.assertNotIn("*Z", rendered)


if __name__ == "__main__":
    unittest.main()
