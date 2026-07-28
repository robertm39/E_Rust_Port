#!/usr/bin/env python3
"""Verify and summarize the layered-clause-selection experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


DELETE_BAD_RE = re.compile(
    r"Deleted\s+(\d+)\s+orphaned clauses and\s+(\d+)\s+bad clauses",
    re.IGNORECASE,
)
THEOREM_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NON_THEOREM_STATUSES = {"CounterSatisfiable", "Satisfiable"}
THEOREM_EXPECTATIONS = {"theorem", "unsatisfiable"}
NON_THEOREM_EXPECTATIONS = {"non_theorem", "satisfiable"}


class AnalysisError(RuntimeError):
    """An incomplete or internally inconsistent experiment result."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


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
        stdout = stdout_path.read_text(encoding="utf-8", errors="replace")
        bad_deleted = sum(
            int(match.group(2)) for match in DELETE_BAD_RE.finditer(stdout)
        )
        results.append(
            {
                **result,
                "_path": result_path.as_posix(),
                "_telemetry": telemetry,
                "_bad_deleted": bad_deleted,
            }
        )

    expected = (
        len(contract["selected_problem_ids"])
        * len(contract["strategies"])
        * contract["repetitions"]
    )
    if len(results) != expected:
        raise AnalysisError(f"expected {expected} results, found {len(results)}")
    keys = {
        (result["problem_id"], result["strategy"], result["repetition"])
        for result in results
    }
    if len(keys) != expected:
        raise AnalysisError("duplicate result coordinates found")
    return contract, results


def reproducible_coverage(
    results: Sequence[dict[str, Any]],
    strategy: str,
    split: str,
    repetitions: int,
) -> set[str]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        if result["strategy"] == strategy and result["holdout_split"] == split:
            grouped[result["problem_id"]].append(result)
    return {
        problem_id
        for problem_id, group in grouped.items()
        if len(group) == repetitions
        and all(result["expected_status_match"] for result in group)
    }


def status_contradicts_expected(result: dict[str, Any]) -> bool:
    status = result["szs_status"]
    if result["expected_class"] in THEOREM_EXPECTATIONS:
        return status in NON_THEOREM_STATUSES
    if result["expected_class"] in NON_THEOREM_EXPECTATIONS:
        return status in THEOREM_STATUSES
    raise AnalysisError(f"unknown expected class: {result['expected_class']}")


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
    split: str,
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == strategy and result["holdout_split"] == split
    ]
    coverage = reproducible_coverage(
        results, strategy, split, contract["repetitions"]
    )
    solved_selected = [
        result
        for result in selected
        if result["problem_id"] in coverage and result["expected_status_match"]
    ]
    category_problems: dict[str, set[str]] = defaultdict(set)
    for result in selected:
        if result["problem_id"] in coverage:
            category_problems[result["category"]].add(result["problem_id"])
    cpu_values = [
        value
        for result in selected
        if (value := metric(result, "resources", "total_cpu_seconds")) is not None
    ]
    generated_values = [
        value
        for result in selected
        if (value := metric(result, "search_funnel", "generated")) is not None
    ]
    processed_values = [
        value
        for result in selected
        if (value := metric(result, "search_funnel", "processed")) is not None
    ]
    high_water_values = [
        value
        for result in selected
        if (value := metric(result, "search_funnel", "high_water_total")) is not None
    ]
    generated_per_processed = []
    for result in selected:
        generated = metric(result, "search_funnel", "generated")
        processed = metric(result, "search_funnel", "processed")
        if generated is not None and processed not in (None, 0):
            generated_per_processed.append(generated / processed)
    solved_cpu_values = [
        value
        for result in solved_selected
        if (value := metric(result, "resources", "total_cpu_seconds")) is not None
    ]
    solved_generated_per_processed = []
    for result in solved_selected:
        generated = metric(result, "search_funnel", "generated")
        processed = metric(result, "search_funnel", "processed")
        if generated is not None and processed not in (None, 0):
            solved_generated_per_processed.append(generated / processed)

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
    return {
        "runs": len(selected),
        "telemetry_records": sum(
            result["_telemetry"] is not None for result in selected
        ),
        "reproducible_solved": len(coverage),
        "reproducible_solved_by_category": {
            category: len(category_problems.get(category, set()))
            for category in contract["categories"]
        },
        "status_mismatches": sum(
            status_contradicts_expected(result) for result in selected
        ),
        "no_status": sum(result["szs_status"] is None for result in selected),
        "external_timeouts": sum(result["external_timeout"] for result in selected),
        "median_cpu_seconds": rounded(median(cpu_values)),
        "median_solved_cpu_seconds": rounded(median(solved_cpu_values)),
        "median_wall_seconds": rounded(
            median(result["wall_seconds"] for result in selected)
        ),
        "median_processed": rounded(median(processed_values)),
        "median_generated": rounded(median(generated_values)),
        "median_generated_per_processed": rounded(
            median(generated_per_processed)
        ),
        "median_solved_generated_per_processed": rounded(
            median(solved_generated_per_processed)
        ),
        "median_high_water_total": rounded(median(high_water_values)),
        "bad_deleted": sum(result["_bad_deleted"] for result in selected),
        "queue": {
            "max_schedule_gap": max(
                (queue["max_schedule_gap"] for queue in queue_rows), default=0
            ),
            "max_preferred_wait": max(
                (queue["max_preferred_wait"] for queue in queue_rows), default=0
            ),
            "preferred_bypass_steps": sum(
                queue["preferred_bypass_steps"] for queue in queue_rows
            ),
            "fairness_bound_violations": fairness_violations,
        },
    }


