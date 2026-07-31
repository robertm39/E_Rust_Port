#!/usr/bin/env python3
"""Analyze cooperative multicore experiment results."""

from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path
from typing import Any, Iterable, Sequence

from common import (
    PROOF_STATUSES,
    ExperimentError,
    atomic_json,
    canonical_json,
    sha256_bytes,
)
from run_experiment import ARMS, SHARING_CAPS


def median(values: Iterable[float]) -> float | None:
    collected = list(values)
    return statistics.median(collected) if collected else None


def rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 9)


def proof(result: dict[str, Any]) -> bool:
    replay = result.get("proof_replay")
    return (
        result.get("status") in PROOF_STATUSES
        and isinstance(replay, dict)
        and replay.get("reproduced") is True
    )


def load_results(root: Path, expected: int) -> list[dict[str, Any]]:
    paths = sorted(root.glob("*/*-r*/result.json"))
    if len(paths) != expected:
        raise ExperimentError(
            f"{root} has {len(paths)} coordinate results, expected {expected}"
        )
    results: list[dict[str, Any]] = []
    contracts: set[str] = set()
    for path in paths:
        result = json.loads(path.read_text(encoding="utf-8"))
        if result.get("kind") != "cooperative-multicore-coordinate":
            raise ExperimentError(f"invalid coordinate: {path}")
        contracts.add(str(result["contract_id"]))
        results.append(result)
    if len(contracts) != 1:
        raise ExperimentError(f"multiple contracts in {root}: {sorted(contracts)}")
    return results


def by_arm_problem(
    results: Sequence[dict[str, Any]],
) -> dict[str, dict[str, list[dict[str, Any]]]]:
    grouped: dict[str, dict[str, list[dict[str, Any]]]] = {
        arm: {} for arm in ARMS
    }
    for result in results:
        grouped[str(result["arm"])].setdefault(
            str(result["problem_id"]), []
        ).append(result)
    for arm in grouped:
        for problem in grouped[arm]:
            grouped[arm][problem].sort(key=lambda item: item["repetition"])
    return grouped


def reproducible_solved(
    grouped: dict[str, dict[str, list[dict[str, Any]]]], arm: str
) -> set[str]:
    return {
        problem
        for problem, repetitions in grouped[arm].items()
        if len(repetitions) == 2 and all(proof(result) for result in repetitions)
    }


def one_repeat_solved(
    grouped: dict[str, dict[str, list[dict[str, Any]]]], arm: str
) -> set[str]:
    return {
        problem
        for problem, repetitions in grouped[arm].items()
        if sum(proof(result) for result in repetitions) == 1
    }


def paired(
    grouped: dict[str, dict[str, list[dict[str, Any]]]],
    candidate: str,
    baseline: str,
) -> dict[str, Any]:
    baseline_by_key = {
        (result["problem_id"], result["repetition"]): result
        for repetitions in grouped[baseline].values()
        for result in repetitions
        if proof(result)
    }
    rows: list[dict[str, Any]] = []
    for repetitions in grouped[candidate].values():
        for result in repetitions:
            key = (result["problem_id"], result["repetition"])
            control = baseline_by_key.get(key)
            if control is None or not proof(result):
                continue
            candidate_resources = result["aggregate_resources"]
            control_resources = control["aggregate_resources"]
            row = {
                "problem_id": key[0],
                "repetition": key[1],
            }
            for metric in ("total_cpu_seconds", "wall_seconds", "peak_rss_kib"):
                denominator = float(control_resources[metric])
                numerator = float(candidate_resources[metric])
                row[f"{metric}_ratio"] = (
                    numerator / denominator if denominator > 0 else None
                )
            rows.append(row)
    return {
        "common_coordinates": len(rows),
        "median_cpu_ratio": rounded(
            median(
                row["total_cpu_seconds_ratio"]
                for row in rows
                if row["total_cpu_seconds_ratio"] is not None
            )
        ),
        "median_peak_rss_ratio": rounded(
            median(
                row["peak_rss_kib_ratio"]
                for row in rows
                if row["peak_rss_kib_ratio"] is not None
            )
        ),
        "median_wall_ratio": rounded(
            median(
                row["wall_seconds_ratio"]
                for row in rows
                if row["wall_seconds_ratio"] is not None
            )
        ),
        "rows": rows,
    }


def preprocess_summary(root: Path) -> dict[str, Any]:
    reports = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(root.glob("_preprocess/*/audit.json"))
    ]
    single_cpu: list[float] = []
    parallel_cpu: list[float] = []
    hash_failures: list[str] = []
    for report in reports:
        hashes = {
            report["single"]["stdout_sha256"],
            *(run["stdout_sha256"] for run in report["parallel"]),
        }
        if len(hashes) != 1:
            hash_failures.append(str(report["problem"]))
        timing = report["single"].get("timing")
        if timing is not None:
            single_cpu.append(float(timing["total_cpu_seconds"]))
        times = [run.get("timing") for run in report["parallel"]]
        if all(timing is not None for timing in times):
            parallel_cpu.append(
                sum(float(timing["total_cpu_seconds"]) for timing in times)
            )
    return {
        "hash_failures": hash_failures,
        "median_four_cpu_seconds": rounded(median(parallel_cpu)),
        "median_one_cpu_seconds": rounded(median(single_cpu)),
        "problem_count": len(reports),
        "total_four_cpu_seconds": rounded(sum(parallel_cpu)),
        "total_one_cpu_seconds": rounded(sum(single_cpu)),
    }


