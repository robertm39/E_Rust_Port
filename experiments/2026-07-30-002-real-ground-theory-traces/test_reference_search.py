#!/usr/bin/env python3
"""Focused tests for exact reference decisions and pruned search."""

from __future__ import annotations

import unittest

import reference_search
import trace_model


class ReferenceSearchTests(unittest.TestCase):
    def test_exact_sat_model_and_unsat_core_replay(self) -> None:
        feasible = [
            {
                "kind": "difference",
                "label": "a",
                "lhs": "x",
                "rhs": "zero",
                "bound": "3",
            },
            {
                "kind": "difference",
                "label": "b",
                "lhs": "zero",
                "rhs": "x",
                "bound": "-1",
            },
        ]
        sat = reference_search.decide_difference(feasible, "Int")
        self.assertEqual(sat["status"], "sat")
        self.assertTrue(reference_search.verify_model(feasible, sat["model"]))
        inconsistent = [
            feasible[0],
            {**feasible[1], "bound": "-4"},
        ]
        unsat = reference_search.decide_difference(inconsistent, "Int")
        self.assertEqual(unsat["status"], "unsat")
        self.assertTrue(
            reference_search.verify_negative_cycle(
                inconsistent,
                unsat["core"],
            )
        )

    def test_root_inconsistency_closes_finite_abstraction(self) -> None:
        transcript = trace_model.parse_transcript(
            """
            tff(dx,type,x:$int).
            tcf(c1,plain,($lesseq(x,0))).
            tcf(c2,plain,($greater(x,0))).
            """
        )
        abstraction = trace_model.build_abstraction(
            transcript,
            source_id="toy",
            source_sha256="a" * 64,
            family="TOY",
            partition="train",
        )
        result = reference_search.run_reference_search(abstraction)
        self.assertTrue(result["closed"])
        self.assertEqual(result["theory_prunes"], 1)
        self.assertEqual(result["open_leaves"], 0)
        self.assertEqual(result["queries"][0]["reference"]["status"], "unsat")


if __name__ == "__main__":
    unittest.main()
