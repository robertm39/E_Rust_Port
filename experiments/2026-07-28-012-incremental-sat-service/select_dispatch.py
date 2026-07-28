#!/usr/bin/env python3
"""Evaluate simple clause-count dispatch policies on a validation split."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

from analyze import percentile

THRESHOLDS = (0, 16, 32, 64, 128, 256, 512, 1024, 2048)


def load(path: Path) -> list[dict[str, object]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line and json.loads(line).get("record_type", "query") == "query"
    ]


def session_costs(
    records: list[dict[str, object]]
) -> dict[str, dict[tuple[str, int], dict[str, int | str]]]:
    grouped: dict[tuple[str, str, int], list[dict[str, object]]] = defaultdict(list)
    for record in records:
        grouped[
            (
                str(record["backend"]),
                str(record["session"]),
                int(record["repetition"]),
            )
        ].append(record)
    output: dict[str, dict[tuple[str, int], dict[str, int | str]]] = defaultdict(dict)
    for (backend, session, repetition), values in grouped.items():
        statuses = ",".join(sorted(str(value["status"]) for value in values))
        output[backend][(session, repetition)] = {
            "clauses": int(values[0]["clauses"]),
            "cost_ns": int(values[0].get("insertion_ns", 0))
            + sum(
                int(value["elapsed_ns"]) + int(value.get("core_ns", 0))
                for value in values
            ),
            "statuses": statuses,
        }
    return output


def evaluate(
    records: list[dict[str, object]], candidate: str
) -> list[dict[str, object]]:
    costs = session_costs(records)
    baseline = costs["internal-dpll"]
    selected = costs[candidate]
    if set(baseline) != set(selected):
        raise ValueError("candidate and internal session keys differ")
    for key in baseline:
        if baseline[key]["statuses"] != selected[key]["statuses"]:
            raise ValueError(f"status mismatch for {key}")
    baseline_total = sum(int(value["cost_ns"]) for value in baseline.values())
    queries_by_session: dict[
        tuple[str, str, int], list[dict[str, object]]
    ] = defaultdict(list)
    for record in records:
        queries_by_session[
            (
                str(record["backend"]),
                str(record["session"]),
                int(record["repetition"]),
            )
        ].append(record)
    baseline_query_ns = [
        int(record["elapsed_ns"]) + int(record.get("core_ns", 0))
        for record in records
        if record["backend"] == "internal-dpll"
    ]
    policies = []
    for threshold in THRESHOLDS:
        ratios = []
        total = 0
        candidate_count = 0
        material_losses: list[dict[str, object]] = []
        selected_query_ns: list[int] = []
        for key, internal in baseline.items():
            use_candidate = int(internal["clauses"]) >= threshold
            chosen = selected[key] if use_candidate else internal
            chosen_cost = int(chosen["cost_ns"])
            internal_cost = int(internal["cost_ns"])
            total += chosen_cost
            candidate_count += int(use_candidate)
            if internal_cost > 0:
                ratio = chosen_cost / internal_cost
                ratios.append(ratio)
                if ratio > 1.25:
                    material_losses.append(
                        {
                            "session": key[0],
                            "repetition": key[1],
                            "clauses": int(internal["clauses"]),
                            "ratio": ratio,
                            "internal_cost_ns": internal_cost,
                            "candidate_cost_ns": chosen_cost,
                        }
                    )
            selected_backend = candidate if use_candidate else "internal-dpll"
            selected_query_ns.extend(
                int(record["elapsed_ns"]) + int(record.get("core_ns", 0))
                for record in queries_by_session[
                    (selected_backend, key[0], key[1])
                ]
            )
        policies.append(
            {
                "backend": candidate,
                "threshold_clauses": threshold,
                "sessions": len(baseline),
                "candidate_sessions": candidate_count,
                "total_cost_ns": total,
                "baseline_total_cost_ns": baseline_total,
                "total_ratio_to_internal": (
                    total / baseline_total if baseline_total else None
                ),
                "median_session_ratio": (
                    statistics.median(ratios) if ratios else None
                ),
                "p95_session_ratio": percentile(ratios, 0.95),
                "selected_query_p95_ns": percentile(selected_query_ns, 0.95),
                "internal_query_p95_ns": percentile(baseline_query_ns, 0.95),
                "selected_query_p95_ratio": (
                    percentile(selected_query_ns, 0.95)
                    / percentile(baseline_query_ns, 0.95)
                    if baseline_query_ns and selected_query_ns
                    else None
                ),
                "material_loss_sessions": len(material_losses),
                "material_loss_examples": sorted(
                    material_losses,
                    key=lambda loss: float(loss["ratio"]),
                    reverse=True,
                )[:20],
            }
        )
    return policies


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--candidate", action="append", required=True)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    records = load(arguments.results)
    policies = [
        policy
        for candidate in arguments.candidate
        for policy in evaluate(records, candidate)
    ]
    ranked = sorted(
        policies,
        key=lambda policy: (
            float(policy["total_ratio_to_internal"]),
            int(policy["material_loss_sessions"]),
            str(policy["backend"]),
            int(policy["threshold_clauses"]),
        ),
    )
    result = {
        "schema": 1,
        "thresholds": list(THRESHOLDS),
        "policies": policies,
        "best": ranked[0],
    }
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
