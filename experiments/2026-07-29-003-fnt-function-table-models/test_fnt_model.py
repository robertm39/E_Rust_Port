#!/usr/bin/env python3
"""Unit tests for typed finite-model parsing, encoding, and rendering."""

from __future__ import annotations

import importlib.util
import os
import sys
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("fnt_model", HERE / "fnt_model.py")
assert SPEC is not None and SPEC.loader is not None
fnt_model = importlib.util.module_from_spec(SPEC)
sys.modules["fnt_model"] = fnt_model
SPEC.loader.exec_module(fnt_model)
ADAPTER_SPEC = importlib.util.spec_from_file_location(
    "vampire_model_check", HERE / "vampire_model_check.py"
)
assert ADAPTER_SPEC is not None and ADAPTER_SPEC.loader is not None
vampire_model_check = importlib.util.module_from_spec(ADAPTER_SPEC)
sys.modules["vampire_model_check"] = vampire_model_check
ADAPTER_SPEC.loader.exec_module(vampire_model_check)
ADVERSARIAL_SPEC = importlib.util.spec_from_file_location(
    "adversarial_validation", HERE / "adversarial_validation.py"
)
assert ADVERSARIAL_SPEC is not None and ADVERSARIAL_SPEC.loader is not None
adversarial_validation = importlib.util.module_from_spec(ADVERSARIAL_SPEC)
sys.modules["adversarial_validation"] = adversarial_validation
ADVERSARIAL_SPEC.loader.exec_module(adversarial_validation)
CORPUS_SPEC = importlib.util.spec_from_file_location(
    "run_corpus", HERE / "run_corpus.py"
)
assert CORPUS_SPEC is not None and CORPUS_SPEC.loader is not None
run_corpus = importlib.util.module_from_spec(CORPUS_SPEC)
sys.modules["run_corpus"] = run_corpus
CORPUS_SPEC.loader.exec_module(run_corpus)
SUMMARY_SPEC = importlib.util.spec_from_file_location(
    "summarize", HERE / "summarize.py"
)
assert SUMMARY_SPEC is not None and SUMMARY_SPEC.loader is not None
summarize = importlib.util.module_from_spec(SUMMARY_SPEC)
sys.modules["summarize"] = summarize
SUMMARY_SPEC.loader.exec_module(summarize)


TWO_SORT_CNF = """
tff(decl_23, type, alice: person).
tff(decl_24, type, red: color).
tff(decl_25, type, favorite: person > color).
tff(decl_26, type, likes: (person * color) > $o).
cnf(i_0_1, plain, ($true:$o)).
tcf(i_0_7, plain, favorite(alice:person):color=red:color).
tcf(i_0_8, plain, likes(alice:person,favorite(alice:person):color):$o).
"""


def solve(
    clauses: list[list[int]], assumptions: list[int], maximum: int
) -> frozenset[int] | None:
    materialized = [list(clause) for clause in clauses]
    materialized.extend([[literal] for literal in assumptions])

    def recurse(
        remaining: list[list[int]], assignment: dict[int, bool]
    ) -> dict[int, bool] | None:
        while True:
            reduced: list[list[int]] = []
            unit: int | None = None
            for clause in remaining:
                pending: list[int] = []
                satisfied = False
                for literal in clause:
                    value = assignment.get(abs(literal))
                    if value is None:
                        pending.append(literal)
                    elif value == (literal > 0):
                        satisfied = True
                        break
                if satisfied:
                    continue
                if not pending:
                    return None
                if len(pending) == 1:
                    unit = pending[0]
                reduced.append(pending)
            remaining = reduced
            if unit is None:
                break
            variable = abs(unit)
            value = unit > 0
            if variable in assignment and assignment[variable] != value:
                return None
            assignment[variable] = value
        if not remaining:
            return assignment
        variable = abs(remaining[0][0])
        for value in (False, True):
            candidate = recurse(remaining, {**assignment, variable: value})
            if candidate is not None:
                return candidate
        return None

    assignment = recurse(materialized, {})
    if assignment is None:
        return None
    return frozenset(
        variable
        for variable in range(1, maximum + 1)
        if assignment.get(variable, False)
    )


