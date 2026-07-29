#!/usr/bin/env python3
"""Focused contracts for the frontend profile controller."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "frontend_profile", ROOT / "frontend_profile.py"
)
assert SPEC is not None and SPEC.loader is not None
PROFILE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROFILE)


class GeneratorTests(unittest.TestCase):
    def test_each_dialect_has_expected_shape(self) -> None:
        self.assertTrue(PROFILE.record_text("cnf", 3).startswith("cnf(c3,"))
        self.assertIn("![X]", PROFILE.record_text("fof", 3))
        self.assertIn("X:$i", PROFILE.record_text("tff", 3))
        self.assertIn("@X", PROFILE.record_text("thf", 3))

    def test_symbol_vocabulary_is_bounded(self) -> None:
        self.assertIn("p0(", PROFILE.record_text("fof", 257))
        self.assertIn("p0(", PROFILE.record_text("fof", 514))

    def test_generate_records_hashes_and_rejects_mutation(self) -> None:
        old_sizes = PROFILE.SIZES
        PROFILE.SIZES = (2,)
        try:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                manifest = PROFILE.generate_corpus(root)
                self.assertEqual(len(manifest["files"]), 4)
                PROFILE.load_manifest(root)
                path = PROFILE.corpus_path(root, "cnf", 2)
                path.write_text("changed", encoding="utf-8")
                with self.assertRaises(PROFILE.ExperimentError):
                    PROFILE.load_manifest(root)
        finally:
            PROFILE.SIZES = old_sizes


class ParsingTests(unittest.TestCase):
    def test_time_record(self) -> None:
        record = PROFILE.parse_time_record("1.25\t1.00\t0.20\t12345\t0\n")
        self.assertEqual(record["wall_seconds"], 1.25)
        self.assertEqual(record["max_rss_kib"], 12345)
        self.assertEqual(record["exit_code"], 0)

    def test_time_record_rejects_extra_rows(self) -> None:
        with self.assertRaises(PROFILE.ExperimentError):
            PROFILE.parse_time_record("1\t1\t0\t1\t0\n2\t2\t0\t1\t0\n")

    def test_dhat_extracts_frozen_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "dhat.json"
            path.write_text(
                json.dumps(
                    {
                        "pps": [
                            {
                                "tb": 10,
                                "tbk": 2,
                                "gb": 8,
                                "gbk": 1,
                                "eb": 3,
                                "ebk": 1,
                            },
                            {
                                "tb": 5,
                                "tbk": 1,
                                "gb": 4,
                                "gbk": 1,
                                "eb": 0,
                                "ebk": 0,
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            result = PROFILE.parse_dhat(path)
            self.assertEqual(result["total_bytes"], 15)
            self.assertEqual(result["peak_live_bytes"], 12)
            self.assertEqual(result["end_live_bytes"], 3)

    def test_percentage_improvement(self) -> None:
        self.assertEqual(PROFILE.percentage_improvement(10.0, 8.0), 20.0)
        with self.assertRaises(PROFILE.ExperimentError):
            PROFILE.percentage_improvement(0.0, 0.0)


def sample(
    implementation: str,
    dialect: str,
    size: int,
    mode: str,
    wall: float,
    repetition: int,
) -> dict[str, object]:
    return {
        "implementation": implementation,
        "dialect": dialect,
        "records": size,
        "mode": mode,
        "repetition": repetition,
        "wall_seconds": wall,
        "user_seconds": wall,
        "system_seconds": 0.0,
        "max_rss_kib": 100,
        "exit_code": 0,
    }


class AnalysisTests(unittest.TestCase):
    def make_records(self) -> list[dict[str, object]]:
        records = []
        for implementation in ("rust", "c"):
            records.append(
                sample(implementation, "all", 0, "startup", 0.01, 0)
            )
            for dialect in PROFILE.DIALECTS:
                for size in PROFILE.SIZES:
                    records.extend(
                        [
                            sample(
                                implementation,
                                dialect,
                                size,
                                "syntax",
                                0.21,
                                0,
                            ),
                            sample(
                                implementation,
                                dialect,
                                size,
                                "cnf_no_preprocessing",
                                0.31,
                                0,
                            ),
                            sample(
                                implementation,
                                dialect,
                                size,
                                "cnf",
                                0.36,
                                0,
                            ),
                        ]
                    )
        return records

    def test_analysis_selects_parse_and_holdout(self) -> None:
        report = PROFILE.analyze_records(
            self.make_records(), expected_repetitions=1
        )
        self.assertTrue(report["selection"]["timing_gate_passed"])
        self.assertEqual(report["selection"]["selected_phase"], "parse")
        self.assertEqual(report["selection"]["profile_records"], 10_000)
        self.assertEqual(report["selection"]["callgrind_mode"], "syntax")

    def test_analysis_rejects_incomplete_groups(self) -> None:
        records = self.make_records()
        records.append(
            sample("rust", "cnf", 1_000, "syntax", 0.21, 1)
        )
        with self.assertRaises(PROFILE.ExperimentError):
            PROFILE.analyze_records(records, expected_repetitions=1)


if __name__ == "__main__":
    unittest.main()
