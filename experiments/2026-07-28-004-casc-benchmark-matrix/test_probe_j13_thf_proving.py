#!/usr/bin/env python3
"""Focused tests for hash-bound J13 THF proving-probe selection."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("probe_j13_thf_proving.py")
SPEC = importlib.util.spec_from_file_location("probe_j13_thf_proving", SCRIPT)
if SPEC is None or SPEC.loader is None:  # pragma: no cover
    raise RuntimeError(f"cannot load {SCRIPT}")
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class ProbeSelectionTests(unittest.TestCase):
    SELECTION = {
        "results": [
            {"problem_id": "accepted-one", "classification": "accepted"},
            {"problem_id": "error-one", "classification": "error"},
            {"problem_id": "timeout-one", "classification": "timeout"},
            {"problem_id": "error-two", "classification": "error"},
        ]
    }

    def test_selects_requested_classifications_in_audit_order(self) -> None:
        selected = PROBE.select_problem_ids(
            self.SELECTION,
            {"timeout", "error"},
            3,
        )

        self.assertEqual(selected, ["error-one", "timeout-one", "error-two"])

    def test_rejects_exact_count_mismatch(self) -> None:
        with self.assertRaisesRegex(ValueError, "selection count mismatch: 2 != 1"):
            PROBE.select_problem_ids(self.SELECTION, {"error"}, 1)

    def test_rejects_duplicate_selected_identifiers(self) -> None:
        selection = {
            "results": [
                {"problem_id": "same", "classification": "error"},
                {"problem_id": "same", "classification": "error"},
            ]
        }

        with self.assertRaisesRegex(ValueError, "duplicate selected identifiers"):
            PROBE.select_problem_ids(selection, {"error"}, 2)

    def test_rejects_empty_selected_classification(self) -> None:
        with self.assertRaisesRegex(ValueError, "no too_many_arguments records"):
            PROBE.select_problem_ids(
                self.SELECTION,
                {"too_many_arguments"},
                None,
            )


if __name__ == "__main__":
    unittest.main()
