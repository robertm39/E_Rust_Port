#!/usr/bin/env python3
"""Unit tests for cooperative multicore experiment controllers."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

import common
import prepare_corpus

if sys.platform == "linux":
    import analyze
    import run_experiment


class CommonTests(unittest.TestCase):
    def test_normalize_tptp_preserves_quoted_whitespace(self) -> None:
        self.assertEqual(
            common.normalize_tptp("( p('a b') | ~q( X ) )"),
            "(p('a b')|~q(X))",
        )

    def test_literal_count_handles_nested_formula_arguments(self) -> None:
        self.assertEqual(
            common.literal_count("(p(f(a,b))|~q(g(X,Y))|X=Y)"), 3
        )
        self.assertEqual(common.literal_count("($false)"), 0)

    def test_parse_saturated_clauses_checks_info_and_novelty(self) -> None:
        text = "\n".join(
            (
                "cnf(i_0_1, axiom, (p(a))). % info(1, 0, 0, 2, 1, 1, 0, 0)",
                "cnf(i_0_2, plain, (p(X)|~q(f(X)))). "
                "% info(2, 3, 5, 6, 2, 2, 2, 1)",
                "cnf(i_0_3, plain, ($false)). % info(3, 4, 8, 1, 0, 0, 0, 0)",
            )
        )
        clauses, errors = common.parse_saturated_clauses(
            text,
            producer=2,
            wave=1,
            original_bodies={"(p(a))"},
        )
        self.assertEqual(errors, [])
        self.assertEqual(len(clauses), 1)
        self.assertEqual(clauses[0]["producer"], 2)
        self.assertEqual(clauses[0]["info"]["proof_depth"], 3)

    def test_parse_saturated_clauses_rejects_malformed_info(self) -> None:
        clauses, errors = common.parse_saturated_clauses(
            "cnf(i_0_1, plain, (p(a)|q(a))). % info(1, 0, 0, 2)",
            producer=0,
            wave=1,
            original_bodies=set(),
        )
        self.assertEqual(clauses, [])
        self.assertEqual(len(errors), 1)

    def test_parse_saturated_clauses_ignores_unannotated_problem_output(self) -> None:
        clauses, errors = common.parse_saturated_clauses(
            "cnf(input_clause, axiom, (p(a))).",
            producer=0,
            wave=1,
            original_bodies=set(),
        )
        self.assertEqual(clauses, [])
        self.assertEqual(errors, [])

    def test_parse_saturated_clauses_rejects_literal_mismatch(self) -> None:
        clauses, errors = common.parse_saturated_clauses(
            "cnf(i_0_1, plain, (p(a)|q(a))). "
            "% info(1, 0, 0, 4, 1, 1, 0, 0)",
            producer=0,
            wave=1,
            original_bodies=set(),
        )
        self.assertEqual(clauses, [])
        self.assertIn("literal count", errors[0])

    def test_rank_peer_clauses_excludes_self_and_deduplicates(self) -> None:
        def clause(body: str, producer: int, symbols: int) -> dict:
            return {
                "body": body,
                "body_normalized": common.normalize_tptp(body),
                "body_sha256": common.sha256_bytes(
                    common.normalize_tptp(body).encode()
                ),
                "info": {
                    "literal_count": 1,
                    "symbol_count": symbols,
                    "proof_depth": 2,
                },
                "producer": producer,
                "wave": 1,
            }

        pools = [
            [clause("(self(a))", 0, 2)],
            [clause("(p(a))", 1, 3)],
            [clause("(p(a))", 2, 3)],
            [clause("(q(a))", 3, 2)],
        ]
        ranked = common.rank_peer_clauses(pools, recipient=0, cap=8)
        self.assertEqual([item["body"] for item in ranked], ["(q(a))", "(p(a))"])
        self.assertEqual(ranked[1]["peer_coverage"], 2)
        self.assertNotIn("(self(a))", [item["body"] for item in ranked])

    def test_render_wrapper_uses_static_watchlist_role(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            original = Path(directory) / "problem.p"
            original.write_text("cnf(a,axiom,p(a)).\n", encoding="utf-8")
            text = common.render_wrapper(
                original,
                [{"body": "(p(a))"}],
                wave=2,
                recipient=1,
            )
        self.assertIn("include('", text)
        self.assertIn("cnf(coop_w2_r1_0, watchlist, (p(a))).", text)

    def test_safe_member_name_rejects_traversal(self) -> None:
        with self.assertRaises(common.ExperimentError):
            prepare_corpus.safe_member_name("../problem.p")


@unittest.skipUnless(sys.platform == "linux", "Linux-only controller imports")
class LinuxControllerTests(unittest.TestCase):
    def test_cnf_bodies_extracts_third_top_level_argument(self) -> None:
        text = (
            "% comment\n"
            "cnf(a, axiom, (p(f(a,b))|q(a))).\n"
            "cnf(b, negated_conjecture, (~p(X))).\n"
        )
        self.assertEqual(
            run_experiment.cnf_bodies(text),
            {"(p(f(a,b))|q(a))", "(~p(X))"},
        )

    def test_worker_seeds_and_guidance_are_explicit(self) -> None:
        control_weight, control = run_experiment.worker_heuristic(1, False)
        watch_weight, watch = run_experiment.worker_heuristic(1, True)
        self.assertIn(",19,23,29)", control_weight)
        self.assertIn("ConstPrio", control_weight)
        self.assertIn("PreferWatchlist", watch_weight)
        self.assertEqual(control, watch)

    def test_final_decision_adopts_smallest_qualifying_cap(self) -> None:
        def phase(unique: bool) -> dict:
            arms = {}
            for arm in run_experiment.ARMS:
                arms[arm] = {
                    "lost_vs": {
                        "independent_equal": [],
                        "independent_unequal": [],
                        "restart_control": [],
                    },
                    "paired_vs": {
                        "independent_equal": {
                            "common_coordinates": 4,
                            "median_cpu_ratio": 0.9,
                            "median_wall_ratio": 0.9,
                            "median_peak_rss_ratio": 1.0,
                        },
                        "restart_control": {
                            "common_coordinates": 4,
                            "median_cpu_ratio": 0.9,
                            "median_wall_ratio": 0.9,
                            "median_peak_rss_ratio": 1.0,
                        }
                    },
                    "unique_vs_all_controls": (
                        ["x"] if unique and arm.startswith("share_") else []
                    ),
                    "median_total_cpu_seconds": 1.0,
                    "reproducible_solves": ["base"],
                }
            return {
                "analysis_id": "analysis",
                "arms": arms,
                "correctness_failures": [],
                "preprocessing": {
                    "total_four_cpu_seconds": 4.0,
                    "total_one_cpu_seconds": 1.0,
                },
                "problem_count": 8,
                "repetitions": 2,
            }

        decision = analyze.final_decision(phase(False), phase(True))
        self.assertEqual(decision["verdict"], "adopt")
        self.assertEqual(decision["selected_arm"], "share_4")

    def test_final_decision_stops_on_correctness_failure(self) -> None:
        empty_arm = {
            "lost_vs": {
                "independent_equal": [],
                "independent_unequal": [],
                "restart_control": [],
            },
            "paired_vs": {
                "independent_equal": {
                    "common_coordinates": 0,
                    "median_cpu_ratio": None,
                    "median_wall_ratio": None,
                    "median_peak_rss_ratio": None,
                },
                "restart_control": {
                    "common_coordinates": 0,
                    "median_cpu_ratio": None,
                    "median_wall_ratio": None,
                    "median_peak_rss_ratio": None,
                }
            },
            "median_total_cpu_seconds": 1.0,
            "reproducible_solves": [],
            "unique_vs_all_controls": [],
        }
        phase = {
            "analysis_id": "x",
            "arms": {
                arm: json.loads(json.dumps(empty_arm))
                for arm in run_experiment.ARMS
            },
            "correctness_failures": ["bad"],
            "preprocessing": {
                "total_four_cpu_seconds": 0.0,
                "total_one_cpu_seconds": 0.0,
            },
            "problem_count": 8,
            "repetitions": 2,
        }
        decision = analyze.final_decision(phase, phase)
        self.assertEqual(decision["verdict"], "stop")
        self.assertEqual(decision["reason"], "correctness_failure")


if __name__ == "__main__":
    unittest.main()
