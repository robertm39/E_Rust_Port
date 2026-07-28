#!/usr/bin/env python3
"""Verify and summarize the fresh-family goal-hard-priority experiment."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_ANALYZER_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-005-layered-clause-selection"
    / "analyze.py"
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


BASE = load_module("layered_clause_selection_base_analyze", BASE_ANALYZER_PATH)


class AnalysisError(RuntimeError):
    """An incomplete or internally inconsistent experiment result."""


def sha256_file(path: Path) -> str:
    return BASE.sha256_file(path)


def canonical_json(value: Any) -> bytes:
    return BASE.canonical_json(value)


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 6)


def load_verified_results(
    run_root: Path,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    contract_path = run_root / "contract.json"
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract_body = {
        key: value
        for key, value in contract.items()
        if key not in {"contract_id", "created_at", "host"}
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    if contract_id != contract["contract_id"]:
        raise AnalysisError("contract ID does not match the canonical contract")

    results = []
    for result_path in sorted((run_root / "runs").rglob("result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result["contract_id"] != contract_id:
            raise AnalysisError(f"foreign result contract: {result_path}")
        stdout_path = result_path.parent / "stdout.txt"
        stderr_path = result_path.parent / "stderr.txt"
        if sha256_file(stdout_path) != result["stdout_sha256"]:
            raise AnalysisError(f"stdout hash mismatch: {stdout_path}")
        if sha256_file(stderr_path) != result["stderr_sha256"]:
            raise AnalysisError(f"stderr hash mismatch: {stderr_path}")
        telemetry = None
        telemetry_path = result_path.parent / "telemetry.json"
        if result["telemetry_sha256"] is not None:
            if sha256_file(telemetry_path) != result["telemetry_sha256"]:
                raise AnalysisError(f"telemetry hash mismatch: {telemetry_path}")
        if result["telemetry_present"]:
            if result["telemetry_sha256"] is None:
                raise AnalysisError(f"telemetry is present without a hash: {result_path}")
            telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
            if telemetry.get("schema") != "umlaut.search-telemetry":
                raise AnalysisError(f"unexpected telemetry schema: {telemetry_path}")
            selection = telemetry["clause_selection"]
            scheduled = sum(
                queue["scheduled_selections"] for queue in selection["queues"]
            )
            if scheduled != selection["selection_steps"]:
                raise AnalysisError(
                    f"queue schedule count mismatch: {telemetry_path}"
                )
        results.append(
            {
                **result,
                "_path": result_path.as_posix(),
                "_telemetry": telemetry,
            }
        )

    expected = (
        len(contract["selected_problem_ids"])
        * len(contract["strategies"])
        * len(contract["budgets"])
        * contract["repetitions"]
    )
    if len(results) != expected:
        raise AnalysisError(f"expected {expected} results, found {len(results)}")
    coordinates = {
        (
            result["problem_id"],
            result["strategy"],
            result["budget"],
            result["repetition"],
        )
        for result in results
    }
    if len(coordinates) != expected:
        raise AnalysisError("duplicate result coordinates found")
    return contract, results


def reproducible_coverage(
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
    repetitions: int,
) -> set[str]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        if result["strategy"] == strategy and result["budget"] == budget:
            grouped[result["problem_id"]].append(result)
    return {
        problem_id
        for problem_id, group in grouped.items()
        if len(group) == repetitions
        and all(result["expected_status_match"] for result in group)
    }


def metric(result: dict[str, Any], *path: str) -> int | float | None:
    value: Any = result["_telemetry"]
    if value is None:
        return None
    for key in path:
        value = value[key]
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return value
    return None


def aggregate_strategy(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == strategy and result["budget"] == budget
    ]
    coverage = reproducible_coverage(
        results, strategy, budget, contract["repetitions"]
    )
    solved = [
        result
        for result in selected
        if result["problem_id"] in coverage and result["expected_status_match"]
    ]
    solved_cpu = [
        value
        for result in solved
        if (value := metric(result, "resources", "total_cpu_seconds")) is not None
    ]
    solved_ratios = []
    for result in solved:
        generated = metric(result, "search_funnel", "generated")
        processed = metric(result, "search_funnel", "processed")
        if generated is not None and processed not in (None, 0):
            solved_ratios.append(generated / processed)

    queue_rows = [
        queue
        for result in selected
        if result["_telemetry"] is not None
        for queue in result["_telemetry"]["clause_selection"]["queues"]
    ]
    fairness_violations = 0
    for result in selected:
        if result["_telemetry"] is None:
            continue
        queues = result["_telemetry"]["clause_selection"]["queues"]
        cycle = sum(queue["schedule_quota"] for queue in queues)
        fairness_violations += sum(
            queue["max_schedule_gap"] > cycle - queue["schedule_quota"]
            for queue in queues
        )
    by_category = {
        category: sum(
            problem_id in coverage
            for problem_id in {
                result["problem_id"]
                for result in selected
                if result["category"] == category
            }
        )
        for category in contract["categories"]
    }
    return {
        "runs": len(selected),
        "reproducible_solved": len(coverage),
        "reproducible_solved_by_category": by_category,
        "median_solved_cpu_seconds": rounded(median(solved_cpu)),
        "median_solved_generated_per_processed": rounded(median(solved_ratios)),
        "telemetry_records": sum(
            result["_telemetry"] is not None for result in selected
        ),
        "no_status": sum(result["szs_status"] is None for result in selected),
        "external_timeouts": sum(result["external_timeout"] for result in selected),
        "queue": {
            "max_schedule_gap": max(
                (queue["max_schedule_gap"] for queue in queue_rows), default=0
            ),
            "max_preferred_wait": max(
                (queue["max_preferred_wait"] for queue in queue_rows), default=0
            ),
            "fairness_bound_violations": fairness_violations,
        },
    }


def comparison(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    budget: str,
) -> dict[str, Any]:
    repetitions = contract["repetitions"]
    left_coverage = reproducible_coverage(
        results, left, budget, repetitions
    )
    right_coverage = reproducible_coverage(
        results, right, budget, repetitions
    )
    return {
        "left": left,
        "right": right,
        "budget": budget,
        "left_solved": len(left_coverage),
        "right_solved": len(right_coverage),
        "left_only": sorted(left_coverage - right_coverage),
        "right_only": sorted(right_coverage - left_coverage),
        "common_ids": sorted(left_coverage & right_coverage),
    }


def paired_cpu_ratio(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    budget: str,
) -> float | None:
    repetitions = contract["repetitions"]
    left_coverage = reproducible_coverage(
        results, left, budget, repetitions
    )
    right_coverage = reproducible_coverage(
        results, right, budget, repetitions
    )
    common = left_coverage & right_coverage
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["budget"],
            result["repetition"],
        ): result
        for result in results
    }
    ratios = []
    for problem_id in sorted(common):
        for repetition in range(1, repetitions + 1):
            left_result = indexed[(problem_id, left, budget, repetition)]
            right_result = indexed[(problem_id, right, budget, repetition)]
            left_cpu = metric(left_result, "resources", "total_cpu_seconds")
            right_cpu = metric(right_result, "resources", "total_cpu_seconds")
            if left_cpu is not None and right_cpu not in (None, 0):
                ratios.append(left_cpu / right_cpu)
    return median(ratios)


def analyze(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> dict[str, Any]:
    summaries = {
        budget: {
            strategy: aggregate_strategy(
                contract, results, strategy, budget
            )
            for strategy in contract["strategies"]
        }
        for budget in contract["budgets"]
    }
    comparisons = {
        budget: {
            "hard_vs_baseline": comparison(
                contract,
                results,
                "goal_hard_priority",
                "global_aw",
                budget,
            ),
            "hard_vs_scalar": comparison(
                contract,
                results,
                "goal_hard_priority",
                "goal_relevance_scalar",
                budget,
            ),
        }
        for budget in contract["budgets"]
    }
    paired_ratios = {
        budget: rounded(
            paired_cpu_ratio(
                contract,
                results,
                "goal_hard_priority",
                "global_aw",
                budget,
            )
        )
        for budget in contract["budgets"]
    }
    status_mismatches = [
        {
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "budget": result["budget"],
            "repetition": result["repetition"],
            "expected_class": result["expected_class"],
            "szs_status": result["szs_status"],
        }
        for result in results
        if BASE.status_contradicts_expected(result)
    ]
    larger = comparisons["larger"]["hard_vs_baseline"]
    larger_net_gain = len(larger["left_only"]) - len(larger["right_only"])
    coverage_advances = larger_net_gain >= 2
    efficiency_advances = (
        not larger["left_only"]
        and not larger["right_only"]
        and paired_ratios["larger"] is not None
        and paired_ratios["larger"] <= 0.8
    )
    fairness_violations = sum(
        summaries[budget]["goal_hard_priority"]["queue"][
            "fairness_bound_violations"
        ]
        for budget in contract["budgets"]
    )
    advances = (
        (coverage_advances or efficiency_advances)
        and not status_mismatches
        and fairness_violations == 0
    )
    short_hard_only = set(
        comparisons["short"]["hard_vs_baseline"]["left_only"]
    )
    larger_hard_coverage = reproducible_coverage(
        results,
        "goal_hard_priority",
        "larger",
        contract["repetitions"],
    )
    return {
        "schema_version": 1,
        "contract_id": contract["contract_id"],
        "prior_contract_id": contract["prior_contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "selected_problem_count": len(contract["selected_problem_ids"]),
        "selected_family_count": len(contract["selected_families"]),
        "run_count": len(results),
        "telemetry": {
            "valid": sum(result["_telemetry"] is not None for result in results),
            "invalid": sum(
                result["telemetry_sha256"] is not None
                and result["_telemetry"] is None
                for result in results
            ),
            "missing": sum(
                result["telemetry_sha256"] is None for result in results
            ),
        },
        "strategy_summaries": summaries,
        "comparisons": comparisons,
        "paired_hard_to_baseline_cpu_ratio": paired_ratios,
        "short_hard_only_solved_at_larger": sorted(
            short_hard_only & larger_hard_coverage
        ),
        "status_mismatches": status_mismatches,
        "decision": {
            "goal_hard_priority": "advance" if advances else "reject",
            "larger_budget_net_gain": larger_net_gain,
            "coverage_criterion_met": coverage_advances,
            "efficiency_criterion_met": efficiency_advances,
            "fairness_bound_violations": fairness_violations,
            "criterion": (
                "Advance at 20 seconds for a net held-out coverage gain of at "
                "least two, or identical coverage with a paired median CPU "
                "ratio at or below 0.8, with zero contradictory statuses and "
                "zero schedule-fairness bound violations."
            ),
        },
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Goal-hard-priority escalation results",
        "",
        f"- Contract: `{summary['contract_id']}`",
        f"- Prior contract: `{summary['prior_contract_id']}`",
        f"- Umlaut binary SHA-256: `{summary['binary_sha256']}`",
        (
            f"- Problems/families/runs: {summary['selected_problem_count']}/"
            f"{summary['selected_family_count']}/{summary['run_count']}"
        ),
        (
            "- Telemetry: "
            f"{summary['telemetry']['valid']} valid, "
            f"{summary['telemetry']['invalid']} invalid, "
            f"{summary['telemetry']['missing']} missing"
        ),
        "",
        "## Strategy results",
        "",
        "| Budget | Strategy | Reproducible solves | Median solved CPU (s) | "
        "Median solved generated/processed | Max schedule gap | "
        "Max preferred wait |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for budget, strategies in summary["strategy_summaries"].items():
        for strategy, values in strategies.items():
            lines.append(
                f"| {budget} | `{strategy}` | "
                f"{values['reproducible_solved']} | "
                f"{values['median_solved_cpu_seconds']} | "
                f"{values['median_solved_generated_per_processed']} | "
                f"{values['queue']['max_schedule_gap']} | "
                f"{values['queue']['max_preferred_wait']} |"
            )
    lines.extend(["", "## Coverage comparisons", ""])
    for budget, comparisons in summary["comparisons"].items():
        for name, values in comparisons.items():
            lines.extend(
                [
                    f"### {budget.title()} {name.replace('_', ' ').title()}",
                    "",
                    (
                        f"`{values['left']}` solved {values['left_solved']}; "
                        f"`{values['right']}` solved {values['right_solved']}."
                    ),
                    "",
                    f"- Left-only: {values['left_only'] or 'none'}",
                    f"- Right-only: {values['right_only'] or 'none'}",
                    f"- Common: {values['common_ids'] or 'none'}",
                    "",
                ]
            )
    lines.extend(
        [
            "## Decision",
            "",
            (
                "- Goal hard priority: "
                f"`{summary['decision']['goal_hard_priority']}`."
            ),
            (
                "- Larger-budget net gain: "
                f"{summary['decision']['larger_budget_net_gain']}."
            ),
            (
                "- Paired hard/baseline CPU ratios: "
                f"{summary['paired_hard_to_baseline_cpu_ratio']}."
            ),
            (
                "- Short-budget hard-only solves retained at larger budget: "
                f"{summary['short_hard_only_solved_at_larger'] or 'none'}."
            ),
            (
                f"- Contradictory statuses: "
                f"{len(summary['status_mismatches'])}."
            ),
            (
                "- Fairness bound violations: "
                f"{summary['decision']['fairness_bound_violations']}."
            ),
            f"- Criterion: {summary['decision']['criterion']}",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    contract, results = load_verified_results(arguments.run_root.resolve())
    summary = analyze(contract, results)
    arguments.json_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.json_output.write_bytes(canonical_json(summary) + b"\n")
    arguments.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.markdown_output.write_text(
        render_markdown(summary), encoding="utf-8", newline="\n"
    )
    print(
        f"OK: {summary['run_count']} verified runs; "
        f"decision {summary['decision']['goal_hard_priority']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AnalysisError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
