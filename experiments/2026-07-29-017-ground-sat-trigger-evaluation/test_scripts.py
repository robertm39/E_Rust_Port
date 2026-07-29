#!/usr/bin/env python3
"""Tests for the ground-SAT trigger experiment utilities."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parents[2]
EXPERIMENT = Path(__file__).resolve().parent


def load(name: str, filename: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, EXPERIMENT / filename)
    if spec is None or spec.loader is None:
        raise RuntimeError(filename)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SELECT = load("ground_sat_select_test", "select_corpus.py")
PACK = load("ground_sat_pack_test", "pack_corpus.py")
RUN = load("ground_sat_run_test", "run.py")
REUSE = load("ground_sat_reuse_test", "analyze_reuse.py")


class SelectionTests(unittest.TestCase):
    def test_tracked_corpus_reproduces_exactly(self) -> None:
        header, records = SELECT.select(
            ROOT, ROOT / "benchmarks/casc_2025_manifest.jsonl"
        )
        expected = [
            json.loads(line)
            for line in (EXPERIMENT / "corpus.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()
        ]
        self.assertEqual([header, *records], expected)

    def test_corpus_is_balanced_and_prior_family_disjoint(self) -> None:
        records = [
            json.loads(line)
            for line in (EXPERIMENT / "corpus.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()[1:]
        ]
        excluded, _hashes = SELECT.excluded_families(ROOT)
        families = {record["family"] for record in records}
        self.assertTrue(families.isdisjoint(excluded))
        self.assertEqual(len(families), SELECT.FAMILY_COUNT)
        for family in families:
            self.assertEqual(
                sum(record["family"] == family for record in records),
                SELECT.PER_FAMILY,
            )


class PackingTests(unittest.TestCase):
    def test_safe_relative_rejects_parent_traversal(self) -> None:
        with self.assertRaises(ValueError):
            PACK.safe_relative("../outside")

    def test_archive_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary) / "repository"
            problem = repository / "problems/casc_2025/FEQ/example.p"
            axiom = repository / "problems/casc_2025/Axioms/example.ax"
            problem.parent.mkdir(parents=True)
            axiom.parent.mkdir(parents=True)
            problem.write_text(
                "include('Axioms/example.ax').\nfof(goal,conjecture,p).\n",
                encoding="utf-8",
            )
            axiom.write_text("fof(a,axiom,p).\n", encoding="utf-8")
            selection = repository / "selection.jsonl"
            selection.write_text(
                json.dumps(
                    {
                        "record_type": "problem",
                        "path": "problems/casc_2025/FEQ/example.p",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            sources = PACK.collect(selection, repository)
            first = Path(temporary) / "first.tar.gz"
            second = Path(temporary) / "second.tar.gz"
            PACK.write_archive(first, repository, sources)
            PACK.write_archive(second, repository, sources)
            self.assertEqual(first.read_bytes(), second.read_bytes())


class ContractTests(unittest.TestCase):
    def test_every_candidate_has_one_trigger(self) -> None:
        expected = {
            "off": "--satcheck=NoGrounding",
            "step5000": "--satcheck-proc-interval=5000",
            "step10000": "--satcheck-proc-interval=10000",
            "size10000": "--satcheck-gen-interval=10000",
        }
        self.assertEqual(set(RUN.STRATEGIES), set(expected))
        for name, required in expected.items():
            arguments = RUN.STRATEGIES[name]["args"]
            self.assertIn(required, arguments)
            interval_count = sum(
                argument.startswith(
                    (
                        "--satcheck-proc-interval=",
                        "--satcheck-gen-interval=",
                        "--satcheck-ttinsert-interval=",
                    )
                )
                for argument in arguments
            )
            self.assertEqual(interval_count, int(name != "off"))

    def test_selection_rejects_missing_problem(self) -> None:
        records = [
            json.loads(line)
            for line in (EXPERIMENT / "corpus.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()[1:]
        ]
        with self.assertRaises(RUN.ExperimentError):
            RUN.select_records(
                records[:-1],
                "fresh-family-heldout",
                24,
            )


class ReuseTests(unittest.TestCase):
    def test_clause_parser_normalizes_order_and_counts_duplicates(self) -> None:
        parsed = REUSE.parse_clause_multiset(
            b"p isat 3\na -2 1 0\na 1 -2 0\nq 1 0\n"
        )
        self.assertEqual(parsed, {(1, -2): 2})

    def test_percentile_interpolates(self) -> None:
        self.assertEqual(REUSE.percentile([0.0, 1.0], 0.5), 0.5)


if __name__ == "__main__":
    unittest.main()
