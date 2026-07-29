"""Tests for the independent bounded arithmetic/QE oracle."""

from __future__ import annotations

import os
import random
import sys
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

from arithmetic_qe_oracle import (
    EXACT_SEMANTICS,
    TRUNCATING_SEMANTICS,
    Atom,
    BoundedQuery,
    Decision,
    DifferentialStatus,
    Expr,
    Formula,
    QuantifierSort,
    Relation,
    SmtLibProcessOracle,
    ceil_fraction,
    compare_with_external_solver,
    decide_exact,
    evaluate_expr,
    floor_fraction,
    make_atom,
    quotient_remainder,
    rational_lcm,
    render_smt2,
    replace_ceil_with_floor,
    shrink_query,
    weaken_strict_relations,
)


class ExactRationalTests(unittest.TestCase):
    def test_documented_rational_lcm_vectors(self) -> None:
        self.assertEqual(rational_lcm([Fraction(1, 3), Fraction(1, 2)]), 1)
        self.assertEqual(rational_lcm([Fraction(2, 3), Fraction(4, 5)]), 4)
        self.assertEqual(
            rational_lcm([Fraction(3, 10), Fraction(9, 14)]),
            Fraction(9, 2),
        )

    def test_quotient_remainder_handles_negative_values(self) -> None:
        self.assertEqual(quotient_remainder(Fraction(7), Fraction(3)), (2, 1))
        self.assertEqual(quotient_remainder(Fraction(-1), Fraction(3)), (-1, 2))
        self.assertEqual(
            quotient_remainder(Fraction(-1, 2), Fraction(2, 3)),
            (-1, Fraction(1, 6)),
        )

    def test_generated_quotient_remainder_identity(self) -> None:
        generator = random.Random(0x51A)
        for _ in range(500):
            period = Fraction(generator.randint(1, 30), generator.randint(1, 30))
            value = Fraction(generator.randint(-100, 100), generator.randint(1, 30))
            quotient, remainder = quotient_remainder(value, period)
            self.assertEqual(value, period * quotient + remainder)
            self.assertGreaterEqual(remainder, 0)
            self.assertLess(remainder, period)

    def test_floor_and_ceiling_are_mathematical_for_negatives(self) -> None:
        self.assertEqual(floor_fraction(Fraction(-1, 2)), -1)
        self.assertEqual(floor_fraction(Fraction(-1)), -1)
        self.assertEqual(ceil_fraction(Fraction(-1, 2)), 0)
        self.assertEqual(ceil_fraction(Fraction(-1)), -1)


class ExpressionAndOracleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.x = Expr.variable("x")

    def query(
        self,
        formula: Formula,
        lower: int | Fraction = -2,
        upper: int | Fraction = 2,
        *,
        sort: QuantifierSort = QuantifierSort.REAL,
    ) -> BoundedQuery:
        return BoundedQuery.create(
            "x",
            lower,
            upper,
            formula,
            sort=sort,
            name="unit",
        )

    def test_negative_floor_mutation_changes_semantics(self) -> None:
        expression = Expr.floor(Expr.constant(Fraction(-1, 2)))
        self.assertEqual(evaluate_expr(expression, {}, EXACT_SEMANTICS), -1)
        self.assertEqual(evaluate_expr(expression, {}, TRUNCATING_SEMANTICS), 0)

    def test_real_oracle_checks_strict_open_cells(self) -> None:
        formula = Formula.and_(
            make_atom(self.x, Relation.GT, 0),
            make_atom(self.x, Relation.LT, 1),
        )
        result = decide_exact(self.query(formula, 0, 1))
        self.assertEqual(result.decision, Decision.SAT)
        self.assertIsNotNone(result.witness)
        assert result.witness is not None
        self.assertGreater(result.witness, 0)
        self.assertLess(result.witness, 1)

    def test_real_oracle_distinguishes_empty_open_interval(self) -> None:
        formula = Formula.and_(
            make_atom(self.x, Relation.GT, 0),
            make_atom(self.x, Relation.LT, 0),
        )
        self.assertEqual(
            decide_exact(self.query(formula, -1, 1)).decision,
            Decision.UNSAT,
        )

    def test_floor_discontinuity_and_atom_zero_are_enumerated(self) -> None:
        expression = Expr.subtract(
            Expr.floor(Expr.scale(3, self.x)),
            self.x,
        )
        formula = make_atom(expression, Relation.EQ, Fraction(2, 3))
        result = decide_exact(self.query(formula, -2, 2))
        self.assertEqual(result.decision, Decision.SAT)
        self.assertEqual(result.witness, Fraction(1, 3))

    def test_nested_floor_is_complete_on_bounded_window(self) -> None:
        inner = Expr.floor(Expr.add(self.x, Expr.constant(Fraction(1, 2))))
        nested = Expr.floor(Expr.add(Expr.scale(2, inner), self.x))
        formula = make_atom(nested, Relation.EQ, -3)
        result = decide_exact(self.query(formula, -2, 2))
        self.assertEqual(result.decision, Decision.SAT)
        assert result.witness is not None
        environment = {"x": result.witness}
        self.assertTrue(formula.evaluate(environment))

    def test_boundary_points_are_checked_separately(self) -> None:
        formula = make_atom(Expr.floor(self.x), Relation.EQ, 1)
        result = decide_exact(self.query(formula, 1, 1))
        self.assertEqual(result.decision, Decision.SAT)
        self.assertEqual(result.witness, 1)

    def test_periodic_ceiling_literal_has_a_witness(self) -> None:
        periodic = Expr.subtract(Expr.ceil(self.x), self.x)
        formula = make_atom(periodic, Relation.GE, Fraction(3, 4))
        result = decide_exact(self.query(formula, 0, 1))
        self.assertEqual(result.decision, Decision.SAT)

    def test_integer_query_uses_complete_exact_enumeration(self) -> None:
        formula = Formula.and_(
            make_atom(self.x, Relation.GT, Fraction(1, 3)),
            make_atom(self.x, Relation.LT, Fraction(4, 3)),
        )
        result = decide_exact(
            self.query(formula, -3, 3, sort=QuantifierSort.INTEGER)
        )
        self.assertEqual(result.decision, Decision.SAT)
        self.assertEqual(result.witness, 1)

    def test_resource_cap_is_unknown_not_false(self) -> None:
        formula = make_atom(self.x, Relation.EQ, 500)
        result = decide_exact(
            self.query(formula, -1_000, 1_000, sort=QuantifierSort.INTEGER),
            max_cells=10,
        )
        self.assertEqual(result.decision, Decision.UNKNOWN)

    def test_ceil_metamorphism_is_exact(self) -> None:
        term = Expr.add(
            Expr.scale(Fraction(3, 2), self.x),
            Expr.floor(Expr.add(self.x, Expr.constant(Fraction(1, 3)))),
        )
        direct = Expr.ceil(term)
        rewritten = Expr.negate(Expr.floor(Expr.negate(term)))
        for numerator in range(-30, 31):
            value = Fraction(numerator, 10)
            environment = {"x": value}
            self.assertEqual(
                evaluate_expr(direct, environment),
                evaluate_expr(rewritten, environment),
            )

    def test_formula_rejects_undeclared_parameters(self) -> None:
        with self.assertRaisesRegex(ValueError, "undeclared"):
            BoundedQuery.create(
                "x",
                -1,
                1,
                make_atom(Expr.variable("a"), Relation.EQ, 0),
            )


class SmtLibAndDifferentialTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.x = Expr.variable("x")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _solver(self, verdict: str, *, exit_code: int = 0) -> SmtLibProcessOracle:
        script = self.root / f"solver-{verdict}-{exit_code}.py"
        script.write_text(
            "import sys\n"
            "_ = sys.stdin.read()\n"
            f"print({verdict!r})\n"
            f"raise SystemExit({exit_code})\n",
            encoding="utf-8",
        )
        return SmtLibProcessOracle((sys.executable, str(script)))

    def test_smt_rendering_uses_floor_correct_to_int_encoding(self) -> None:
        query = BoundedQuery.create(
            "x",
            -2,
            2,
            make_atom(
                Expr.ceil(Expr.add(self.x, Expr.constant(Fraction(-1, 2)))),
                Relation.GE,
                Expr.floor(self.x),
            ),
            parameters={},
        )
        text = render_smt2(query)
        self.assertIn("(to_real (to_int", text)
        self.assertIn("(check-sat)", text)
        self.assertIn("(assert (<= (- 2) x))", text)

    def test_integer_smt_query_encodes_integrality(self) -> None:
        query = BoundedQuery.create(
            "x",
            -2,
            2,
            make_atom(self.x, Relation.EQ, 1),
            sort=QuantifierSort.INTEGER,
        )
        self.assertIn("(assert (= x (to_real (to_int x))))", render_smt2(query))

    def test_smt_query_declares_fixed_parameter_even_when_unused(self) -> None:
        query = BoundedQuery.create(
            "x",
            -1,
            1,
            make_atom(self.x, Relation.EQ, 0),
            parameters={"a": Fraction(4, 3)},
        )
        script = render_smt2(query)
        self.assertIn("(declare-const a Real)", script)
        self.assertIn("(assert (= a (/ 4 3)))", script)

    def test_sat_and_unsat_agreements_are_classified(self) -> None:
        sat_query = BoundedQuery.create(
            "x", -1, 1, make_atom(self.x, Relation.EQ, 0)
        )
        sat = compare_with_external_solver(sat_query, self._solver("sat"))
        self.assertEqual(sat.status, DifferentialStatus.SAT)

        unsat_query = BoundedQuery.create(
            "x",
            -1,
            1,
            Formula.and_(
                make_atom(self.x, Relation.GT, 0),
                make_atom(self.x, Relation.LE, 0),
            ),
        )
        unsat = compare_with_external_solver(unsat_query, self._solver("unsat"))
        self.assertEqual(unsat.status, DifferentialStatus.UNSAT)

    def test_unknown_is_never_treated_as_unsat(self) -> None:
        query = BoundedQuery.create(
            "x", -1, 1, make_atom(self.x, Relation.EQ, 0)
        )
        result = compare_with_external_solver(query, self._solver("unknown"))
        self.assertEqual(result.status, DifferentialStatus.UNKNOWN)

    def test_opposite_verdict_is_disagreement(self) -> None:
        query = BoundedQuery.create(
            "x", -1, 1, make_atom(self.x, Relation.EQ, 0)
        )
        result = compare_with_external_solver(query, self._solver("unsat"))
        self.assertEqual(result.status, DifferentialStatus.DISAGREEMENT)

    def test_solver_protocol_failure_is_error(self) -> None:
        query = BoundedQuery.create(
            "x", -1, 1, make_atom(self.x, Relation.EQ, 0)
        )
        result = compare_with_external_solver(query, self._solver("nonsense"))
        self.assertEqual(result.status, DifferentialStatus.ERROR)

    def test_missing_solver_is_unknown(self) -> None:
        query = BoundedQuery.create(
            "x", -1, 1, make_atom(self.x, Relation.EQ, 0)
        )
        missing = SmtLibProcessOracle((str(self.root / "missing-z3"),))
        result = compare_with_external_solver(query, missing)
        self.assertEqual(result.status, DifferentialStatus.UNKNOWN)


class MutationAndShrinkingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.x = Expr.variable("x")

    def test_ceil_to_floor_erratum_is_detected(self) -> None:
        term = Expr.subtract(Expr.ceil(self.x), self.x)
        correct = make_atom(term, Relation.GE, Fraction(2, 3))
        faulty_term = replace_ceil_with_floor(term)
        faulty = make_atom(faulty_term, Relation.GE, Fraction(2, 3))
        correct_query = BoundedQuery.create("x", 0, 1, correct)
        faulty_query = correct_query.with_formula(faulty)
        self.assertEqual(decide_exact(correct_query).decision, Decision.SAT)
        self.assertEqual(decide_exact(faulty_query).decision, Decision.UNSAT)

    def test_strictness_mutation_is_detected_at_equal_endpoint(self) -> None:
        formula = make_atom(self.x, Relation.GT, 0)
        query = BoundedQuery.create("x", 0, 0, formula)
        weakened = query.with_formula(weaken_strict_relations(formula))
        self.assertEqual(decide_exact(query).decision, Decision.UNSAT)
        self.assertEqual(decide_exact(weakened).decision, Decision.SAT)

    def test_negative_floor_failure_shrinks_to_essential_atom(self) -> None:
        floor_atom = make_atom(Expr.floor(self.x), Relation.EQ, -1)
        original = Formula.and_(
            floor_atom,
            make_atom(self.x, Relation.LT, 0),
            make_atom(self.x, Relation.LE, 0),
            make_atom(Expr.add(self.x, Expr.constant(0)), Relation.LT, 1),
        )
        query = BoundedQuery.create(
            "x",
            Fraction(-1, 2),
            Fraction(-1, 2),
            original,
            name="negative-floor-seed",
        )

        def disagreement(candidate: BoundedQuery) -> bool:
            exact = decide_exact(candidate, semantics=EXACT_SEMANTICS)
            faulty = decide_exact(candidate, semantics=TRUNCATING_SEMANTICS)
            return exact.decision is not faulty.decision

        minimized, attempts = shrink_query(query, disagreement)
        self.assertGreater(attempts, 0)
        self.assertLess(
            minimized.formula.complexity(),
            query.formula.complexity(),
        )
        self.assertTrue(disagreement(minimized))
        self.assertIn("floor", str(minimized.formula))

    def test_shrinker_refuses_nonfailing_input(self) -> None:
        query = BoundedQuery.create(
            "x",
            -1,
            1,
            make_atom(self.x, Relation.EQ, 0),
        )
        with self.assertRaisesRegex(ValueError, "does not satisfy"):
            shrink_query(query, lambda _: False)


@unittest.skipUnless(
    os.environ.get("UMLAUT_Z3"),
    "set UMLAUT_Z3 to run the pinned external-Z3 semantic probes",
)
class ExternalZ3ProbeTests(unittest.TestCase):
    def test_negative_floor_semantics_and_differential_query(self) -> None:
        executable = os.environ["UMLAUT_Z3"]
        solver = SmtLibProcessOracle.z3(executable)
        x = Expr.variable("x")
        probes = (
            BoundedQuery.create(
                "x",
                Fraction(-1, 2),
                Fraction(-1, 2),
                make_atom(Expr.floor(x), Relation.EQ, -1),
                name="floor-negative-half",
            ),
            BoundedQuery.create(
                "x",
                Fraction(-1, 2),
                Fraction(-1, 2),
                make_atom(Expr.ceil(x), Relation.EQ, 0),
                name="ceil-negative-half",
            ),
            BoundedQuery.create(
                "x",
                -2,
                2,
                Formula.and_(
                    make_atom(Expr.floor(Expr.scale(3, x)), Relation.GE, 0),
                    make_atom(Expr.ceil(x), Relation.LT, 2),
                ),
                name="mixed-rounding",
            ),
        )
        for probe in probes:
            with self.subTest(probe=probe.name):
                outcome = compare_with_external_solver(probe, solver)
                self.assertIn(
                    outcome.status,
                    {DifferentialStatus.SAT, DifferentialStatus.UNSAT},
                )


if __name__ == "__main__":
    unittest.main()
