#!/usr/bin/env python3
"""Focused tests for source selection and capture guards."""

from __future__ import annotations

import unittest

import select_sources


class SelectionTests(unittest.TestCase):
    def test_heldout_requires_explicit_gate(self) -> None:
        with self.assertRaisesRegex(
            select_sources.SelectionError,
            "requires the explicit",
        ):
            select_sources.main(["--partition", "validation"])

    def test_train_selection_is_family_bounded_and_stable(self) -> None:
        result = select_sources.select(
            select_sources.REPO_ROOT,
            select_sources.DEFAULT_MANIFEST,
            "train",
        )
        self.assertEqual(result["schema"], "umlaut-real-ground-source-selection-v1")
        self.assertEqual(result["source_count"], 26)
        self.assertEqual(
            result["families"],
            {"DAT": 5, "HWV": 5, "ITP": 5, "SWC": 5, "SWW": 5, "SYO": 1},
        )
        for family, count in result["families"].items():
            self.assertLessEqual(count, select_sources.MAX_PER_FAMILY, family)


if __name__ == "__main__":
    unittest.main()
