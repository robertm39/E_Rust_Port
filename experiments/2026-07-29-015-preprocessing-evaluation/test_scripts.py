#!/usr/bin/env python3
"""Controller tests for the preprocessing evaluation."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from types import ModuleType
from typing import Any


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module(
    "preprocessing_run_tests", EXPERIMENT_ROOT / "run.py"
)
ANALYZE = load_module(
    "preprocessing_analyze_tests", EXPERIMENT_ROOT / "analyze.py"
)
VERIFY = load_module(
    "preprocessing_verify_tests", EXPERIMENT_ROOT / "verify.py"
)


def telemetry(
    *,
    cpu: float = 1.0,
    generated: int = 10,
    processed: int = 5,
    high_water: int = 7,
    term_storage: int = 100,
    rss: int = 50,
    bce_removed: int = 0,
    predicate_removed: int = 0,
    predicate_generated: int = 0,
    goal_added: int = 0,
) -> dict[str, Any]:
    return {
        "resources": {
            "total_cpu_seconds": cpu,
            "maximum_resident_pages": rss,
        },
        "search_funnel": {
            "generated": generated,
            "processed": processed,
            "high_water_total": high_water,
        },
        "terms": {"storage_estimate_bytes": term_storage},
        "input_funnel": {
            "transformations": {
                "blocked_clause_elimination": {
                    "removed": bce_removed
                },
                "predicate_elimination": {
                    "removed": predicate_removed,
                    "generated": predicate_generated,
                },
                "goal_definitions": {"added": goal_added},
            }
        },
    }


class RunContractTests(unittest.TestCase):
    def test_frozen_casc_selection_matches_prior_corpus(self) -> None:
        manifest = (
            REPOSITORY_ROOT
            / "benchmarks"
            / "casc_2025_manifest.jsonl"
        )
        _metadata, records = RUN.BASE.load_manifest(manifest)
        selected = RUN.select_records(records, "test", 20)

        self.assertEqual(
            [record["problem_id"] for record in selected],
            [
                entry["problem_id"]
                for entry in RUN.frozen_casc_entries()
            ],
        )

    def test_differential_selection_is_fixed(self) -> None:
        _metadata, records = RUN.BASE.load_manifest(
            RUN.TARGETED_MANIFEST_PATH
        )
        selected = RUN.select_records(
            records, "differential", 3
        )

        self.assertEqual(
            [record["problem_id"] for record in selected],
            [
                "bce-proof",
                "predicate-elimination-proof",
                "goal-definitions-proof",
            ],
        )

    def test_contract_preview_is_content_addressed(self) -> None:
        preview = RUN.contract_preview()
        body = {
            key: value
            for key, value in preview.items()
            if key != "preview_id"
        }

        self.assertEqual(
            preview["preview_id"],
            RUN.hashlib.sha256(
                RUN.BASE.canonical_json(body)
            ).hexdigest(),
        )

    def test_source_revision_requires_full_git_sha(self) -> None:
        with self.assertRaises(RUN.ExperimentError):
            RUN.parse_experiment_inputs(["--source-revision=abc123"])


class AnalysisTests(unittest.TestCase):
    def test_activity_reports_removed_and_generated_totals(self) -> None:
        results = [
            {
                "strategy": "predicate",
                "budget": "heldout",
                "problem_id": "A",
                "_telemetry": telemetry(
                    predicate_removed=2,
                    predicate_generated=1,
                ),
            },
            {
                "strategy": "predicate",
                "budget": "heldout",
                "problem_id": "B",
                "_telemetry": telemetry(),
            },
        ]

        activity = ANALYZE.transformation_activity(
            results, "predicate", "heldout"
        )

        self.assertEqual(activity["active_coordinates"], 1)
        self.assertEqual(activity["active_problem_ids"], ["A"])
        self.assertEqual(activity["removed_total"], 2)
        self.assertEqual(
            activity["generated_or_added_total"], 1
        )

    def test_paired_ratios_use_candidate_over_baseline(self) -> None:
        results = [
            {
                "strategy": "baseline",
                "budget": "heldout",
                "problem_id": "A",
                "repetition": 1,
                "_telemetry": telemetry(cpu=2.0),
            },
            {
                "strategy": "bce",
                "budget": "heldout",
                "problem_id": "A",
                "repetition": 1,
                "_telemetry": telemetry(cpu=1.0),
            },
        ]

        ratios = ANALYZE.paired_ratios(
            results, "heldout", "bce"
        )

        self.assertEqual(ratios["paired_coordinates"], 1)
        self.assertEqual(ratios["median_cpu_ratio"], 0.5)
        self.assertEqual(
            ratios["median_generated_ratio"], 1.0
        )

    def test_decision_requires_reach_before_followup(self) -> None:
        report = {
            "coverage_comparison": {
                "left_only": [],
                "right_only": [],
            },
            "common_solved_ratios": {
                "median_cpu_ratio": 0.90,
                "median_generated_ratio": 1.0,
                "median_high_water_total_ratio": 1.0,
            },
            "transformation_activity": {
                "active_coordinates": 3
            },
            "maximum_rss": {
                "candidate_over_baseline": 1.0
            },
        }

        decision = ANALYZE.candidate_decision(
            report, correctness=True
        )

        self.assertEqual(
            decision["result"], "retain_explicit_default_off"
        )
        self.assertFalse(decision["enough_reach"])


class VerificationTests(unittest.TestCase):
    def test_differential_claims_cover_every_strategy_problem(self) -> None:
        contract = {
            "strategies": {
                "baseline": {},
                "bce": {},
                "predicate": {},
                "goal_defs": {},
            },
            "repetitions": 2,
        }
        results = []
        for strategy in contract["strategies"]:
            for problem_id in (
                "bce-proof",
                "predicate-elimination-proof",
                "goal-definitions-proof",
            ):
                for repetition in (1, 2):
                    results.append(
                        {
                            "strategy": strategy,
                            "budget": "differential",
                            "problem_id": problem_id,
                            "category": "SYNTHETIC",
                            "repetition": repetition,
                            "szs_status": "Unsatisfiable",
                            "expected_status_match": True,
                        }
                    )

        claims = VERIFY.representative_claims(
            "differential", contract, results
        )

        self.assertEqual(len(claims), 12)

    def test_candidate_validity_accepts_any_verified_active_witness(
        self,
    ) -> None:
        cases = [
            {
                "phase": "casc",
                "strategy": "bce",
                "problem_id": "heldout-proof",
                "transformation_active": True,
                "gate_returncode": 0,
                "gate_verdict": "verified",
            },
            {
                "phase": "differential",
                "strategy": "bce",
                "problem_id": "bce-proof",
                "transformation_active": True,
                "gate_returncode": 2,
                "gate_verdict": "coverage_gap",
            },
        ]

        validity = VERIFY.candidate_validity(cases)

        self.assertTrue(validity["bce"])
        self.assertFalse(validity["predicate"])
        self.assertFalse(validity["goal_defs"])
        self.assertNotEqual(
            cases[1]["gate_verdict"], "verified"
        )


if __name__ == "__main__":
    unittest.main()
