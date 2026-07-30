#!/usr/bin/env python3
"""Verify and summarize the bounded online-adaptation experiment."""

from __future__ import annotations

import argparse
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence

import common


EXPERIMENT = "2026-07-29-020-online-stagnation-adaptation"
CALIBRATION_POLICIES = {
    "global_full",
    "goal_full",
    "probe",
    "continuation_global",
    "continuation_goal",
}
EVALUATION_POLICIES = {
    "global_full",
    "goal_full",
    "static_global_restart",
    "static_goal",
    "adaptive",
}


def rounded(value: float) -> float:
    return round(value, 9)


def median_or_none(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return (
        rounded(float(statistics.median(materialized)))
        if materialized
        else None
    )


def load_contract(run_root: Path, phase: str) -> dict[str, Any]:
    path = run_root / "contract.json"
    contract = json.loads(path.read_text(encoding="utf-8"))
    identifier = contract.get("contract_id")
    body = {
        key: value
        for key, value in contract.items()
        if key != "contract_id"
    }
    if identifier != common.sha256_bytes(common.canonical_json(body)):
        raise common.ExperimentError("run contract ID is invalid")
    if (
        contract.get("experiment") != EXPERIMENT
        or contract.get("phase") != phase
        or contract.get("source_revision") != common.SOURCE_REVISION
    ):
        raise common.ExperimentError("run contract identity changed")
    return contract


def load_telemetry(
    result_path: Path, phase: dict[str, Any]
) -> dict[str, Any] | None:
    artifact_dir = result_path.parent / phase["artifact_directory"]
    stdout = artifact_dir / "stdout.pcl"
    stderr = artifact_dir / "stderr.txt"
    if common.sha256_file(stdout) != phase["stdout_sha256"]:
        raise common.ExperimentError(f"stdout hash mismatch: {stdout}")
    if common.sha256_file(stderr) != phase["stderr_sha256"]:
        raise common.ExperimentError(f"stderr hash mismatch: {stderr}")
    observed_steps = common.proof_step_count(
        stdout.read_text(encoding="utf-8", errors="replace")
    )
    if observed_steps != int(phase["proof_steps"]):
        raise common.ExperimentError(f"proof-step mismatch: {stdout}")
    telemetry_path = artifact_dir / "telemetry.json"
    expected = phase["telemetry_sha256"]
    if expected is None:
        if telemetry_path.exists():
            raise common.ExperimentError(
                f"unrecorded telemetry artifact: {telemetry_path}"
            )
        return None
    if common.sha256_file(telemetry_path) != expected:
        raise common.ExperimentError(
            f"telemetry hash mismatch: {telemetry_path}"
        )
    try:
        telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None
    if (
        telemetry.get("schema") != "umlaut.search-telemetry"
        or telemetry.get("schema_version") != 1
    ):
        return None
    return telemetry


def load_results(
    run_root: Path, contract: dict[str, Any]
) -> list[dict[str, Any]]:
    expected_policies = set(contract["policies"])
    expected_records = {
        str(record["problem_id"]): record
        for record in contract["records"]
    }
    expected_repetitions = int(contract["repetitions"])
    results: list[dict[str, Any]] = []
    coordinates: set[tuple[str, str, int]] = set()
    for result_path in sorted((run_root / "runs").rglob("result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result.get("contract_id") != contract["contract_id"]:
            raise common.ExperimentError(
                f"result contract mismatch: {result_path}"
            )
        policy = str(result["policy"])
        problem = str(result["problem_id"])
        repetition = int(result["repetition"])
        if (
            policy not in expected_policies
            or problem not in expected_records
            or not 1 <= repetition <= expected_repetitions
        ):
            raise common.ExperimentError(
                f"unexpected result coordinate: {result_path}"
            )
        expected = expected_records[problem]
        if (
            result["problem_sha256"] != expected["sha256"]
            or result["binary_sha256"] != contract["binary_sha256"]
            or result["phase"] != contract["phase"]
        ):
            raise common.ExperimentError(
                f"result identity mismatch: {result_path}"
            )
        coordinate = (policy, problem, repetition)
        if coordinate in coordinates:
            raise common.ExperimentError(
                f"duplicate result coordinate: {coordinate}"
            )
        coordinates.add(coordinate)
        phase_telemetry = [
            load_telemetry(result_path, item)
            for item in result["phases"]
        ]
        result["_result_path"] = str(result_path)
        result["_phase_telemetry"] = phase_telemetry
        results.append(result)
    expected_count = (
        len(expected_policies)
        * len(expected_records)
        * expected_repetitions
    )
    if len(results) != expected_count:
        raise common.ExperimentError(
            f"found {len(results)} results, expected {expected_count}"
        )
    return results


def proof_status(result: dict[str, Any]) -> bool:
    return result.get("szs_status") in common.PROOF_STATUSES


def policy_cpu(result: dict[str, Any]) -> float | None:
    value = result.get("telemetry_cpu_seconds")
    return float(value) if value is not None else None


def summarize_policy(
    results: Sequence[dict[str, Any]],
    policy: str,
    repetitions: int,
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
                f"{policy}/{problem} repetition count changed"
            )
        if all(solved):
            reproducible.append(problem)
        elif any(solved):
            one_repeat.append(problem)
    return {
        "run_count": len(selected),
        "reproducible_solves": reproducible,
        "one_repeat_solves": one_repeat,
        "median_cpu_seconds": median_or_none(
            value
            for result in selected
            if (value := policy_cpu(result)) is not None
        ),
        "median_wall_seconds": median_or_none(
            float(result["policy_wall_seconds"]) for result in selected
        ),
        "median_configured_cpu_seconds": median_or_none(
            float(result["configured_cpu_seconds"]) for result in selected
        ),
    }


def paired_comparison(
    results: Sequence[dict[str, Any]],
    candidate: str,
    baseline: str,
    repetitions: int,
) -> dict[str, Any]:
    by_policy = {
        policy: {
            (str(result["problem_id"]), int(result["repetition"])): result
            for result in results
            if result["policy"] == policy
        }
        for policy in (candidate, baseline)
    }
    if set(by_policy[candidate]) != set(by_policy[baseline]):
        raise common.ExperimentError(
            f"{candidate}/{baseline} coordinates differ"
        )
    ratios = []
    common_coordinates = 0
    for coordinate in sorted(by_policy[candidate]):
        left = by_policy[candidate][coordinate]
        right = by_policy[baseline][coordinate]
        if proof_status(left) and proof_status(right):
            common_coordinates += 1
            left_cpu = policy_cpu(left)
            right_cpu = policy_cpu(right)
            if (
                left_cpu is not None
                and right_cpu is not None
                and right_cpu > 0.0
            ):
                ratios.append(left_cpu / right_cpu)
    candidate_summary = summarize_policy(
        results, candidate, repetitions
    )
    baseline_summary = summarize_policy(results, baseline, repetitions)
    candidate_solves = set(candidate_summary["reproducible_solves"])
    baseline_solves = set(baseline_summary["reproducible_solves"])
    return {
        "baseline": baseline,
        "candidate": candidate,
        "common_solved_repetition_coordinates": common_coordinates,
        "median_common_solve_cpu_ratio": median_or_none(ratios),
        "candidate_only_reproducible_solves": sorted(
            candidate_solves - baseline_solves
        ),
        "baseline_only_reproducible_solves": sorted(
            baseline_solves - candidate_solves
        ),
    }


def combine_primitives(
    probe: dict[str, Any],
    continuation: dict[str, Any],
    policy: str,
    decision: dict[str, Any] | None,
) -> dict[str, Any]:
    if proof_status(probe):
        status = probe["szs_status"]
        steps = probe["proof_steps"]
        phases = [probe]
    else:
        status = continuation["szs_status"]
        steps = continuation["proof_steps"]
        phases = [probe, continuation]
    cpu_values = [policy_cpu(result) for result in phases]
    return {
        "policy": policy,
        "problem_id": probe["problem_id"],
        "repetition": probe["repetition"],
        "szs_status": status,
        "proof_steps": steps,
        "telemetry_cpu_seconds": (
            sum(float(value) for value in cpu_values if value is not None)
            if all(value is not None for value in cpu_values)
            else None
        ),
        "policy_wall_seconds": sum(
            float(result["policy_wall_seconds"]) for result in phases
        ),
        "configured_cpu_seconds": sum(
            int(result["configured_cpu_seconds"]) for result in phases
        ),
        "decision": decision,
    }


def calibration_matrix(
    results: Sequence[dict[str, Any]],
) -> dict[tuple[str, int], dict[str, dict[str, Any]]]:
    matrix: dict[
        tuple[str, int], dict[str, dict[str, Any]]
    ] = defaultdict(dict)
    for result in results:
        matrix[
            (str(result["problem_id"]), int(result["repetition"]))
        ][str(result["policy"])] = result
    if any(set(arms) != CALIBRATION_POLICIES for arms in matrix.values()):
        raise common.ExperimentError(
            "calibration primitive matrix is incomplete"
        )
    return matrix


def replay_calibration(
    matrix: dict[tuple[str, int], dict[str, dict[str, Any]]],
    threshold: float,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    simulated = []
    traces = []
    for coordinate, arms in sorted(matrix.items()):
        probe = arms["probe"]
        telemetry = probe["_phase_telemetry"][0]
        decision = common.choose_branch(telemetry, threshold)
        continuation_name = (
            "continuation_goal"
            if decision["branch"] == "goal"
            else "continuation_global"
        )
        combined = combine_primitives(
            probe,
            arms[continuation_name],
            "adaptive",
            decision,
        )
        simulated.append(combined)
        traces.append(
            {
                "problem_id": coordinate[0],
                "repetition": coordinate[1],
                **decision,
            }
        )
    return simulated, traces


def replay_static(
    matrix: dict[tuple[str, int], dict[str, dict[str, Any]]],
    branch: str,
) -> list[dict[str, Any]]:
    continuation_name = f"continuation_{branch}"
    policy = (
        "static_goal"
        if branch == "goal"
        else "static_global_restart"
    )
    return [
        combine_primitives(
            arms["probe"],
            arms[continuation_name],
            policy,
            None,
        )
        for _coordinate, arms in sorted(matrix.items())
    ]


def calibration_selection(
    results: Sequence[dict[str, Any]],
    contract: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    matrix = calibration_matrix(results)
    static_global = replay_static(matrix, "global")
    static_goal = replay_static(matrix, "goal")
    repetitions = int(contract["repetitions"])
    global_summary = summarize_policy(
        static_global, "static_global_restart", repetitions
    )
    goal_summary = summarize_policy(static_goal, "static_goal", repetitions)
    global_solves = set(global_summary["reproducible_solves"])
    goal_solves = set(goal_summary["reproducible_solves"])
    candidates = []
    traces_by_threshold = {}
    for threshold in common.THRESHOLDS:
        adaptive, traces = replay_calibration(matrix, threshold)
        traces_by_threshold[str(threshold)] = traces
        summary = summarize_policy(adaptive, "adaptive", repetitions)
        solves = set(summary["reproducible_solves"])
        comparison_goal = paired_comparison(
            [*adaptive, *static_goal],
            "adaptive",
            "static_goal",
            repetitions,
        )
        common_with_both = []
        global_by_coordinate = {
            (str(result["problem_id"]), int(result["repetition"])): result
            for result in static_global
        }
        goal_by_coordinate = {
            (str(result["problem_id"]), int(result["repetition"])): result
            for result in static_goal
        }
        for result in adaptive:
            coordinate = (
                str(result["problem_id"]),
                int(result["repetition"]),
            )
            if (
                proof_status(result)
                and proof_status(global_by_coordinate[coordinate])
                and proof_status(goal_by_coordinate[coordinate])
                and policy_cpu(result) is not None
            ):
                common_with_both.append(float(policy_cpu(result)))
        candidate = {
            "threshold": threshold,
            "reproducible_solves": summary["reproducible_solves"],
            "reproducible_solve_count": len(solves),
            "losses_vs_static_global": sorted(global_solves - solves),
            "wins_vs_static_goal": sorted(solves - goal_solves),
            "median_cpu_on_coordinates_common_to_both_comparators": (
                median_or_none(common_with_both)
            ),
            "comparison_vs_static_goal": comparison_goal,
            "branch_counts": dict(
                sorted(
                    {
                        branch: sum(
                            trace["branch"] == branch for trace in traces
                        )
                        for branch in ("global", "goal")
                    }.items()
                )
            ),
        }
        candidates.append(candidate)

    def rank(candidate: dict[str, Any]) -> tuple[Any, ...]:
        median_cpu = candidate[
            "median_cpu_on_coordinates_common_to_both_comparators"
        ]
        return (
            int(candidate["reproducible_solve_count"]),
            -len(candidate["losses_vs_static_global"]),
            len(candidate["wins_vs_static_goal"]),
            -(
                float(median_cpu)
                if median_cpu is not None
                else math.inf
            ),
            float(candidate["threshold"]),
        )

    selected = max(candidates, key=rank)
    return {
        "selected_threshold": float(selected["threshold"]),
        "candidates": candidates,
        "static_global_restart": global_summary,
        "static_goal": goal_summary,
        "selection_rule": (
            "max solves; min loss vs static global; max wins vs "
            "static goal; min common CPU; max threshold"
        ),
    }, traces_by_threshold[str(float(selected["threshold"]))]


def correctness_summary(
    results: Sequence[dict[str, Any]],
    contract: dict[str, Any],
) -> dict[str, Any]:
    bad_statuses = []
    missing_proofs = []
    external_timeouts = []
    budget_violations = []
    telemetry_missing = []
    branch_violations = []
    for result in results:
        label = (
            f"{result['policy']}/{result['problem_id']}/"
            f"rep-{result['repetition']}"
        )
        if result.get("szs_status") in common.BAD_STATUSES:
            bad_statuses.append(
                {"coordinate": label, "status": result["szs_status"]}
            )
        if proof_status(result) and int(result["proof_steps"]) <= 0:
            missing_proofs.append(label)
        if result.get("external_timeout"):
            external_timeouts.append(label)
        if int(result["configured_cpu_seconds"]) > 5:
            budget_violations.append(label)
        for index, telemetry in enumerate(result["_phase_telemetry"]):
            if telemetry is None:
                telemetry_missing.append(f"{label}/phase-{index + 1}")
        if result["policy"] == "adaptive":
            first_status = result["phases"][0]["szs_status"]
            if first_status in common.PROOF_STATUSES:
                expected_branch = "probe_solved"
            else:
                threshold = float(contract["selected_threshold"])
                expected_branch = common.choose_branch(
                    result["_phase_telemetry"][0], threshold
                )["branch"]
            observed = (result.get("decision") or {}).get("branch")
            if observed != expected_branch:
                branch_violations.append(
                    {
                        "coordinate": label,
                        "expected": expected_branch,
                        "observed": observed,
                    }
                )
    passed = not (
        bad_statuses
        or missing_proofs
        or external_timeouts
        or budget_violations
        or branch_violations
    )
    return {
        "passed": passed,
        "bad_statuses": bad_statuses,
        "proof_statuses_without_pcl_steps": missing_proofs,
        "external_timeouts": external_timeouts,
        "configured_budget_violations": budget_violations,
        "missing_or_invalid_telemetry": telemetry_missing,
        "adaptive_branch_violations": branch_violations,
    }


def adaptive_diagnostics(
    results: Sequence[dict[str, Any]], repetitions: int
) -> dict[str, Any]:
    adaptive = [
        result for result in results if result["policy"] == "adaptive"
    ]
    traces = []
    branches_by_problem: dict[str, list[str]] = defaultdict(list)
    for result in adaptive:
        decision = dict(result.get("decision") or {})
        branch = str(decision.get("branch"))
        branches_by_problem[str(result["problem_id"])].append(branch)
        traces.append(
            {
                "problem_id": result["problem_id"],
                "repetition": result["repetition"],
                "status": result["szs_status"],
                "configured_cpu_seconds": result[
                    "configured_cpu_seconds"
                ],
                "telemetry_cpu_seconds": result[
                    "telemetry_cpu_seconds"
                ],
                "decision_cpu_seconds": result[
                    "decision_cpu_seconds"
                ],
                "decision_wall_seconds": result[
                    "decision_wall_seconds"
                ],
                **decision,
            }
        )
    unstable = sorted(
        problem
        for problem, branches in branches_by_problem.items()
        if len(branches) == repetitions and len(set(branches)) != 1
    )
    return {
        "traces": traces,
        "branch_counts": {
            branch: sum(trace["branch"] == branch for trace in traces)
            for branch in ("global", "goal", "probe_solved")
        },
        "branch_unstable_problems": unstable,
        "max_decision_cpu_seconds": max(
            (
                float(result["decision_cpu_seconds"])
                for result in adaptive
            ),
            default=0.0,
        ),
        "max_decision_wall_seconds": max(
            (
                float(result["decision_wall_seconds"])
                for result in adaptive
            ),
            default=0.0,
        ),
        "median_decision_cpu_seconds": median_or_none(
            float(result["decision_cpu_seconds"]) for result in adaptive
        ),
        "median_decision_wall_seconds": median_or_none(
            float(result["decision_wall_seconds"]) for result in adaptive
        ),
    }


def evaluation_summary(
    results: Sequence[dict[str, Any]],
    contract: dict[str, Any],
) -> dict[str, Any]:
    repetitions = int(contract["repetitions"])
    policies = {
        policy: summarize_policy(results, policy, repetitions)
        for policy in sorted(EVALUATION_POLICIES)
    }
    comparisons = {
        baseline: paired_comparison(
            results, "adaptive", baseline, repetitions
        )
        for baseline in (
            "static_global_restart",
            "static_goal",
            "global_full",
            "goal_full",
        )
    }
    return {
        "policies": policies,
        "comparisons": comparisons,
        "adaptive": adaptive_diagnostics(results, repetitions),
    }


def final_decision(
    validation: dict[str, Any],
    test: dict[str, Any],
) -> dict[str, Any]:
    validation_eval = validation["evaluation"]
    test_eval = test["evaluation"]
    correctness = (
        validation["correctness"]["passed"]
        and test["correctness"]["passed"]
    )
    branches_reproducible = not (
        validation_eval["adaptive"]["branch_unstable_problems"]
        or test_eval["adaptive"]["branch_unstable_problems"]
    )
    overhead_ok = (
        validation_eval["adaptive"]["max_decision_wall_seconds"] <= 0.01
        and test_eval["adaptive"]["max_decision_wall_seconds"] <= 0.01
    )
    comparator_names = ("static_global_restart", "static_goal")
    losses = {
        phase: sorted(
            {
                problem
                for comparator in comparator_names
                for problem in summary["evaluation"]["comparisons"][
                    comparator
                ]["baseline_only_reproducible_solves"]
            }
        )
        for phase, summary in (
            ("validation", validation),
            ("test", test),
        )
    }
    test_adaptive = set(
        test_eval["policies"]["adaptive"]["reproducible_solves"]
    )
    test_comparator_union = set()
    for comparator in comparator_names:
        test_comparator_union.update(
            test_eval["policies"][comparator]["reproducible_solves"]
        )
    test_unique_both = sorted(test_adaptive - test_comparator_union)

    def efficiency_pass(summary: dict[str, Any]) -> bool:
        comparison = summary["evaluation"]["comparisons"]["static_goal"]
        ratio = comparison["median_common_solve_cpu_ratio"]
        return (
            comparison["common_solved_repetition_coordinates"] >= 4
            and ratio is not None
            and float(ratio) <= 0.95
        )

    efficiency = {
        "validation": efficiency_pass(validation),
        "test": efficiency_pass(test),
    }
    sufficient_telemetry = (
        len(validation["correctness"]["missing_or_invalid_telemetry"]) == 0
        and len(test["correctness"]["missing_or_invalid_telemetry"]) == 0
    )
    if (
        correctness
        and branches_reproducible
        and overhead_ok
        and not losses["validation"]
        and not losses["test"]
        and (
            len(test_unique_both) >= 2
            or (efficiency["validation"] and efficiency["test"])
        )
    ):
        outcome = "continue"
    elif (
        not correctness
        or losses["test"]
        or (
            not test_unique_both
            and not efficiency["validation"]
            and not efficiency["test"]
            and branches_reproducible
            and sufficient_telemetry
        )
    ):
        outcome = "stop"
    else:
        outcome = "uncertain"
    return {
        "outcome": outcome,
        "correctness_passed": correctness,
        "branches_reproducible": branches_reproducible,
        "decision_overhead_passed": overhead_ok,
        "losses": losses,
        "test_adaptive_only_vs_both_comparators": test_unique_both,
        "efficiency_pass": efficiency,
        "sufficient_telemetry": sufficient_telemetry,
        "production_effect": (
            "No production change; an integrated prototype requires "
            "outcome=continue."
        ),
    }


def report_body(
    *,
    phase: str,
    run_root: Path,
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    validation_report: dict[str, Any] | None,
) -> dict[str, Any]:
    correctness = correctness_summary(results, contract)
    body: dict[str, Any] = {
        "schema_version": 1,
        "experiment": EXPERIMENT,
        "source_revision": common.SOURCE_REVISION,
        "phase": phase,
        "contract_id": contract["contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "corpus_sha256": contract["corpus_sha256"],
        "run_count": len(results),
        "correctness": correctness,
    }
    if phase == "calibration":
        selection, traces = calibration_selection(results, contract)
        body["calibration"] = selection
        body["selected_intervention_traces"] = traces
    else:
        body["selection_id"] = contract["selection_id"]
        body["selected_threshold"] = contract["selected_threshold"]
        body["evaluation"] = evaluation_summary(results, contract)
        if phase == "validation":
            body["decision"] = {
                "outcome": (
                    "ready_for_test"
                    if correctness["passed"]
                    else "stop_correctness"
                ),
                "production_effect": "No production change.",
            }
        else:
            if validation_report is None:
                raise common.ExperimentError(
                    "test analysis requires a validation report"
                )
            body["validation_report_id"] = validation_report["report_id"]
            body["decision"] = final_decision(validation_report, body)
    del run_root
    return body


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        f"# Online adaptation {report['phase']} results",
        "",
        f"- Contract: `{report['contract_id']}`",
        f"- Runs: {report['run_count']}",
        f"- Correctness: `{report['correctness']['passed']}`",
        "",
    ]
    if report["phase"] == "calibration":
        calibration = report["calibration"]
        lines.extend(
            [
                "## Threshold selection",
                "",
                (
                    "Selected threshold: "
                    f"`{calibration['selected_threshold']}`."
                ),
                "",
                "| Threshold | Reproducible solves | Losses vs static global | "
                "Wins vs static goal | Goal branches |",
                "| ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for candidate in calibration["candidates"]:
            lines.append(
                f"| {candidate['threshold']} | "
                f"{candidate['reproducible_solve_count']} | "
                f"{len(candidate['losses_vs_static_global'])} | "
                f"{len(candidate['wins_vs_static_goal'])} | "
                f"{candidate['branch_counts']['goal']} |"
            )
    else:
        lines.extend(
            [
                "## Coverage",
                "",
                "| Policy | Reproducible solves | One-repeat solves | "
                "Median CPU (s) |",
                "| --- | ---: | ---: | ---: |",
            ]
        )
        for policy, summary in report["evaluation"]["policies"].items():
            lines.append(
                f"| {policy} | "
                f"{len(summary['reproducible_solves'])} | "
                f"{len(summary['one_repeat_solves'])} | "
                f"{summary['median_cpu_seconds']} |"
            )
        lines.extend(
            [
                "",
                "## Adaptive comparisons",
                "",
                "| Baseline | Adaptive-only | Baseline-only | "
                "Common coordinates | Median CPU ratio |",
                "| --- | ---: | ---: | ---: | ---: |",
            ]
        )
        for comparison in report["evaluation"]["comparisons"].values():
            lines.append(
                f"| {comparison['baseline']} | "
                f"{len(comparison['candidate_only_reproducible_solves'])} | "
                f"{len(comparison['baseline_only_reproducible_solves'])} | "
                f"{comparison['common_solved_repetition_coordinates']} | "
                f"{comparison['median_common_solve_cpu_ratio']} |"
            )
        lines.extend(
            [
                "",
                (
                    "Branch-instability problems: "
                    f"`{report['evaluation']['adaptive']['branch_unstable_problems']}`."
                ),
                (
                    "Maximum decision wall overhead: "
                    f"`{report['evaluation']['adaptive']['max_decision_wall_seconds']}` "
                    "seconds."
                ),
                "",
                f"Decision: `{report['decision']['outcome']}`.",
            ]
        )
    return "\n".join(lines) + "\n"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("calibration", "validation", "test"), required=True
    )
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    parser.add_argument("--selection-output", type=Path)
    parser.add_argument("--validation-report", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    run_root = arguments.run_root.resolve()
    contract = load_contract(run_root, arguments.phase)
    results = load_results(run_root, contract)
    validation_report = None
    if arguments.phase == "test":
        if arguments.validation_report is None:
            raise common.ExperimentError(
                "test analysis requires --validation-report"
            )
        validation_report = json.loads(
            arguments.validation_report.read_text(encoding="utf-8")
        )
    elif arguments.validation_report is not None:
        raise common.ExperimentError(
            "--validation-report is accepted only for test"
        )
    body = report_body(
        phase=arguments.phase,
        run_root=run_root,
        contract=contract,
        results=results,
        validation_report=validation_report,
    )
    report_id = common.sha256_bytes(common.canonical_json(body))
    report = {**body, "report_id": report_id}
    common.atomic_json(arguments.json_output, report)
    arguments.markdown_output.write_text(
        render_markdown(report), encoding="utf-8", newline="\n"
    )
    if arguments.phase == "calibration":
        if arguments.selection_output is None:
            raise common.ExperimentError(
                "calibration requires --selection-output"
            )
        selection_body = {
            "schema_version": 1,
            "experiment": EXPERIMENT,
            "source_revision": common.SOURCE_REVISION,
            "calibration_contract_id": contract["contract_id"],
            "calibration_report_id": report_id,
            "corpus_sha256": contract["corpus_sha256"],
            "selected_threshold": body["calibration"][
                "selected_threshold"
            ],
            "candidates": body["calibration"]["candidates"],
            "selection_rule": body["calibration"]["selection_rule"],
        }
        selection = {
            **selection_body,
            "selection_id": common.sha256_bytes(
                common.canonical_json(selection_body)
            ),
        }
        common.atomic_json(arguments.selection_output, selection)
    elif arguments.selection_output is not None:
        raise common.ExperimentError(
            "--selection-output is accepted only for calibration"
        )
    print(
        json.dumps(
            {
                "report_id": report_id,
                "phase": arguments.phase,
                "run_count": len(results),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except common.ExperimentError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