def candidate_order_key(summary: dict[str, Any]) -> tuple[Any, ...]:
    cpu = summary["median_solved_cpu_seconds"]
    generated = summary["median_solved_generated_per_processed"]
    return (
        summary["reproducible_solved"],
        -(cpu if cpu is not None and math.isfinite(cpu) else math.inf),
        -(generated if generated is not None and math.isfinite(generated) else math.inf),
    )


def comparison(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    split: str,
) -> dict[str, Any]:
    repetitions = contract["repetitions"]
    left_coverage = reproducible_coverage(
        results, left, split, repetitions
    )
    right_coverage = reproducible_coverage(
        results, right, split, repetitions
    )
    return {
        "left": left,
        "right": right,
        "split": split,
        "left_solved": len(left_coverage),
        "right_solved": len(right_coverage),
        "left_only": sorted(left_coverage - right_coverage),
        "right_only": sorted(right_coverage - left_coverage),
        "common": len(left_coverage & right_coverage),
    }


def analyze(contract: dict[str, Any], results: Sequence[dict[str, Any]]) -> dict[str, Any]:
    summaries = {
        split: {
            strategy: aggregate_strategy(contract, results, strategy, split)
            for strategy in contract["strategies"]
        }
        for split in contract["splits"]
    }
    layered = [
        name
        for name, strategy in contract["strategies"].items()
        if strategy["kind"] == "layered"
    ]
    chosen = max(
        layered,
        key=lambda name: candidate_order_key(summaries["validation"][name]),
    )
    comparisons = {
        "validation_chosen_vs_baseline": comparison(
            contract, results, chosen, "global_aw", "validation"
        ),
        "test_chosen_vs_baseline": comparison(
            contract, results, chosen, "global_aw", "test"
        ),
        "test_chosen_vs_scalar": comparison(
            contract, results, chosen, "goal_relevance_scalar", "test"
        ),
        "test_hard_priority_vs_baseline": comparison(
            contract, results, "goal_hard_priority", "global_aw", "test"
        ),
        "test_static_prune_vs_baseline": comparison(
            contract, results, "global_static_prune", "global_aw", "test"
        ),
    }
    status_mismatches = [
        {
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "repetition": result["repetition"],
            "expected_class": result["expected_class"],
            "szs_status": result["szs_status"],
        }
        for result in results
        if status_contradicts_expected(result)
    ]
    test_comparison = comparisons["test_chosen_vs_baseline"]
    layered_advances = (
        len(test_comparison["left_only"]) >= 2
        and not status_mismatches
        and summaries["test"][chosen]["queue"]["fairness_bound_violations"] == 0
    )
    return {
        "schema_version": 1,
        "contract_id": contract["contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "selected_problem_count": len(contract["selected_problem_ids"]),
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
        "validation_selected_layered_strategy": chosen,
        "strategy_summaries": summaries,
        "comparisons": comparisons,
        "status_mismatches": status_mismatches,
        "decision": {
            "layered_selection": (
                "advance" if layered_advances else "reject_current_candidates"
            ),
            "criterion": (
                "Advance only for at least two reproducible held-out unique solves, "
                "zero contradictory statuses, and zero schedule-fairness bound "
                "violations."
            ),
            "limited_resource_strategy": "reject_direct_port",
            "limited_resource_reason": (
                "Vampire LRS is an Otter-loop time-reachability policy. "
                "The measured static delete-bad control is only a falsification "
                "proxy for whether non-redundant passive pruning helps Umlaut's "
                "DISCOUNT loop; it is not labeled as LRS."
            ),
        },
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Layered clause-selection results",
        "",
        f"- Contract: `{summary['contract_id']}`",
        f"- Umlaut binary SHA-256: `{summary['binary_sha256']}`",
        f"- Problems: {summary['selected_problem_count']}",
        f"- Runs: {summary['run_count']}",
        (
            "- Telemetry: "
            f"{summary['telemetry']['valid']} valid, "
            f"{summary['telemetry']['invalid']} invalid, "
            f"{summary['telemetry']['missing']} missing"
        ),
        (
            "- Validation-selected layered strategy: "
            f"`{summary['validation_selected_layered_strategy']}`"
        ),
        "",
        "## Strategy results",
        "",
        "| Split | Strategy | Reproducible solves | Median solved CPU (s) | "
        "Median solved generated/processed | Max schedule gap | "
        "Max preferred wait | Bad deleted |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for split, strategies in summary["strategy_summaries"].items():
        for strategy, values in strategies.items():
            lines.append(
                f"| {split} | `{strategy}` | {values['reproducible_solved']} | "
                f"{values['median_solved_cpu_seconds']} | "
                f"{values['median_solved_generated_per_processed']} | "
                f"{values['queue']['max_schedule_gap']} | "
                f"{values['queue']['max_preferred_wait']} | "
                f"{values['bad_deleted']} |"
            )
    lines.extend(["", "## Coverage comparisons", ""])
    for name, values in summary["comparisons"].items():
        lines.extend(
            [
                f"### {name.replace('_', ' ').title()}",
                "",
                (
                    f"`{values['left']}` solved {values['left_solved']}; "
                    f"`{values['right']}` solved {values['right_solved']}; "
                    f"common {values['common']}."
                ),
                "",
                f"- Left-only: {values['left_only'] or 'none'}",
                f"- Right-only: {values['right_only'] or 'none'}",
                "",
            ]
        )
    lines.extend(
        [
            "## Decision",
            "",
            (
                "- Layered selection: "
                f"`{summary['decision']['layered_selection']}`."
            ),
            f"- Criterion: {summary['decision']['criterion']}",
            (
                "- Limited Resource Strategy: "
                f"`{summary['decision']['limited_resource_strategy']}`."
            ),
            f"- Rationale: {summary['decision']['limited_resource_reason']}",
            (
                f"- Status mismatches: {len(summary['status_mismatches'])}."
            ),
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
        f"decision {summary['decision']['layered_selection']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AnalysisError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
