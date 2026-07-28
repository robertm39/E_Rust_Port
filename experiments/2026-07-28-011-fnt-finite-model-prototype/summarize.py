#!/usr/bin/env python3
"""Summarize finite-model inventory, prototype, and equal-budget baselines."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


MODEL_STATUSES = {"Satisfiable", "CounterSatisfiable"}


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def counts(values: Iterable[str]) -> dict[str, int]:
    return dict(sorted(Counter(values).items()))


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def inventory_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    by_split: dict[str, Counter[str]] = defaultdict(Counter)
    by_category: dict[str, Counter[str]] = defaultdict(Counter)
    for record in records:
        by_split[record["split"]][record["outcome"]] += 1
        by_category[record["category"]][record["outcome"]] += 1
    return {
        "total": len(records),
        "outcomes": counts(record["outcome"] for record in records),
        "by_split": {
            split: dict(sorted(value.items()))
            for split, value in sorted(by_split.items())
        },
        "by_category": {
            category: dict(sorted(value.items()))
            for category, value in sorted(by_category.items())
        },
        "supported_problem_ids": sorted(
            record["problem_id"]
            for record in records
            if record["outcome"] == "supported"
        ),
    }


def prototype_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    by_mode: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        by_mode[record["mode"]].append(record)
    solved: dict[str, set[str]] = {}
    for mode, mode_records in sorted(by_mode.items()):
        models = [record for record in mode_records if record["outcome"] == "model"]
        verified = [
            record
            for record in models
            if record.get("validation_verdict") == "verified"
        ]
        last_bounds = [
            next(
                bound
                for bound in reversed(record["bounds"])
                if bound.get("sat_status") != "not_run"
            )
            for record in mode_records
            if any(
                bound.get("sat_status") != "not_run"
                for bound in record.get("bounds", [])
            )
        ]
        solved[mode] = {record["problem_id"] for record in models}
        result[mode] = {
            "runs": len(mode_records),
            "outcomes": counts(record["outcome"] for record in mode_records),
            "model_problem_ids": sorted(solved[mode]),
            "verified_models": len(verified),
            "unverified_models": len(models) - len(verified),
            "median_last_completed_bound": {
                "propositional_variables": median(
                    bound["propositional_variables"] for bound in last_bounds
                ),
                "propositional_clauses": median(
                    bound["propositional_clauses"] for bound in last_bounds
                ),
                "encoding_seconds": median(
                    bound["encoding_seconds"] for bound in last_bounds
                ),
                "sat_seconds": median(bound["sat_seconds"] for bound in last_bounds),
            },
        }

    if "naive" in solved:
        for mode in ("sorted", "sorted-symmetry"):
            if mode in solved:
                result[mode]["naive_models_lost"] = sorted(solved["naive"] - solved[mode])
                result[mode]["models_added_over_naive"] = sorted(
                    solved[mode] - solved["naive"]
                )
    return result


def baseline_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    by_system: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        by_system[record["system"]].append(record)
    return {
        system: {
            "runs": len(system_records),
            "statuses": counts(
                str(record["status"]) if record["status"] is not None else "None"
                for record in system_records
            ),
            "model_status_problem_ids": sorted(
                record["problem_id"]
                for record in system_records
                if record["status"] in MODEL_STATUSES
            ),
            "median_wall_seconds": median(
                record["wall_seconds"] for record in system_records
            ),
        }
        for system, system_records in sorted(by_system.items())
    }


def comparisons(
    prototypes: list[dict[str, Any]], baselines: list[dict[str, Any]]
) -> dict[str, Any]:
    umlaut_models = {
        record["problem_id"]
        for record in baselines
        if record["system"] == "umlaut-auto"
        and record["status"] in MODEL_STATUSES
    }
    by_mode: dict[str, set[str]] = defaultdict(set)
    for record in prototypes:
        if (
            record["outcome"] == "model"
            and record.get("validation_verdict") == "verified"
        ):
            by_mode[record["mode"]].add(record["problem_id"])
    return {
        mode: {
            "verified_models": sorted(problem_ids),
            "unique_against_umlaut_auto": sorted(problem_ids - umlaut_models),
        }
        for mode, problem_ids in sorted(by_mode.items())
    }


def named_paths(values: list[str]) -> dict[str, Path]:
    result: dict[str, Path] = {}
    for value in values:
        name, separator, path = value.partition("=")
        if not separator or not name or not path:
            raise ValueError(f"expected NAME=PATH, got {value!r}")
        result[name] = Path(path)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--inventory-overlay", action="append", type=Path, default=[])
    parser.add_argument("--prototype", action="append", default=[], metavar="NAME=PATH")
    parser.add_argument("--baseline", action="append", default=[], metavar="NAME=PATH")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prototype_paths = named_paths(args.prototype)
    baseline_paths = named_paths(args.baseline)

    inventory_records = read_jsonl(args.inventory)
    inventory_positions = {
        record["problem_id"]: index
        for index, record in enumerate(inventory_records)
    }
    for overlay_path in args.inventory_overlay:
        for record in read_jsonl(overlay_path):
            problem_id = record["problem_id"]
            if problem_id not in inventory_positions:
                raise ValueError(f"inventory overlay contains unknown problem {problem_id}")
            inventory_records[inventory_positions[problem_id]] = record

    summary: dict[str, Any] = {
        "schema_version": 1,
        "inventory": inventory_summary(inventory_records),
        "datasets": {},
    }
    for name in sorted(set(prototype_paths) | set(baseline_paths)):
        dataset: dict[str, Any] = {}
        prototypes: list[dict[str, Any]] = []
        baselines: list[dict[str, Any]] = []
        if name in prototype_paths:
            prototypes = read_jsonl(prototype_paths[name])
            dataset["prototype"] = prototype_summary(prototypes)
        if name in baseline_paths:
            baselines = read_jsonl(baseline_paths[name])
            dataset["baselines"] = baseline_summary(baselines)
        if prototypes and baselines:
            dataset["comparisons"] = comparisons(prototypes, baselines)
        summary["datasets"][name] = dataset

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(args.output.read_text(encoding="utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
