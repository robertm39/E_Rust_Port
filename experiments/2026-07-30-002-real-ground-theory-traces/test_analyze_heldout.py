#!/usr/bin/env python3
"""Focused tests for held-out gate aggregation."""

from __future__ import annotations

import unittest

import analyze_heldout


class HeldoutAnalysisTests(unittest.TestCase):
    def test_control_flow_ignores_only_theory_telemetry(self) -> None:
        baseline = [
            {
                "node": 7,
                "outcome": "open_leaf",
                "assignment": [{"atom": 1, "value": True}],
                "theory_query": "occurrence_7",
            }
        ]
        candidate = [
            {
                "node": 7,
                "outcome": "open_leaf",
                "assignment": [{"atom": 1, "value": True}],
                "theory_query": "unique_2",
                "theory_cache_hit": True,
                "theory_status": "sat",
            }
        ]
        self.assertEqual(
            analyze_heldout.control_flow_events(baseline),
            analyze_heldout.control_flow_events(candidate),
        )

    def test_control_flow_keeps_search_outcomes(self) -> None:
        baseline = [{"node": 1, "outcome": "open_leaf"}]
        candidate = [{"node": 1, "outcome": "theory_pruned"}]
        self.assertNotEqual(
            analyze_heldout.control_flow_events(baseline),
            analyze_heldout.control_flow_events(candidate),
        )


if __name__ == "__main__":
    unittest.main()
