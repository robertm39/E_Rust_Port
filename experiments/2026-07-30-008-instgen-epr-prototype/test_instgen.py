#!/usr/bin/env python3
"""Focused unit tests for the bounded Inst-Gen-style prototype."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def load(name: str, filename: str):
    specification = importlib.util.spec_from_file_location(name, ROOT / filename)
    assert specification is not None and specification.loader is not None
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


instgen = load("instgen_under_test", "instgen.py")
verify = load("instgen_verifier_under_test", "verify_certificate.py")
selector = load("instgen_selector_under_test", "select_corpus.py")


class ParserTests(unittest.TestCase):
    def test_comments_quotes_and_multiple_statements(self) -> None:
        problem = instgen.parse_problem(
            """
            % line comment
            cnf('c,1', axiom, (p('Upper',X) | ~q(X))).
            /* block */
            cnf(c2, axiom, r(a)).
            """
        )
        self.assertEqual(len(problem.clauses), 2)
        self.assertEqual(problem.clauses[0].variables, ("X",))
        self.assertEqual(problem.constants, ("'Upper'", "a"))

    def test_function_and_equality_are_rejected(self) -> None:
        with self.assertRaisesRegex(instgen.InstGenError, "positive-arity"):
            instgen.parse_problem("cnf(c,axiom,p(f(a))).")
        with self.assertRaisesRegex(instgen.InstGenError, "equality"):
            instgen.parse_problem("cnf(c,axiom,a=b).")

    def test_missing_constant_gets_fresh_domain_member(self) -> None:
        problem = instgen.parse_problem("cnf(c,axiom,p(X)).")
        self.assertEqual(problem.constants, ("instgen_default_constant",))
        self.assertEqual(problem.ground_instance_count, 1)

    def test_truth_constants_and_tautologies_normalize(self) -> None:
        self.assertIsNone(
            instgen.normalize_ground_clause(
                [instgen.Literal(instgen.Atom("$true", ()), True)]
            )
        )
        atom = instgen.Atom("p", ("a",))
        self.assertIsNone(
            instgen.normalize_ground_clause(
                [
                    instgen.Literal(atom, True),
                    instgen.Literal(atom, False),
                ]
            )
        )
        self.assertEqual(
            instgen.normalize_ground_clause(
                [instgen.Literal(instgen.Atom("$false", ()), True)]
            ),
            (),
        )


class GroundingTests(unittest.TestCase):
    def test_grounding_and_false_instance_detection(self) -> None:
        problem = instgen.parse_problem("cnf(c,axiom,(p(X)|~q(Y))).")
        clause = problem.clauses[0]
        ground = instgen.ground_clause(clause, {"X": "a", "Y": "b"})
        assert ground is not None
        self.assertEqual(
            instgen.clause_key(ground),
            (("p(a)", True), ("q(b)", False)),
        )
        self.assertFalse(instgen.ground_clause_is_false(ground, {}))
        self.assertTrue(
            instgen.ground_clause_is_false(
                ground,
                {
                    instgen.Atom("p", ("a",)): False,
                    instgen.Atom("q", ("b",)): True,
                },
            )
        )

    def test_instance_deduplication_preserves_first_ancestry(self) -> None:
        problem = instgen.parse_problem(
            "cnf(c1,axiom,p(a)). cnf(c2,axiom,p(a))."
        )
        known: set[tuple[tuple[str, bool], ...]] = set()
        records: list[dict] = []
        grounds: list[tuple] = []
        first = instgen.ground_clause(problem.clauses[0], {})
        second = instgen.ground_clause(problem.clauses[1], {})
        assert first is not None and second is not None
        self.assertTrue(
            instgen.add_instance(
                clause=problem.clauses[0],
                substitution={},
                ground=first,
                known=known,
                instances=records,
                ground_clauses=grounds,
                phase="initial",
                iteration=0,
            )
        )
        self.assertFalse(
            instgen.add_instance(
                clause=problem.clauses[1],
                substitution={},
                ground=second,
                known=known,
                instances=records,
                ground_clauses=grounds,
                phase="initial",
                iteration=0,
            )
        )
        self.assertEqual(len(records), 1)

    def test_same_batch_duplicate_is_not_a_solved_clause(self) -> None:
        problem = instgen.parse_problem(
            "cnf(c1,axiom,p(X)). cnf(c2,axiom,p(X)). cnf(k,axiom,r(a))."
        )
        ground = instgen.ground_clause(
            problem.clauses[0], {"X": problem.constants[0]}
        )
        assert ground is not None
        solved: set[tuple[tuple[str, bool], ...]] = set()
        known = set(solved)
        records: list[dict] = []
        grounds: list[tuple] = []
        self.assertTrue(
            instgen.add_instance(
                clause=problem.clauses[0],
                substitution={"X": problem.constants[0]},
                ground=ground,
                known=known,
                instances=records,
                ground_clauses=grounds,
                phase="refinement",
                iteration=1,
            )
        )
        key = instgen.clause_key(ground)
        self.assertIn(key, known)
        self.assertNotIn(key, solved)

    def test_dimacs_mapping_is_deterministic_and_complete(self) -> None:
        first = (
            instgen.Literal(instgen.Atom("z", ()), False),
            instgen.Literal(instgen.Atom("a", ("b",)), True),
        )
        mapping = instgen.atom_map([first])
        self.assertEqual(
            {atom.canonical(): value for atom, value in mapping.items()},
            {"a(b)": 1, "z()": 2},
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.cnf"
            instgen.write_dimacs(path, [first], mapping)
            self.assertEqual(path.read_text(), "p cnf 2 1\n-2 1 0\n")

    def test_incomplete_sat_model_is_rejected(self) -> None:
        mapping = {
            instgen.Atom("p", ()): 1,
            instgen.Atom("q", ()): 2,
        }
        with self.assertRaisesRegex(instgen.InstGenError, "complete model"):
            instgen.model_from_result({"model": [1]}, mapping)


class IndependentVerifierTests(unittest.TestCase):
    def test_normalization_and_dimacs_parser(self) -> None:
        self.assertIsNone(
            verify.normalize_clause([("p(a)", True), ("p(a)", False)])
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.cnf"
            path.write_text("p cnf 2 2\n1 -2 0\n0\n", encoding="ascii")
            self.assertEqual(
                verify.parse_dimacs(path),
                (2, [(1, -2), ()]),
            )

    def test_record_clause_rejects_noncanonical_or_duplicate_data(self) -> None:
        with self.assertRaisesRegex(verify.VerificationError, "canonical"):
            verify.record_clause(
                [
                    {"atom": "z()", "positive": True},
                    {"atom": "a()", "positive": True},
                ]
            )
        with self.assertRaisesRegex(verify.VerificationError, "canonical"):
            verify.record_clause(
                [
                    {"atom": "a()", "positive": True},
                    {"atom": "a()", "positive": True},
                ]
            )


class SelectionTests(unittest.TestCase):
    def test_rank_is_stable_and_sensitive_to_source_identity(self) -> None:
        record = {
            "holdout_split": "train",
            "family": "SYN",
            "expected_class": "satisfiable",
            "problem_id": "SYN000-1",
            "sha256": "a" * 64,
        }
        first = selector.stable_score(record)
        self.assertEqual(first, selector.stable_score(dict(record)))
        record["sha256"] = "b" * 64
        self.assertNotEqual(first, selector.stable_score(record))

    def test_frozen_corpus_has_expected_cells(self) -> None:
        records = [
            json.loads(line)
            for line in (ROOT / "corpus.jsonl").read_text().splitlines()
            if line
        ]
        self.assertEqual(records[0]["problem_count"], 29)
        counts: dict[tuple[str, str, str], int] = {}
        for record in records[1:]:
            key = (
                record["holdout_split"],
                record["family"],
                record["expected_class"],
            )
            counts[key] = counts.get(key, 0) + 1
        self.assertEqual(counts, selector.QUOTAS)


if __name__ == "__main__":
    unittest.main()
