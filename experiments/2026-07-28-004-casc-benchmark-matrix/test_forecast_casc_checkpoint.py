#!/usr/bin/env python3
"""Focused tests for checkpoint throughput forecasting."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("forecast_casc_checkpoint.py")
SPEC = importlib.util.spec_from_file_location("forecast_casc_checkpoint", SCRIPT)
if SPEC is None or SPEC.loader is None:  # pragma: no cover
    raise RuntimeError(f"cannot load {SCRIPT}")
FORECAST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FORECAST)


class ForecastTests(unittest.TestCase):
    def test_reports_first_missing_and_stationary_projection(self) -> None:
        records = [
            {
                "problem_id": "ONE",
                "category": "FOO",
                "limit_kind": "wall",
                "limit_seconds": 10,
            },
            {
                "problem_id": "TWO",
                "category": "BAR",
                "limit_kind": "cpu",
                "limit_seconds": 20,
            },
        ]
        results = {
            ("umlaut", "ONE"): {
                "solver": "umlaut",
                "completed_at": "2026-01-01T00:00:01Z",
                "classification": "solved",
                "wall_seconds": 2.0,
                "cpu_seconds": 8.0,
            },
            ("vampire", "ONE"): {
                "solver": "vampire",
                "completed_at": "2026-01-01T00:00:02Z",
                "classification": "timeout",
                "wall_seconds": 4.0,
                "cpu_seconds": 16.0,
            },
        }
        run = {
            "contract_id": "a" * 64,
            "_contract": {"solvers": {"umlaut": {}, "vampire": {}}},
            "_records": records,
            "_results": results,
        }
        value = FORECAST.build_forecast(
            run, session_seconds=12, recent_window=100
        )
        self.assertEqual(value["remaining_results"], 2)
        self.assertEqual(value["first_missing"]["problem_id"], "TWO")
        self.assertEqual(value["first_missing"]["solver"], "umlaut")
        self.assertEqual(
            value["stationary_projection"]["projected_new_results"], 2
        )
        self.assertEqual(value["remaining_timeout_upper_bound_seconds"], 40)
        self.assertEqual(
            value["recent_window"]["solvers"]["umlaut"]["mean_cpu_cores"],
            4.0,
        )


if __name__ == "__main__":
    unittest.main()
