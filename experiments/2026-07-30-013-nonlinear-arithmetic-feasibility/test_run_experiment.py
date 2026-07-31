#!/usr/bin/env python3
"""Focused tests for the nonlinear arithmetic feasibility harness."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "nonlinear_feasibility",
    HERE / "run_experiment.py",
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def analyze_text(text: str):
    statements, includes = MODULE.parse_document(text)
    declarations, error = MODULE.scalar_declarations(statements)
    assert error is None
    formulas = []
    max_degree = 0
    quantifiers = 0
    nonlinear = False
    for statement in statements:
        if statement.role == "type":
            continue
        formula = MODULE.FormulaParser(statement.body_tokens).parse()
        degree, count, _symbols = MODULE.analyze_formula(
            formula,
            declarations,
        )
        formulas.append((statement.role, formula))
        max_degree = max(max_degree, degree)
        quantifiers += count
        nonlinear = nonlinear or any(
            MODULE.term_has_nonlinear_syntax(term)
            for term in MODULE.iter_terms(formula)
        )
    return includes, declarations, formulas, max_degree, quantifiers, nonlinear


class ParserTests(unittest.TestCase):
    def test_comments_nesting_and_symbolic_product(self) -> None:
        text = """
        % line comment
        tff(x_type,type,x: $real).
        /* block comment */
        tff(y_type,type,y: $real).
        tff(goal,conjecture,
            ( $less($product(x,y),3.0)
            | $greatereq($difference(x,y),-1/2) )).
        """
        included, _decls, formulas, degree, quantifiers, nonlinear = (
            analyze_text(text)
        )
        self.assertFalse(included)
        self.assertEqual(len(formulas), 1)
        self.assertEqual(degree, 2)
        self.assertEqual(quantifiers, 0)
        self.assertTrue(nonlinear)

    def test_constant_product_and_division_stay_linear(self) -> None:
        text = """
        tff(x_type,type,x: $real).
        tff(a,axiom,
            $lesseq($quotient($product(2,x),4),9)).
        """
        _included, _decls, _formulas, degree, _quantifiers, nonlinear = (
            analyze_text(text)
        )
        self.assertEqual(degree, 1)
        self.assertFalse(nonlinear)

    def test_linear_time_nonlinear_scan_distinguishes_symbolic_operands(
        self,
    ) -> None:
        cases = {
            "$product(2,3)": False,
            "$product(2,X)": False,
            "$product(X,Y)": True,
            "$product($sum(X,1),$product(2,Y))": True,
            "$quotient(X,2)": False,
            "$quotient(X,Y)": True,
            "$floor(X)": True,
        }
        for text, expected in cases.items():
            with self.subTest(text=text):
                self.assertEqual(
                    MODULE.tokens_have_nonlinear_syntax(
                        MODULE.tokenize(text)
                    ),
                    expected,
                )

    def test_quantifier_and_reverse_implication_render(self) -> None:
        text = """
        tff(goal,conjecture,
            ! [X: $real] :
              ( $less($product(X,X),2) <= $less(0,X) )).
        """
        _included, declarations, formulas, degree, quantifiers, nonlinear = (
            analyze_text(text)
        )
        self.assertEqual(degree, 2)
        self.assertEqual(quantifiers, 1)
        self.assertTrue(nonlinear)
        renderer = MODULE.SmtRenderer(declarations)
        rendered = renderer.render_formula(formulas[0][1])
        self.assertEqual(
            rendered,
            "(forall ((v_0 Real)) "
            "(=> (< 0 v_0) (< (* v_0 v_0) 2)))",
        )

    def test_user_function_is_excluded(self) -> None:
        text = """
        tff(f_type,type,f: $real > $real).
        tff(a,axiom,! [X: $real] : $less($product(X,f(X)),1)).
        """
        statements, _includes = MODULE.parse_document(text)
        declarations, _error = MODULE.scalar_declarations(statements)
        formula = MODULE.FormulaParser(statements[1].body_tokens).parse()
        with self.assertRaisesRegex(MODULE.ExperimentError, "unsupported function"):
            MODULE.analyze_formula(formula, declarations)

    def test_symbolic_and_zero_division_are_excluded(self) -> None:
        for denominator, code in (("Y", "symbolic_division"), ("0", "zero_division")):
            text = f"""
            tff(a,axiom,! [X: $real,Y: $real] :
                $less($quotient(X,{denominator}),1)).
            """
            statements, _includes = MODULE.parse_document(text)
            formula = MODULE.FormulaParser(statements[0].body_tokens).parse()
            with self.assertRaises(MODULE.ExperimentError) as raised:
                MODULE.analyze_formula(formula, {})
            self.assertEqual(raised.exception.code, code)

    def test_include_is_recorded(self) -> None:
        text = """
        include('Axioms/Field.ax').
        tff(a,axiom,$true).
        """
        statements, includes = MODULE.parse_document(text)
        self.assertTrue(includes)
        self.assertEqual(len(statements), 1)

    def test_status_normalization_is_fail_closed(self) -> None:
        self.assertEqual(MODULE.normalize_solver_status("unsat\n"), "unsat")
        self.assertEqual(
            MODULE.normalize_solver_status("(error \"x\")\nunknown\n"),
            "unknown",
        )
        self.assertIsNone(MODULE.normalize_solver_status("(error \"x\")\n"))

    def test_problem_hash_mismatch_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            problem = root / "p.p"
            problem.write_text("tff(a,axiom,$true).\n")
            record = {
                "path": "p.p",
                "sha256": "0" * 64,
                "problem_id": "p",
                "category": "TFE",
                "division": "TFA",
                "family": "X",
                "holdout_split": "train",
                "expected_class": "theorem",
            }
            with self.assertRaises(MODULE.ExperimentError) as raised:
                MODULE.analyze_problem(root, record, 10_000)
            self.assertEqual(raised.exception.code, "problem_hash")

    def test_resume_rejects_query_hash_mismatch(self) -> None:
        problem = MODULE.ProblemAnalysis(
            path="p.p",
            problem_id="p",
            category="TFE",
            division="TFA",
            family="X",
            split="train",
            expected_class="theorem",
            arithmetic_active=True,
            nonlinear_active=True,
            whole_real_polynomial=True,
            fragment="whole_qf_nra",
            exclusion_reason=None,
            formula_count=1,
            quantifier_count=0,
            max_degree=2,
            query_sha256="0" * 64,
            expected_status="unsat",
            solver_runs=[],
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.json"
            query_dir = root / "queries"
            query_dir.mkdir()
            (query_dir / "p.smt2").write_text("(check-sat)\n")
            inventory.write_text(
                json.dumps(
                    {
                        "schema": "umlaut-nonlinear-arithmetic-feasibility-v1",
                        "protocol": {
                            "inventory_only": True,
                            "expected_z3_commit": MODULE.EXPECTED_Z3_COMMIT,
                            "timeout_ms": 10_000,
                        },
                        "inputs": {},
                        "problems": [MODULE.asdict(problem)],
                    }
                )
            )
            with self.assertRaisesRegex(SystemExit, "query hash mismatch"):
                MODULE.main(
                    [
                        "--repo-root",
                        str(root),
                        "--manifest",
                        str(root / "unused.jsonl"),
                        "--z3-source-root",
                        str(HERE),
                        "--output",
                        str(root / "output.json"),
                        "--query-dir",
                        str(query_dir),
                        "--inventory-input",
                        str(inventory),
                        "--inventory-only",
                    ]
                )


if __name__ == "__main__":
    unittest.main()
