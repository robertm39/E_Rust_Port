#!/usr/bin/env python3
"""Unit tests for the restricted integer-induction experiment scripts."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import schema
import verify_schema


FIXTURES = Path(__file__).resolve().parent / "fixtures"


class SchemaTests(unittest.TestCase):
    def test_every_fixture_generates_one_reconstructable_schema(self) -> None:
        for path in sorted(FIXTURES.rglob("*.p")):
            with self.subTest(path=path.name):
                text = path.read_text(encoding="utf-8")
                augmented, generated = schema.augment_problem(text)
                self.assertIn(generated.name, augmented)
                self.assertEqual(len(schema.conjectures(augmented)), 1)
                self.assertEqual(len(generated.schema_id), 64)
                report = verify_schema.verify_structure(text, augmented)
                self.assertEqual(report["schema_id"], generated.schema_id)
                prepared = schema.prepare_problem(text)
                for symbol, _ in schema.ARITHMETIC_DECLARATIONS:
                    self.assertIn(symbol, prepared)

    def test_negated_existential_normalizes_violation(self) -> None:
        text = """
tff(p_type,type,p:$int>$o).
tff(goal,conjecture,
    ~ ? [X:$int] : ($greatereq(X,3) & ~ p(X))).
"""
        target = schema.extract_target(text)
        self.assertEqual(target.source_form, "negated_existential")
        self.assertEqual(target.bound, "3")
        self.assertIn("p", target.property)

    def test_negated_existential_accepts_disequality_violation(self) -> None:
        text = """
tff(small_type,type,small:$int>$int).
tff(fast_type,type,fast:$int>$int).
tff(goal,conjecture,
    ~ ? [X:$int] :
        ($greatereq(X,0) & (small(X) != fast(X)))).
"""
        augmented, generated = schema.augment_problem(text)
        report = verify_schema.verify_structure(text, augmented)
        self.assertEqual(report["schema_id"], generated.schema_id)

    def test_rejects_quantified_property(self) -> None:
        text = """
tff(p_type,type,p:($int*$int)>$o).
tff(goal,conjecture,
    ! [X:$int] :
      ($greatereq(X,0) => ! [Y:$int] : p(X,Y))).
"""
        with self.assertRaisesRegex(schema.SchemaError, "nested quantifier"):
            schema.extract_target(text)

    def test_rejects_symbolic_bound(self) -> None:
        text = """
tff(b_type,type,b:$int).
tff(p_type,type,p:$int>$o).
tff(goal,conjecture,! [X:$int] : ($greatereq(X,b) => p(X))).
"""
        with self.assertRaisesRegex(schema.SchemaError, "lower-bound guard"):
            schema.extract_target(text)

    def test_token_substitution_does_not_replace_identifier_substrings(self) -> None:
        target = schema.InductionTarget(
            conjecture_name="goal",
            variable="N",
            bound_tokens=("0",),
            property_tokens=("p", "(", "N", ",", "NN", ")"),
            source_form="universal_implication",
        )
        generated = schema.expected_schema_tokens(target)
        self.assertIn("NN", generated)

    def test_independent_verifier_rejects_mutated_successor(self) -> None:
        path = FIXTURES / "calibration" / "predicate_chain.p"
        text = path.read_text(encoding="utf-8")
        augmented, _ = schema.augment_problem(text)
        mutated = augmented.replace(
            "$sum ( UMLAUT_IND_N , 1 )",
            "$sum ( UMLAUT_IND_N , 2 )",
            1,
        )
        with self.assertRaisesRegex(
            verify_schema.VerificationError, r"not P\(N\+1\)"
        ):
            verify_schema.verify_structure(text, mutated)

    def test_prepare_does_not_duplicate_explicit_arithmetic_type(self) -> None:
        text = """
tff(sum_type,type,$sum:($int*$int)>$int).
tff(p_type,type,p:$int>$o).
tff(goal,conjecture,! [N:$int] : ($greatereq(N,0) => p(N))).
"""
        prepared = schema.prepare_problem(text)
        self.assertEqual(prepared.count("$sum:"), 1)

    def test_augment_cli_output_can_be_written_atomically_by_caller(self) -> None:
        source = (
            FIXTURES / "calibration" / "predicate_chain.p"
        ).read_text(encoding="utf-8")
        augmented, _ = schema.augment_problem(source)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "augmented.p"
            output.write_text(augmented, encoding="utf-8")
            self.assertEqual(schema.extract_target(output.read_text()).bound, "0")

    def test_phase_fixture_counts_are_frozen(self) -> None:
        import run

        self.assertEqual(len(run.fixture_records("calibration")), 2)
        self.assertEqual(len(run.fixture_records("validation")), 2)
        self.assertEqual(len(run.fixture_records("test")), 2)
        self.assertEqual(run.PHASES["test"]["repetitions"], 2)
        self.assertEqual(run.PHASES["transfer"]["budget"]["hard_cpu_seconds"], 10)


if __name__ == "__main__":
    unittest.main()
