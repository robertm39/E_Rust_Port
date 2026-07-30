#!/usr/bin/env python3
"""Select the candidate-blind family-disjoint EPU/UEQ corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Sequence

import common


FAMILIES = {
    "calibration": {
        "CSR",
        "GEO",
        "GRP",
        "HWV",
        "KLE",
        "LAT",
        "MGT",
        "SWB",
        "SWW",
        "SYN",
    },
    "validation": {"LCL", "PUZ", "ROB", "SWV"},
    "test": {"NUN", "PLA", "REL", "SEU", "SWX"},
}
QUOTAS = {
    "calibration": {"EPU": 4, "UEQ": 4},
    "validation": {"EPU": 4, "UEQ": 4},
    "test": {"EPU": 2, "UEQ": 6},
}


def selection_rank(split: str, record: dict[str, Any]) -> str:
    material = "\0".join(
        (
            common.SELECTION_SALT,
            split,
            str(record["category"]),
            str(record["problem_id"]),
            str(record["sha256"]),
        )
    )
    return hashlib.sha256(material.encode()).hexdigest()


def eligible(
    record: dict[str, Any], split: str, category: str
) -> bool:
    return (
        record.get("record_type") == "problem"
        and record.get("category") == category
        and str(record.get("problem_id", ""))[:3] in FAMILIES[split]
        and record.get("expected_class") == "unsatisfiable"
        and 200 <= int(record.get("size_bytes", 0)) <= 100_000
    )


def select_records(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for split in ("calibration", "validation", "test"):
        for category in ("EPU", "UEQ"):
            candidates = [
                record
                for record in records
                if eligible(record, split, category)
            ]
            candidates.sort(
                key=lambda record: (
                    selection_rank(split, record),
                    str(record["problem_id"]),
                )
            )
            quota = QUOTAS[split][category]
            if len(candidates) < quota:
                raise common.ExperimentError(
                    f"{split}/{category} has {len(candidates)} candidates, "
                    f"needs {quota}"
                )
            for record in candidates[:quota]:
                selected_record = dict(record)
                selected_record["experiment_split"] = split
                selected_record["family"] = str(record["problem_id"])[:3]
                selected_record["selection_rank"] = selection_rank(
                    split, record
                )
                selected.append(selected_record)
    return selected


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
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
    selected = select_records(rows[1:])
    family_sets = {
        split: {
            str(record["family"])
            for record in selected
            if record["experiment_split"] == split
        }
        for split in FAMILIES
    }
    if (
        family_sets["calibration"] & family_sets["validation"]
        or family_sets["calibration"] & family_sets["test"]
        or family_sets["validation"] & family_sets["test"]
    ):
        raise common.ExperimentError("selected families are not disjoint")
    header = {
        "record_type": "manifest",
        "kind": "umlaut-online-stagnation-corpus",
        "schema_version": 1,
        "source_manifest": "benchmarks/casc_2025_manifest.jsonl",
        "source_manifest_sha256": common.SOURCE_MANIFEST_SHA256,
        "source_revision": common.SOURCE_REVISION,
        "selection_salt": common.SELECTION_SALT,
        "selection_policy": {
            "candidate_blind": True,
            "categories": ["EPU", "UEQ"],
            "expected_class": "unsatisfiable",
            "minimum_size_bytes": 200,
            "maximum_size_bytes": 100_000,
            "quotas": QUOTAS,
            "families": {
                split: sorted(families)
                for split, families in FAMILIES.items()
            },
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
