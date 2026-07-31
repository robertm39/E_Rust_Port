#!/usr/bin/env python3
"""Unit tests for the TSM ranking profile controller."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("profile.py")
SPEC = importlib.util.spec_from_file_location("tsm_ranking_profile", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
PROFILE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROFILE)


class ProfileTests(unittest.TestCase):
    def test_make_empty_test_preserves_training_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "full.tsm"
            target = root / "empty.tsm"
            source.write_text(
                "Training:\na : 1:(1.0,1.0).\n.\n"
                "Test:\nb : 1:(1.0,-1.0).\n.\n",
                encoding="utf-8",
            )

            PROFILE.make_empty_test(source, target)

            self.assertEqual(
                target.read_text(encoding="utf-8"),
                "Training:\na : 1:(1.0,1.0).\n.\nTest:\n.\n",
            )

    def test_telemetry_signature_ignores_resources(self) -> None:
        record = {
            "outcome": {"reason": "processed_limit"},
            "input_funnel": {"parsed_axioms": 1},
            "search_funnel": {"processed": 128},
            "inferences": {"paramodulations": 7},
            "simplification": {"rewrite_steps": 9},
            "proof": {"search_given_clauses": 128},
            "resources": {"total_cpu_seconds": 1.0},
        }

        signature = PROFILE.telemetry_signature(record)

        self.assertNotIn("resources", signature)
        self.assertEqual(signature["search_funnel"]["processed"], 128)


if __name__ == "__main__":
    unittest.main()
