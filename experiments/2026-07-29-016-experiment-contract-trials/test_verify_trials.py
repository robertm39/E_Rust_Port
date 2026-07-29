"""Unit tests for archive-backed experiment-contract trials."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from types import ModuleType


EXPERIMENT_ROOT = Path(__file__).resolve().parent


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


VERIFY = load_module(
    "experiment_contract_verify_trials",
    EXPERIMENT_ROOT / "verify_trials.py",
)


def result(
    *,
    problem: str,
    strategy: str,
    repetition: int,
    status: str,
    cpu: float | None,
) -> dict[str, object]:
    return {
        "phase": "trial",
        "budget": "heldout",
        "problem_id": problem,
        "repetition": repetition,
        "strategy": strategy,
        "szs_status": status,
        "_telemetry": (
            None
            if cpu is None
            else {"resources": {"total_cpu_seconds": cpu}}
        ),
    }


class VariationTests(unittest.TestCase):
    def test_relative_range_uses_the_coordinate_median(self) -> None:
        self.assertAlmostEqual(
            VERIFY.relative_range([2.0, 4.0]),
            2.0 / 3.0,
        )

    def test_exact_replay_has_zero_variation(self) -> None:
        self.assertEqual(VERIFY.relative_range([5.0, 5.0]), 0.0)


class EvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = VERIFY.TrialSpec(
            record_name="unused.json",
            archive_path="unused.tar.gz",
            summary_path="unused-summary.json",
            phase="trial",
            budget="heldout",
            baseline="baseline",
            candidate="candidate",
            source_decision="stop",
            expected_outcome="stop",
        )
        self.results = [
            result(
                problem="A",
                strategy="baseline",
                repetition=1,
                status="Theorem",
                cpu=2.0,
            ),
            result(
                problem="A",
                strategy="baseline",
                repetition=2,
                status="Theorem",
                cpu=4.0,
            ),
            result(
                problem="A",
                strategy="candidate",
                repetition=1,
                status="Theorem",
                cpu=1.0,
            ),
            result(
                problem="A",
                strategy="candidate",
                repetition=2,
                status="Theorem",
                cpu=2.0,
            ),
            result(
                problem="B",
                strategy="baseline",
                repetition=1,
                status="ResourceOut",
                cpu=None,
            ),
            result(
                problem="B",
                strategy="baseline",
                repetition=2,
                status="ResourceOut",
                cpu=None,
            ),
            result(
                problem="B",
                strategy="candidate",
                repetition=1,
                status="Theorem",
                cpu=1.0,
            ),
            result(
                problem="B",
                strategy="candidate",
                repetition=2,
                status="Theorem",
                cpu=1.0,
            ),
        ]

    def test_status_audit_keeps_exact_and_polarity_counts_separate(self) -> None:
        audit = VERIFY.audit_status_pairs(
            self.results,
            "baseline",
            "candidate",
        )

        self.assertEqual(audit["paired_coordinates"], 4)
        self.assertEqual(audit["exact_matches"], 2)
        self.assertEqual(audit["polarity_disagreements"], 0)

    def test_common_solve_evidence_reports_unique_solves_and_noise(self) -> None:
        evidence = VERIFY.common_solve_evidence(
            self.results,
            self.spec,
        )

        self.assertEqual(evidence["common_ids"], ["A"])
        self.assertEqual(
            evidence["coverage"],
            {
                "baseline_reproducible_solves": 1,
                "candidate_reproducible_solves": 2,
                "common_reproducible_solves": 1,
                "candidate_only": ["B"],
                "baseline_only": [],
            },
        )
        observation = evidence["observation"]
        self.assertEqual(observation["paired_coordinates"], 2)
        self.assertEqual(
            observation["candidate_over_baseline_median"],
            0.5,
        )
        self.assertEqual(
            observation["noise"][
                "paired_ratio_max_relative_range"
            ],
            0.0,
        )


if __name__ == "__main__":
    unittest.main()