def summarize_phase(root: Path, phase: str) -> dict[str, Any]:
    repetitions = 1 if phase == "train" else 2
    problems = 16 if phase == "train" else 8
    results = load_results(root, problems * repetitions * len(ARMS))
    grouped = by_arm_problem(results)
    correctness = sorted(
        {
            f"{result['problem_id']}:{result['arm']}:r{result['repetition']}:{failure}"
            for result in results
            for failure in result["correctness_failures"]
        }
    )
    controls = (
        "independent_equal",
        "independent_unequal",
        "restart_control",
    )
    solved = {
        arm: (
            sorted(reproducible_solved(grouped, arm))
            if repetitions == 2
            else sorted(
                problem
                for problem, values in grouped[arm].items()
                if any(proof(result) for result in values)
            )
        )
        for arm in ARMS
    }
    arms: dict[str, Any] = {}
    for arm in ARMS:
        solve_set = set(solved[arm])
        union_controls = set().union(*(set(solved[name]) for name in controls))
        resources = [result["aggregate_resources"] for result in results if result["arm"] == arm]
        arms[arm] = {
            "exchange_clauses": sum(
                int(result["exchange"]["clause_count"])
                for result in results
                if result["arm"] == arm
            ),
            "lost_vs": {
                control: sorted(set(solved[control]) - solve_set)
                for control in controls
            },
            "median_peak_rss_kib": rounded(
                median(float(item["peak_rss_kib"]) for item in resources)
            ),
            "median_total_cpu_seconds": rounded(
                median(float(item["total_cpu_seconds"]) for item in resources)
            ),
            "median_wall_seconds": rounded(
                median(float(item["wall_seconds"]) for item in resources)
            ),
            "one_repeat_solves": (
                sorted(one_repeat_solved(grouped, arm))
                if repetitions == 2
                else []
            ),
            "paired_vs": {
                control: paired(grouped, arm, control) for control in controls
            },
            "reproducible_solves": solved[arm],
            "unique_vs_all_controls": sorted(solve_set - union_controls),
        }
    summary: dict[str, Any] = {
        "arms": arms,
        "contract_id": results[0]["contract_id"],
        "coordinate_count": len(results),
        "correctness_failures": correctness,
        "kind": "cooperative-multicore-phase-analysis",
        "phase": phase,
        "preprocessing": preprocess_summary(root),
        "problem_count": problems,
        "repetitions": repetitions,
        "schema_version": 1,
    }
    summary["analysis_id"] = sha256_bytes(canonical_json(summary))
    return summary


def qualifies(summary: dict[str, Any], arm: str) -> bool:
    data = summary["arms"][arm]
    no_loss = all(not losses for losses in data["lost_vs"].values())
    if not no_loss:
        return False
    paired_data = data["paired_vs"]["restart_control"]
    return (
        paired_data["common_coordinates"] >= 4
        and paired_data["median_cpu_ratio"] is not None
        and paired_data["median_cpu_ratio"] <= 0.95
        and paired_data["median_wall_ratio"] is not None
        and paired_data["median_wall_ratio"] <= 0.95
        and paired_data["median_peak_rss_ratio"] is not None
        and paired_data["median_peak_rss_ratio"] <= 1.05
    )


