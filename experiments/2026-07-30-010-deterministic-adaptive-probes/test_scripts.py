#!/usr/bin/env python3
"""Focused tests for deterministic adaptive-probe controllers."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import analyze
import common
import prepare_corpus
import run
import select_corpus


def telemetry(
    *, processed: int = 256, generated: int = 128
) -> dict:
    return {
        "schema": "umlaut.search-telemetry",
        "schema_version": 1,
        "record_kind": "final",
        "search_funnel": {
            "processed_non_trivial": processed,
            "generated_non_trivial": generated,
            "high_water_unprocessed": 512,
        },
        "resources": {
            "maximum_resident_pages": 100,
            "total_cpu_seconds": 0.5,
        },
    }


class CommonTests(unittest.TestCase):
    def test_branch_uses_frozen_growth_threshold(self) -> None:
        self.assertEqual(
            common.choose_branch(telemetry(generated=16_383))["branch"],
            "global",
        )
        self.assertEqual(
            common.choose_branch(telemetry(generated=16_384))["branch"],
            "goal",
        )

    def test_branch_falls_back_for_insufficient_processed(self) -> None:
        decision = common.choose_branch(
            telemetry(processed=63, generated=1)
        )
        self.assertEqual(decision["branch"], "goal")
        self.assertEqual(
            decision["fallback_reason"], "insufficient_processed_clauses"
        )

    def test_atomic_checkpoint_has_deterministic_goal_fallback(self) -> None:
        checkpoint = telemetry(processed=0, generated=0)
        checkpoint["record_kind"] = "checkpoint"
        decision = common.choose_branch(checkpoint)
        self.assertEqual(decision["branch"], "goal")
        self.assertEqual(
            decision["fallback_reason"], "insufficient_processed_clauses"
        )
        self.assertTrue(analyze.telemetry_schema_valid(checkpoint))

    def test_unknown_record_kind_is_rejected(self) -> None:
        malformed = telemetry()
        malformed["record_kind"] = "partial"
        decision = common.choose_branch(malformed)
        self.assertEqual(decision["branch"], "goal")
        self.assertEqual(decision["fallback_reason"], "unknown_record_kind")
        self.assertFalse(analyze.telemetry_schema_valid(malformed))

    def test_status_and_processed_statistics_parsing(self) -> None:
        text = (
            "% Processed clauses                    : 256\n"
            "% SZS status ResourceOut\n"
        )
        self.assertEqual(common.final_status(text), "ResourceOut")
        self.assertEqual(common.processed_clause_count(text), 256)
        self.assertTrue(
            common.status_is_acceptable("ResourceOut", "theorem")
        )
        self.assertFalse(
            common.status_is_acceptable("Unsatisfiable", "theorem")
        )


class CorpusTests(unittest.TestCase):
    def test_selector_takes_one_candidate_per_frozen_cell(self) -> None:
        records = []
        for (split, category), bands in (
            select_corpus.BANDS_BY_SPLIT_CATEGORY.items()
        ):
            for index, band in enumerate(bands):
                records.append(
                    {
                        "record_type": "problem",
                        "holdout_split": split,
                        "category": category,
                        "difficulty_band": band,
                        "expected_class": "theorem",
                        "problem_id": (
                            f"{split[:1]}{category}{index:03d}+1"
                        ),
                        "family": f"{split[:1]}{index:02d}",
                        "sha256": f"{index:064x}",
                        "size_bytes": 500,
                    }
                )
        selected = select_corpus.select_records(records, set())
        self.assertEqual(len(selected), 24)
        self.assertEqual(
            {
                (record["experiment_split"], record["category"])
                for record in selected
            },
            {
                (split, category)
                for split in select_corpus.SPLITS
                for category in select_corpus.CATEGORIES
            },
        )

    def test_archive_member_rejects_traversal(self) -> None:
        with self.assertRaises(common.ExperimentError):
            prepare_corpus.safe_member_name("../problem.p")


class RunnerTests(unittest.TestCase):
    def test_elapsed_time_parser(self) -> None:
        self.assertEqual(run.parse_elapsed("1:02.50"), 62.5)
        self.assertEqual(run.parse_elapsed("1:02:03"), 3723.0)

    def test_gnu_time_parser_handles_colons_in_key(self) -> None:
        text = "\n".join(
            (
                "User time (seconds): 1.25",
                "System time (seconds): 0.25",
                "Elapsed (wall clock) time (h:mm:ss or m:ss): 0:02.00",
                "Maximum resident set size (kbytes): 1234",
            )
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "time.txt"
            path.write_text(text, encoding="utf-8")
            parsed = run.parse_timing(path)
        self.assertEqual(
            parsed,
            {
                "user_cpu_seconds": 1.25,
                "system_cpu_seconds": 0.25,
                "total_cpu_seconds": 1.5,
                "wall_seconds": 2.0,
                "peak_rss_kib": 1234,
            },
        )


class AnalysisTests(unittest.TestCase):
    @staticmethod
    def phase_report(
        *, observability: float = 1.0, unique: int = 0
    ) -> dict:
        comparison = {
            "common_solved_repetition_coordinates": 4,
            "median_cpu_ratio": 0.9,
            "baseline_only_reproducible_solves": [],
        }
        return {
            "phase": "validation",
            "analysis_id": "analysis",
            "correctness_failures": [],
            "observability": {"success_rate": observability},
            "overhead": {
                "median_cpu_ratio": 1.0,
                "median_wall_ratio": 1.0,
                "median_peak_rss_ratio": 1.0,
            },
            "adaptive_vs_controls": {
                "static_global_restart": dict(comparison),
                "static_goal": dict(comparison),
            },
            "adaptive_unique_vs_both_controls": [
                f"p{index}" for index in range(unique)
            ],
        }

    def test_final_decision_continues_on_all_gates(self) -> None:
        validation = self.phase_report()
        test = self.phase_report(unique=2)
        test["phase"] = "test"
        decision = analyze.final_decision(validation, test)
        self.assertEqual(decision["verdict"], "continue")
        self.assertEqual(decision["reason"], "all_gates_passed")

    def test_final_decision_stops_below_observability_gate(self) -> None:
        validation = self.phase_report(observability=0.94)
        test = self.phase_report(unique=2)
        test["phase"] = "test"
        decision = analyze.final_decision(validation, test)
        self.assertEqual(decision["verdict"], "stop")
        self.assertEqual(
            decision["reason"], "observability_below_95_percent"
        )

    def test_overhead_detects_processed_mismatch(self) -> None:
        def result(policy: str, processed: int) -> dict:
            return {
                "policy": policy,
                "problem_id": "X",
                "repetition": 1,
                "szs_status": "ResourceOut",
                "resources": {
                    "total_cpu_seconds": 1.0,
                    "wall_seconds": 1.0,
                    "peak_rss_kib": 100,
                },
                "phases": [{"processed_clauses": processed}],
            }

        report, failures = analyze.overhead(
            [
                result("probe_with_telemetry", 256),
                result("probe_without_telemetry", 255),
            ]
        )
        self.assertEqual(len(report["processed_mismatches"]), 1)
        self.assertEqual(len(failures), 1)


if __name__ == "__main__":
    unittest.main()
