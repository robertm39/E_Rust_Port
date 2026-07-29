#!/usr/bin/env python3
"""Controller tests for the inference-gap audit."""

from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


AUDIT = load("inference_gap_audit_matrix", ROOT / "audit_matrix.py")
RUN = load("inference_gap_run", ROOT / "run.py")
ANALYZE = load("inference_gap_analyze", ROOT / "analyze.py")


class MatrixTests(unittest.TestCase):
    def test_static_matrix_validates(self) -> None:
        matrix = json.loads(
            (ROOT / "capability-matrix.json").read_text(encoding="utf-8")
        )
        report = AUDIT.validate_matrix(REPO_ROOT, matrix, False)
        self.assertEqual(report["row_count"], 17)
        self.assertEqual(report["status_counts"]["direct"], 11)
        self.assertEqual(report["status_counts"]["library_only"], 1)
        self.assertEqual(report["focused_test_count"], 13)
        self.assertEqual(
            [entry["id"] for entry in report["shortlist"]],
            ["inner_rewriting", "ur_resolution", "term_algebra_rules"],
        )

    def test_contract_preview_is_stable(self) -> None:
        first = RUN.contract_preview()
        second = RUN.contract_preview()
        self.assertEqual(first, second)
        self.assertEqual(len(first["preview_id"]), 64)
        self.assertEqual(
            first["strategies"]["local_rw"]["args"][-1], "--local-rw=true"
        )

    def test_selection_reuses_prior_test_split(self) -> None:
        _, records = RUN.BASE.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        selected = RUN.select_records(records, "test", 20)
        counts = {
            category: sum(row["category"] == category for row in selected)
            for category in RUN.CATEGORIES
        }
        self.assertEqual(counts, RUN.QUOTAS)
        self.assertEqual(len({row["problem_id"] for row in selected}), 20)

    def test_audit_phase_rejects_selection_file(self) -> None:
        with self.assertRaises(RUN.ExperimentError):
            RUN.phase_strategies("audit", ROOT / "unexpected.json")

    def test_analysis_status_polarity_is_explicit(self) -> None:
        self.assertEqual(ANALYZE.status_polarity("Theorem"), "proof")
        self.assertEqual(
            ANALYZE.status_polarity("CounterSatisfiable"), "model"
        )
        self.assertIsNone(ANALYZE.status_polarity("ResourceOut"))

    def test_behavior_effects_skip_missing_telemetry(self) -> None:
        baseline = {
            "problem_id": "P",
            "strategy": "baseline",
            "budget": "short",
            "repetition": 1,
            "_telemetry": {
                "search_funnel": {
                    "generated": 10,
                    "processed": 5,
                    "high_water_total": 8,
                    "final_total": 7,
                },
                "simplification": {"rewrite_steps": 3},
            },
        }
        candidate = {
            **baseline,
            "strategy": "local_rw",
            "_telemetry": None,
        }
        self.assertEqual(
            ANALYZE.behavior_effects([baseline, candidate], "short"), []
        )


if __name__ == "__main__":
    unittest.main()
