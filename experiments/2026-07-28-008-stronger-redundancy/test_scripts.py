#!/usr/bin/env python3
"""Unit tests for the stronger-redundancy experiment scripts."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module("stronger_redundancy_test_run", EXPERIMENT_ROOT / "run.py")
SELECT = load_module(
    "stronger_redundancy_test_select", EXPERIMENT_ROOT / "select.py"
)
ANALYZE = load_module(
    "stronger_redundancy_test_analyze", EXPERIMENT_ROOT / "analyze.py"
)
PROOF_ADAPTER = load_module(
    "stronger_redundancy_test_proof_adapter",
    EXPERIMENT_ROOT / "proof_adapter.py",
)


class CorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        _, cls.records = RUN.BASE.load_manifest(
            REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )

    def test_split_quotas_are_exact_and_deterministic(self) -> None:
        for split, target in (
            ("train", 24),
            ("validation", 24),
            ("test", 20),
        ):
            first = RUN.select_records(self.records, split, target)
            second = RUN.select_records(self.records, split, target)
            self.assertEqual(first, second)
            self.assertEqual(len(first), target)
            for category, quota in RUN.SPLIT_QUOTAS[split].items():
                self.assertEqual(
                    sum(r["category"] == category for r in first), quota
                )

    def test_source_families_do_not_cross_splits(self) -> None:
        families = {}
        for split, target in (
            ("train", 24),
            ("validation", 24),
            ("test", 20),
        ):
            families[split] = {
                r["family"]
                for r in RUN.select_records(self.records, split, target)
            }
        self.assertFalse(families["train"] & families["validation"])
        self.assertFalse(families["train"] & families["test"])
        self.assertFalse(families["validation"] & families["test"])


class StatusTests(unittest.TestCase):
    def test_expected_status_polarity_is_strict(self) -> None:
        theorem = {"expected_class": "theorem"}
        satisfiable = {"expected_class": "satisfiable"}
        self.assertTrue(RUN.expected_status_match(theorem, "Theorem"))
        self.assertFalse(
            RUN.expected_status_match(theorem, "CounterSatisfiable")
        )
        self.assertTrue(
            RUN.expected_status_match(satisfiable, "Satisfiable")
        )
        self.assertFalse(
            RUN.expected_status_match(satisfiable, "Unsatisfiable")
        )
        self.assertFalse(RUN.expected_status_match(theorem, "ResourceOut"))


class StrategyTests(unittest.TestCase):
    def selection(self, source_phase: str, selected: list[str]) -> dict:
        body = {
            "schema_version": 1,
            "source_phase": source_phase,
            "source_contract_id": "contract",
            "source_binary_sha256": "binary",
            "budget": source_phase,
            "eligible_strategies": list(RUN.CANDIDATE_NAMES),
            "selected_strategies": selected,
            "ranking": [],
            "rule": "fixture",
        }
        return {
            **body,
            "selection_id": __import__("hashlib").sha256(
                RUN.BASE.canonical_json(body)
            ).hexdigest(),
        }

    def test_test_phase_constructs_exact_direct_twin(self) -> None:
        chosen = "contextual_sr_full"
        selection = self.selection("validation", [chosen])
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "selection.json"
            path.write_bytes(RUN.BASE.canonical_json(selection) + b"\n")
            strategies, loaded, _ = RUN.phase_strategies("test", path)
        self.assertEqual(loaded, selection)
        self.assertEqual(
            strategies["selected_direct"]["args"][:-1],
            strategies[chosen]["args"],
        )
        self.assertEqual(
            strategies["selected_direct"]["args"][-1],
            "--conventional-subsumption",
        )
        self.assertEqual(
            strategies["selected_direct"]["reference_for"], chosen
        )

    def test_calibration_exposes_eight_candidates(self) -> None:
        strategies, selection, selection_hash = RUN.phase_strategies(
            "calibration", None
        )
        self.assertIsNone(selection)
        self.assertIsNone(selection_hash)
        self.assertEqual(
            sum(
                config["kind"] == "redundancy_candidate"
                for config in strategies.values()
            ),
            8,
        )


class DecisionTests(unittest.TestCase):
    def test_unique_solve_path_advances(self) -> None:
        comparison = {"left_only": ["a", "b"], "right_only": []}
        ratios = {
            "median_cpu_ratio": 1.0,
            "median_generated_ratio": 1.0,
            "median_high_water_total_ratio": 1.0,
            "median_maximum_resident_pages_ratio": 1.0,
        }
        result = ANALYZE.decision(
            comparison,
            ratios,
            proof_complete=True,
            reference_disagreements=0,
            behavior_effects=1,
            contradictory_statuses=0,
        )
        self.assertTrue(result["advances"])

    def test_reference_disagreement_blocks_advance(self) -> None:
        comparison = {"left_only": ["a", "b"], "right_only": []}
        ratios = {
            "median_cpu_ratio": 0.5,
            "median_generated_ratio": 0.5,
            "median_high_water_total_ratio": 0.5,
            "median_maximum_resident_pages_ratio": 0.5,
        }
        result = ANALYZE.decision(
            comparison,
            ratios,
            proof_complete=True,
            reference_disagreements=1,
            behavior_effects=4,
            contradictory_statuses=0,
        )
        self.assertFalse(result["advances"])

    def test_efficiency_path_requires_all_thresholds(self) -> None:
        comparison = {"left_only": [], "right_only": []}
        ratios = {
            "median_cpu_ratio": 0.94,
            "median_generated_ratio": 0.89,
            "median_high_water_total_ratio": 0.94,
            "median_maximum_resident_pages_ratio": 1.04,
        }
        self.assertTrue(
            ANALYZE.decision(
                comparison,
                ratios,
                proof_complete=True,
                reference_disagreements=0,
                behavior_effects=2,
                contradictory_statuses=0,
            )["advances"]
        )
        ratios["median_generated_ratio"] = 0.91
        self.assertFalse(
            ANALYZE.decision(
                comparison,
                ratios,
                proof_complete=True,
                reference_disagreements=0,
                behavior_effects=2,
                contradictory_statuses=0,
            )["advances"]
        )


class ProofAdapterTests(unittest.TestCase):
    def test_adds_only_missing_skolem_metadata(self) -> None:
        proof = (
            "fof(a, axiom, ?[X]:p(X), file('x.p',a)).\n"
            "fof(s, plain, p(esk1_0), "
            "inference(skolemize,[status(esa)],"
            "[inference(variable_rename,[status(thm)],[a])])).\n"
        )
        prepared, report = PROOF_ADAPTER.add_skolem_metadata(proof)
        self.assertIn(
            "new_symbols(skolem,[esk1_0])", prepared
        )
        self.assertIn("skolemize(X,esk1_0)", prepared)
        self.assertIn("p(esk1_0)", prepared)
        self.assertEqual(report["changed_statement_count"], 1)
        self.assertTrue(report["logical_formula_fields_unchanged"])

    def test_rejects_skolem_step_without_new_symbol(self) -> None:
        proof = (
            "fof(a, axiom, p(a), file('x.p',a)).\n"
            "fof(s, plain, p(a), "
            "inference(skolemize,[status(esa)],[a])).\n"
        )
        with self.assertRaises(PROOF_ADAPTER.AdapterError):
            PROOF_ADAPTER.add_skolem_metadata(proof)

    def test_flattens_only_audited_compound_skolem_wrappers(self) -> None:
        proof = (
            "fof(a, axiom, ?[X]:p(X), file('x.p',a)).\n"
            "fof(s, plain, p(esk1_0), "
            "inference(distribute,[status(thm)],"
            "[inference(fof_nnf,[status(thm)],"
            "[inference(skolemize,[status(esa)],"
            "[inference(variable_rename,[status(thm)],[a])])])])).\n"
        )
        prepared, report = PROOF_ADAPTER.add_skolem_metadata(proof)
        self.assertIn(
            "inference(skolemize,[status(esa),", prepared
        )
        self.assertIn("fof(s_skolem, plain, p(esk1_0)", prepared)
        self.assertIn(
            "inference(distribute,[status(thm)],[s_skolem])",
            prepared,
        )
        self.assertTrue(
            report["changes"][0]["compound_source_split"]
        )


if __name__ == "__main__":
    unittest.main()
