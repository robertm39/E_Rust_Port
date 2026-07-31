#!/usr/bin/env python3
"""Unit tests for the bounded connection worker and independent replay."""

from __future__ import annotations

import copy
import time
import unittest
from pathlib import Path

import connection_common as common
import connection_worker as worker
import verify_connection as verifier


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]


def term(symbol: str, *arguments: common.Term) -> common.Term:
    return common.Term(symbol, tuple(arguments))


def literal(
    predicate: str, positive: bool, *arguments: common.Term
) -> common.Literal:
    return common.Literal(predicate, tuple(arguments), positive)


def chain_matrix() -> list[common.Clause]:
    atom = term("a")
    return [
        common.Clause(
            0,
            "start",
            "negated_conjecture",
            (literal("p", False, atom),),
            "start-hash",
        ),
        common.Clause(
            1,
            "chain_1",
            "plain",
            (literal("p", True, atom), literal("q", True, atom)),
            "chain-1-hash",
        ),
        common.Clause(
            2,
            "chain_2",
            "plain",
            (literal("q", False, atom), literal("p", True, atom)),
            "chain-2-hash",
        ),
    ]


class ConnectionWorkerTests(unittest.TestCase):
    def test_occurs_check_rejects_recursive_binding(self) -> None:
        variable = term("X")
        function = term("f", variable)
        bindings: dict[str, common.Term] = {}
        self.assertFalse(worker.unify_terms(variable, function, bindings))
        self.assertEqual(bindings, {})

        with self.assertRaises(verifier.VerificationError):
            verifier.verifier_unify_term(variable, function, {})

    def test_shared_variable_unification_is_consistent(self) -> None:
        variable = term("X")
        left = literal("r", True, variable, variable)
        right = literal("r", False, term("a"), term("a"))
        bindings: dict[str, common.Term] = {}
        self.assertTrue(worker.unify_atoms(left, right, bindings))
        self.assertEqual(worker.apply_term(variable, bindings), term("a"))

        bad: dict[str, common.Term] = {}
        self.assertFalse(
            worker.unify_atoms(
                left,
                literal("r", False, term("a"), term("b")),
                bad,
            )
        )

    def test_search_builds_extension_and_reduction_proof(self) -> None:
        matrix = chain_matrix()
        context = worker.SearchContext(
            clauses=matrix,
            deadline=time.monotonic() + 10,
            maximum_nodes=10_000,
        )
        result = context.search()
        self.assertIsNotNone(result)
        start_index, start_instance, proof = result or (-1, -1, {})
        counts = worker.proof_rule_counts(proof)
        self.assertEqual(start_index, 0)
        self.assertGreater(start_instance, 0)
        self.assertEqual(counts, {"extension": 2, "reduction": 1})

        replay = verifier.Replay(matrix)
        replay.claim_instance(start_instance)
        start_goals = tuple(
            verifier.verifier_fresh_literal(item, start_instance)
            for item in matrix[start_index].literals
        )
        replay.replay(proof, start_goals, (), {}, 0)
        self.assertEqual(replay.rule_counts, counts)

    def test_goal_diagnostic_mutation_is_rejected(self) -> None:
        matrix = chain_matrix()
        context = worker.SearchContext(
            clauses=matrix,
            deadline=time.monotonic() + 10,
            maximum_nodes=10_000,
        )
        result = context.search()
        self.assertIsNotNone(result)
        start_index, start_instance, proof = result or (-1, -1, {})
        changed = copy.deepcopy(proof)
        changed["goal"] = "mutated"
        replay = verifier.Replay(matrix)
        replay.claim_instance(start_instance)
        goals = tuple(
            verifier.verifier_fresh_literal(item, start_instance)
            for item in matrix[start_index].literals
        )
        with self.assertRaises(verifier.VerificationError):
            replay.replay(changed, goals, (), {}, 0)

    def test_parser_retains_roles_functions_and_variables(self) -> None:
        transcript = "\n".join(
            (
                "cnf(a, plain, (p(f(X)) | ~q(X))).",
                "cnf(c, negated_conjecture, (~p(f(a)))).",
            )
        )
        clauses = common.parse_cnf_transcript(
            transcript,
            repo_root=REPO_ROOT,
            module_name="connection_unit_trace_parser",
        )
        self.assertEqual([clause.role for clause in clauses], ["plain", "negated_conjecture"])
        self.assertEqual(
            clauses[0].literals[0].canonical(),
            "p(f(X))",
        )


if __name__ == "__main__":
    unittest.main()

