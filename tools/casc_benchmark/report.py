#!/usr/bin/env python3
"""Summarize a resumable CASC-30 batch without treating either prover as an oracle."""

from __future__ import annotations

import argparse
import collections
import json
import math
import sys
from pathlib import Path
from typing import Any, Iterable, Sequence

from batch import BatchError, canonical_json, sha256_file
from manifest import ManifestError, load_manifest

REPORT_SCHEMA_VERSION = 1


def percentile(values: Sequence[float], quantile: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    position = (len(ordered) - 1) * quantile
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] * (1 - fraction) + ordered[upper] * fraction


def distribution(values: Iterable[float]) -> dict[str, float | int | None]:
    data = list(values)
    return {
        "count": len(data),
        "min": min(data) if data else None,
        "p50": percentile(data, 0.50),
        "p90": percentile(data, 0.90),
        "p95": percentile(data, 0.95),
        "max": max(data) if data else None,
        "mean": sum(data) / len(data) if data else None,
    }


def accepted_solve(result: dict[str, Any] | None) -> bool:
    return bool(
        result
        and result.get("classification") == "solved"
        and result.get("expected_status_match") is True
    )


def status_polarity(status: str | None) -> str:
    if status in {
        "Theorem",
        "Unsatisfiable",
        "ContradictoryAxioms",
        "TautologousConclusion",
    }:
        return "proof"
    if status in {"CounterSatisfiable", "Satisfiable"}:
        return "model"
    return "none"


def load_results(
    run_root: Path, contract: dict[str, Any]
) -> dict[tuple[str, str], dict[str, Any]]:
    results: dict[tuple[str, str], dict[str, Any]] = {}
    for path in sorted((run_root / "results").glob("*/*/*.json")):
        try:
            result = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise BatchError(f"cannot read result {path}: {error}") from error
        if result.get("contract_id") != contract["contract_id"]:
            raise BatchError(f"result belongs to another contract: {path}")
        key = (result.get("solver"), result.get("problem_id"))
        if not all(isinstance(value, str) for value in key) or key in results:
            raise BatchError(f"duplicate or invalid result identity in {path}")
        for field in ("stdout", "stderr"):
            artifact = run_root / result[f"{field}_path"]
            if not artifact.is_file():
                raise BatchError(f"missing result artifact: {artifact}")
            if sha256_file(artifact) != result[f"{field}_sha256"]:
                raise BatchError(f"result artifact hash mismatch: {artifact}")
        results[key] = result
    return results


def group_keys(record: dict[str, Any]) -> list[tuple[str, str]]:
    return [
        ("overall", "all"),
        ("division", record["division"]),
        ("category", record["category"]),
        ("split", record["holdout_split"]),
        ("difficulty_band", record["difficulty_band"]),
    ]


def solver_summary(
    solver: str,
    records: Sequence[dict[str, Any]],
    results: dict[tuple[str, str], dict[str, Any]],
) -> dict[str, Any]:
    groups: dict[str, dict[str, dict[str, Any]]] = collections.defaultdict(dict)
    classification_counts: collections.Counter[str] = collections.Counter()
    final_status_counts: collections.Counter[str] = collections.Counter()
    all_solver_results: list[dict[str, Any]] = []

    grouped_records: dict[tuple[str, str], list[dict[str, Any]]] = (
        collections.defaultdict(list)
    )
    for record in records:
        for key in group_keys(record):
            grouped_records[key].append(record)

    for (kind, name), members in sorted(grouped_records.items()):
        completed = [
            results[(solver, record["problem_id"])]
            for record in members
            if (solver, record["problem_id"]) in results
        ]
        solves = [result for result in completed if accepted_solve(result)]
        wrong = [
            result
            for result in completed
            if result.get("classification") == "solved"
            and not result.get("expected_status_match")
        ]
        groups[kind][name] = {
            "targeted": len(members),
            "completed": len(completed),
            "missing": len(members) - len(completed),
            "accepted_solved": len(solves),
            "wrong_status": len(wrong),
            "solve_rate": len(solves) / len(members) if members else None,
            "wall_seconds_solved": distribution(
                float(result["wall_seconds"]) for result in solves
            ),
            "cpu_seconds_solved": distribution(
                float(result["cpu_seconds"]) for result in solves
            ),
            "peak_memory_mib_completed": distribution(
                float(result["peak_memory_mib"]) for result in completed
            ),
        }
        if kind == "overall":
            all_solver_results = completed

    for result in all_solver_results:
        classification_counts[result["classification"]] += 1
        final_status_counts[result.get("final_szs_status") or "<none>"] += 1

    time_thresholds = [0.1, 1, 5, 15, 30, 60, 120, 240, 480]
    accepted = [result for result in all_solver_results if accepted_solve(result)]
    return {
        "groups": {kind: dict(sorted(values.items())) for kind, values in groups.items()},
        "classification_counts": dict(sorted(classification_counts.items())),
        "final_status_counts": dict(sorted(final_status_counts.items())),
        "time_curve": {
            str(threshold): sum(
                float(result["wall_seconds"]) <= threshold for result in accepted
            )
            for threshold in time_thresholds
        },
    }


