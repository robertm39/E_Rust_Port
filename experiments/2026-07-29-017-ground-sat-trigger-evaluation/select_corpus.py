#!/usr/bin/env python3
"""Freeze a candidate-blind corpus for the ground-SAT trigger evaluation."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


SELECTION_SALT = "umlaut-ground-sat-trigger-v1"
DIVISIONS = {"EPR", "FOF", "UEQ"}
EXPECTED_CLASSES = {"theorem", "unsatisfiable"}
FAMILY_COUNT = 6
PER_FAMILY = 4
MIN_BYTES = 500
MAX_BYTES = 250_000
PRIOR_SELECTIONS = (
    Path("experiments/2026-07-28-012-incremental-sat-service/"
         "capture-selection.jsonl"),
    Path("experiments/2026-07-28-012-incremental-sat-service/"
         "capture-test-escalation-selection.jsonl"),
    Path("experiments/2026-07-28-012-incremental-sat-service/"
         "capture-test-escalation2-selection.jsonl"),
    Path("experiments/2026-07-28-012-incremental-sat-service/"
         "capture-test-escalation3-selection.jsonl"),
    Path("experiments/2026-07-29-001-cadical-production-gate/"
         "fresh-selection.jsonl"),
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
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


def excluded_families(repo_root: Path) -> tuple[set[str], dict[str, str]]:
    families: set[str] = set()
    source_hashes: dict[str, str] = {}
    for relative in PRIOR_SELECTIONS:
        path = repo_root / relative
        source_hashes[relative.as_posix()] = sha256_file(path)
        for record in read_jsonl(path):
            family = record.get("family")
            if isinstance(family, str):
                families.add(family)
    return families, source_hashes


def select(
    repo_root: Path, manifest_path: Path
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    excluded, prior_hashes = excluded_families(repo_root)
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    manifest_records = read_jsonl(manifest_path)
    source_header = manifest_records[0]
    if source_header.get("record_type") != "manifest":
        raise ValueError("source manifest must start with a manifest record")
    for record in manifest_records[1:]:
        if record.get("record_type") != "problem":
            continue
        if (
            record["holdout_split"] == "train"
            and record["division"] in DIVISIONS
            and record["expected_class"] in EXPECTED_CLASSES
            and MIN_BYTES <= int(record["size_bytes"]) <= MAX_BYTES
            and record["family"] not in excluded
        ):
            grouped[str(record["family"])].append(record)

    eligible_families = sorted(
        (
            family
            for family, records in grouped.items()
            if len(records) >= PER_FAMILY
        ),
        key=lambda family: (hash_rank("family", family), family),
    )
    chosen_families = eligible_families[:FAMILY_COUNT]
    if len(chosen_families) != FAMILY_COUNT:
        raise ValueError(
            f"found {len(chosen_families)} eligible families, "
            f"expected {FAMILY_COUNT}"
        )

    selected: list[dict[str, Any]] = []
    for family in chosen_families:
        candidates = sorted(
            grouped[family],
            key=lambda record: (
                hash_rank(
                    "problem",
                    family,
                    str(record["sha256"]),
                ),
                str(record["problem_id"]),
            ),
        )
        for source in candidates[:PER_FAMILY]:
            record = dict(source)
            record["casc_holdout_split"] = record["holdout_split"]
            record["holdout_split"] = "fresh-family-heldout"
            record["selection_salt"] = SELECTION_SALT
            record["selection_rank"] = hash_rank(
                "problem",
                family,
                str(record["sha256"]),
            )
            selected.append(record)

    header = {
        **source_header,
        "schema_version": 1,
        "kind": "umlaut-ground-sat-trigger-corpus",
        "selection_salt": SELECTION_SALT,
        "selection_policy": {
            "candidate_blind": True,
            "source_split": "train",
            "whole_family_exclusion": True,
            "divisions": sorted(DIVISIONS),
            "expected_classes": sorted(EXPECTED_CLASSES),
            "minimum_size_bytes": MIN_BYTES,
            "maximum_size_bytes": MAX_BYTES,
            "family_count": FAMILY_COUNT,
            "problems_per_family": PER_FAMILY,
        },
        "selected_families": chosen_families,
        "excluded_families": sorted(excluded),
        "source_manifest": manifest_path.relative_to(repo_root).as_posix(),
        "source_manifest_sha256": sha256_file(manifest_path),
        "prior_selection_sha256": prior_hashes,
        "problem_count": len(selected),
        "family_count": len(chosen_families),
    }
    return header, selected


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
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
    repo_root = arguments.repo_root.resolve()
    manifest = arguments.manifest
    if not manifest.is_absolute():
        manifest = repo_root / manifest
    header, records = select(repo_root, manifest.resolve())
    output = arguments.output.resolve()
    output.write_text(
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
                "output": str(output),
                "problems": len(records),
                "sha256": sha256_file(output),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
