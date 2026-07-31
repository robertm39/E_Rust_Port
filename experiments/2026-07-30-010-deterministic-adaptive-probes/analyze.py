#!/usr/bin/env python3
"""Verify and analyze deterministic adaptive-probe results."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence

import common
import run


def rounded(value: float) -> float:
    return round(value, 9)


def median_or_none(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return (
        rounded(float(statistics.median(materialized)))
        if materialized
        else None
    )


def load_contract(root: Path, phase: str) -> dict[str, Any]:
    contract = json.loads(
        (root / "contract.json").read_text(encoding="utf-8")
    )
    identifier = contract.get("contract_id")
    unsigned = {
        key: value for key, value in contract.items() if key != "contract_id"
    }
    if identifier != common.sha256_bytes(common.canonical_json(unsigned)):
        raise common.ExperimentError("contract hash mismatch")
    if (
        contract.get("kind")
        != "deterministic-adaptive-probe-contract"
        or contract.get("phase") != phase
        or contract.get("source_revision") != common.SOURCE_REVISION
    ):
        raise common.ExperimentError("contract identity mismatch")
    return contract


def verify_hash(
    path_value: str | None, expected: str | None, label: str
) -> None:
    if path_value is None or expected is None:
        if path_value is not None or expected is not None:
            raise common.ExperimentError(f"{label} hash pair is partial")
        return
    path = Path(path_value)
    if not path.is_file() or common.sha256_file(path) != expected:
        raise common.ExperimentError(f"{label} hash mismatch: {path}")


def load_telemetry(search: dict[str, Any]) -> dict[str, Any] | None:
    verify_hash(
        search.get("telemetry_path"),
        search.get("telemetry_sha256"),
        "telemetry",
    )
    path_value = search.get("telemetry_path")
    if path_value is None:
        return None
    try:
        return json.loads(Path(path_value).read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None


def verify_search(search: dict[str, Any], label: str) -> None:
    for path_key, hash_key in (
        ("output_path", "output_sha256"),
        ("stderr_path", "stderr_sha256"),
        ("timing_path", "timing_sha256"),
        ("telemetry_path", "telemetry_sha256"),
    ):
        verify_hash(search.get(path_key), search.get(hash_key), f"{label}:{path_key}")


def verify_gate(gate: dict[str, Any], label: str) -> None:
    for path_key, hash_key in (
        ("report_path", "report_sha256"),
        ("stdout_path", "stdout_sha256"),
        ("stderr_path", "stderr_sha256"),
    ):
        verify_hash(gate.get(path_key), gate.get(hash_key), f"{label}:{path_key}")


def load_results(
    root: Path, contract: dict[str, Any]
) -> list[dict[str, Any]]:
    paths = sorted((root / "runs").rglob("result.json"))
    expected = (
        len(contract["records"])
        * int(contract["repetitions"])
        * len(run.POLICIES)
    )
    if len(paths) != expected:
        raise common.ExperimentError(
            f"found {len(paths)} results, expected {expected}"
        )
    expected_records = {
        str(record["problem_id"]): record
        for record in contract["records"]
    }
    coordinates = set()
    results = []
    for path in paths:
        result = json.loads(path.read_text(encoding="utf-8"))
        coordinate = (
            str(result["policy"]),
            str(result["problem_id"]),
            int(result["repetition"]),
        )
        if coordinate in coordinates:
            raise common.ExperimentError(
                f"duplicate coordinate: {coordinate}"
            )
        coordinates.add(coordinate)
        expected_record = expected_records.get(str(result["problem_id"]))
        if (
            result.get("contract_id") != contract["contract_id"]
            or result.get("policy") not in run.POLICIES
            or expected_record is None
            or result.get("problem_sha256")
            != expected_record["sha256"]
            or result.get("binary_sha256")
            != contract["binary"]["sha256"]
        ):
            raise common.ExperimentError(
                f"result identity mismatch: {path}"
            )
        for index, search in enumerate(result["phases"], start=1):
            verify_search(search, f"{coordinate}:phase-{index}")
        replay = result["proof_replay"]
        if replay is not None:
            verify_search(replay, f"{coordinate}:proof-replay")
            verify_gate(replay["gate"], f"{coordinate}:proof-gate")
        result["_path"] = str(path)
        results.append(result)
    return results


def proof_status(result: dict[str, Any]) -> bool:
    return result["szs_status"] in common.PROOF_STATUSES


def telemetry_schema_valid(value: dict[str, Any] | None) -> bool:
    return bool(
        value is not None
        and value.get("schema") == "umlaut.search-telemetry"
        and value.get("schema_version") == 1
        and value.get("record_kind") in {"checkpoint", "final"}
    )


def metric(result: dict[str, Any], name: str) -> float | None:
    value = result["resources"].get(name)
    return float(value) if value is not None else None


def summarize_policy(
    results: Sequence[dict[str, Any]], policy: str, repetitions: int
) -> dict[str, Any]:
    selected = [result for result in results if result["policy"] == policy]
    by_problem: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in selected:
        by_problem[str(result["problem_id"])].append(result)
    reproducible = []
    one_repeat = []
    for problem, records in sorted(by_problem.items()):
        solved = [proof_status(result) for result in records]
        if len(solved) != repetitions:
            raise common.ExperimentError(
                f"{policy}/{problem} repetition count mismatch"
            )
        if all(solved):
            reproducible.append(problem)
        elif any(solved):
            one_repeat.append(problem)
    return {
        "run_count": len(selected),
        "reproducible_solves": reproducible,
        "one_repeat_solves": one_repeat,
        "median_total_cpu_seconds": median_or_none(
            value
            for result in selected
            if (value := metric(result, "total_cpu_seconds")) is not None
        ),
        "median_wall_seconds": median_or_none(
            value
            for result in selected
            if (value := metric(result, "wall_seconds")) is not None
        ),
        "median_peak_rss_kib": median_or_none(
            value
            for result in selected
            if (value := metric(result, "peak_rss_kib")) is not None
        ),
    }


def paired_comparison(
    results: Sequence[dict[str, Any]],
    candidate: str,
    baseline: str,
    repetitions: int,
) -> dict[str, Any]:
    mappings = {
        policy: {
            (str(result["problem_id"]), int(result["repetition"])): result
            for result in results
            if result["policy"] == policy
        }
        for policy in (candidate, baseline)
    }
    if set(mappings[candidate]) != set(mappings[baseline]):
        raise common.ExperimentError(
            f"{candidate}/{baseline} coordinates differ"
        )
    ratios = {
        "total_cpu_seconds": [],
        "wall_seconds": [],
        "peak_rss_kib": [],
    }
    common_coordinates = 0
    for coordinate in sorted(mappings[candidate]):
        left = mappings[candidate][coordinate]
        right = mappings[baseline][coordinate]
        if proof_status(left) and proof_status(right):
            common_coordinates += 1
            for name in ratios:
                left_value = metric(left, name)
                right_value = metric(right, name)
                if (
                    left_value is not None
                    and right_value is not None
                    and right_value > 0.0
                ):
                    ratios[name].append(left_value / right_value)
    left_summary = summarize_policy(
        results, candidate, repetitions
    )
    right_summary = summarize_policy(
        results, baseline, repetitions
    )
    left_solves = set(left_summary["reproducible_solves"])
    right_solves = set(right_summary["reproducible_solves"])
    return {
        "baseline": baseline,
        "common_solved_repetition_coordinates": common_coordinates,
        "median_cpu_ratio": median_or_none(
            ratios["total_cpu_seconds"]
        ),
        "median_wall_ratio": median_or_none(ratios["wall_seconds"]),
        "median_peak_rss_ratio": median_or_none(
            ratios["peak_rss_kib"]
        ),
        "candidate_only_reproducible_solves": sorted(
            left_solves - right_solves
        ),
        "baseline_only_reproducible_solves": sorted(
            right_solves - left_solves
        ),
    }


def observability(results: Sequence[dict[str, Any]]) -> dict[str, Any]:
    policies = {
        "probe_with_telemetry",
        "static_global_restart",
        "static_goal",
        "adaptive",
    }
    denominator = 0
    successes = 0
    missing = []
    invalid = []
    for result in results:
        if result["policy"] not in policies:
            continue
        probe = result["phases"][0]
        if probe["szs_status"] in common.PROOF_STATUSES:
            continue
        denominator += 1
        telemetry = load_telemetry(probe)
        coordinate = (
            f"{result['policy']}:{result['problem_id']}:"
            f"r{result['repetition']}"
        )
        signal = common.signal_from_telemetry(telemetry)
        complete = signal["fallback_reason"] in {
            None,
            "insufficient_processed_clauses",
        }
        if telemetry_schema_valid(telemetry) and complete:
            successes += 1
        elif probe["telemetry_path"] is None:
            missing.append(coordinate)
        else:
            invalid.append(coordinate)
    return {
        "non_proof_probe_count": denominator,
        "schema_valid_count": successes,
        "success_rate": (
            rounded(successes / denominator) if denominator else 1.0
        ),
        "missing": missing,
        "invalid": invalid,
    }


def overhead(
    results: Sequence[dict[str, Any]]
) -> tuple[dict[str, Any], list[str]]:
    mappings = {
        policy: {
            (str(result["problem_id"]), int(result["repetition"])): result
            for result in results
            if result["policy"] == policy
        }
        for policy in (
            "probe_with_telemetry",
            "probe_without_telemetry",
        )
    }
    if set(mappings["probe_with_telemetry"]) != set(
        mappings["probe_without_telemetry"]
    ):
        raise common.ExperimentError("overhead coordinates differ")
    ratios = {
        "total_cpu_seconds": [],
        "wall_seconds": [],
        "peak_rss_kib": [],
    }
    status_mismatches = []
    processed_mismatches = []
    rows = []
    for coordinate in sorted(mappings["probe_with_telemetry"]):
        enabled = mappings["probe_with_telemetry"][coordinate]
        disabled = mappings["probe_without_telemetry"][coordinate]
        enabled_probe = enabled["phases"][0]
        disabled_probe = disabled["phases"][0]
        if enabled["szs_status"] != disabled["szs_status"]:
            status_mismatches.append(coordinate)
        enabled_processed = enabled_probe["processed_clauses"]
        disabled_processed = disabled_probe["processed_clauses"]
        if (
            enabled_processed is None
            or disabled_processed is None
            or enabled_processed != disabled_processed
        ):
            processed_mismatches.append(coordinate)
        row = {
            "problem_id": coordinate[0],
            "repetition": coordinate[1],
            "status": enabled["szs_status"],
            "processed_clauses": enabled_processed,
        }
        for name in ratios:
            left = metric(enabled, name)
            right = metric(disabled, name)
            ratio = (
                left / right
                if left is not None and right is not None and right > 0.0
                else None
            )
            row[f"{name}_ratio"] = ratio
            if ratio is not None:
                ratios[name].append(ratio)
        rows.append(row)
    result = {
        "coordinate_count": len(rows),
        "status_mismatches": [
            {"problem_id": item[0], "repetition": item[1]}
            for item in status_mismatches
        ],
        "processed_mismatches": [
            {"problem_id": item[0], "repetition": item[1]}
            for item in processed_mismatches
        ],
        "median_cpu_ratio": median_or_none(
            ratios["total_cpu_seconds"]
        ),
        "median_wall_ratio": median_or_none(ratios["wall_seconds"]),
        "median_peak_rss_ratio": median_or_none(
            ratios["peak_rss_kib"]
        ),
        "rows": rows,
    }
    failures = [
        *(
            f"overhead_status_mismatch:{problem}:r{repetition}"
            for problem, repetition in status_mismatches
        ),
        *(
            f"overhead_processed_mismatch:{problem}:r{repetition}"
            for problem, repetition in processed_mismatches
        ),
    ]
    return result, failures


def adaptive_traces(
    results: Sequence[dict[str, Any]], repetitions: int
) -> tuple[list[dict[str, Any]], list[str]]:
    traces = []
    by_problem: dict[str, list[str]] = defaultdict(list)
    failures = []
    for result in results:
        if result["policy"] != "adaptive":
            continue
        decision = result["decision"]
        probe = result["phases"][0]
        telemetry = load_telemetry(probe)
        if probe["szs_status"] in common.PROOF_STATUSES:
            expected_branch = "probe_solved"
        else:
            expected = common.choose_branch(telemetry)
            expected_branch = str(expected["branch"])
            for key in (
                "threshold",
                "valid",
                "fallback_reason",
                "processed_non_trivial",
                "generated_non_trivial",
                "clause_growth",
                "passive_pressure",
            ):
                if decision.get(key) != expected.get(key):
                    failures.append(
                        f"adaptive_decision_mismatch:{result['problem_id']}:"
                        f"r{result['repetition']}:{key}"
                    )
        if decision is None or decision.get("branch") != expected_branch:
            failures.append(
                f"adaptive_branch_mismatch:{result['problem_id']}:"
                f"r{result['repetition']}"
            )
            branch = "missing"
        else:
            branch = str(decision["branch"])
        by_problem[str(result["problem_id"])].append(branch)
        traces.append(
            {
                "problem_id": result["problem_id"],
                "repetition": result["repetition"],
                **(decision or {"branch": "missing"}),
            }
        )
    unstable = [
        problem
        for problem, branches in sorted(by_problem.items())
        if repetitions > 1 and len(set(branches)) != 1
    ]
    failures.extend(f"unstable_branch:{problem}" for problem in unstable)
    return traces, failures


def phase_analysis(root: Path, phase: str) -> dict[str, Any]:
    contract = load_contract(root, phase)
    results = load_results(root, contract)
    repetitions = int(contract["repetitions"])
    failures = [
        f"{result['policy']}:{result['problem_id']}:"
        f"r{result['repetition']}:{failure}"
        for result in results
        for failure in result["correctness_failures"]
    ]
    overhead_report, overhead_failures = overhead(results)
    failures.extend(overhead_failures)
    traces, trace_failures = adaptive_traces(results, repetitions)
    failures.extend(trace_failures)
    policies = {
        policy: summarize_policy(results, policy, repetitions)
        for policy in run.POLICIES
    }
    adaptive_solves = set(
        policies["adaptive"]["reproducible_solves"]
    )
    controls = {
        name: paired_comparison(
            results, "adaptive", name, repetitions
        )
        for name in ("static_global_restart", "static_goal")
    }
    control_union = set()
    for name in controls:
        control_union.update(
            policies[name]["reproducible_solves"]
        )
    analysis = {
        "schema_version": 1,
        "kind": "deterministic-adaptive-probe-analysis",
        "phase": phase,
        "contract_id": contract["contract_id"],
        "coordinate_count": len(results),
        "problem_count": len(contract["records"]),
        "repetitions": repetitions,
        "correctness_failures": sorted(set(failures)),
        "proof_replay_count": sum(proof_status(result) for result in results),
        "observability": observability(results),
        "overhead": overhead_report,
        "adaptive_traces": traces,
        "maximum_decision_wall_seconds": max(
            (
                float(result["decision_wall_seconds"])
                for result in results
                if result["policy"] == "adaptive"
            ),
            default=0.0,
        ),
        "maximum_decision_cpu_seconds": max(
            (
                float(result["decision_cpu_seconds"])
                for result in results
                if result["policy"] == "adaptive"
            ),
            default=0.0,
        ),
        "policies": policies,
        "adaptive_vs_controls": controls,
        "adaptive_unique_vs_both_controls": sorted(
            adaptive_solves - control_union
        ),
    }
    analysis["analysis_id"] = common.sha256_bytes(
        common.canonical_json(analysis)
    )
    return analysis


def efficiency_qualifies(report: dict[str, Any]) -> bool:
    comparisons = report["adaptive_vs_controls"]
    return all(
        comparison["common_solved_repetition_coordinates"] >= 4
        and comparison["median_cpu_ratio"] is not None
        and comparison["median_cpu_ratio"] <= 0.95
        for comparison in comparisons.values()
    )


def overhead_qualifies(report: dict[str, Any]) -> bool:
    overhead_report = report["overhead"]
    return all(
        overhead_report[name] is not None
        and overhead_report[name] <= 1.05
        for name in (
            "median_cpu_ratio",
            "median_wall_ratio",
            "median_peak_rss_ratio",
        )
    )


def final_decision(
    validation: dict[str, Any], test: dict[str, Any]
) -> dict[str, Any]:
    reports = (validation, test)
    correctness = [
        f"{report['phase']}:{failure}"
        for report in reports
        for failure in report["correctness_failures"]
    ]
    observability_passed = all(
        report["observability"]["success_rate"] >= 0.95
        for report in reports
    )
    branch_stable = all(
        not any(
            failure.startswith("unstable_branch:")
            for failure in report["correctness_failures"]
        )
        for report in reports
    )
    overhead_passed = all(overhead_qualifies(report) for report in reports)
    no_loss = all(
        not comparison["baseline_only_reproducible_solves"]
        for report in reports
        for comparison in report["adaptive_vs_controls"].values()
    )
    unique_test = test["adaptive_unique_vs_both_controls"]
    efficacy_passed = (
        len(unique_test) >= 2
        or all(efficiency_qualifies(report) for report in reports)
    )
    passed = (
        not correctness
        and observability_passed
        and branch_stable
        and overhead_passed
        and no_loss
        and efficacy_passed
    )
    if correctness:
        reason = "correctness_failure"
    elif not observability_passed:
        reason = "observability_below_95_percent"
    elif not branch_stable:
        reason = "adaptive_branch_instability"
    elif not overhead_passed:
        reason = "telemetry_overhead_exceeded"
    elif not no_loss:
        reason = "adaptive_reproducible_loss"
    elif not efficacy_passed:
        reason = "no_heldout_efficacy"
    else:
        reason = "all_gates_passed"
    return {
        "schema_version": 1,
        "kind": "deterministic-adaptive-probe-final-decision",
        "verdict": "continue" if passed else "stop",
        "reason": reason,
        "correctness_failures": correctness,
        "observability_passed": observability_passed,
        "branch_stable": branch_stable,
        "overhead_passed": overhead_passed,
        "no_loss": no_loss,
        "efficacy_passed": efficacy_passed,
        "test_unique_solves": unique_test,
        "validation_analysis_id": validation["analysis_id"],
        "test_analysis_id": test["analysis_id"],
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path)
    parser.add_argument(
        "--phase", choices=("train", "validation", "test")
    )
    parser.add_argument("--validation", type=Path)
    parser.add_argument("--test", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.root is not None and arguments.phase is not None:
        report = phase_analysis(arguments.root.resolve(), arguments.phase)
    elif (
        arguments.validation is not None
        and arguments.test is not None
        and arguments.root is None
        and arguments.phase is None
    ):
        validation = json.loads(
            arguments.validation.read_text(encoding="utf-8")
        )
        test = json.loads(arguments.test.read_text(encoding="utf-8"))
        report = final_decision(validation, test)
    else:
        raise common.ExperimentError(
            "provide --root/--phase or --validation/--test"
        )
    common.atomic_json(arguments.output.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except common.ExperimentError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
