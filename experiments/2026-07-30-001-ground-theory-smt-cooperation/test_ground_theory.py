"""Solver-free tests for the frozen ground-theory experiment."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

from ground_theory import (
    ProtocolError,
    SolverResult,
    fraction_smt,
    load_corpus,
    numeral_fraction,
    parse_core,
    parse_fraction,
    parse_get_value,
    verify_model,
    verify_result,
    verify_unsat_core,
    write_ffi_protocol,
)


ROOT = Path(__file__).resolve().parent
CORPUS = ROOT / "corpus.json"


def load_builder():
    specification = importlib.util.spec_from_file_location(
        "ground_theory_build_corpus", ROOT / "build_corpus.py"
    )
    assert specification is not None and specification.loader is not None
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


class ExactNumberTests(unittest.TestCase):
    def test_fraction_smt_preserves_sign_and_ratio(self) -> None:
        self.assertEqual(fraction_smt(Fraction(3, 2)), "(/ 3 2)")
        self.assertEqual(fraction_smt(Fraction(-3, 2)), "(- (/ 3 2))")
        self.assertEqual(fraction_smt(Fraction(-4)), "(- 4)")

    def test_z3_numeral_parser_is_exact(self) -> None:
        self.assertEqual(numeral_fraction(["/", "3", "2"]), Fraction(3, 2))
        self.assertEqual(
            numeral_fraction(["-", ["/", "3", "2"]]), Fraction(-3, 2)
        )
        with self.assertRaises(ProtocolError):
            numeral_fraction("1.414?")

    def test_malformed_fraction_fails_closed(self) -> None:
        with self.assertRaises(ProtocolError):
            parse_fraction("1/0")


class EvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = load_corpus(CORPUS)
        self.by_id = {
            workload["id"]: workload for workload in self.corpus["workloads"]
        }

    def test_negative_cycle_core_accepts_and_mutation_rejects(self) -> None:
        workload = self.by_id["train-int-closed"]
        branch = workload["branches"][0]
        constraints = [*workload["base"], *branch["constraints"]]
        core = [constraint["label"] for constraint in constraints]
        self.assertTrue(verify_unsat_core(constraints, core))
        self.assertFalse(verify_unsat_core(constraints, core[1:]))

    def test_exact_sat_model_accepts_and_corruption_rejects(self) -> None:
        workload = self.by_id["train-real-sat"]
        branch = workload["branches"][0]
        constraints = [*workload["base"], *branch["constraints"]]
        zero_model = {variable: "0" for variable in workload["variables"]}
        self.assertTrue(
            verify_model(workload["variables"], constraints, zero_model)
        )
        corrupted = zero_model | {"x0": "100", "x1": "-100"}
        self.assertFalse(
            verify_model(workload["variables"], constraints, corrupted)
        )

    def test_untrusted_fragment_remains_unknown(self) -> None:
        workload = self.by_id["validation-int-unsupported"]
        branch = workload["branches"][0]
        result = SolverResult(
            workload["id"],
            branch["id"],
            "sat",
            1,
            model=(("x0", "0"), ("x1", "0")),
        )
        verification = verify_result(workload, branch, result)
        self.assertEqual(verification.trusted_status, "unknown")
        self.assertFalse(verification.verified)

    def test_core_and_model_parsers_reject_malformed_evidence(self) -> None:
        self.assertEqual(parse_core("(a b)"), ("a", "b"))
        self.assertEqual(
            parse_get_value("((x 0) (y (- (/ 1 2))))"),
            (("x", "0"), ("y", "-1/2")),
        )
        with self.assertRaises(ProtocolError):
            parse_core("(a a)")
        with self.assertRaises(ProtocolError):
            parse_get_value("((x root-obj))")


class CorpusAndProtocolTests(unittest.TestCase):
    def test_frozen_corpus_is_byte_deterministic(self) -> None:
        builder = load_builder()
        self.assertEqual(
            CORPUS.read_bytes(),
            builder.canonical_bytes(builder.build_corpus()),
        )

    def test_corpus_has_every_preregistered_partition_and_cohort(self) -> None:
        corpus = load_corpus(CORPUS)
        partitions = {workload["partition"] for workload in corpus["workloads"]}
        cohorts = {workload["cohort"] for workload in corpus["workloads"]}
        self.assertEqual(partitions, {"train", "validation", "test"})
        self.assertEqual(
            cohorts,
            {
                "theory_heavy_closed",
                "theory_heavy_mixed",
                "theory_heavy_sat",
                "neutral",
                "unsupported_general_linear",
            },
        )

    def test_ffi_protocol_is_deterministic_and_excludes_no_fields(self) -> None:
        corpus = load_corpus(CORPUS)
        workloads = [
            workload
            for workload in corpus["workloads"]
            if workload["cohort"] != "neutral"
        ]
        with tempfile.TemporaryDirectory() as temporary:
            left = Path(temporary) / "left.txt"
            right = Path(temporary) / "right.txt"
            write_ffi_protocol(left, workloads)
            write_ffi_protocol(right, workloads)
            self.assertEqual(left.read_bytes(), right.read_bytes())
            text = left.read_text(encoding="utf-8")
            self.assertIn("WORKLOAD\ttrain-int-closed\tInt", text)
            self.assertIn("BASE\tbase_0\t(<= (- x0 x1) 0)", text)
            self.assertTrue(text.endswith("END\n"))


if __name__ == "__main__":
    unittest.main()
