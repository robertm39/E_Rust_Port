#!/usr/bin/env python3
"""Unit tests for the TSTP ancestry experiment controller."""

from __future__ import annotations

import unittest

import run_experiment


class ExperimentControllerTests(unittest.TestCase):
    def test_mutate_body_preserves_nested_annotation(self) -> None:
        line = (
            "cnf(source, axiom, (~q(A)|p(A,B)), "
            "file('/tmp/problem,one.p', source))."
        )

        mutated = run_experiment.mutate_annotated_body(line)

        self.assertEqual(
            mutated,
            "cnf(source, axiom, umlaut_mutation_symbol, "
            "file('/tmp/problem,one.p', source)).",
        )

    def test_leaf_indexes_ignore_derived_steps(self) -> None:
        proof = (
            "cnf(source, axiom, p(a), file('p.p', source)).\n"
            "cnf(derived, plain, p(a), inference(copy,[status(thm)],[source])).\n"
        )

        self.assertEqual(run_experiment.leaf_line_indexes(proof), [0])

    def test_definition_indexes_select_introduced_symbol_record(self) -> None:
        proof = (
            "fof(def, definition, (epred1<=>p(a)), "
            "introduced(definition,[new_symbols(definition,[epred1])],[])).\n"
        )

        self.assertEqual(run_experiment.definition_line_indexes(proof), [0])

    def test_truncation_removes_only_first_end_marker(self) -> None:
        proof = (
            "% SZS output start CNFRefutation\n"
            "cnf(false, plain, $false).\n"
            "% SZS output end CNFRefutation\n"
        )

        truncated = run_experiment.truncate_refutation(proof)

        self.assertIn(run_experiment.START_MARKER, truncated)
        self.assertNotIn(run_experiment.END_MARKER, truncated)


if __name__ == "__main__":
    unittest.main()
