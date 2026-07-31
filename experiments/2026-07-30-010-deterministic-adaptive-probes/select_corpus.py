#!/usr/bin/env python3
"""Select a fresh candidate-blind family-held-out FNE/FEQ corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Sequence

import common


SPLITS = ("train", "validation", "test")
CATEGORIES = ("FNE", "FEQ")
BANDS_BY_SPLIT_CATEGORY = {
    ("train", "FNE"): ("q1", "q2", "q3", "q4"),
    ("train", "FEQ"): ("q1", "q2", "q3", "q4"),
    ("validation", "FNE"): ("q2", "q3", "q4", "q5"),
    ("validation", "FEQ"): ("q1", "q2", "q3", "q4"),
    ("test", "FNE"): ("q1", "q2", "q3", "q5"),
    ("test", "FEQ"): ("q1", "q2", "q3", "q4"),
}


def selection_rank(split: str, record: dict[str, Any]) -> str:
    material = "\0".join(
        (
            common.SELECTION_SALT,
            split,
            str(record["category"]),
            str(record["difficulty_band"]),
            str(record["problem_id"]),
            str(record["sha256"]),
        )
    )
    return hashlib.sha256(material.encode()).hexdigest()


def excluded_problem_ids(path: Path) -> set[str]:
    if not path.is_file():
        raise common.ExperimentError(f"exclude corpus is missing: {path}")
    rows = common.read_jsonl(path)
    if not rows or rows[0].get("record_type") != "manifest":
        raise common.ExperimentError("exclude corpus header is missing")
    return {str(row["problem_id"]) for row in rows[1:]}


def eligible(
    record: dict[str, Any],
    split: str,
    category: str,
    band: str,
    excluded: set[str],
) -> bool:
    return (
        record.get("record_type") == "problem"
        and record.get("holdout_split") == split
        and record.get("category") == category
        and record.get("difficulty_band") == band
        and record.get("expected_class") == "theorem"
        and str(record.get("problem_id")) not in excluded
        and 200 <= int(record.get("size_bytes", 0)) <= 100_000
    )


def select_records(
    records: list[dict[str, Any]], excluded: set[str]
) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for split in SPLITS:
        for category in CATEGORIES:
            for band in BANDS_BY_SPLIT_CATEGORY[(split, category)]:
                candidates = [
                    record
                    for record in records
                    if eligible(
                        record,
                        split,
                        category,
                        band,
                        excluded,
                    )
                ]
                candidates.sort(
                    key=lambda record: (
                        selection_rank(split, record),
                        str(record["problem_id"]),
                    )
                )
                if not candidates:
                    raise common.ExperimentError(
                        f"no candidate for {split}/{category}/{band}"
                    )
                chosen = dict(candidates[0])
                chosen["experiment_split"] = split
                chosen["selection_rank"] = selection_rank(split, chosen)
                selected.append(chosen)
    return selected


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--exclude-corpus", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    manifest = arguments.manifest.resolve()
    observed = common.sha256_file(manifest)
    if observed != common.SOURCE_MANIFEST_SHA256:
        raise common.ExperimentError(
            f"source manifest hash mismatch: {observed}"
        )
    rows = common.read_jsonl(manifest)
    if not rows or rows[0].get("record_type") != "manifest":
        raise common.ExperimentError("source manifest header is missing")
    excluded_path = arguments.exclude_corpus.resolve()
    excluded = excluded_problem_ids(excluded_path)
    selected = select_records(rows[1:], excluded)
    families = {
        split: {
            str(record["family"])
            for record in selected
            if record["experiment_split"] == split
        }
        for split in SPLITS
    }
    if any(
        families[left] & families[right]
        for left in SPLITS
        for right in SPLITS
        if left < right
    ):
        raise common.ExperimentError("selected families are not disjoint")
    header = {
        "record_type": "manifest",
        "kind": "umlaut-deterministic-adaptive-probe-corpus",
        "schema_version": 1,
        "source_manifest": "benchmarks/casc_2025_manifest.jsonl",
        "source_manifest_sha256": common.SOURCE_MANIFEST_SHA256,
        "source_revision": common.SOURCE_REVISION,
        "selection_salt": common.SELECTION_SALT,
        "selection_policy": {
            "candidate_blind": True,
            "categories": list(CATEGORIES),
            "difficulty_bands": {
                f"{split}/{category}": list(bands)
                for (split, category), bands in sorted(
                    BANDS_BY_SPLIT_CATEGORY.items()
                )
            },
            "expected_class": "theorem",
            "minimum_size_bytes": 200,
            "maximum_size_bytes": 100_000,
            "source_split": "holdout_split",
            "one_per_category_band_split": True,
            "exclude_corpus": (
                "experiments/2026-07-29-018-tsm-learning-baseline/"
                "corpus.jsonl"
            ),
            "exclude_corpus_sha256": common.sha256_file(excluded_path),
        },
        "problem_count": len(selected),
    }
    common.write_jsonl(arguments.output, [header, *selected])
    print(
        json.dumps(
            {
                "output": str(arguments.output),
                "problem_count": len(selected),
                "sha256": common.sha256_file(arguments.output),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except common.ExperimentError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
