#!/usr/bin/env python3
"""Analyze frozen phase results or make the preregistered final decision."""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import Any, Iterable, Sequence

import audit_results
import connection_common as common


def theorem_claim(result: dict[str, Any]) -> bool:
    return audit_results.theorem_claim(result)


def median_or_none(values: Iterable[float | int | None]) -> float | None:
    retained = [float(value) for value in values if value is not None]
    return statistics.median(retained) if retained else None


def reproducible_problem(
    results: Sequence[dict[str, Any]],
    *,
    problem_id: str,
    method: str,
    repetitions: int,
) -> bool:
    coordinates = [
        result for result in results
        if result["problem_id"] == problem_id and result["method"] == method
    ]
    return (
        len(coordinates) == repetitions
        and all(theorem_claim(result) for result in coordinates)
        and all(result.get("proof_verified") for result in coordinates)
    )


def method_summary(
    results: Sequence[dict[str, Any]],
    *,
    problem_ids: Sequence[str],
    method: str,
    repetitions: int,
) -> dict[str, Any]:
    solved = [
        problem_id for problem_id in problem_ids
        if reproducible_problem(
            results,
            problem_id=problem_id,
            method=method,
            repetitions=repetitions,
        )
    ]
    solved_runs = [
        result for result in results
        if result["method"] == method and result["problem_id"] in solved
    ]
    return {
        "method": method,
        "reproducible_solved": solved,
        "reproducible_solve_count": len(solved),
        "verified_theorem_runs": sum(
            theorem_claim(result) and bool(result.get("proof_verified"))
            for result in results
            if result["method"] == method
        ),
        "median_solved_wall_seconds": median_or_none(
            result.get("solver_wall_seconds") for result in solved_runs
        ),
        "median_solved_user_cpu_seconds": median_or_none(
            result.get("user_cpu_seconds") for result in solved_runs
        ),
        "aggregate_connection_rule_nodes": sum(
            int(result["proof_rule_nodes"])
            for result in solved_runs
            if result.get("proof_rule_nodes") is not None
        ),
        "aggregate_saturation_proof_formulas": sum(
            int(result["proof_formula_count"])
            for result in solved_runs
            if result.get("proof_formula_count") is not None
        ),
        "aggregate_processed_clauses": sum(
            int(result["processed_clauses"])
            for result in solved_runs
            if result.get("processed_clauses") is not None
        ),
    }


def common_cost(
    results: Sequence[dict[str, Any]],
    *,
    common_problems: Sequence[str],
) -> dict[str, Any]:
    connection_nodes = 0
    saturation_formulas = 0
    wall_ratios: list[float] = []
    for problem_id in common_problems:
        for repetition in sorted(
            {
                int(result["repetition"])
                for result in results
                if result["problem_id"] == problem_id
            }
        ):
            connection = next(
                result for result in results
                if result["problem_id"] == problem_id
                and result["method"] == "connection"
                and result["repetition"] == repetition
            )
            saturation = next(
                result for result in results
                if result["problem_id"] == problem_id
                and result["method"] == "goal_hard_priority"
                and result["repetition"] == repetition
            )
            connection_nodes += int(connection["proof_rule_nodes"])
            saturation_formulas += int(saturation["proof_formula_count"])
            saturation_wall = float(saturation["solver_wall_seconds"])
            if saturation_wall > 0:
                wall_ratios.append(
                    float(connection["solver_wall_seconds"]) / saturation_wall
                )
    return {
        "common_problem_count": len(common_problems),
        "common_problems": list(common_problems),
        "connection_rule_nodes": connection_nodes,
        "goal_saturation_proof_formulas": saturation_formulas,
        "rule_node_ratio": (
            connection_nodes / saturation_formulas
            if saturation_formulas
            else None
        ),
        "median_wall_ratio": (
            statistics.median(wall_ratios) if wall_ratios else None
        ),
    }


