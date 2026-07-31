#!/usr/bin/env python3
"""Unit and small integration tests for the offline neural study."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from neural_common import (
    HIDDEN_DIM,
    IntegrityError,
    ManifestRecord,
    Observation,
    RecursiveEncoder,
    RecursiveModel,
    canonical_clause,
    evaluate_scores,
    extract_trace,
    load_model,
    parse_clause,
    save_model,
    scalar_features,
    score_observations,
    train_recursive,
)
from run_study import test_gate, validation_gate


def observation(index: int, clause: str, label: int) -> Observation:
    literals = parse_clause(clause)
    return Observation(
        problem="SYNTH",
        family="SYNTH",
        split="train",
        index=index,
        raw_clause=clause,
        literals=literals,
        canonical=canonical_clause(literals),
        label=label,
    )


def metric_bundle(ap: float, top10: float, prefix: float) -> dict[str, object]:
    return {
        "macro": {
            "average_precision": ap,
            "roc_auc": ap,
            "pairwise_accuracy": ap,
            "top_1_percent_recall": top10,
            "top_5_percent_recall": top10,
            "top_10_percent_recall": top10,
            "top_20_percent_recall": top10,
            "all_positive_prefix_fraction": prefix,
        },
        "problems": {},
    }


class ParserTests(unittest.TestCase):
    def test_cnf_and_proof_syntax_have_same_canonical_form(self) -> None:
        cnf = parse_clause("(~p(X)|f(X)=a)")
        proof = parse_clause("[--p(Y),++equal(f(Y),a)]", proof_syntax=True)
        self.assertEqual(canonical_clause(cnf), canonical_clause(proof))
        self.assertEqual(
            scalar_features(cnf),
            [2.0, 1.0, 1.0, 6.0, 3.0, 2.0, 1.0, 4.0, 4.0, 1.0],
        )

    def test_trace_extraction_and_manifest_counts(self) -> None:
        trace = """% Initializing proof state
%cnf(i_0_1, plain, (p(X)|~q(a))).
%cnf(i_0_2, plain, (r(b))).
% SZS output start CNFRefutation
  0 : : [++p(Y),--q(a)] : evalgc(1)
  1 : : [] : sr(0, 0) : 'proof'
% SZS output end CNFRefutation
"""
        record = ManifestRecord(
            archive_member="trace.pcl",
            family="SYNTH",
            problem="SYNTH",
            split="train",
            sha256="unused",
            given_count=2,
            positive_count=1,
            proof_evalgc_count=1,
            unmatched_evalgc_count=0,
        )
        rows, summary = extract_trace(trace, record)
        self.assertEqual([row.label for row in rows], [1, 0])
        self.assertEqual(summary["positive_count"], 1)
        bad_record = ManifestRecord(**{**record.__dict__, "positive_count": 2})
        with self.assertRaises(IntegrityError):
            extract_trace(trace, bad_record)

    def test_recursive_encoder_is_seeded_and_deterministic(self) -> None:
        clause = parse_clause("(p(f(X),a)|~q(X))")
        first = RecursiveEncoder(11).clause(clause)
        self.assertEqual(first, RecursiveEncoder(11).clause(clause))
        self.assertNotEqual(first, RecursiveEncoder(23).clause(clause))
        self.assertEqual(len(first), 24)


class ModelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.rows = [
            observation(0, "(goal(a))", 1),
            observation(1, "(goal(f(X)))", 1),
            observation(2, "(goal(X)|support(X))", 1),
            observation(3, "(~goal(X)|answer(X))", 1),
            observation(4, "(noise(a,b,c))", 0),
            observation(5, "(noise(f(a),g(b)))", 0),
            observation(6, "(other(X)|other(Y)|other(Z))", 0),
            observation(7, "(~irrelevant(a))", 0),
        ]

    def test_recursive_training_and_serialization_are_exact(self) -> None:
        first = train_recursive(self.rows, 11)
        second = train_recursive(self.rows, 11)
        self.assertEqual(first.to_dict(), second.to_dict())
        self.assertEqual(len(first.weights1), HIDDEN_DIM)
        scores = score_observations(first, self.rows)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "model.json"
            size = save_model(first, path)
            self.assertEqual(size, path.stat().st_size)
            loaded = load_model(path)
            self.assertIsInstance(loaded, RecursiveModel)
            self.assertEqual(scores, score_observations(loaded, self.rows))

    def test_metrics_use_chronological_tie_break_and_macro(self) -> None:
        rows = [
            observation(0, "(p(a))", 1),
            observation(1, "(q(a))", 0),
            observation(2, "(r(a))", 1),
            observation(3, "(s(a))", 0),
        ]
        metrics = evaluate_scores(rows, [4.0, 3.0, 2.0, 1.0])
        self.assertAlmostEqual(metrics["macro"]["average_precision"], 5.0 / 6.0)
        self.assertAlmostEqual(metrics["macro"]["roc_auc"], 0.75)
        self.assertEqual(
            metrics["macro"]["all_positive_prefix_fraction"], 0.75
        )

    def test_external_worker_round_trip(self) -> None:
        model = train_recursive(self.rows, 11)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "model.json"
            save_model(model, path)
            worker = Path(__file__).with_name("inference_worker.py")
            process = subprocess.run(
                [sys.executable, str(worker), "--model", str(path)],
                input=json.dumps({"clauses": ["(goal(a))", "(noise(a))"]}) + "\n",
                text=True,
                encoding="utf-8",
                capture_output=True,
                check=True,
            )
            response = json.loads(process.stdout)
            self.assertEqual(len(response["scores"]), 2)


class GateTests(unittest.TestCase):
    def test_validation_gate_is_conjunctive(self) -> None:
        linear = metric_bundle(0.40, 0.30, 0.90)
        candidates = [
            {"seed": seed, "metrics": metric_bundle(0.45, 0.40, 0.70)}
            for seed in (11, 23, 37, 53, 71)
        ]
        checks = validation_gate(
            linear,
            candidates,
            candidates[2],
            {"microseconds_per_clause": 10.0, "repeat_exact": True},
            {"microseconds_per_clause": 20.0, "repeat_exact": True},
            1000,
            10_000_000,
        )
        self.assertTrue(all(checks.values()))
        candidates[0] = {
            "seed": 11,
            "metrics": metric_bundle(0.10, 0.10, 1.0),
        }
        failed = validation_gate(
            linear,
            candidates,
            candidates[2],
            {"microseconds_per_clause": 10.0, "repeat_exact": True},
            {"microseconds_per_clause": 20.0, "repeat_exact": True},
            1000,
            10_000_000,
        )
        self.assertFalse(failed["ap_seed_range"])

    def test_test_gate_uses_frozen_selected_seed(self) -> None:
        linear = metric_bundle(0.40, 0.30, 0.90)
        candidates = [
            {"seed": seed, "metrics": metric_bundle(0.45, 0.40, 0.70)}
            for seed in (11, 23, 37, 53, 71)
        ]
        self.assertTrue(all(test_gate(linear, candidates, 37).values()))
        candidates[2] = {
            "seed": 37,
            "metrics": metric_bundle(0.41, 0.40, 0.70),
        }
        self.assertFalse(test_gate(linear, candidates, 37)["ap_effect"])


if __name__ == "__main__":
    unittest.main()
