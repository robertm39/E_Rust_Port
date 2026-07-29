#!/usr/bin/env python3
"""Controller tests for the shared rewrite-cache experiment."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from types import ModuleType
from typing import Any


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_ROOT.parents[1]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


RUN = load_module("rewrite_cache_run_tests", EXPERIMENT_ROOT / "run.py")
ANALYZE = load_module(
    "rewrite_cache_analyze_tests", EXPERIMENT_ROOT / "analyze.py"
)


def telemetry(
    *,
    cpu: float,
    generated: int,
    processed: int,
    high_water: int,
    term_storage: int,
    rss: int,
    rewrite_steps: int,
    uncached: int,
    lookups: int,
    hits: int,
    edges: int,
    date_checks: int,
    date_hits: int,
) -> dict[str, Any]:
    return {
        "resources": {
            "total_cpu_seconds": cpu,
            "maximum_resident_pages": rss,
        },
        "search_funnel": {
            "generated": generated,
            "processed": processed,
            "high_water_total": high_water,
        },
        "terms": {"storage_estimate_bytes": term_storage},
        "simplification": {
            "rewrite_steps": rewrite_steps,
            "rewrite_uncached_links": uncached,
            "rewrite_cache": {
                "link_lookups": lookups,
                "link_hits": hits,
                "link_edges_followed": edges,
                "normal_form_date_checks": date_checks,
                "normal_form_date_hits": date_hits,
            },
        },
    }


class RunContractTests(unittest.TestCase):
    def test_frozen_casc_selection_matches_official_manifest(self) -> None:
        manifest = (
            REPOSITORY_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
        )
        _metadata, records = RUN.BASE.load_manifest(manifest)
        selected = RUN.select_records(records, "test", 20)

        self.assertEqual(
            [record["problem_id"] for record in selected],
            [
                entry["problem_id"]
                for entry in RUN.frozen_casc_entries(
                    RUN.load_frozen_corpus()
                )
            ],
        )

    def test_targeted_selection_matches_frozen_paths(self) -> None:
        manifest = EXPERIMENT_ROOT / "targeted-manifest.jsonl"
        _metadata, records = RUN.BASE.load_manifest(manifest)
        selected = RUN.select_records(records, "rewrite_heavy", 5)

        self.assertEqual(
            [record["path"] for record in selected],
            RUN.load_frozen_corpus()["rewrite_heavy"],
        )

    def test_manifest_hash_arguments_are_frozen(self) -> None:
        RUN.verify_manifest_argument(
            [
                "--phase=casc",
                f"--manifest={REPOSITORY_ROOT / 'benchmarks' / 'casc_2025_manifest.jsonl'}",
            ]
        )
        RUN.verify_manifest_argument(
            [
                "--phase=targeted",
                f"--manifest={EXPERIMENT_ROOT / 'targeted-manifest.jsonl'}",
            ]
        )

    def test_contract_preview_is_content_addressed(self) -> None:
        preview = RUN.contract_preview()
        body = {
            key: value
            for key, value in preview.items()
            if key != "preview_id"
        }
        self.assertEqual(
            preview["preview_id"],
            RUN.hashlib.sha256(RUN.BASE.canonical_json(body)).hexdigest(),
        )


class AnalysisTests(unittest.TestCase):
    def setUp(self) -> None:
        self.results = [
            {
                "problem_id": "P",
                "strategy": "cache",
                "budget": "larger",
                "repetition": 1,
                "_telemetry": telemetry(
                    cpu=1.0,
                    generated=100,
                    processed=20,
                    high_water=30,
                    term_storage=400,
                    rss=50,
                    rewrite_steps=20,
                    uncached=8,
                    lookups=40,
                    hits=10,
                    edges=15,
                    date_checks=30,
                    date_hits=12,
                ),
            },
            {
                "problem_id": "P",
                "strategy": "recompute",
                "budget": "larger",
                "repetition": 1,
                "_telemetry": telemetry(
                    cpu=2.0,
                    generated=100,
                    processed=20,
                    high_water=30,
                    term_storage=400,
                    rss=50,
                    rewrite_steps=20,
                    uncached=20,
                    lookups=60,
                    hits=0,
                    edges=0,
                    date_checks=0,
                    date_hits=0,
                ),
            },
        ]

    def test_cache_activity_reports_rates_and_saved_traversal(self) -> None:
        activity = ANALYZE.cache_activity(
            self.results, "cache", "larger"
        )

        self.assertEqual(activity["link_hit_rate"], 0.25)
        self.assertEqual(activity["mean_followed_path"], 1.5)
        self.assertEqual(activity["normal_form_date_hit_rate"], 0.4)
        self.assertEqual(activity["cached_rewrite_fraction"], 0.6)
        self.assertEqual(activity["saved_traversal_proxy"], 27)

    def test_paired_ratios_use_cache_over_recompute(self) -> None:
        ratios = ANALYZE.paired_ratios(self.results, "larger")

        self.assertEqual(ratios["paired_coordinates"], 1)
        self.assertEqual(ratios["median_cpu_ratio"], 0.5)
        self.assertEqual(ratios["median_generated_ratio"], 1.0)
        self.assertEqual(ratios["median_link_hits_ratio"], None)


if __name__ == "__main__":
    unittest.main()

