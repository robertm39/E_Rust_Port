#!/usr/bin/env python3
"""Focused tests for production extraction and control rendering."""

from __future__ import annotations

import unittest
import hashlib
import json
import tempfile
from pathlib import Path

from extract_production import load_parser, parse_transcript
from render_controls import render_smt, render_tptp
from select_production import select


class ExtractionTests(unittest.TestCase):
    def test_exact_rat_real_extraction_and_exclusions(self) -> None:
        transcript = """
        tff(a_type,type,a:$real).
        tcf(c0,plain,![X:$rat]:($greater($sum(X,1/2),0))).
        tcf(c1,plain,![Y:$real]:($lesseq($product(2,Y),3))).
        tcf(c2,plain,![Z:$real]:(p(a)|$greater(Z,0))).
        tcf(c3,plain,![Z:$real]:(p(Z)|$greater(Z,0))).
        tcf(c4,plain,![Z:$real]:($greater(f(Z),0))).
        tcf(c5,plain,![Z:$real]:(Z=0)).
        """
        clauses, exclusions, raw_count = parse_transcript(
            transcript,
            load_parser(),
        )
        self.assertEqual(raw_count, 6)
        self.assertEqual(len(clauses), 3)
        self.assertEqual(exclusions["nonground_opaque_literal"], 1)
        self.assertEqual(exclusions["uninterpreted_arithmetic_term"], 1)
        self.assertEqual(exclusions["equality_or_disequality"], 1)
        first = clauses[0]["literals"][0]
        self.assertEqual(first["sort"], "Rat")
        self.assertEqual(first["coefficients"], {"X": "1"})
        self.assertEqual(first["constant"], "1/2")
        self.assertTrue(first["strict"])
        second = clauses[1]["literals"][0]
        self.assertEqual(second["sort"], "Real")
        self.assertEqual(second["coefficients"], {"Y": "-2"})
        self.assertEqual(second["constant"], "3")
        self.assertFalse(second["strict"])
        self.assertEqual(clauses[2]["literals"][0]["kind"], "prop")

    def test_source_selection_caps_each_partition_family_at_five(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            problem_root = repository / "problems"
            problem_root.mkdir()
            records: list[dict[str, object]] = []
            for index in range(6):
                path = problem_root / f"F{index}.p"
                raw = f"tff(t,type,x:$real). % $sum {index}\n".encode()
                path.write_bytes(raw)
                records.append(
                    {
                        "record_type": "problem",
                        "division": "TFA",
                        "holdout_split": "train",
                        "family": "FAM",
                        "path": f"problems/F{index}.p",
                        "problem_id": f"F{index}",
                        "size_bytes": 100 - index,
                        "sha256": hashlib.sha256(raw).hexdigest(),
                        "expected_class": "theorem",
                    }
                )
            other = problem_root / "G0.p"
            other_raw = b"tff(t,type,y:$rat). % $greater\n"
            other.write_bytes(other_raw)
            records.append(
                {
                    "record_type": "problem",
                    "division": "TFA",
                    "holdout_split": "validation",
                    "family": "GAM",
                    "path": "problems/G0.p",
                    "problem_id": "G0",
                    "size_bytes": 1,
                    "sha256": hashlib.sha256(other_raw).hexdigest(),
                    "expected_class": "theorem",
                }
            )
            manifest = repository / "manifest.jsonl"
            manifest.write_text(
                "\n".join(json.dumps(record) for record in records) + "\n",
                encoding="utf-8",
            )
            result = select(manifest, repository)
            family = [
                item for item in result["selected"] if item["family"] == "FAM"
            ]
            self.assertEqual(len(family), 5)
            self.assertEqual(
                [item["problem_id"] for item in family],
                ["F5", "F4", "F3", "F2", "F1"],
            )
            self.assertEqual(
                {
                    (item["partition"], item["family"])
                    for item in result["selected"]
                },
                {("train", "FAM"), ("validation", "GAM")},
            )


class RenderingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.workload = {
            "id": "render",
            "clauses": [
                {
                    "id": "c0",
                    "literals": [
                        {
                            "kind": "prop",
                            "name": "guard",
                            "positive": False,
                        },
                        {
                            "kind": "arith",
                            "sort": "Rat",
                            "strict": True,
                            "coefficients": {"x": "2/3"},
                            "constant": "-1/2",
                        },
                    ],
                }
            ],
        }

    def test_smt_is_universally_quantified_and_exact(self) -> None:
        rendered = render_smt(self.workload)
        self.assertIn("(forall ((x_0_0 Real))", rendered)
        self.assertIn("(* (/ 2 3) x_0_0)", rendered)
        self.assertIn("(- (/ 1 2))", rendered)
        self.assertIn("(not p_", rendered)

    def test_tptp_is_typed_and_exact(self) -> None:
        rendered = render_tptp(self.workload)
        self.assertIn("X_0_0:$rat", rendered)
        self.assertIn("$product(2/3,X_0_0)", rendered)
        self.assertIn("-1/2", rendered)
        self.assertIn("$greater", rendered)


if __name__ == "__main__":
    unittest.main()
