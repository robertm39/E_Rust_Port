#!/usr/bin/env python3
"""Apply the preregistered correctness and 128/256 dispatch gate."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path

THRESHOLDS = (128, 256)
INTERNAL_BACKEND = "internal-dpll"
CADICAL_BACKEND = "cadical-3.0.1-static"


def percentile(values: list[int], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return math.nan
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return float(ordered[lower])
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def workload_class(session: str) -> str:
    lowered = session.replace("\\", "/").lower()
    if "/avatar/" in lowered:
        return "avatar-style"
    if "/satcheck/" in lowered:
        return "satcheck-fresh-family"
    raise ValueError(f"cannot classify session {session}")


def load(path: Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    queries = []
    failures = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        record = json.loads(line)
        if record["record_type"] == "query":
            queries.append(record)
        else:
            failures.append(record)
    return queries, failures


def analyze(path: Path) -> dict[str, object]:
    queries, process_failures = load(path)
    grouped: dict[
        tuple[str, str, int], list[dict[str, object]]
    ] = defaultdict(list)
    for record in queries:
        grouped[
            (
                str(record["backend"]),
                str(record["session"]),
                int(record["repetition"]),
            )
        ].append(record)

    keys_by_backend: dict[str, set[tuple[str, int]]] = defaultdict(set)
    for backend, session, repetition in grouped:
        keys_by_backend[backend].add((session, repetition))
    internal_keys = keys_by_backend[INTERNAL_BACKEND]
    cadical_keys = keys_by_backend[CADICAL_BACKEND]
    missing_keys = sorted(internal_keys ^ cadical_keys)
    common_keys = internal_keys & cadical_keys

    status_mismatches = []
    for session, repetition in sorted(common_keys):
        internal = {
            str(record["query"]): str(record["status"])
            for record in grouped[(INTERNAL_BACKEND, session, repetition)]
        }
        cadical = {
            str(record["query"]): str(record["status"])
            for record in grouped[(CADICAL_BACKEND, session, repetition)]
        }
        if internal != cadical:
            status_mismatches.append(
                {
                    "session": session,
                    "repetition": repetition,
                    "internal": internal,
                    "cadical": cadical,
                }
            )

    correctness_passed = (
        not process_failures and not missing_keys and not status_mismatches
    )

    costs: dict[tuple[str, str, int], int] = {}
    query_costs: dict[tuple[str, str, int], list[int]] = {}
    clauses: dict[tuple[str, int], int] = {}
    for (backend, session, repetition), records in grouped.items():
        costs[(backend, session, repetition)] = sum(
            int(record["elapsed_ns"])
            + int(record.get("core_ns", 0))
            + int(record.get("insertion_ns", 0))
            for record in records
        )
        query_costs[(backend, session, repetition)] = [
            int(record["elapsed_ns"]) + int(record.get("core_ns", 0))
            for record in records
        ]
        clauses[(session, repetition)] = int(records[0]["clauses"])

    baseline_total = sum(
        costs[(INTERNAL_BACKEND, session, repetition)]
        for session, repetition in common_keys
    )
    baseline_queries = [
        cost
        for session, repetition in common_keys
        for cost in query_costs[(INTERNAL_BACKEND, session, repetition)]
    ]
    baseline_p95 = percentile(baseline_queries, 0.95)
    policies = []
    for threshold in THRESHOLDS:
        total = 0
        selected_queries = []
        dispatched_by_class: dict[str, int] = defaultdict(int)
        totals_by_class: dict[str, int] = defaultdict(int)
        baseline_by_class: dict[str, int] = defaultdict(int)
        for session, repetition in sorted(common_keys):
            category = workload_class(session)
            use_cadical = clauses[(session, repetition)] >= threshold
            backend = CADICAL_BACKEND if use_cadical else INTERNAL_BACKEND
            key_cost = costs[(backend, session, repetition)]
            total += key_cost
            totals_by_class[category] += key_cost
            baseline_by_class[category] += costs[
                (INTERNAL_BACKEND, session, repetition)
            ]
            dispatched_by_class[category] += int(use_cadical)
            selected_queries.extend(query_costs[(backend, session, repetition)])
        class_ratios = {
            category: totals_by_class[category] / baseline
            for category, baseline in sorted(baseline_by_class.items())
        }
        p95 = percentile(selected_queries, 0.95)
        policies.append(
            {
                "threshold_clauses": threshold,
                "total_cost_ns": total,
                "total_ratio_to_internal": total / baseline_total,
                "query_p95_ns": p95,
                "query_p95_ratio_to_internal": p95 / baseline_p95,
                "dispatched_sessions_by_class": dict(
                    sorted(dispatched_by_class.items())
                ),
                "class_total_ratios": class_ratios,
            }
        )

    by_threshold = {int(policy["threshold_clauses"]): policy for policy in policies}
    for threshold, other in ((128, 256), (256, 128)):
        policy = by_threshold[threshold]
        required_classes = {"avatar-style", "satcheck-fresh-family"}
        dispatched = policy["dispatched_sessions_by_class"]
        policy["passes_without_relative_winner"] = (
            correctness_passed
            and required_classes <= set(dispatched)
            and all(int(dispatched[category]) > 0 for category in required_classes)
            and float(policy["total_ratio_to_internal"]) <= 0.75
            and float(policy["query_p95_ratio_to_internal"]) <= 0.75
            and all(
                float(ratio) <= 1.10
                for ratio in policy["class_total_ratios"].values()
            )
        )
        policy["ratio_to_other_threshold"] = (
            int(policy["total_cost_ns"])
            / int(by_threshold[other]["total_cost_ns"])
        )
        policy["passes"] = bool(policy["passes_without_relative_winner"]) and (
            float(policy["ratio_to_other_threshold"]) <= 0.95
        )

    passing = [policy for policy in policies if bool(policy["passes"])]
    selected = (
        min(passing, key=lambda policy: int(policy["total_cost_ns"]))
        if passing
        else None
    )
    failures_by_session: dict[str, int] = defaultdict(int)
    for failure in process_failures:
        failures_by_session[str(failure["session"])] += 1
    return {
        "schema": 1,
        "thresholds": list(THRESHOLDS),
        "correctness_passed": correctness_passed,
        "process_failure_count": len(process_failures),
        "process_failures_by_session": dict(sorted(failures_by_session.items())),
        "missing_backend_key_count": len(missing_keys),
        "missing_backend_keys": missing_keys,
        "status_mismatches": status_mismatches,
        "sessions": len({session for session, _repetition in common_keys}),
        "complete_measured_session_repetitions": len(common_keys),
        "baseline_total_cost_ns": baseline_total,
        "baseline_query_p95_ns": baseline_p95,
        "policies": policies,
        "gate_passed": selected is not None,
        "selected_threshold": (
            int(selected["threshold_clauses"]) if selected is not None else None
        ),
        "decision": (
            f"threshold-{selected['threshold_clauses']}-eligible"
            if selected is not None
            else "automatic-dispatch-remains-nondefault"
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    result = analyze(arguments.results)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    arguments.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(rendered, end="")
    return int(not bool(result["correctness_passed"]))


if __name__ == "__main__":
    raise SystemExit(main())
