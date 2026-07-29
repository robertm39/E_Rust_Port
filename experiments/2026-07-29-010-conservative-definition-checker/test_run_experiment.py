#!/usr/bin/env python3
"""Focused controller tests for the conservative-definition matrix."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("run_experiment.py")
SPEC = importlib.util.spec_from_file_location("definition_checker_experiment", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


PROBLEM = """\
cnf(q_source,axiom,q).
cnf(mixed_source,axiom,(p|~q)).
cnf(not_p_source,axiom,~p).
"""
PROOF = """\
% SZS status Unsatisfiable
% SZS output start CNFRefutation
cnf(q_source,axiom,q,file('used-definition-problem.p',q_source)).
cnf(mixed_source,axiom,(p|~q),file('used-definition-problem.p',mixed_source)).
cnf(not_p_source,axiom,~p,file('used-definition-problem.p',not_p_source)).
fof(test_definition,definition,(epred1_0<=>q),introduced(definition,[new_symbols(definition,[epred1_0])],[])).
cnf(test_split,plain,(epred1_0|~q),inference(split_equiv,[status(thm)],[test_definition])).
cnf(false,plain,$false,inference(cn,[status(thm)],[test_split])).
% SZS output end CNFRefutation
"""


class ControllerTests(unittest.TestCase):
    def test_status_parser_uses_last_status(self) -> None:
        self.assertEqual(
            MODULE.parse_status(
                "% SZS status Unsatisfiable\n% SZS status VerifiedGood\n"
            ),
            "verifiedgood",
        )

    def test_mutation_matrix_is_exact_and_distinct(self) -> None:
        cases = MODULE.mutation_cases(PROBLEM, PROOF)
        self.assertEqual(
            set(cases),
            {
                "reused-symbol",
                "circular-definition",
                "altered-body",
                "omitted-ancestry",
            },
        )
        pairs = set(cases.values())
        self.assertEqual(len(pairs), 4)
        self.assertIn("prior_epred", cases["reused-symbol"][0])
        self.assertIn(
            "epred1_0<=>(epred1_0|q)",
            cases["circular-definition"][1],
        )
        self.assertIn(
            "(epred1_0<=>p)",
            cases["altered-body"][1],
        )
        self.assertIn(
            "[q_source]",
            cases["omitted-ancestry"][1],
        )

    def test_mutation_builder_fails_closed_on_fixture_drift(self) -> None:
        with self.assertRaises(MODULE.ExperimentError):
            MODULE.mutation_cases(PROBLEM, PROOF.replace("epred1_0<=>q", "p"))


if __name__ == "__main__":
    unittest.main()
