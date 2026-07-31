#!/usr/bin/env python3
"""Unit tests for Callgrind parsing."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("analyze_profiles.py")
SPEC = importlib.util.spec_from_file_location("tsm_profile_analysis", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
ANALYZE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ANALYZE)


class CallgrindParserTests(unittest.TestCase):
    def test_separates_self_cost_from_incoming_call_cost(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.out"
            profile.write_text(
                "events: Ir\n"
                "summary: 100\n"
                "fn=(1) caller\n"
                "1 10\n"
                "cfn=(2) callee\n"
                "calls=3 1\n"
                "* * 70\n"
                "+1 20\n"
                "fn=(2)\n"
                "1 70\n"
                "totals: 100\n",
                encoding="utf-8",
            )

            parsed = ANALYZE.parse_callgrind(profile)

            self_costs = {
                item["function"]: item["instructions"]
                for item in parsed["top_self"]
            }
            inclusive = {
                item["function"]: item["instructions"]
                for item in parsed["top_inclusive"]
            }
            self.assertEqual(self_costs, {"caller": 30, "callee": 70})
            self.assertEqual(inclusive, {"callee": 70})


if __name__ == "__main__":
    unittest.main()
