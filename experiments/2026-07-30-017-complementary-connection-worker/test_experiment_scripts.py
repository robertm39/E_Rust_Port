#!/usr/bin/env python3
"""Contract and decision tests for the connection-worker experiment."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import analyze
import connection_common as common
import run_experiment


HERE = Path(__file__).resolve().parent


def phase_stub(
    phase: str,
    *,
    unique: list[str] | None = None,
    losses: list[str] | None = None,
    common_count: int = 0,
    rule_ratio: float | None = None,
    wall_ratio: float | None = None,
    valid: bool = True,
) -> dict[str, object]:
    return {
        "phase": phase,
        "correctness_gates_passed": valid,
        "connection_unique_vs_goal": unique or [],
        "connection_losses_vs_goal": losses or [],
        "common_connection_goal_cost": {
            "common_problem_count": common_count,
            "rule_node_ratio": rule_ratio,
            "median_wall_ratio": wall_ratio,
        },
        "independent_portfolio": {"adds_over_goal": unique or []},
    }


class ExperimentScriptTests(unittest.TestCase):
    def test_corpus_is_frozen_and_family_disjoint(self) -> None:
        header, records = common.load_corpus(HERE / "corpus.jsonl")
        self.assertEqual(header["problem_count"], 12)
        self.assertEqual(
            {
                split: sum(record["experiment_split"] == split for record in records)
                for split in common.REPETITIONS
            },
            {"train": 4, "validation": 4, "test": 4},
        )
        for record in records:
            self.assertEqual(
                set(record["includes"]),
                set(record.get("include_sha256", {})),
            )

    def test_unique_test_solve_advances_without_loss(self) -> None:
        decision = analyze.final_decision(
            phase_stub("validation"),
            phase_stub("test", unique=["new"]),
        )
        self.assertEqual(decision["verdict"], "advance-native-prototype")

    def test_test_loss_is_a_veto(self) -> None:
        decision = analyze.final_decision(
            phase_stub("validation", unique=["signal"]),
            phase_stub("test", unique=["new"], losses=["lost"]),
        )
        self.assertEqual(decision["verdict"], "stop")

    def test_cost_gate_requires_two_common_solves(self) -> None:
        accepted = analyze.final_decision(
            phase_stub("validation"),
            phase_stub(
                "test",
                common_count=2,
                rule_ratio=0.5,
                wall_ratio=1.5,
            ),
        )
        rejected = analyze.final_decision(
            phase_stub("validation"),
            phase_stub(
                "test",
                common_count=1,
                rule_ratio=0.1,
                wall_ratio=0.1,
            ),
        )
        self.assertEqual(accepted["verdict"], "advance-native-prototype")
        self.assertEqual(rejected["verdict"], "stop")

    def test_resume_rejects_changed_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary)
            artifact = run_dir / "proof.txt"
            artifact.write_text("proof\n", encoding="utf-8")
            result_path = run_dir / "result.json"
            result_path.write_text(
                json.dumps(
                    {
                        "contract_id": "contract",
                        "artifact_hashes": run_experiment.artifact_hashes(run_dir),
                    }
                ),
                encoding="utf-8",
            )
            self.assertIsNotNone(
                run_experiment.resumable(result_path, "contract")
            )
            artifact.write_text("changed\n", encoding="utf-8")
            self.assertIsNone(
                run_experiment.resumable(result_path, "contract")
            )


if __name__ == "__main__":
    unittest.main()

