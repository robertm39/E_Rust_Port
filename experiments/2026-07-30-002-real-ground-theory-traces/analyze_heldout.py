#!/usr/bin/env python3
"""Aggregate the frozen held-out trace and backend advancement gates."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


PARTITIONS = ("validation", "test")
MINIMUM_QUERIES = 40
MINIMUM_FAMILIES = 2
MINIMUM_PRUNES = 20
MINIMUM_PRUNE_RATE = 0.05
MINIMUM_IMPROVED_WORKLOADS = 3
MINIMUM_NODE_REDUCTION = 0.10
NATIVE_P95_LIMIT_NS = 250_000


class AnalysisError(RuntimeError):
    """An input artifact is missing or internally inconsistent."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def control_flow_events(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Remove theory telemetry while preserving every control-flow field."""
    return [
        {
            key: value
            for key, value in event.items()
            if not key.startswith("theory_")
        }
        for event in events
    ]


def capture_determinism(
    left: dict[str, Any],
    right: dict[str, Any],
) -> dict[str, Any]:
    left_records = {record["problem_id"]: record for record in left["records"]}
    right_records = {
        record["problem_id"]: record for record in right["records"]
    }
    if left_records.keys() != right_records.keys():
        raise AnalysisError("repeat capture source sets differ")
    comparisons = []
    stable_fields = (
        "source_sha256",
        "return_code",
        "timed_out",
        "stdout_bytes",
        "stdout_sha256",
        "stderr_bytes",
        "stderr_sha256",
    )
    for problem_id in sorted(left_records):
        first = left_records[problem_id]
        second = right_records[problem_id]
        mismatches = [
            field
            for field in stable_fields
            if first[field] != second[field]
        ]
        comparisons.append(
            {
                "problem_id": problem_id,
                "stable": not mismatches,
                "mismatches": mismatches,
            }
        )
    return {
        "sources": len(comparisons),
        "comparisons": comparisons,
        "passed": all(item["stable"] for item in comparisons),
    }


def source_effects(
    artifact_root: Path,
    partition: str,
) -> list[dict[str, Any]]:
    trace_root = artifact_root / f"{partition}-traces"
    reference_root = artifact_root / f"{partition}-reference"
    trace = load(trace_root / "trace-build.json")
    reference = load(reference_root / "reference-batch.json")
    baseline_records = {
        record["problem_id"]: record for record in trace["records"]
    }
    reference_records = {
        record["problem_id"]: record for record in reference["records"]
    }
    if baseline_records.keys() != reference_records.keys():
        raise AnalysisError(f"{partition} source sets differ")
    effects = []
    for problem_id in sorted(baseline_records):
        baseline = baseline_records[problem_id]
        candidate = reference_records[problem_id]
        baseline_trace = load(trace_root / problem_id / "trace.json")
        candidate_trace = load(
            reference_root / problem_id / "reference-search.json"
        )
        event_identity = (
            control_flow_events(baseline_trace["events"])
            == control_flow_events(candidate_trace["events"])
        )
        outcome_identity = (
            baseline_trace["status"] == candidate_trace["status"]
            and baseline_trace["nodes"] == candidate_trace["nodes"]
            and baseline_trace["leaves"] == candidate_trace["open_leaves"]
        )
        node_reduction = (
            (baseline["nodes"] - candidate["nodes"]) / baseline["nodes"]
            if baseline["nodes"]
            else 0.0
        )
        improved = candidate["closed"] or (
            node_reduction >= MINIMUM_NODE_REDUCTION
        )
        neutral = candidate["theory_prunes"] == 0
        effects.append(
            {
                "problem_id": problem_id,
                "partition": partition,
                "family": baseline["family"],
                "baseline_status": baseline["trace_status"],
                "candidate_status": candidate["search_status"],
                "baseline_nodes": baseline["nodes"],
                "candidate_nodes": candidate["nodes"],
                "baseline_leaves": baseline["leaves"],
                "candidate_open_leaves": candidate["open_leaves"],
                "eligible_query_occurrences": baseline["eligible_queries"],
                "unique_queries": candidate["query_count"],
                "cache_hits": candidate["theory_cache_hits"],
                "theory_prunes": candidate["theory_prunes"],
                "closed": candidate["closed"],
                "node_reduction": node_reduction,
                "improved": improved,
                "neutral": neutral,
                "neutral_event_identity": event_identity if neutral else None,
                "neutral_outcome_identity": outcome_identity if neutral else None,
            }
        )
    return effects


