#!/usr/bin/env python3
"""Freeze syntax-only cohorts from the existing CASC-30 family holdout."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any

from tptp_split import SplitError, analyze_file


SCHEMA_VERSION = 1
SELECTION_SALT = "umlaut-avatar-restart-v1"
MAX_SIZE_BYTES = 3_500_000
MAX_CNF_COUNT = 20_000
MAX_SPLIT_CLAUSES = 6
MAX_SELECTORS = 32
QUOTAS = {
    "train": {"split_sensitive": 12, "neutral": 12},
    "validation": {"split_sensitive": 4, "neutral": 8},
    "test": {"split_sensitive": 3, "neutral": 7},
}


def stable_score(partition: str, cohort: str, problem_id: str) -> str:
    value = f"{SELECTION_SALT}\0{partition}\0{cohort}\0{problem_id}"
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    records = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    headers = [
        record for record in records if record.get("record_type") == "manifest"
    ]
    if len(headers) != 1:
        raise ValueError("expected exactly one manifest header")
    problems = [
        record for record in records if record.get("record_type") == "problem"
    ]
    return headers[0], problems


def classify_problem(
    root: Path, record: dict[str, Any]
) -> tuple[str, dict[str, Any]] | None:
    if record["division"] not in {"EPR", "UEQ"}:
        return None
    if record["expected_class"] != "unsatisfiable":
        return None
    if record["size_bytes"] > MAX_SIZE_BYTES:
        return None
    if record.get("includes"):
        return None
    problem_path = root / record["path"]
    try:
        abstraction = analyze_file(problem_path, MAX_SPLIT_CLAUSES)
    except (OSError, UnicodeError, SplitError):
        return None
    if abstraction["cnf_count"] > MAX_CNF_COUNT:
        return None
    if abstraction["splittable_clause_count"] == 0:
        cohort = "neutral"
    elif abstraction["selector_count"] <= MAX_SELECTORS:
        cohort = "split_sensitive"
    else:
        return None
    metrics = {
        "cnf_count": abstraction["cnf_count"],
        "selected_split_count": abstraction["selected_split_count"],
        "selector_count": abstraction["selector_count"],
        "splittable_clause_count": abstraction["splittable_clause_count"],
        "statement_count": abstraction["statement_count"],
    }
    return cohort, metrics


def select_records(
    root: Path, problems: list[dict[str, Any]]
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    candidates: dict[tuple[str, str], list[dict[str, Any]]] = {
        (partition, cohort): []
        for partition in QUOTAS
        for cohort in ("split_sensitive", "neutral")
    }
    excluded = Counter()
    for record in problems:
        partition = record["holdout_split"]
        if partition not in QUOTAS:
            excluded["unknown_partition"] += 1
            continue
        classified = classify_problem(root, record)
        if classified is None:
            excluded["outside_fragment_or_bounds"] += 1
            continue
        cohort, metrics = classified
        output = {
            key: record[key]
            for key in (
                "category",
                "difficulty_band",
                "division",
                "expected_class",
                "family",
                "holdout_split",
                "path",
                "problem_id",
                "sha256",
                "size_bytes",
            )
        }
        output.update(
            {
                "record_type": "problem",
                "cohort": cohort,
                "selection_score": stable_score(
                    partition, cohort, record["problem_id"]
                ),
                "syntax_metrics": metrics,
            }
        )
        candidates[(partition, cohort)].append(output)

    selected: list[dict[str, Any]] = []
    for partition, cohorts in QUOTAS.items():
        for cohort, quota in cohorts.items():
            pool = sorted(
                candidates[(partition, cohort)],
                key=lambda item: (
                    item["selection_score"],
                    item["problem_id"],
                ),
            )
            if len(pool) < quota:
                raise ValueError(
                    f"{partition}/{cohort}: need {quota}, found {len(pool)}"
                )
            selected.extend(pool[:quota])
    selected.sort(
        key=lambda item: (
            ("train", "validation", "test").index(item["holdout_split"]),
            ("split_sensitive", "neutral").index(item["cohort"]),
            item["selection_score"],
            item["problem_id"],
        )
    )
    return selected, dict(excluded)


def write_selection(
    output_path: Path,
    source_manifest: Path,
    source_header: dict[str, Any],
    selected: list[dict[str, Any]],
    excluded: dict[str, int],
) -> None:
    family_partitions: dict[str, set[str]] = {}
    for record in selected:
        family_partitions.setdefault(record["family"], set()).add(
            record["holdout_split"]
        )
    leaking = {
        family: sorted(partitions)
        for family, partitions in family_partitions.items()
        if len(partitions) != 1
    }
    if leaking:
        raise ValueError(f"family leakage across partitions: {leaking}")

    counts = Counter(
        (record["holdout_split"], record["cohort"]) for record in selected
    )
    header = {
        "schema_version": SCHEMA_VERSION,
        "record_type": "manifest",
        "kind": "umlaut-avatar-restart-corpus",
        "selection_policy": {
            "salt": SELECTION_SALT,
            "outcome_blind": True,
            "source_holdout_unit": source_header["partition_policy"]["unit"],
            "eligible_divisions": ["EPR", "UEQ"],
            "eligible_expected_classes": ["unsatisfiable"],
            "max_size_bytes": MAX_SIZE_BYTES,
            "max_cnf_count": MAX_CNF_COUNT,
            "max_split_clauses": MAX_SPLIT_CLAUSES,
            "max_selectors": MAX_SELECTORS,
            "quotas": QUOTAS,
            "ranking": "SHA-256(salt, partition, cohort, problem_id)",
        },
        "source_manifest": str(source_manifest.as_posix()),
        "source_manifest_sha256": hashlib.sha256(
            source_manifest.read_bytes()
        ).hexdigest(),
        "problem_count": len(selected),
        "partition_cohort_counts": {
            f"{partition}/{cohort}": count
            for (partition, cohort), count in sorted(counts.items())
        },
        "family_count": len(family_partitions),
        "excluded_source_records": excluded,
    }
    lines = [json.dumps(header, sort_keys=True)]
    lines.extend(json.dumps(record, sort_keys=True) for record in selected)
    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("benchmarks/casc_2025_manifest.jsonl"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).with_name("corpus.jsonl"),
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    root = Path.cwd()
    source_header, problems = load_manifest(args.manifest)
    selected, excluded = select_records(root, problems)
    write_selection(
        args.output,
        args.manifest,
        source_header,
        selected,
        excluded,
    )
    print(f"wrote {len(selected)} records to {args.output}")


if __name__ == "__main__":
    main()