def final_decision(
    validation: dict[str, Any], test: dict[str, Any]
) -> dict[str, Any]:
    failures = [
        *validation["correctness_failures"],
        *test["correctness_failures"],
    ]
    candidates: list[dict[str, Any]] = []
    for arm in SHARING_CAPS:
        validation_data = validation["arms"][arm]
        test_data = test["arms"][arm]
        no_loss = all(
            not losses
            for losses in (
                *validation_data["lost_vs"].values(),
                *test_data["lost_vs"].values(),
            )
        )
        unique_test = bool(test_data["unique_vs_all_controls"])
        validation_efficiency = qualifies(validation, arm)
        test_efficiency = qualifies(test, arm)
        qualifies_candidate = no_loss and (
            unique_test or validation_efficiency and test_efficiency
        )
        candidates.append(
            {
                "arm": arm,
                "cap": SHARING_CAPS[arm],
                "qualifies": qualifies_candidate,
                "test_cpu_ratio": test_data["paired_vs"]["restart_control"][
                    "median_cpu_ratio"
                ],
                "test_unique_solves": len(test_data["unique_vs_all_controls"]),
                "validation_efficiency_qualifies": validation_efficiency,
                "test_efficiency_qualifies": test_efficiency,
            }
        )
    adoptable = [
        candidate for candidate in candidates if candidate["qualifies"]
    ]
    if failures:
        verdict = "stop"
        selected = None
        reason = "correctness_failure"
    elif adoptable:
        adoptable.sort(
            key=lambda item: (
                -item["test_unique_solves"],
                (
                    item["test_cpu_ratio"]
                    if item["test_cpu_ratio"] is not None
                    else float("inf")
                ),
                item["cap"],
            )
        )
        verdict = "adopt"
        selected = adoptable[0]["arm"]
        reason = "unique_solve_or_paired_efficiency"
    else:
        test_losses = any(
            test["arms"][arm]["lost_vs"][control]
            for arm in SHARING_CAPS
            for control in (
                "independent_equal",
                "independent_unequal",
                "restart_control",
            )
        )
        incomplete = all(
            test["arms"][arm]["paired_vs"]["restart_control"][
                "common_coordinates"
            ]
            < 4
            for arm in SHARING_CAPS
        )
        if test_losses:
            verdict = "stop"
            reason = "reproducible_test_solve_lost"
        elif incomplete:
            verdict = "uncertain"
            reason = "insufficient_common_solved_coordinates"
        else:
            verdict = "stop"
            reason = "no_unique_solve_or_efficiency_signal"
        selected = None
    equal_validation = set(
        validation["arms"]["independent_equal"]["reproducible_solves"]
    )
    unequal_validation_solves = set(
        validation["arms"]["independent_unequal"]["reproducible_solves"]
    )
    equal_test = set(test["arms"]["independent_equal"]["reproducible_solves"])
    unequal_test_solves = set(
        test["arms"]["independent_unequal"]["reproducible_solves"]
    )
    unequal_validation_pair = validation["arms"]["independent_unequal"][
        "paired_vs"
    ]["independent_equal"]
    unequal_test_pair = test["arms"]["independent_unequal"]["paired_vs"][
        "independent_equal"
    ]
    unequal_no_loss = (
        equal_validation <= unequal_validation_solves
        and equal_test <= unequal_test_solves
    )
    unequal_unique = bool(unequal_test_solves - equal_test)
    unequal_efficiency = all(
        pair["common_coordinates"] >= 4
        and pair["median_cpu_ratio"] is not None
        and pair["median_cpu_ratio"] <= 0.95
        and pair["median_wall_ratio"] is not None
        and pair["median_wall_ratio"] <= 0.95
        and pair["median_peak_rss_ratio"] is not None
        and pair["median_peak_rss_ratio"] <= 1.05
        for pair in (unequal_validation_pair, unequal_test_pair)
    )
    preprocess_four = float(
        test["preprocessing"]["total_four_cpu_seconds"] or 0.0
    )
    preprocess_one = float(
        test["preprocessing"]["total_one_cpu_seconds"] or 0.0
    )
    equal_cpu = float(
        test["arms"]["independent_equal"]["median_total_cpu_seconds"] or 0.0
    ) * int(test["problem_count"]) * int(test["repetitions"])
    redundant_ratio = (
        max(0.0, preprocess_four - preprocess_one) / equal_cpu
        if equal_cpu > 0
        else None
    )
    return {
        "correctness_failures": failures,
        "independent_unequal": {
            "advance": unequal_no_loss and (unequal_unique or unequal_efficiency),
            "efficiency_qualifies": unequal_efficiency,
            "no_loss": unequal_no_loss,
            "test_unique_solves": sorted(unequal_test_solves - equal_test),
        },
        "kind": "cooperative-multicore-final-decision",
        "reason": reason,
        "schema_version": 1,
        "selected_arm": selected,
        "sharing_candidates": candidates,
        "shared_preprocessing": {
            "advance": False,
            "original_problem_proof_path_demonstrated": False,
            "redundant_cpu_ratio_of_equal_portfolio": rounded(redundant_ratio),
        },
        "test_analysis_id": test["analysis_id"],
        "validation_analysis_id": validation["analysis_id"],
        "verdict": verdict,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--phase", choices=("train", "validation", "test"))
    parser.add_argument("--validation", type=Path)
    parser.add_argument("--test", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    phase_mode = arguments.root is not None or arguments.phase is not None
    final_mode = arguments.validation is not None or arguments.test is not None
    if phase_mode == final_mode:
        raise ExperimentError(
            "choose either --root/--phase or --validation/--test"
        )
    if phase_mode:
        if arguments.root is None or arguments.phase is None:
            raise ExperimentError("--root and --phase are both required")
        result = summarize_phase(arguments.root.resolve(), arguments.phase)
    else:
        if arguments.validation is None or arguments.test is None:
            raise ExperimentError("--validation and --test are both required")
        validation = json.loads(
            arguments.validation.read_text(encoding="utf-8")
        )
        test = json.loads(arguments.test.read_text(encoding="utf-8"))
        result = final_decision(validation, test)
    atomic_json(arguments.output.resolve(), result)
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