def backend_summary(
    artifact_root: Path,
    partition: str,
) -> dict[str, Any]:
    report_path = artifact_root / f"{partition}-backends/backend-report.json"
    report = load(report_path)
    if not report["all_correctness_gates_passed"]:
        raise AnalysisError(f"{partition} backend gates did not pass")
    corpus = artifact_root / f"{partition}-queries/query-corpus.json"
    if report["corpus"]["sha256"] != sha256(corpus):
        raise AnalysisError(f"{partition} backend corpus hash mismatch")
    return {
        "report_sha256": sha256(report_path),
        "queries": report["corpus"]["queries"],
        "all_correctness_gates_passed": True,
        "gates": report["gates"],
        "raw": {
            backend: report["backends"][backend]["raw"]
            for backend in ("native", "process", "ffi")
        },
        "native_p95_ns": report["timing"]["native_calls"]["p95_ns"],
        "process_p95_ns": report["timing"]["process_calls"]["p95_ns"],
        "ffi_p95_ns": report["timing"]["ffi_calls"]["p95_ns"],
        "replayed_certificates": report["replay"]["verified"],
        "mutations_rejected": sum(
            outcome["rejected"]
            for outcome in report["replay"]["mutations"]["mutations"].values()
        ),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact-root", required=True, type=Path)
    parser.add_argument("--selection-root", required=True, type=Path)
    parser.add_argument("--package-report", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    selections = {
        partition: load(
            arguments.selection_root / f"{partition}_sources.json"
        )
        for partition in PARTITIONS
    }
    captures = {
        partition: capture_determinism(
            load(arguments.artifact_root / f"{partition}-capture-a/capture.json"),
            load(arguments.artifact_root / f"{partition}-capture-b/capture.json"),
        )
        for partition in PARTITIONS
    }
    effects = [
        effect
        for partition in PARTITIONS
        for effect in source_effects(arguments.artifact_root, partition)
    ]
    backends = {
        partition: backend_summary(arguments.artifact_root, partition)
        for partition in PARTITIONS
    }
    package = load(arguments.package_report)
    train_report_path = (
        arguments.artifact_root / "train-backends/backend-report.json"
    )
    train_report = load(train_report_path)
    if not train_report["all_correctness_gates_passed"]:
        raise AnalysisError("training backend correctness gates did not pass")

    selected_families = sorted(
        {
            source["family"]
            for selection in selections.values()
            for source in selection["sources"]
        }
    )
    eligible_families = sorted(
        {
            effect["family"]
            for effect in effects
            if effect["eligible_query_occurrences"] > 0
        }
    )
    unique_queries = sum(item["queries"] for item in backends.values())
    eligible_occurrences = sum(
        effect["eligible_query_occurrences"] for effect in effects
    )
    cache_hits = sum(effect["cache_hits"] for effect in effects)
    checker_decisions = unique_queries + cache_hits
    theory_prunes = sum(effect["theory_prunes"] for effect in effects)
    improved = [effect for effect in effects if effect["improved"]]
    neutral = [effect for effect in effects if effect["neutral"]]
    neutral_no_loss = all(
        effect["neutral_event_identity"]
        and effect["neutral_outcome_identity"]
        for effect in neutral
    )
    backend_correctness = all(
        item["all_correctness_gates_passed"]
        for item in backends.values()
    )
    native_p95_ns = max(
        item["native_p95_ns"] for item in backends.values()
    )
    gates = {
        "repeat_capture_determinism": all(
            capture["passed"] for capture in captures.values()
        ),
        "minimum_unique_queries": unique_queries >= MINIMUM_QUERIES,
        "minimum_eligible_families": (
            len(eligible_families) >= MINIMUM_FAMILIES
        ),
        "backend_correctness_and_replay": backend_correctness,
        "minimum_verified_prunes": theory_prunes >= MINIMUM_PRUNES,
        "minimum_prune_rate": (
            theory_prunes / checker_decisions >= MINIMUM_PRUNE_RATE
            if checker_decisions
            else False
        ),
        "minimum_improved_workloads": (
            len(improved) >= MINIMUM_IMPROVED_WORKLOADS
        ),
        "neutral_no_loss": neutral_no_loss,
        "native_latency": native_p95_ns <= NATIVE_P95_LIMIT_NS,
        "native_package": package["passed"],
    }
    advancement_gates = (
        "minimum_unique_queries",
        "minimum_eligible_families",
        "backend_correctness_and_replay",
        "minimum_verified_prunes",
        "minimum_prune_rate",
        "minimum_improved_workloads",
        "neutral_no_loss",
        "native_latency",
        "native_package",
    )
    report = {
        "schema": "umlaut-real-ground-heldout-analysis-v1",
        "inputs": {
            "train_backend_report_sha256": sha256(train_report_path),
            "package_report_sha256": sha256(arguments.package_report),
            "selection_sha256": {
                partition: sha256(
                    arguments.selection_root / f"{partition}_sources.json"
                )
                for partition in PARTITIONS
            },
        },
        "corpus": {
            "selected_sources": sum(
                selection["source_count"]
                for selection in selections.values()
            ),
            "selected_families": selected_families,
            "eligible_families": eligible_families,
            "eligible_query_occurrences": eligible_occurrences,
            "unique_queries": unique_queries,
            "cache_hits": cache_hits,
            "checker_decisions": checker_decisions,
        },
        "capture_determinism": captures,
        "backends": backends,
        "effectiveness": {
            "verified_theory_prunes": theory_prunes,
            "prune_rate": (
                theory_prunes / checker_decisions
                if checker_decisions
                else 0.0
            ),
            "improved_workloads": len(improved),
            "improved_problem_ids": [
                effect["problem_id"] for effect in improved
            ],
            "closed_workloads": sum(effect["closed"] for effect in effects),
            "source_effects": effects,
        },
        "neutral": {
            "workloads": len(neutral),
            "event_and_outcome_identical": neutral_no_loss,
            "problem_ids": [
                effect["problem_id"] for effect in neutral
            ],
        },
        "performance": {
            "heldout_native_p95_ns": native_p95_ns,
            "native_p95_limit_ns": NATIVE_P95_LIMIT_NS,
            "package": package,
        },
        "gates": gates,
        "all_advancement_gates_passed": all(
            gates[name] for name in advancement_gates
        ),
        "verdict": (
            "advance_native_checker"
            if all(gates[name] for name in advancement_gates)
            else "do_not_advance"
        ),
        "production_changed": False,
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(gates, indent=2, sort_keys=True))
    print(f"verdict: {report['verdict']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
