#!/usr/bin/env python3
"""Freeze the family-held-out TSM training and evaluation corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


SELECTION_SALT = "umlaut-tsm-family-heldout-v1"
CATEGORIES = ("FNE", "FEQ", "EPU", "UEQ")
QUOTAS = {"train": 4, "validation": 2, "test": 2}
TRAIN_BANDS = {"q1", "q2"}
EVAL_BANDS = {"q1", "q2", "q3", "q4"}
EXPECTED_CLASSES = {"theorem", "unsatisfiable"}
MIN_BYTES = 200
MAX_BYTES = 100_000


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def hash_rank(*parts: str) -> str:
    material = "\0".join((SELECTION_SALT, *parts)).encode()
    return hashlib.sha256(material).hexdigest()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def select(
    manifest_path: Path,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = read_jsonl(manifest_path)
    if not rows or rows[0].get("record_type") != "manifest":
        raise ValueError("source manifest must start with a manifest record")
    source_header = rows[0]
    source_records = rows[1:]
    selected: list[dict[str, Any]] = []

    for split, quota in QUOTAS.items():
        allowed_bands = TRAIN_BANDS if split == "train" else EVAL_BANDS
        for category in CATEGORIES:
            candidates = [
                record
                for record in source_records
                if record.get("record_type") == "problem"
                and record["holdout_split"] == split
                and record["category"] == category
                and record["expected_class"] in EXPECTED_CLASSES
                and record["difficulty_band"] in allowed_bands
                and MIN_BYTES <= int(record["size_bytes"]) <= MAX_BYTES
            ]
            candidates.sort(
                key=lambda record: (
                    hash_rank(
                        split,
                        category,
                        str(record["sha256"]),
                    ),
                    str(record["problem_id"]),
                )
            )
            if len(candidates) < quota:
                raise ValueError(
                    f"{split}/{category} has {len(candidates)} eligible "
                    f"records, expected at least {quota}"
                )
            for source in candidates[:quota]:
                record = dict(source)
                record["experiment_split"] = split
                record["selection_rank"] = hash_rank(
                    split,
                    category,
                    str(record["sha256"]),
                )
                selected.append(record)

    family_sets = {
        split: {
            str(record["family"])
            for record in selected
            if record["experiment_split"] == split
        }
        for split in QUOTAS
    }
    for left, right in (("train", "validation"), ("train", "test"), ("validation", "test")):
        overlap = family_sets[left] & family_sets[right]
        if overlap:
            raise ValueError(f"{left}/{right} family leakage: {sorted(overlap)}")

    selected.sort(
        key=lambda record: (
            tuple(QUOTAS).index(str(record["experiment_split"])),
            CATEGORIES.index(str(record["category"])),
            str(record["selection_rank"]),
        )
    )
    header = {
        **source_header,
        "schema_version": 1,
        "kind": "umlaut-tsm-learning-corpus",
        "source_manifest": manifest_path.as_posix(),
        "source_manifest_sha256": sha256_file(manifest_path),
        "selection_salt": SELECTION_SALT,
        "selection_policy": {
            "candidate_blind": True,
            "unit": "complete source family",
            "categories": list(CATEGORIES),
            "quotas_per_category": QUOTAS,
            "training_difficulty_bands": sorted(TRAIN_BANDS),
            "evaluation_difficulty_bands": sorted(EVAL_BANDS),
            "expected_classes": sorted(EXPECTED_CLASSES),
            "minimum_size_bytes": MIN_BYTES,
            "maximum_size_bytes": MAX_BYTES,
        },
        "selected_families": {
            split: sorted(families) for split, families in family_sets.items()
        },
        "split_counts": {
            split: sum(
                record["experiment_split"] == split for record in selected
            )
            for split in QUOTAS
        },
        "problem_count": len(selected),
        "family_count": len(set().union(*family_sets.values())),
    }
    return header, selected


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("benchmarks/casc_2025_manifest.jsonl"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent / "corpus.jsonl",
    )
    arguments = parser.parse_args()
    manifest = arguments.manifest.resolve()
    header, records = select(manifest)
    arguments.output.write_text(
        "".join(
            json.dumps(record, sort_keys=True) + "\n"
            for record in (header, *records)
        ),
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "families": header["selected_families"],
                "output": str(arguments.output.resolve()),
                "problems": len(records),
                "sha256": sha256_file(arguments.output),
                "split_counts": header["split_counts"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
