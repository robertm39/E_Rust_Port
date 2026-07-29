#!/usr/bin/env python3
"""Summarize typed FNT coverage, telemetry, validation, and unique solves."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


MODEL_STATUSES = {"Satisfiable", "CounterSatisfiable"}


def read_jsonl(paths: Iterable[Path]) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for path in paths
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def counts(values: Iterable[object]) -> dict[str, int]:
    return dict(sorted(Counter(str(value) for value in values).items()))


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", action="append", type=Path, default=[])
    parser.add_argument("--prototype", action="append", type=Path, default=[])
    parser.add_argument("--baseline", action="append", type=Path, default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    inventory = read_jsonl(args.inventory)
    prototype = read_jsonl(args.prototype)
    baseline = read_jsonl(args.baseline)
    verified = {
        record["problem_id"]
        for record in prototype
        if record.get("outcome") == "model"
        and record.get("validation_verdict") == "verified"
    }
    unverified = {
        record["problem_id"]
        for record in prototype
        if record.get("outcome") == "model"
        and record.get("validation_verdict") != "verified"
    }
    baseline_models = {
        record["problem_id"]
        for record in baseline
        if record.get("status") in MODEL_STATUSES
    }
    bounds = [
        bound
        for record in prototype
        for bound in record.get("bounds", [])
    ]
    multi_bound = [
        record for record in prototype if len(record.get("bounds", [])) > 1
    ]
    report: dict[str, Any] = {
        "schema_version": 2,
        "inventory": {
            "records": len(inventory),
            "outcomes": counts(record.get("outcome") for record in inventory),
            "supported_problem_ids": sorted(
                record["problem_id"]
                for record in inventory
                if record.get("outcome") == "supported"
            ),
        },
        "prototype": {
            "records": len(prototype),
            "outcomes": counts(record.get("outcome") for record in prototype),
            "verified_models": sorted(verified),
            "unverified_models": sorted(unverified),
            "multi_bound_runs": len(multi_bound),
            "bounds": len(bounds),
            "median_bound": {
                "new_ground_instances": median(
                    bound["new_ground_instances"] for bound in bounds
                ),
                "cumulative_ground_instances": median(
                    bound["cumulative_ground_instances"] for bound in bounds
                ),
                "new_clauses": median(bound["new_clauses"] for bound in bounds),
                "cumulative_clauses": median(
                    bound["cumulative_clauses"] for bound in bounds
                ),
                "propositional_variables": median(
                    bound["propositional_variables"] for bound in bounds
                ),
                "grounding_seconds": median(
                    bound.get("grounding_seconds", 0.0) for bound in bounds
                ),
                "insertion_seconds": median(
                    bound["insertion_seconds"] for bound in bounds
                ),
                "sat_seconds": median(bound["sat_seconds"] for bound in bounds),
            },
        },
        "baseline": {
            "records": len(baseline),
            "statuses": counts(record.get("status") for record in baseline),
            "model_status_problem_ids": sorted(baseline_models),
        },
        "comparison": {
            "verified_models": sorted(verified),
            "unique_against_umlaut_auto": sorted(verified - baseline_models),
            "baseline_models_not_found": sorted(baseline_models - verified),
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(args.output.read_text(encoding="utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
