#!/usr/bin/env python3
"""Focused tests for the SAT-subsumption experiment contract."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]


def load(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ANALYZE = load("sat_subsumption_analyze", EXPERIMENT_ROOT / "analyze.py")
CAPTURE = load("sat_subsumption_capture", EXPERIMENT_ROOT / "capture.py")
ORACLE = load("sat_subsumption_oracle", EXPERIMENT_ROOT / "oracle.py")
RUNNER = load(
    "sat_subsumption_run_experiment",
    EXPERIMENT_ROOT / "run_experiment.py",
)


class ExperimentContractTests(unittest.TestCase):
    def test_capture_patch_applies_cleanly(self) -> None:
        completed = subprocess.run(
            [
                "git",
                "apply",
                "--check",
                "--ignore-space-change",
                "--ignore-whitespace",
                str(EXPERIMENT_ROOT / "capture.patch"),
            ],
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(
            completed.returncode,
            0,
            completed.stderr.decode("utf-8", errors="replace"),
        )

    def test_instrumented_diff_does_not_require_git_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output_path = Path(directory) / "instrumented.diff"
            RUNNER.write_instrumented_source_diff(
                EXPERIMENT_ROOT, REPO_ROOT, output_path
            )
            output = output_path.read_text(encoding="utf-8")
        self.assertIn("diff --git a/src/clauses/mod.rs", output)
        self.assertGreaterEqual(output.count("diff --git "), 3)
        self.assertIn("sat_subsumption.rs", output)
        self.assertIn("new file mode", output)

    def test_manifest_selection_has_frozen_family_separation(self) -> None:
        _, records = CAPTURE.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        selections = {
            phase: CAPTURE.SELECTION.select_records(
                records, config["split"], config["count"]
            )
            for phase, config in CAPTURE.PHASES.items()
        }
        self.assertEqual(
            {phase: len(rows) for phase, rows in selections.items()},
            {"calibration": 24, "validation": 24, "test": 20},
        )
        family_splits: dict[str, set[str]] = {}
        for phase, rows in selections.items():
            for row in rows:
                family_splits.setdefault(row["family"], set()).add(phase)
        self.assertTrue(
            all(len(phases) == 1 for phases in family_splits.values()),
            family_splits,
        )

    def test_independent_oracle_covers_both_rules(self) -> None:
        rng = __import__("random").Random(0x5A75_5B5)
        outcomes = [
            ORACLE.validate_case(*ORACLE.generated_pair(rng, index))
            for index in range(400)
        ]
        self.assertTrue(any(subsumption for subsumption, _ in outcomes))
        self.assertTrue(any(resolution for _, resolution in outcomes))
        self.assertTrue(any(not subsumption for subsumption, _ in outcomes))
        self.assertTrue(any(not resolution for _, resolution in outcomes))

    def test_oracle_corruption_is_rejected(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(EXPERIMENT_ROOT / "oracle.py"),
                "--cases",
                "1",
                "--corrupt-expected",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn(
            "expected_subsumption",
            completed.stderr.decode("utf-8", errors="replace"),
        )

    def test_calibration_policy_requires_broad_material_win(self) -> None:
        records = []
        for index in range(240):
            problem = f"P{index % 6}"
            records.append(
                {
                    "problem": problem,
                    "family": f"F{index % 6}",
                    "category": "FEQ",
                    "digest": f"{index:016x}",
                    "baseline": index % 13 == 0,
                    "ordinary": index % 13 == 0,
                    "resolution": index % 11 == 0,
                    "baseline_ns": 1_000,
                    "sat_ns": 500,
                    "match_ns": 200,
                    "ordinary_solve_ns": 300,
                    "resolution_solve_ns": 350,
                    "side_literals": 4,
                    "main_literals": 8,
                    "positive_choices": 16,
                    "negative_choices": 8,
                    "estimated_bytes": 12_000,
                }
            )
        selection = ANALYZE.make_selection(records, "contract")
        self.assertEqual(selection["decision"], "advance")
        self.assertIsNotNone(selection["selected_policy"])
        body = {
            key: value
            for key, value in selection.items()
            if key != "selection_id"
        }
        self.assertEqual(
            selection["selection_id"],
            __import__("hashlib").sha256(
                ANALYZE.canonical_json(body)
            ).hexdigest(),
        )

    def test_summary_rejects_semantic_disagreement(self) -> None:
        record = {
            "problem": "P0",
            "family": "F0",
            "category": "FEQ",
            "digest": "0" * 16,
            "ordinal": 1,
            "baseline": True,
            "ordinary": False,
            "resolution": False,
            "baseline_ns": 100,
            "sat_ns": 100,
            "match_ns": 50,
            "resolution_solve_ns": 50,
            "side_literals": 2,
            "main_literals": 2,
            "positive_choices": 1,
            "negative_choices": 0,
            "estimated_bytes": 128,
        }
        with self.assertRaises(ANALYZE.AnalysisError):
            ANALYZE.summarize_records([record])


if __name__ == "__main__":
    unittest.main()
