#!/usr/bin/env python3
"""Unit tests for the bounded finite-model prototype and checker adapter."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent


def load(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, HERE / filename)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


finite_model = load("finite_model", "finite_model.py")
model_check = load("vampire_model_check", "vampire_model_check.py")
fetch_samples = load("fetch_samples", "fetch_samples.py")
adversarial = load("adversarial_validation", "adversarial_validation.py")
summarize = load("summarize", "summarize.py")
runner = load("experiment_runner", "run.py")


class PrototypeTests(unittest.TestCase):
    def test_parser_accepts_function_free_predicates_and_equality(self) -> None:
        problem = finite_model.parse_cnf(
            """
            cnf(a,plain,(p(X,a)|~q(X))).
            cnf(b,plain,(X=a|~p(X,a))).
            """
        )
        self.assertEqual(problem.constants, ("a",))
        self.assertEqual(problem.predicates, {"p": 2, "q": 1})
        self.assertEqual(len(problem.clauses), 2)

    def test_parser_rejects_positive_arity_functions(self) -> None:
        with self.assertRaises(finite_model.UnsupportedInput):
            finite_model.parse_cnf("cnf(a,plain,p(f(X))).")

    def test_sorted_inference_separates_independent_positions(self) -> None:
        problem = finite_model.parse_cnf(
            "cnf(a,plain,(p(X,a)|q(Y,b)))."
        )
        layout = finite_model.infer_sorts(problem, "sorted")
        self.assertEqual(layout.sort_count, 4)
        self.assertNotEqual(
            layout.predicate_sorts[("p", 0)],
            layout.predicate_sorts[("q", 0)],
        )
        self.assertEqual(
            layout.predicate_sorts[("p", 1)], layout.constant_sorts["a"]
        )

    def test_constant_guards_make_unit_clause_conditional(self) -> None:
        problem = finite_model.parse_cnf("cnf(a,plain,p(c)).")
        layout = finite_model.infer_sorts(problem, "naive")
        encoding = finite_model.Encoding(problem, layout, (2,), False, 100)
        encoding.build()
        c0 = encoding.constant_variable("c", 0)
        c1 = encoding.constant_variable("c", 1)
        p0 = encoding.predicate_variable("p", (0,))
        p1 = encoding.predicate_variable("p", (1,))
        self.assertIn([-c0, p0], encoding.clauses)
        self.assertIn([-c1, p1], encoding.clauses)

    def test_symmetry_fixes_first_constant(self) -> None:
        problem = finite_model.parse_cnf("cnf(a,plain,p(c)).")
        layout = finite_model.infer_sorts(problem, "sorted")
        encoding = finite_model.Encoding(problem, layout, (3,), True, 100)
        encoding.build()
        self.assertIn([-encoding.constant_variable("c", 1)], encoding.clauses)
        self.assertIn([-encoding.constant_variable("c", 2)], encoding.clauses)

    def test_empty_clause_is_encoded_as_unsatisfiable(self) -> None:
        problem = finite_model.parse_cnf("cnf(a,plain,$false).")
        layout = finite_model.infer_sorts(problem, "naive")
        encoding = finite_model.Encoding(problem, layout, (1,), False, 100)
        encoding.build()
        self.assertIn([], encoding.clauses)

    def test_checker_negates_conjectures_without_parsing_formula(self) -> None:
        transformed = model_check.semantic_problem(
            """
            fof(a,axiom,! [X] : p(X)).
            fof(c,conjecture,? [Y] : (p(Y) & q(Y))).
            """
        )
        self.assertIn("fof(a,axiom,! [X] : p(X)).", transformed)
        self.assertIn(
            "fof(c,axiom,~(? [Y] : (p(Y) & q(Y)))).", transformed
        )

    def test_bounds_limit_is_fail_closed(self) -> None:
        problem = finite_model.parse_cnf("cnf(a,plain,(p(X)|q(Y))).")
        layout = finite_model.infer_sorts(problem, "naive")
        encoding = finite_model.Encoding(problem, layout, (3,), False, 2)
        with self.assertRaises(finite_model.EncodingLimit):
            encoding.build()

    def test_size_vectors_use_increasing_total_domain(self) -> None:
        self.assertEqual(
            list(finite_model.domain_size_vectors(2, 2)),
            [(1, 1), (1, 2), (2, 1), (2, 2)],
        )

    def test_sample_extractor_decodes_problem_body(self) -> None:
        extractor = fetch_samples.FirstPreformattedBlock()
        extractor.feed(
            "<html><pre>% File : X\nfof(a,axiom,(p &lt;=&gt; q)).\n"
            "</pre><pre>ignored</pre></html>"
        )
        self.assertTrue(extractor.finished)
        self.assertEqual(
            "".join(extractor.parts),
            "% File : X\nfof(a,axiom,(p <=> q)).\n",
        )

    def test_adversarial_replacement_must_be_unique(self) -> None:
        self.assertEqual(adversarial.replace_once("abc", "b", "B"), "aBc")
        with self.assertRaises(ValueError):
            adversarial.replace_once("bbb", "b", "B")

    def test_summary_counts_only_verified_unique_models(self) -> None:
        prototypes = [
            {
                "problem_id": "A",
                "mode": "sorted",
                "outcome": "model",
                "validation_verdict": "verified",
            },
            {
                "problem_id": "B",
                "mode": "sorted",
                "outcome": "model",
                "validation_verdict": "rejected",
            },
        ]
        baselines = [
            {"problem_id": "B", "system": "umlaut-auto", "status": "Satisfiable"}
        ]
        self.assertEqual(
            summarize.comparisons(prototypes, baselines)["sorted"][
                "unique_against_umlaut_auto"
            ],
            ["A"],
        )

    def test_controller_timeout_preserves_text_output(self) -> None:
        returncode, _, stdout, stderr = runner.run_command(
            [
                sys.executable,
                "-c",
                "import time; print('started', flush=True); time.sleep(1)",
            ],
            0.05,
        )
        self.assertEqual(returncode, 124)
        self.assertIsInstance(stdout, str)
        self.assertIsInstance(stderr, str)
        self.assertIn("started", stdout)


if __name__ == "__main__":
    unittest.main()