class TypedFiniteModelTests(unittest.TestCase):
    def test_parser_preserves_native_sorts_and_function_signature(self) -> None:
        problem = fnt_model.parse_typed_cnf(TWO_SORT_CNF)
        self.assertEqual(problem.sorts, ("color", "person"))
        self.assertEqual(
            problem.functions["favorite"],
            fnt_model.SymbolType(("person",), "color"),
        )
        self.assertEqual(
            problem.predicates["likes"],
            fnt_model.SymbolType(("person", "color"), "$o"),
        )

    def test_parser_rejects_missing_term_type(self) -> None:
        with self.assertRaisesRegex(fnt_model.UnsupportedInput, "type suffix"):
            fnt_model.parse_typed_cnf("cnf(a,plain,p(a)).")

    def test_parser_accepts_bare_false_empty_clause(self) -> None:
        problem = fnt_model.parse_typed_cnf("cnf(a,plain,$false).")
        self.assertEqual(problem.clauses[0].literals, ())

    def test_parser_rejects_conflicting_symbol_types(self) -> None:
        with self.assertRaisesRegex(fnt_model.UnsupportedInput, "inconsistent"):
            fnt_model.parse_typed_cnf(
                """
                tff(f_type,type,f:a > a).
                tcf(one,plain,f(X:a):a=X:a).
                tcf(two,plain,f(Y:b):b=Y:b).
                """
            )

    def test_parser_rejects_interpreted_sort(self) -> None:
        with self.assertRaisesRegex(fnt_model.UnsupportedInput, "interpreted sort"):
            fnt_model.parse_typed_cnf(
                "tff(one,type,n:$int).\ntcf(two,plain,n:$int=n:$int)."
            )

    def test_two_sort_function_model_decodes_and_validates(self) -> None:
        problem = fnt_model.parse_typed_cnf(TWO_SORT_CNF)
        encoding = fnt_model.Encoding(problem, 2, 100_000, 100_000)
        sizes = {"color": 1, "person": 1}
        encoding.extend_grounding(sizes)
        assignment = solve(
            encoding.database.clauses,
            encoding.assumptions(sizes),
            encoding.variable_count,
        )
        self.assertIsNotNone(assignment)
        assert assignment is not None
        functions = fnt_model.decode_functions(encoding, sizes, assignment)
        predicates = fnt_model.decode_predicates(encoding, sizes, assignment)
        fnt_model.validate_interpretation(problem, sizes, functions, predicates)
        self.assertEqual(functions[("favorite", (0,))], 0)
        self.assertTrue(predicates[("likes", (0, 0))])

    def test_nested_function_requires_second_element(self) -> None:
        problem = fnt_model.parse_typed_cnf(
            """
            tff(a_type,type,a:$i).
            tff(f_type,type,f:$i > $i).
            tcf(no_fixed_point,plain,f(f(a:$i):$i):$i!=a:$i).
            """
        )
        encoding = fnt_model.Encoding(problem, 2, 100_000, 100_000)
        one = {"$i": 1}
        encoding.extend_grounding(one)
        self.assertIsNone(
            solve(
                encoding.database.clauses,
                encoding.assumptions(one),
                encoding.variable_count,
            )
        )
        two = {"$i": 2}
        encoding.extend_grounding(two)
        assignment = solve(
            encoding.database.clauses,
            encoding.assumptions(two),
            encoding.variable_count,
        )
        self.assertIsNotNone(assignment)
        assert assignment is not None
        functions = fnt_model.decode_functions(encoding, two, assignment)
        predicates = fnt_model.decode_predicates(encoding, two, assignment)
        fnt_model.validate_interpretation(problem, two, functions, predicates)

    def test_universal_function_clause_is_guarded_by_domain_activity(self) -> None:
        problem = fnt_model.parse_typed_cnf(
            """
            tff(f_type,type,f:$i > $i).
            tcf(no_fixed_points,plain,f(X:$i):$i!=X:$i).
            """
        )
        encoding = fnt_model.Encoding(problem, 2, 100_000, 100_000)
        one = {"$i": 1}
        self.assertEqual(encoding.extend_grounding(one), 1)
        self.assertIsNone(
            solve(
                encoding.database.clauses,
                encoding.assumptions(one),
                encoding.variable_count,
            )
        )
        two = {"$i": 2}
        self.assertEqual(encoding.extend_grounding(two), 1)
        assignment = solve(
            encoding.database.clauses,
            encoding.assumptions(two),
            encoding.variable_count,
        )
        self.assertIsNotNone(assignment)

    def test_internal_validation_rejects_corrupted_function_row(self) -> None:
        problem = fnt_model.parse_typed_cnf(TWO_SORT_CNF)
        sizes = {"color": 1, "person": 1}
        functions = {
            ("alice", ()): 0,
            ("red", ()): 0,
            ("favorite", (0,)): 0,
        }
        predicates = {("likes", (0, 0)): True}
        fnt_model.validate_interpretation(problem, sizes, functions, predicates)
        predicates[("likes", (0, 0))] = False
        with self.assertRaisesRegex(fnt_model.SolverFailure, "falsifies"):
            fnt_model.validate_interpretation(
                problem, sizes, functions, predicates
            )

    def test_renderer_declares_separate_native_domains_and_function_rows(self) -> None:
        problem = fnt_model.parse_typed_cnf(TWO_SORT_CNF)
        model = fnt_model.render_model(
            "native",
            problem,
            {"color": 1, "person": 1},
            {
                ("alice", ()): 0,
                ("red", ()): 0,
                ("favorite", (0,)): 0,
            },
            {("likes", (0, 0)): True},
        )
        self.assertIn(":color).", model)
        self.assertIn(":person).", model)
        self.assertIn("tff(finite_domain_color,axiom", model)
        self.assertIn("tff(finite_domain_person,axiom", model)
        self.assertIn("favorite(umlaut_fmb_d_person_0)", model)
        self.assertIn(
            "likes(umlaut_fmb_d_person_0,umlaut_fmb_d_color_0)", model
        )

    def test_size_vectors_increase_total_cardinality(self) -> None:
        self.assertEqual(
            list(fnt_model.domain_size_vectors(("a", "b"), 2)),
            [
                {"a": 1, "b": 1},
                {"a": 1, "b": 2},
                {"a": 2, "b": 1},
                {"a": 2, "b": 2},
            ],
        )

    def test_ground_instance_limit_fails_closed(self) -> None:
        problem = fnt_model.parse_typed_cnf(
            "tff(p_type,type,p:$i>$o).\ntcf(c,plain,p(X:$i):$o)."
        )
        encoding = fnt_model.Encoding(problem, 2, 100_000, 1)
        with self.assertRaises(fnt_model.EncodingLimit):
            encoding.extend_grounding({"$i": 2})

    def test_typed_checker_adapter_negates_tff_conjecture(self) -> None:
        transformed = vampire_model_check.semantic_problem(
            """
            tff(person_type,type,person:$tType).
            tff(p_type,type,p:person>$o).
            tff(c,conjecture,?[X:person]:p(X)).
            """
        )
        self.assertIn("tff(person_type,type,person:$tType).", transformed)
        self.assertIn("tff(c,axiom,~(?[X:person]:p(X))).", transformed)

    def test_adversarial_replacement_must_be_unique(self) -> None:
        self.assertEqual(
            adversarial_validation.replace_once("abc", "b", "B"), "aBc"
        )
        with self.assertRaises(ValueError):
            adversarial_validation.replace_once("bbb", "b", "B")

    def test_controller_uses_final_szs_status(self) -> None:
        self.assertEqual(
            run_corpus.final_status(
                "% SZS status Unknown\n% SZS status CounterSatisfiable\n"
            ),
            "CounterSatisfiable",
        )

    def test_controller_timeout_preserves_partial_output(self) -> None:
        code, _, stdout, _ = run_corpus.run_command(
            [
                sys.executable,
                "-c",
                "import time; print('started', flush=True); time.sleep(1)",
            ],
            0.05,
            dict(os.environ),
        )
        self.assertEqual(code, 124)
        self.assertIn("started", stdout)

    def test_summary_counts_missing_status_explicitly(self) -> None:
        self.assertEqual(
            summarize.counts(["ResourceOut", None, "ResourceOut"]),
            {"None": 1, "ResourceOut": 2},
        )


if __name__ == "__main__":
    unittest.main()
