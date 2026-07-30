#!/usr/bin/env python3
"""Focused tests for backend query-corpus preparation."""

from __future__ import annotations

import unittest

import prepare_query_corpus


class QueryPreparationTests(unittest.TestCase):
    def test_protocol_id_preserves_safe_ids(self) -> None:
        self.assertEqual(
            prepare_query_corpus.protocol_id("NUM861_1_q_00001"),
            "NUM861_1_q_00001",
        )

    def test_protocol_id_encodes_dotted_tptp_ids(self) -> None:
        self.assertEqual(
            prepare_query_corpus.protocol_id("ANA143_1.002.016_q_00001"),
            "ANA143_1_002_016_q_00001",
        )

    def test_protocol_id_prefixes_leading_digit(self) -> None:
        self.assertEqual(prepare_query_corpus.protocol_id("1.q"), "q_1_q")


if __name__ == "__main__":
    unittest.main()
