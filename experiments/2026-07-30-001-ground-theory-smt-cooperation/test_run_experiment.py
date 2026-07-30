"""Regression tests for experiment-level analysis and trust gates."""

from __future__ import annotations

import unittest
from pathlib import Path

from ground_theory import SolverResult, load_corpus
from run_experiment import analyze_backend


ROOT = Path(__file__).resolve().parent


class AnalysisGateTests(unittest.TestCase):
    def test_unverified_eligible_unsat_is_a_visible_gate_failure(self) -> None:
        corpus = load_corpus(ROOT / "corpus.json")
        workload = next(
            item for item in corpus["workloads"] if item["id"] == "train-int-closed"
        )
        branch = workload["branches"][0]
        result = SolverResult(
            workload_id=workload["id"],
            branch_id=branch["id"],
            raw_status="unsat",
            elapsed_ns=1,
            core=(),
        )
        analysis, verified = analyze_backend("broken", [result], corpus)
        self.assertEqual(verified, [])
        self.assertEqual(analysis["eligible_raw_decisions"], 1)
        self.assertEqual(len(analysis["unverified_eligible_decisions"]), 1)
        self.assertEqual(analysis["python_verified_over_eligible_raw"], 0.0)


if __name__ == "__main__":
    unittest.main()
