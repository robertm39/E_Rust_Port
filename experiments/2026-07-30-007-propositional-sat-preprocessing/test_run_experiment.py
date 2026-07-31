import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("run_experiment.py")
SPEC = importlib.util.spec_from_file_location("prop_sat_preprocessing", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class PropositionalSatPreprocessingTests(unittest.TestCase):
    def test_isat_queries_preserve_assumption_scopes(self):
        maximum, clauses, queries = MODULE.parse_isat(
            b"p isat 3\na 1 -2 0\nq cold -1 0 0\nq assumed 10 0 -3 0\n"
        )
        self.assertEqual(maximum, 3)
        self.assertEqual(clauses, [(1, -2)])
        self.assertEqual(queries[1]["assumptions"], [-3])
        dimacs, normalized = MODULE.dimacs_bytes(
            maximum, clauses + [(literal,) for literal in queries[1]["assumptions"]]
        )
        self.assertEqual(normalized, [(1, -2), (-3,)])
        self.assertIn(b"-3 0\n", dimacs)

    def test_session_integrity_uses_generated_payload_hash(self):
        data = b"p isat 1\nq cold -1 0 0\n"
        session = {
            "bytes": 1,
            "session_sha256": MODULE.sha256_bytes(data),
        }
        self.assertTrue(MODULE.session_payload_valid(data, session))
        session["session_sha256"] = "0" * 64
        self.assertFalse(MODULE.session_payload_valid(data, session))

    def test_probe_json_parser_tolerates_cadical_commentary(self):
        parsed = MODULE.parse_probe_stdout(
            'c parsed input\n{"status":"sat","model":[1]}\n'
        )
        self.assertEqual(parsed, {"status": "sat", "model": [1]})
        with self.assertRaisesRegex(ValueError, "no JSON"):
            MODULE.parse_probe_stdout("")

    def test_drat_verification_marker_is_exact(self):
        accepted = subprocess.CompletedProcess([], 0, "s VERIFIED\n", "")
        rejected = subprocess.CompletedProcess([], 1, "s NOT VERIFIED\n", "")
        self.assertTrue(MODULE.drat_verified(accepted))
        self.assertFalse(MODULE.drat_verified(rejected))

    def test_declared_fragment_accepts_only_zero_arity_cnf(self):
        accepted = MODULE.classify_whole_problem(
            "cnf(a,axiom,(p | ~q)).\ncnf(b,negated_conjecture,q).\n"
        )
        self.assertEqual(accepted["atoms"], {"p": 1, "q": 2})
        self.assertEqual(accepted["clauses"], [(1, -2), (2,)])
        with self.assertRaisesRegex(ValueError, "non-propositional"):
            MODULE.classify_whole_problem("cnf(a,axiom,p(X)).")
        with self.assertRaisesRegex(ValueError, "include"):
            MODULE.classify_whole_problem("include('Axioms/A.ax').")
        with self.assertRaisesRegex(ValueError, "non_cnf_record"):
            MODULE.classify_whole_problem("fof(a,axiom,p).")
        with self.assertRaisesRegex(ValueError, "unterminated_statement"):
            MODULE.classify_whole_problem("cnf(a,axiom,p)")

    def test_boolean_constants_and_tautologies_are_exact(self):
        parsed = MODULE.classify_whole_problem(
            "cnf(a,axiom,$false).\n"
            "cnf(b,axiom,(p | ~p)).\n"
            "cnf(c,axiom,(~$true | q)).\n"
        )
        self.assertEqual(parsed["atoms"], {"q": 1})
        self.assertEqual(parsed["clauses"], [(), (1,)])

    def test_complete_model_validation_rejects_corruption(self):
        clauses = [(1, -2), (2,)]
        self.assertTrue(MODULE.complete_model_valid(2, clauses, [1, 2]))
        self.assertFalse(MODULE.complete_model_valid(2, clauses, [1]))
        self.assertFalse(MODULE.complete_model_valid(2, clauses, [1, -2]))

    def test_dimacs_round_trip_and_exhaustive_oracle(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "formula.cnf"
            data, _clauses = MODULE.dimacs_bytes(1, [(1,), (-1,)])
            path.write_bytes(data)
            variables, clauses = MODULE.parse_dimacs(path)
        self.assertEqual((variables, clauses), (1, [(1,), (-1,)]))
        self.assertEqual(MODULE.exhaustive_status(variables, clauses), "unsat")

    def test_status_reduction_allows_timeouts_but_not_polarity_conflict(self):
        records = [
            {
                "coordinate": "x",
                "arm": "default",
                "status": "unknown",
                "valid": True,
            },
            {
                "coordinate": "x",
                "arm": "default",
                "status": "sat",
                "valid": True,
            },
        ]
        self.assertEqual(MODULE.unique_statuses(records)[("x", "default")], "sat")
        records.append(
            {
                "coordinate": "x",
                "arm": "default",
                "status": "unsat",
                "valid": True,
            }
        )
        with self.assertRaisesRegex(ValueError, "contradictory"):
            MODULE.unique_statuses(records)

    def test_mapping_corruption_is_rejected(self):
        mapping = {
            "atoms": {"p": 1, "q": 2},
            "clauses": [[1], [-2]],
            "source_mappings": [
                {
                    "source_name": "a",
                    "role": "axiom",
                    "dimacs_clause": 1,
                    "tautology": False,
                    "source_literals": [{"atom": "p", "positive": True}],
                },
                {
                    "source_name": "b",
                    "role": "axiom",
                    "dimacs_clause": 2,
                    "tautology": False,
                    "source_literals": [{"atom": "q", "positive": False}],
                },
            ]
        }
        self.assertTrue(MODULE.mapping_roundtrip_valid(mapping))
        mapping["source_mappings"][1]["dimacs_clause"] = 9
        self.assertFalse(MODULE.mapping_roundtrip_valid(mapping))

    def test_decision_requires_all_frozen_gates(self):
        report = {
            "polarity_disagreements": [],
            "proofs": {"required": 2, "attempted": 2, "checked": 2},
            "mapping_roundtrips": {"required": 20, "checked": 20},
            "arm_summaries": {
                "plain": {
                    "model_validation": {
                        "claimed_sat_records": 10,
                        "checked": 10,
                    }
                },
                "default": {
                    "model_validation": {
                        "claimed_sat_records": 10,
                        "checked": 10,
                    }
                },
            },
            "mutation_checks": {"a": True},
            "comparisons": {
                "captured_default_vs_plain": {
                    "added_solves": 0,
                    "lost_solves": 0,
                    "wall_ratio": {"median": 0.8, "p95": 1.0},
                    "rss_ratio": {"maximum": 1.05},
                },
                "whole_default_vs_umlaut": {
                    "added_solves": 2,
                    "lost_solves": 0,
                    "wall_ratio": {"median": 0.9, "p95": 1.0},
                },
            },
            "default_reduction": {
                "at_least_ten_percent": 20,
                "sessions": 100,
            },
            "whole_accepted": 20,
            "whole_accepted_by_family": {"A": 5, "B": 5, "C": 5, "D": 5},
            "recurring_overlap": {
                "pairs": 10,
                "add_only_rate": 0.8,
                "median_simplified_retention": 0.8,
                "stable_identity_available": False,
            },
        }
        decision = MODULE.decide(report)
        self.assertTrue(decision["recommend_default_preprocessing_for_extracted_sat"])
        self.assertTrue(decision["recommend_whole_problem_specialist_followup"])
        self.assertFalse(decision["recommend_cross_call_reuse"])
        report["comparisons"]["captured_default_vs_plain"]["lost_solves"] = 1
        self.assertFalse(
            MODULE.decide(report)[
                "recommend_default_preprocessing_for_extracted_sat"
            ]
        )
        report["comparisons"]["captured_default_vs_plain"]["lost_solves"] = 0
        report["comparisons"]["captured_default_vs_plain"]["rss_ratio"]["maximum"] = 1.11
        self.assertFalse(
            MODULE.decide(report)[
                "recommend_default_preprocessing_for_extracted_sat"
            ]
        )


if __name__ == "__main__":
    unittest.main()
