#!/usr/bin/env python3
"""Unit tests for the TSM experiment controllers and analyzers."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SELECT = load_module("tsm_select_test", ROOT / "select_corpus.py")
INPUTS = load_module("tsm_inputs_test", ROOT / "make_classifier_inputs.py")
ANALYZE = load_module("tsm_analyze_test", ROOT / "analyze.py")
CLASSIFY = load_module("tsm_classify_test", ROOT / "classify.py")


class SelectionTests(unittest.TestCase):
    def test_tracked_corpus_is_exact_reproduction(self) -> None:
        header, records = SELECT.select(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        tracked = [
            json.loads(line)
            for line in (ROOT / "corpus.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()
            if line
        ]
        self.assertEqual(tracked, [header, *records])
        self.assertEqual(
            tracked[0]["source_manifest"],
            "benchmarks/casc_2025_manifest.jsonl",
        )

    def test_family_sets_are_pairwise_disjoint(self) -> None:
        header, _records = SELECT.select(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        families = {
            split: set(values)
            for split, values in header["selected_families"].items()
        }
        self.assertFalse(families["train"] & families["validation"])
        self.assertFalse(families["train"] & families["test"])
        self.assertFalse(families["validation"] & families["test"])


class AnnotationTests(unittest.TestCase):
    def test_annotation_entries_aggregate_sources_and_proofs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "clausepatterns"
            path.write_text(
                "% Annotated terms:\n"
                "$cnil : 1:(2,1,0,0,0,0,0),"
                "2:(4,3,0,0,0,0,0).\n"
                "$or(a=b,$cnil) : 1:(3,0,0,0,0,0,0).\n",
                encoding="utf-8",
            )
            entries = INPUTS.annotation_entries(path)
        self.assertEqual(entries[0]["sources"], 6.0)
        self.assertEqual(entries[0]["proof_sources"], 4.0)
        self.assertEqual(entries[0]["label"], 1.0)
        self.assertEqual(entries[1]["label"], -1.0)

    def test_classifier_text_has_frozen_section_shape(self) -> None:
        entries = [{"term": "a", "sources": 2.0, "label": 1.0}]
        rendered = INPUTS.classifier_text(entries, entries)
        self.assertEqual(
            rendered,
            "Training:\na : 1:(2.000000,1.0).\n.\n"
            "Test:\na : 1:(2.000000,1.0).\n.\n",
        )

    def test_pcl_label_command_changes_only_output_and_telemetry(self) -> None:
        result = {
            "command": [
                "/tmp/umlaut",
                "--tstp-out",
                "--proof-object=1",
                "--search-telemetry=/tmp/original.json",
                "/tmp/problem.p",
            ]
        }
        rendered = INPUTS.pcl_label_command(
            result, Path("/tmp/classifier.json")
        )
        self.assertEqual(
            rendered,
            [
                "/tmp/umlaut",
                "--pcl-out",
                "--proof-object=1",
                f"--search-telemetry={Path('/tmp/classifier.json')}",
                "/tmp/problem.p",
            ],
        )


class AnalysisTests(unittest.TestCase):
    def test_logistic_calibrator_separates_monotonic_scores(self) -> None:
        scores = [-3.0, -2.0, -1.0, 1.0, 2.0, 3.0]
        labels = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]
        weights = [1.0] * len(scores)
        model = ANALYZE.fit_logistic(scores, labels, weights)
        self.assertLess(
            ANALYZE.calibrated_probability(-2.0, model),
            ANALYZE.calibrated_probability(2.0, model),
        )

    def test_weighted_classifier_metrics_report_both_classes(self) -> None:
        parsed = [(-2.0, -1.0, True), (2.0, 1.0, True)]
        entries = [
            {"sources": 3.0, "label": -1.0},
            {"sources": 1.0, "label": 1.0},
        ]
        model = ANALYZE.fit_logistic(
            [-2.0, 2.0], [-1.0, 1.0], [3.0, 1.0]
        )
        metrics = ANALYZE.weighted_classifier_metrics(
            parsed, entries, model
        )
        self.assertEqual(metrics["weighted_patterns"], 4.0)
        self.assertEqual(metrics["accuracy"], 1.0)
        self.assertEqual(metrics["balanced_accuracy"], 1.0)

    def test_relative_range_uses_median_denominator(self) -> None:
        self.assertAlmostEqual(
            ANALYZE.relative_range([9.0, 10.0, 11.0]), 0.2
        )

    def test_missing_test_labels_produce_uncertain_decision(self) -> None:
        search = {
            "bad_statuses": [],
            "load_failures": [],
            "telemetry_failures": 0,
            "control_only_reproducible_solves": [],
            "learned_only_reproducible_solves": [],
            "one_repeat_only_solves": {"control": [], "learned": []},
            "median_common_solve_cpu_ratio": None,
        }
        reason = "no_successful_repetition_1_control_proofs"
        summary = {
            "classification": {
                "test": ANALYZE.unavailable_classifier_metrics(reason)
            },
            "ranking_cost": {
                "test": ANALYZE.unavailable_ranking_cost(reason)
            },
            "search": {"validation": search, "test": search},
        }

        decision = ANALYZE.decide(summary)

        self.assertEqual(decision["verdict"], "uncertain")
        self.assertFalse(decision["sufficient_classifier_coverage"])
        self.assertFalse(decision["ranking_cost_pass"])


class ClassifierCommandTests(unittest.TestCase):
    def test_classifier_command_uses_frozen_identity_index_name(self) -> None:
        binary = Path("umlaut-tsm-classify")
        input_path = Path("input.tsm")
        command = CLASSIFY.classifier_command(binary, input_path)
        self.assertEqual(
            command,
            [
                str(binary),
                "-l",
                "1",
                "-i",
                "IndexIdentity",
                "-d",
                "100000",
                "-t",
                "Flat",
                str(input_path),
            ],
        )


if __name__ == "__main__":
    unittest.main()
