#!/usr/bin/env python3
"""Unit tests for the lemma/watchlist experiment controllers."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]
sys.path.insert(0, str(ROOT))


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


COMMON = load_module("lemma_watchlist_common_test", ROOT / "common.py")
PREPARE = load_module("lemma_watchlist_prepare_test", ROOT / "prepare.py")
RUN = load_module("lemma_watchlist_run_test", ROOT / "run.py")
ANALYZE = load_module("lemma_watchlist_analyze_test", ROOT / "analyze.py")


class TptpParsingTests(unittest.TestCase):
    def test_records_preserve_nested_and_quoted_periods(self) -> None:
        text = (
            "% line comment\n"
            "include('Axioms/example.ax').\n"
            "/* block.comment */\n"
            "cnf(c1,lemma,(p(f(a,b))|~q('x.y')),"
            "inference(foo,[],[1,2])).\n"
        )
        records = COMMON.split_tptp_records(text)
        self.assertEqual(len(records), 2)
        parsed = COMMON.annotated_record(records[1])
        self.assertEqual(parsed["kind"], "cnf")
        self.assertEqual(parsed["role"], "lemma")
        self.assertEqual(parsed["body"], "(p(f(a,b))|~q('x.y'))")

    def test_axiom_only_target_drops_all_goal_roles(self) -> None:
        text = (
            "include('Axioms/a.ax').\n"
            "fof(a,axiom,p(a)).\n"
            "fof(c,conjecture,q(a)).\n"
            "cnf(n,negated_conjecture,~q(a)).\n"
            "fof(t,type,p:$i>$o).\n"
        )
        rendered = COMMON.axiom_only_target(text)
        self.assertIn("include('Axioms/a.ax').", rendered)
        self.assertIn("fof(a,axiom,p(a)).", rendered)
        self.assertIn("fof(t,type,p:$i>$o).", rendered)
        self.assertNotIn("conjecture", rendered)

    def test_empty_clause_normalization(self) -> None:
        self.assertTrue(COMMON.is_empty_clause("(( $false ))"))
        self.assertTrue(COMMON.is_empty_clause("[]"))
        self.assertFalse(COMMON.is_empty_clause("(p(a))"))


class CandidatePreparationTests(unittest.TestCase):
    def candidate(
        self,
        candidate_id: str,
        category: str,
        body: str,
        source: str,
    ) -> dict[str, object]:
        return {
            "candidate_id": candidate_id,
            "source_problem": source,
            "source_category": category,
            "source_family": source[:3],
            "source_trace_sha256": "a" * 64,
            "selected_index": 0,
            "kind": "cnf",
            "body": body,
        }

    def test_same_and_cross_pools_are_category_partitioned_and_deduplicated(
        self,
    ) -> None:
        candidates = [
            self.candidate("a", "FNE", "p(X)", "MGT001+1"),
            self.candidate("b", "FNE", "p(X)", "MGT002+1"),
            self.candidate("c", "FEQ", "q(X)", "SWW001+1"),
        ]
        same = PREPARE.pool_for_target(
            candidates, target_category="FNE", mode="same"
        )
        cross = PREPARE.pool_for_target(
            candidates, target_category="FNE", mode="cross"
        )
        self.assertEqual(len(same), 1)
        self.assertEqual(same[0]["source_category"], "FNE")
        self.assertEqual(len(cross), 1)
        self.assertEqual(cross[0]["source_category"], "FEQ")

    def test_wrappers_separate_guidance_from_logical_lemmas(self) -> None:
        target = {
            "problem_id": "ABC001+1",
            "path": "problems/casc_2025/FNE/ABC001+1.p",
        }
        candidates = [
            self.candidate("abcdef0123456789", "FNE", "(p(X)|q(X))", "MGT001+1")
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            watch = root / "watch.p"
            lemma = root / "lemma.p"
            PREPARE.write_wrapper(
                watch,
                target=target,
                mode="same",
                mechanism="watch",
                candidates=candidates,
            )
            PREPARE.write_wrapper(
                lemma,
                target=target,
                mode="same",
                mechanism="lemma",
                candidates=candidates,
            )
            watch_text = watch.read_text(encoding="utf-8")
            lemma_text = lemma.read_text(encoding="utf-8")
        self.assertIn("include('FNE/ABC001+1.p').", watch_text)
        self.assertIn(",watchlist,(p(X)|q(X))).", watch_text)
        self.assertIn("watchlist_nontermination_sentinel", watch_text)
        self.assertIn(",lemma,(p(X)|q(X))).", lemma_text)
        self.assertNotIn("watchlist", lemma_text)

    def test_source_revision_and_frozen_treatments(self) -> None:
        self.assertEqual(
            PREPARE.SOURCE_REVISION,
            "ce75ea3b68c34ab1640e0f362438a656626a5b0e",
        )
        self.assertEqual(set(RUN.STRATEGIES), set(ANALYZE.STRATEGIES))
        self.assertEqual(RUN.PHASE_CONFIGS["test"]["repetitions"], 2)


class AnalysisTests(unittest.TestCase):
    @staticmethod
    def result(
        strategy: str,
        *,
        problem: str = "P",
        repetition: int = 1,
        cpu: float = 1.0,
        steps: int = 10,
        status: str = "Theorem",
        admissibility: float = 0.0,
    ) -> dict[str, object]:
        return {
            "problem_id": problem,
            "strategy": strategy,
            "repetition": repetition,
            "szs_status": status,
            "admissibility_cpu_seconds": admissibility,
            "_proof_steps": steps,
            "_telemetry": {
                "resources": {
                    "total_cpu_seconds": cpu,
                    "maximum_resident_pages": 100,
                },
                "search_funnel": {"generated": 20, "processed": 10},
            },
        }

    def test_explicit_net_cpu_amortizes_target_validation(self) -> None:
        results = [
            self.result("control", repetition=1, cpu=1.0),
            self.result("control", repetition=2, cpu=1.0),
            self.result(
                "lemma_same",
                repetition=1,
                cpu=0.5,
                admissibility=1.0,
            ),
            self.result(
                "lemma_same",
                repetition=2,
                cpu=0.5,
                admissibility=1.0,
            ),
        ]
        paired = ANALYZE.paired_ratios(results, "lemma_same")
        self.assertEqual(paired["median_cpu_ratio"], 0.5)
        self.assertEqual(paired["median_net_cpu_ratio"], 1.0)
        self.assertEqual(paired["median_proof_step_ratio"], 1.0)

    def test_decision_stops_on_lost_reproducible_solve(self) -> None:
        summary = {
            "strategies": {
                "watch_same": {"unique_target_guidance_clauses": 4}
            },
            "comparisons": {
                "watch_same": {
                    "control_only_reproducible_solves": ["P"],
                    "treatment_only_reproducible_solves": [],
                    "paired": {
                        "common_solved_repetition_coordinates": 4,
                        "median_cpu_ratio": 0.5,
                        "median_net_cpu_ratio": 0.5,
                        "median_proof_step_ratio": 0.5,
                    },
                }
            },
        }
        decision = ANALYZE.decide(
            treatment="watch_same",
            test_summary=summary,
            correctness_ok=True,
            replay_verified=True,
        )
        self.assertEqual(decision["verdict"], "stop")
        self.assertEqual(
            decision["reason"], "reproducible_control_solve_lost"
        )

    def test_zero_explicit_clauses_have_no_value(self) -> None:
        summary = {
            "strategies": {
                "lemma_cross": {"unique_target_added_clauses": 0}
            },
            "comparisons": {
                "lemma_cross": {
                    "control_only_reproducible_solves": [],
                    "treatment_only_reproducible_solves": [],
                    "paired": {
                        "common_solved_repetition_coordinates": 0,
                        "median_cpu_ratio": None,
                        "median_net_cpu_ratio": None,
                        "median_proof_step_ratio": None,
                    },
                }
            },
        }
        decision = ANALYZE.decide(
            treatment="lemma_cross",
            test_summary=summary,
            correctness_ok=True,
            replay_verified=True,
        )
        self.assertEqual(decision["verdict"], "stop_no_value")


class CorpusTests(unittest.TestCase):
    def test_reused_corpus_hash_is_frozen(self) -> None:
        path = (
            REPO_ROOT
            / "experiments"
            / "2026-07-29-018-tsm-learning-baseline"
            / "corpus.jsonl"
        )
        self.assertEqual(
            COMMON.sha256_file(path),
            PREPARE.CORPUS_SHA256,
        )
        rows = [
            json.loads(line)
            for line in path.read_text(encoding="utf-8").splitlines()
            if line
        ]
        families = {
            split: {
                record["family"]
                for record in rows[1:]
                if record["experiment_split"] == split
            }
            for split in ("train", "validation", "test")
        }
        self.assertFalse(families["train"] & families["validation"])
        self.assertFalse(families["train"] & families["test"])
        self.assertFalse(families["validation"] & families["test"])


if __name__ == "__main__":
    unittest.main()

