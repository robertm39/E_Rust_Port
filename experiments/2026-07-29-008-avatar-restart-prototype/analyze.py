#!/usr/bin/env python3
"""Analyze the preregistered AVATAR restart comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


METHODS = ("baseline", "static_split", "avatar")


class AnalysisError(RuntimeError):
    """Result evidence is incomplete or internally inconsistent."""


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def quantile(values: Iterable[float], fraction: float) -> float | None:
    ordered = sorted(values)
    if not ordered:
        return None
    index = max(0, math.ceil(fraction * len(ordered)) - 1)
    return ordered[index]


def summarize_runs(records: list[dict[str, Any]]) -> dict[str, Any]:
    solved = [record for record in records if record["proof_verified"]]
    walls = [record["measured_wall_seconds"] for record in solved]
    rss = [
        record.get("max_rss_kib", 0)
        for record in records
        if record.get("max_rss_kib") is not None
    ]
    return {
        "records": len(records),
        "verified_solves": len(solved),
        "median_solved_wall_seconds": (
            statistics.median(walls) if walls else None
        ),
        "p95_solved_wall_seconds": quantile(walls, 0.95),
        "maximum_rss_kib": max(rss, default=0),
    }


def paired_wall_ratio(
    by_problem: dict[str, dict[str, dict[str, Any]]],
    numerator: str,
    denominator: str,
    problem_ids: set[str],
) -> dict[str, Any]:
    ratios = []
    for problem_id in sorted(problem_ids):
        pair = by_problem[problem_id]
        if (
            pair[numerator]["proof_verified"]
            and pair[denominator]["proof_verified"]
            and pair[denominator]["measured_wall_seconds"] > 0
        ):
            ratios.append(
                pair[numerator]["measured_wall_seconds"]
                / pair[denominator]["measured_wall_seconds"]
            )
    return {
        "paired_solves": len(ratios),
        "median_ratio": statistics.median(ratios) if ratios else None,
        "p95_ratio": quantile(ratios, 0.95),
        "ratios": ratios,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--driver-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    records = [
        json.loads(line)
        for line in arguments.results.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    driver_report = json.loads(
        arguments.driver_report.read_text(encoding="utf-8")
    )
    by_problem: dict[str, dict[str, dict[str, Any]]] = {}
    duplicates = []
    for record in records:
        problem = by_problem.setdefault(record["problem_id"], {})
        method = record.get("method")
        if method not in METHODS or method in problem:
            duplicates.append((record.get("problem_id"), method))
        else:
            problem[method] = record
    incomplete = {
        problem_id: sorted(set(METHODS) - set(methods))
        for problem_id, methods in by_problem.items()
        if set(methods) != set(METHODS)
    }
    if duplicates or incomplete or len(records) != 3 * len(by_problem):
        raise AnalysisError(
            f"malformed result matrix: duplicates={duplicates}, "
            f"incomplete={incomplete}"
        )

    claimed_unverified = [
        (record["problem_id"], record["method"], record["claimed_status"])
        for record in records
        if record["claimed_status"]
        .replace("-", "")
        .replace("_", "")
        .lower()
        in {"unsatisfiable", "contradictoryaxioms"}
        and not record["proof_verified"]
    ]
    avatar_certificate_failures = [
        record["problem_id"]
        for record in records
        if record["method"] == "avatar"
        and not record["certificate_verified"]
    ]
    soundness = {
        "driver_integration_passed": driver_report.get("passed") is True,
        "unverified_prover_claims_not_counted": claimed_unverified,
        "avatar_certificate_failures": avatar_certificate_failures,
        "all_gates_passed": (
            driver_report.get("passed") is True
            and not avatar_certificate_failures
        ),
    }

    summaries: dict[str, Any] = {}
    for partition in ("train", "validation", "test", "heldout"):
        for cohort in ("split_sensitive", "neutral", "all"):
            for method in METHODS:
                subset = [
                    record
                    for record in records
                    if (
                        partition == "heldout"
                        and record["partition"] in {"validation", "test"}
                        or record["partition"] == partition
                    )
                    and (cohort == "all" or record["cohort"] == cohort)
                    and record["method"] == method
                ]
                summaries[f"{partition}/{cohort}/{method}"] = summarize_runs(
                    subset
                )

    solved_by_method = {
        method: {
            record["problem_id"]
            for record in records
            if record["method"] == method and record["proof_verified"]
        }
        for method in METHODS
    }
    unique_solves = {
        method: sorted(
            solved_by_method[method]
            - set().union(
                *(
                    solved_by_method[other]
                    for other in METHODS
                    if other != method
                )
            )
        )
        for method in METHODS
    }
    pairwise = {}
    for left in METHODS:
        for right in METHODS:
            if left < right:
                pairwise[f"{left}_over_{right}"] = {
                    "wins": sorted(
                        solved_by_method[left] - solved_by_method[right]
                    ),
                    "losses": sorted(
                        solved_by_method[right] - solved_by_method[left]
                    ),
                }

    heldout_split = {
        problem_id
        for problem_id, methods in by_problem.items()
        if next(iter(methods.values()))["partition"]
        in {"validation", "test"}
        and next(iter(methods.values()))["cohort"] == "split_sensitive"
    }
    heldout_neutral = {
        problem_id
        for problem_id, methods in by_problem.items()
        if next(iter(methods.values()))["partition"]
        in {"validation", "test"}
        and next(iter(methods.values()))["cohort"] == "neutral"
    }
    split_pair = paired_wall_ratio(
        by_problem, "avatar", "baseline", heldout_split
    )
    neutral_pair = paired_wall_ratio(
        by_problem, "avatar", "baseline", heldout_neutral
    )
    split_baseline_only = sorted(
        problem_id
        for problem_id in heldout_split
        if by_problem[problem_id]["baseline"]["proof_verified"]
        and not by_problem[problem_id]["avatar"]["proof_verified"]
    )
    split_avatar_unique = sorted(
        problem_id
        for problem_id in heldout_split
        if by_problem[problem_id]["avatar"]["proof_verified"]
        and not any(
            by_problem[problem_id][method]["proof_verified"]
            for method in ("baseline", "static_split")
        )
    )
    neutral_baseline_only = sorted(
        problem_id
        for problem_id in heldout_neutral
        if by_problem[problem_id]["baseline"]["proof_verified"]
        and not by_problem[problem_id]["avatar"]["proof_verified"]
    )
    heldout_records = [
        record
        for record in records
        if record["partition"] in {"validation", "test"}
    ]
    baseline_max_rss = max(
        (
            record.get("max_rss_kib", 0)
            for record in heldout_records
            if record["method"] == "baseline"
        ),
        default=0,
    )
    avatar_max_rss = max(
        (
            record.get("max_rss_kib", 0)
            for record in heldout_records
            if record["method"] == "avatar"
        ),
        default=0,
    )
    memory_ratio = (
        avatar_max_rss / baseline_max_rss if baseline_max_rss else None
    )
    speed_condition = bool(split_avatar_unique) or (
        split_pair["paired_solves"] >= 3
        and split_pair["median_ratio"] is not None
        and split_pair["median_ratio"] <= 0.90
    )
    advance_conditions = {
        "soundness": soundness["all_gates_passed"],
        "no_split_baseline_only": not split_baseline_only,
        "split_benefit": speed_condition,
        "no_neutral_baseline_only": not neutral_baseline_only,
        "neutral_median_ratio_at_most_1_10": (
            neutral_pair["median_ratio"] is not None
            and neutral_pair["median_ratio"] <= 1.10
        ),
        "memory_ratio_at_most_1_15": (
            memory_ratio is not None and memory_ratio <= 1.15
        ),
    }
    avatar_records = [
        record for record in records if record["method"] == "avatar"
    ]
    activation = {
        "branches": sum(record["branch_count"] for record in avatar_records),
        "verified_conflicts": sum(
            record["verified_conflicts"] for record in avatar_records
        ),
        "sat_calls": sum(record["sat_calls"] for record in avatar_records),
        "sat_elapsed_ns": sum(
            record["sat_elapsed_ns"] for record in avatar_records
        ),
        "inactive_component_clauses": sum(
            sum(record["inactive_component_counts"])
            for record in avatar_records
        ),
        "active_component_clauses": sum(
            sum(record["active_selector_counts"])
            for record in avatar_records
        ),
        "termination_reasons": dict(
            Counter(record["termination_reason"] for record in avatar_records)
        ),
    }
    report = {
        "schema_version": 1,
        "result_count": len(records),
        "problem_count": len(by_problem),
        "soundness": soundness,
        "summaries": summaries,
        "unique_solves": unique_solves,
        "pairwise": pairwise,
        "activation": activation,
        "heldout_decision": {
            "conditions": advance_conditions,
            "advance": all(advance_conditions.values()),
            "split_baseline_only": split_baseline_only,
            "split_avatar_unique": split_avatar_unique,
            "split_avatar_over_baseline_wall": split_pair,
            "neutral_baseline_only": neutral_baseline_only,
            "neutral_avatar_over_baseline_wall": neutral_pair,
            "baseline_max_rss_kib": baseline_max_rss,
            "avatar_max_rss_kib": avatar_max_rss,
            "memory_ratio": memory_ratio,
        },
    }
    report["report_id"] = hashlib.sha256(canonical_json(report)).hexdigest()
    arguments.output.write_bytes(canonical_json(report) + b"\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (AnalysisError, OSError, ValueError) as error:
        print(f"analysis error: {error}")
        raise SystemExit(1) from error
