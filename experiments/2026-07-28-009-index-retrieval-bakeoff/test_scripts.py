"""Unit tests for the fingerprint-index experiment controllers."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parent
CALIBRATION_HARNESS_SHA256 = (
    "46d83097ef9a1f23ff1fd2401d7a9606d186379ea1824ea9427f14e71c53c3e1"
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module("index_retrieval_test_run", ROOT / "run.py")
SELECT = load_module("index_retrieval_test_select", ROOT / "select.py")
ANALYZE = load_module("index_retrieval_test_analyze", ROOT / "analyze.py")


class ExperimentScriptTests(unittest.TestCase):
    def test_calibration_harness_is_immutably_preserved(self) -> None:
        digest = hashlib.sha256(
            (ROOT / "run_calibration.py").read_bytes()
        ).hexdigest()
        self.assertEqual(digest, CALIBRATION_HARNESS_SHA256)

    def test_validation_phase_keeps_fp7_and_selected_candidates(self) -> None:
        RUN.configure_base()
        body = {
            "schema_version": 1,
            "source_phase": "calibration",
            "source_contract_id": "contract",
            "source_binary_sha256": "binary",
            "budget": "calibration",
            "eligible_strategies": list(RUN.CANDIDATE_NAMES),
            "selected_strategies": ["fp3d", "npdt", "fp2"],
            "ranking": [],
            "rule": "test",
        }
        selection = {
            **body,
            "selection_id": hashlib.sha256(
                RUN.BASE.BASE.canonical_json(body)
            ).hexdigest(),
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "selection.json"
            path.write_text(
                json.dumps(selection, sort_keys=True), encoding="utf-8"
            )
            strategies, loaded, digest = RUN.phase_strategies(
                "validation", path
            )
        self.assertEqual(
            list(strategies),
            ["baseline_fp7", "fp3d", "npdt", "fp2"],
        )
        self.assertEqual(loaded, selection)
        self.assertIsNotNone(digest)

    def test_selection_prefers_reproducible_coverage(self) -> None:
        faster = {
            "reproducible_solved": 4,
            "median_solved_cpu_seconds": 0.5,
            "median_solved_generated": 10,
            "median_solved_high_water_total": 10,
        }
        more_coverage = {
            **faster,
            "reproducible_solved": 5,
            "median_solved_cpu_seconds": 2.0,
        }
        self.assertLess(
            SELECT.selection_key(more_coverage, "coverage"),
            SELECT.selection_key(faster, "fast"),
        )

    def test_default_change_requires_preregistered_gain(self) -> None:
        coverage = {"left_only": [], "right_only": []}
        ratios = {
            "median_selected_over_baseline": {
                "cpu": 0.96,
                "generated": 1.0,
                "high_water": 1.0,
                "rss": 1.0,
            }
        }
        retained = ANALYZE.decision(coverage, ratios, True, [])
        self.assertEqual(retained["result"], "retain_fp7_default")

        ratios["median_selected_over_baseline"]["cpu"] = 0.94
        adopted = ANALYZE.decision(coverage, ratios, True, [])
        self.assertEqual(
            adopted["result"], "adopt_selected_index_default"
        )


if __name__ == "__main__":
    unittest.main()