def analyze_phase(root: Path, phase: str, corpus: Path) -> dict[str, Any]:
    audit = audit_results.audit(root, phase, corpus)
    results = common.read_jsonl(root / "results.jsonl")
    _header, records = common.load_corpus(corpus)
    selected = [
        record for record in records if record["experiment_split"] == phase
    ]
    problem_ids = [record["problem_id"] for record in selected]
    summaries = {
        method: method_summary(
            results,
            problem_ids=problem_ids,
            method=method,
            repetitions=common.REPETITIONS[phase],
        )
        for method in common.METHODS
    }
    connection = set(summaries["connection"]["reproducible_solved"])
    global_aw = set(summaries["global_aw"]["reproducible_solved"])
    goal = set(summaries["goal_hard_priority"]["reproducible_solved"])
    portfolio = sorted(connection | goal)
    common_solved = sorted(connection & goal)
    return {
        "schema_version": 1,
        "phase": phase,
        "contract_id": audit["contract_id"],
        "correctness_gates_passed": audit["valid"],
        "audit": audit,
        "method_summaries": summaries,
        "connection_unique_vs_global": sorted(connection - global_aw),
        "connection_unique_vs_goal": sorted(connection - goal),
        "connection_losses_vs_goal": sorted(goal - connection),
        "independent_portfolio": {
            "workers": ["connection", "goal_hard_priority"],
            "resource_interpretation": "two_workers_five_second_wall",
            "reproducible_solved": portfolio,
            "reproducible_solve_count": len(portfolio),
            "adds_over_goal": sorted(set(portfolio) - goal),
        },
        "common_connection_goal_cost": common_cost(
            results, common_problems=common_solved
        ),
        "clause_exchange": {
            "run": False,
            "decision": "reject_unsound_open-branch_exchange",
            "reason": (
                "an open tableau branch is not a derived consequence; a closed "
                "tableau is already terminal"
            ),
        },
    }


def final_decision(
    validation: dict[str, Any], test: dict[str, Any]
) -> dict[str, Any]:
    if validation.get("phase") != "validation" or test.get("phase") != "test":
        raise common.ExperimentError("final inputs must be validation and test analyses")
    correctness = bool(
        validation.get("correctness_gates_passed")
        and test.get("correctness_gates_passed")
    )
    validation_unique = list(validation["connection_unique_vs_goal"])
    test_unique = list(test["connection_unique_vs_goal"])
    test_losses = list(test["connection_losses_vs_goal"])
    cost = test["common_connection_goal_cost"]
    cost_advantage = bool(
        cost["common_problem_count"] >= 2
        and cost["rule_node_ratio"] is not None
        and cost["rule_node_ratio"] <= 0.5
        and cost["median_wall_ratio"] is not None
        and cost["median_wall_ratio"] <= 1.5
        and not test_losses
    )
    if not correctness:
        verdict = "invalid"
    elif test_losses:
        verdict = "stop"
    elif test_unique or cost_advantage:
        verdict = "advance-native-prototype"
    elif validation_unique:
        verdict = "validation-only-signal"
    else:
        verdict = "stop"
    return {
        "schema_version": 1,
        "verdict": verdict,
        "correctness_gates_passed": correctness,
        "validation_unique_vs_goal": validation_unique,
        "test_unique_vs_goal": test_unique,
        "test_losses_vs_goal": test_losses,
        "test_cost_advantage": cost_advantage,
        "test_common_cost": cost,
        "portfolio_test_adds_over_goal": test["independent_portfolio"][
            "adds_over_goal"
        ],
        "production_change": False,
        "native_follow_up_permitted": verdict == "advance-native-prototype",
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--root", type=Path)
    mode.add_argument("--validation-analysis", type=Path)
    parser.add_argument("--phase", choices=common.REPETITIONS)
    parser.add_argument("--corpus", type=Path)
    parser.add_argument("--test-analysis", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.root is not None:
        if arguments.phase is None or arguments.corpus is None:
            raise common.ExperimentError(
                "phase analysis requires --phase and --corpus"
            )
        if arguments.test_analysis is not None:
            raise common.ExperimentError(
                "--test-analysis is accepted only in final-decision mode"
            )
        output = analyze_phase(
            arguments.root.resolve(),
            arguments.phase,
            arguments.corpus.resolve(),
        )
    else:
        if (
            arguments.phase is not None
            or arguments.corpus is not None
            or arguments.test_analysis is None
        ):
            raise common.ExperimentError(
                "final mode requires --validation-analysis and --test-analysis only"
            )
        validation = json.loads(
            arguments.validation_analysis.read_text(encoding="utf-8")
        )
        test = json.loads(arguments.test_analysis.read_text(encoding="utf-8"))
        output = final_decision(validation, test)
    common.atomic_json(arguments.output.resolve(), output)
    print(json.dumps(output, sort_keys=True))
    return 0 if output.get("correctness_gates_passed", True) else 1


if __name__ == "__main__":
    sys.exit(main())

