"""Regression tests for the higher-order experiment controllers."""

from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module("higher_order_gap_test_run", ROOT / "run.py")
AUDIT = load_module("higher_order_gap_test_audit", ROOT / "audit.py")
ADAPTER = load_module(
    "higher_order_gap_test_norgler_adapter",
    ROOT / "norgler_adapter.py",
)
HOLDOUT = load_module(
    "higher_order_gap_test_holdout",
    ROOT / "holdout.py",
)


class HigherOrderExperimentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with (REPO / "benchmarks/casc_2025_manifest.jsonl").open(
            encoding="utf-8"
        ) as stream:
            rows = [json.loads(line) for line in stream if line.strip()]
        cls.records = rows[1:]

    def test_thf_samples_obey_frozen_split_and_category_quotas(self) -> None:
        targets = {"train": 45, "validation": 27, "test": 30}
        for split, target in targets.items():
            selected = RUN.select_records(self.records, split, target)
            self.assertEqual(len(selected), target)
            self.assertTrue(
                all(record["holdout_split"] == split for record in selected)
            )
            counts = {
                category: sum(
                    record["category"] == category for record in selected
                )
                for category in RUN.THF_CATEGORIES
            }
            self.assertEqual(counts, RUN.THF_SPLIT_QUOTAS[split])
        train = {
            record["family"]
            for record in RUN.select_records(self.records, "train", 45)
        }
        validation = {
            record["family"]
            for record in RUN.select_records(
                self.records, "validation", 27
            )
        }
        test = {
            record["family"]
            for record in RUN.select_records(self.records, "test", 30)
        }
        self.assertFalse(train & validation)
        self.assertFalse(train & test)
        self.assertFalse(validation & test)

    def test_fof_control_has_equal_category_quotas(self) -> None:
        selected = RUN.select_records(self.records, "test", 18)
        self.assertEqual(
            {
                category: sum(
                    record["category"] == category for record in selected
                )
                for category in RUN.FOF_CONTROL_QUOTAS
            },
            RUN.FOF_CONTROL_QUOTAS,
        )

    def test_positive_extensionality_candidate_is_independently_gated(
        self,
    ) -> None:
        args = RUN.STRATEGIES["pos_ext_all"]["args"]
        self.assertIn("--pos-ext=all", args)
        self.assertIn("--neg-ext=off", args)
        self.assertEqual(
            RUN.STRATEGIES["baseline_auto"]["args"], ["--auto"]
        )
        strategies, selection, selection_hash = HOLDOUT.phase_strategies(
            "pos_ext_holdout", None
        )
        self.assertEqual(
            tuple(strategies), ("baseline_auto", "pos_ext_all")
        )
        self.assertIsNone(selection)
        self.assertIsNone(selection_hash)

    def test_taxonomy_distinguishes_parser_and_reference_only_failures(
        self,
    ) -> None:
        base = {
            "return_code": 0,
            "external_timeout": False,
            "stdout": b"",
            "szs_status": None,
        }
        syntax_ok = {
            **base,
            "stdout": b"% Parsing successful!\n% SZS status Unknown\n",
            "szs_status": "Unknown",
        }
        syntax_bad = {**base, "return_code": 1}
        vampire_solved = {**base, "szs_status": "Theorem"}
        self.assertEqual(
            AUDIT.classify(syntax_bad, base, base),
            "syntax_or_typing_rejection",
        )
        self.assertEqual(
            AUDIT.classify(syntax_ok, base, vampire_solved),
            "search_limited_reference_solved",
        )

    def test_norgler_adapter_uses_structurally_matching_source(self) -> None:
        rewritten, mapping = ADAPTER.source_rewrite(
            "![X1:person]:((((f @ X1)=(g @ X1))|(p)))",
            "![X:person]: (((f @ X) = (g @ X)) | p)",
        )
        self.assertEqual(mapping, {"X1": "X"})
        self.assertIn("f @ X", rewritten)
        self.assertNotIn("X1", rewritten)

    def test_norgler_adapter_rejects_non_variable_change(self) -> None:
        with self.assertRaises(ADAPTER.AdapterError):
            ADAPTER.source_rewrite(
                "![X1:person]:((f @ X1)=(g @ X1))",
                "![X:person]:((f @ X)!=(g @ X))",
            )


if __name__ == "__main__":
    unittest.main()
