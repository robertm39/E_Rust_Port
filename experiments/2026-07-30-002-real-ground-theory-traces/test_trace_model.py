#!/usr/bin/env python3
"""Focused tests for real ground-theory trace extraction."""

from __future__ import annotations

import unittest

import trace_model


class TranscriptTests(unittest.TestCase):
    def test_typed_clause_is_grounded_by_sort_position(self) -> None:
        transcript = trace_model.parse_transcript(
            """
            % ignored
            tff(d1,type,a:$int).
            tcf(c1,plain,![X:$int,Y:$real]:
                (($lesseq(X,a)|p(Y)))).
            """
        )
        self.assertEqual(transcript.declarations["a"], "$int")
        clause = transcript.clauses[0]
        self.assertEqual(
            clause.grounding,
            {"X": "ground_int_0", "Y": "ground_real_0"},
        )
        self.assertEqual(
            [literal.atom.canonical() for literal in clause.literals],
            ["le(ground_int_0,a)", "p(ground_real_0)"],
        )

    def test_infix_disequality_is_negative_equality(self) -> None:
        literal = trace_model.parse_literal("f(X)!=g(X)")
        self.assertEqual(literal.atom.relation, "eq")
        self.assertFalse(literal.positive)

    def test_strict_integer_and_nonstrict_complement_translate(self) -> None:
        declarations = {"x": "$int", "y": "$int"}
        atom = trace_model.Atom(
            "lt",
            (trace_model.Term("x"), trace_model.Term("y")),
        )
        sort, constraints, reason = trace_model.relation_constraints(
            atom, True, declarations
        )
        self.assertEqual(sort, "Int")
        self.assertIsNone(reason)
        self.assertEqual(constraints[0]["kind"], "difference")
        self.assertEqual(constraints[0]["bound"], "-1")
        self.assertEqual(
            constraints[0]["opaque_terms"][constraints[0]["lhs"]],
            "x",
        )
        self.assertEqual(
            constraints[0]["opaque_terms"][constraints[0]["rhs"]],
            "y",
        )
        _, complement, reason = trace_model.relation_constraints(
            atom, False, declarations
        )
        self.assertIsNone(reason)
        self.assertEqual(complement[0]["bound"], "0")
        self.assertEqual(complement[0]["lhs"], constraints[0]["rhs"])
        self.assertEqual(complement[0]["rhs"], constraints[0]["lhs"])

    def test_strict_real_and_disequality_are_unsupported(self) -> None:
        declarations = {"x": "$real", "y": "$real"}
        less = trace_model.Atom(
            "lt",
            (trace_model.Term("x"), trace_model.Term("y")),
        )
        self.assertEqual(
            trace_model.relation_constraints(less, True, declarations)[2],
            "STRICT_REAL",
        )
        equality = trace_model.Atom(
            "eq",
            (trace_model.Term("x"), trace_model.Term("y")),
        )
        self.assertEqual(
            trace_model.relation_constraints(equality, False, declarations)[2],
            "DISEQUALITY",
        )

    def test_linear_difference_and_constant_scale_are_supported(self) -> None:
        declarations = {"x": "$int", "y": "$int"}
        atom = trace_model.Atom(
            "le",
            (
                trace_model.parse_term(
                    "$sum($product(2,x),$product(-1,x))"
                ),
                trace_model.parse_term("$sum(y,3)"),
            ),
        )
        sort, constraints, reason = trace_model.relation_constraints(
            atom, True, declarations
        )
        self.assertEqual(sort, "Int")
        self.assertIsNone(reason)
        self.assertEqual(constraints[0]["bound"], "3")

    def test_nonlinear_product_is_unknown(self) -> None:
        declarations = {"x": "$int", "y": "$int"}
        atom = trace_model.Atom(
            "le",
            (trace_model.parse_term("$product(x,y)"), trace_model.Term("0")),
        )
        self.assertEqual(
            trace_model.relation_constraints(atom, True, declarations)[2],
            "NONLINEAR_PRODUCT",
        )

    def test_dpll_trace_records_eligible_query_and_provenance(self) -> None:
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
        trace = trace_model.build_no_theory_trace(abstraction)
        self.assertEqual(trace["status"], "complete")
        self.assertEqual(trace["eligible_queries"], 1)
        query = trace["queries"][0]
        self.assertEqual(len(query["constraints"]), 2)
        self.assertTrue(all("atom" in constraint for constraint in query["constraints"]))
        self.assertEqual(trace["leaves"], 1)

    def test_unsupported_literal_does_not_hide_unsat_capable_subset(self) -> None:
        transcript = trace_model.parse_transcript(
            """
            tff(dx,type,x:$int).
            tff(dy,type,y:$int).
            tcf(c1,plain,($lesseq(x,0))).
            tcf(c2,plain,($greater(x,0))).
            tcf(c3,plain,(x!=y)).
            """
        )
        abstraction = trace_model.build_abstraction(
            transcript,
            source_id="toy",
            source_sha256="a" * 64,
            family="TOY",
            partition="train",
        )
        trace = trace_model.build_no_theory_trace(abstraction)
        self.assertEqual(trace["eligible_queries"], 1)
        query = trace["queries"][0]
        self.assertEqual(len(query["constraints"]), 2)
        self.assertEqual(
            query["excluded_unsupported"][0]["reason"],
            "DISEQUALITY",
        )

    def test_atom_and_clause_bounds_fail_closed(self) -> None:
        transcript = trace_model.parse_transcript(
            """
            tff(dx,type,x:$int).
            tcf(c1,plain,($lesseq(x,0)|p)).
            """
        )
        abstraction = trace_model.build_abstraction(
            transcript,
            source_id="toy",
            source_sha256="a" * 64,
            family="TOY",
            partition="train",
            max_atoms=1,
        )
        self.assertEqual(abstraction["bounds_crossed"], ["atoms"])
        self.assertEqual(
            trace_model.build_no_theory_trace(abstraction)["status"],
            "bound",
        )


if __name__ == "__main__":
    unittest.main()