def overlap_summary(
    left: str,
    right: str,
    records: Sequence[dict[str, Any]],
    results: dict[tuple[str, str], dict[str, Any]],
) -> dict[str, dict[str, dict[str, Any]]]:
    grouped_records: dict[tuple[str, str], list[dict[str, Any]]] = (
        collections.defaultdict(list)
    )
    for record in records:
        for key in group_keys(record):
            grouped_records[key].append(record)

    output: dict[str, dict[str, dict[str, Any]]] = collections.defaultdict(dict)
    for (kind, name), members in sorted(grouped_records.items()):
        counts: collections.Counter[str] = collections.Counter()
        status_pairs: collections.Counter[str] = collections.Counter()
        for record in members:
            left_result = results.get((left, record["problem_id"]))
            right_result = results.get((right, record["problem_id"]))
            if left_result is None or right_result is None:
                counts["incomplete"] += 1
                continue
            left_solved = accepted_solve(left_result)
            right_solved = accepted_solve(right_result)
            if left_solved and right_solved:
                counts["both_solved"] += 1
            elif left_solved:
                counts[f"{left}_only"] += 1
            elif right_solved:
                counts[f"{right}_only"] += 1
            else:
                counts["neither_solved"] += 1
            left_status = left_result.get("final_szs_status")
            right_status = right_result.get("final_szs_status")
            status_pairs[f"{left_status or '<none>'}|{right_status or '<none>'}"] += 1
            left_polarity = status_polarity(left_status)
            right_polarity = status_polarity(right_status)
            if (
                left_polarity != "none"
                and right_polarity != "none"
                and left_polarity != right_polarity
            ):
                counts["polarity_disagreements"] += 1
        output[kind][name] = {
            "targeted": len(members),
            "both_solved": counts["both_solved"],
            f"{left}_only": counts[f"{left}_only"],
            f"{right}_only": counts[f"{right}_only"],
            "neither_solved": counts["neither_solved"],
            "incomplete": counts["incomplete"],
            "polarity_disagreements": counts["polarity_disagreements"],
            "final_status_pairs": dict(sorted(status_pairs.items())),
        }
    return {kind: dict(sorted(values.items())) for kind, values in output.items()}


def build_report(
    manifest_path: Path, run_root: Path, *, require_complete: bool
) -> dict[str, Any]:
    metadata, manifest_records = load_manifest(manifest_path)
    try:
        contract = json.loads((run_root / "contract.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BatchError(f"cannot read run contract: {error}") from error
    if sha256_file(manifest_path) != contract["manifest_sha256"]:
        raise BatchError("report manifest does not match the run contract")
    by_id = {record["problem_id"]: record for record in manifest_records}
    try:
        selected = [by_id[problem_id] for problem_id in contract["selected_problem_ids"]]
    except KeyError as error:
        raise BatchError(f"run contract selects unknown problem {error}") from error
    solvers = sorted(contract["solvers"])
    results = load_results(run_root, contract)
    expected_results = len(selected) * len(solvers)
    missing_results = expected_results - len(results)
    if missing_results < 0:
        raise BatchError("run contains more results than its contract permits")
    if require_complete and missing_results:
        raise BatchError(
            f"run is incomplete: {len(results)}/{expected_results} results exist"
        )
    report: dict[str, Any] = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "kind": "umlaut-casc-benchmark-report",
        "contract_id": contract["contract_id"],
        "manifest_sha256": contract["manifest_sha256"],
        "canonical_full_selection": contract["canonical_full_selection"],
        "targeted_problems": len(selected),
        "expected_results": expected_results,
        "completed_results": len(results),
        "missing_results": missing_results,
        "complete": missing_results == 0,
        "official_context_warning": (
            "The checked-in CASC-30 CSVs describe official competition entries. "
            "This local pinned Vampire run is not claimed to reproduce Vampire's "
            "official CASC configuration or StarExec environment."
        ),
        "manifest_partition_counts": metadata["partition_counts"],
        "solvers": {
            solver: solver_summary(solver, selected, results) for solver in solvers
        },
    }
    if len(solvers) == 2:
        report["overlap"] = overlap_summary(
            solvers[0], solvers[1], selected, results
        )
    return report


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--allow-partial",
        action="store_true",
        help="write a report with explicit missing-result counts",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        run_root = arguments.run_root.resolve()
        output = (
            arguments.output.resolve()
            if arguments.output
            else run_root / "summary.json"
        )
        report = build_report(
            arguments.manifest.resolve(),
            run_root,
            require_complete=not arguments.allow_partial,
        )
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(canonical_json(report))
        print(
            f"OK: {report['completed_results']}/{report['expected_results']} "
            f"results; summary {output}"
        )
        return 0
    except (BatchError, ManifestError, OSError, ValueError, KeyError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
