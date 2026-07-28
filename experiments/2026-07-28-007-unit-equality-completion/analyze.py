#!/usr/bin/env python3
"""Verify and summarize the staged UEQ completion experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NON_PROOF_STATUSES = {"CounterSatisfiable", "Satisfiable"}


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


def metric(result: dict[str, Any], *path: str) -> int | float | None:
    value: Any = result["_telemetry"]
    if value is None:
        return None
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return value
    return None


def load_phase(
    experiment_root: Path, phase: str
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    phase_root = experiment_root / phase
    contract_path = phase_root / "contract.json"
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract_body = {
        key: value
        for key, value in contract.items()
        if key not in {"contract_id", "created_at", "host"}
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    if contract_id != contract.get("contract_id"):
        raise AnalysisError(f"{phase} contract ID is invalid")
    if contract.get("phase") != phase:
        raise AnalysisError(f"{phase} contract names another phase")

    results: list[dict[str, Any]] = []
    for result_path in sorted((phase_root / "runs").rglob("result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result.get("contract_id") != contract_id:
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
            telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
            if telemetry.get("schema") != "umlaut.search-telemetry":
                raise AnalysisError(f"unexpected telemetry schema: {telemetry_path}")
            selection = telemetry["clause_selection"]
            if sum(
                queue["scheduled_selections"] for queue in selection["queues"]
            ) != selection["selection_steps"]:
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
        raise AnalysisError(
            f"{phase}: expected {expected} results, found {len(results)}"
        )
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
        raise AnalysisError(f"{phase}: duplicate result coordinates")
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
        result for result in selected if result["problem_id"] in coverage
    ]

    def values(*path: str, solved_only: bool = False) -> list[int | float]:
        source = solved if solved_only else selected
        return [
            value
            for result in source
            if (value := metric(result, *path)) is not None
        ]

    def per_processed(*path: str, solved_only: bool = False) -> list[float]:
        source = solved if solved_only else selected
        ratios = []
        for result in source:
            numerator = metric(result, *path)
            processed = metric(result, "search_funnel", "processed")
            if numerator is not None and processed not in (None, 0):
                ratios.append(numerator / processed)
        return ratios

    by_family: dict[str, set[str]] = defaultdict(set)
    by_band: dict[str, set[str]] = defaultdict(set)
    for result in selected:
        if result["problem_id"] in coverage:
            by_family[result["family"]].add(result["problem_id"])
            by_band[result["difficulty_band"]].add(result["problem_id"])
    return {
        "runs": len(selected),
        "reproducible_solved": len(coverage),
        "reproducible_solved_ids": sorted(coverage),
        "reproducible_solved_by_family": {
            family: len(ids) for family, ids in sorted(by_family.items())
        },
        "reproducible_solved_by_difficulty_band": {
            band: len(ids) for band, ids in sorted(by_band.items())
        },
        "median_solved_cpu_seconds": rounded(
            median(values("resources", "total_cpu_seconds", solved_only=True))
        ),
        "median_solved_generated": rounded(
            median(values("search_funnel", "generated", solved_only=True))
        ),
        "median_solved_processed": rounded(
            median(values("search_funnel", "processed", solved_only=True))
        ),
        "median_solved_paramodulations": rounded(
            median(values("inferences", "paramodulations", solved_only=True))
        ),
        "median_solved_rewrite_steps": rounded(
            median(values("simplification", "rewrite_steps", solved_only=True))
        ),
        "median_solved_paramodulations_per_processed": rounded(
            median(
                per_processed(
                    "inferences", "paramodulations", solved_only=True
                )
            )
        ),
        "median_solved_rewrite_steps_per_processed": rounded(
            median(
                per_processed(
                    "simplification", "rewrite_steps", solved_only=True
                )
            )
        ),
        "median_solved_high_water_total": rounded(
            median(
                values(
                    "search_funnel", "high_water_total", solved_only=True
                )
            )
        ),
        "median_solved_maximum_resident_pages": rounded(
            median(
                values(
                    "resources",
                    "maximum_resident_pages",
                    solved_only=True,
                )
            )
        ),
        "telemetry_records": sum(
            result["_telemetry"] is not None for result in selected
        ),
        "no_status": sum(result["szs_status"] is None for result in selected),
        "external_timeouts": sum(
            result["external_timeout"] for result in selected
        ),
        "contradictory_statuses": sum(
            result["szs_status"] in NON_PROOF_STATUSES for result in selected
        ),
    }


def candidate_order_key(summary: dict[str, Any], name: str) -> tuple[Any, ...]:
    cpu = summary["median_solved_cpu_seconds"]
    generated = summary["median_solved_generated"]
    return (
        -summary["reproducible_solved"],
        cpu if cpu is not None and math.isfinite(cpu) else math.inf,
        generated
        if generated is not None and math.isfinite(generated)
        else math.inf,
        name,
    )


def comparison(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    budget: str,
) -> dict[str, Any]:
    repetitions = contract["repetitions"]
    left_coverage = reproducible_coverage(results, left, budget, repetitions)
    right_coverage = reproducible_coverage(results, right, budget, repetitions)
    common = left_coverage & right_coverage
    metric_paths = {
        "cpu": ("resources", "total_cpu_seconds"),
        "generated": ("search_funnel", "generated"),
        "paramodulations": ("inferences", "paramodulations"),
        "rewrite_steps": ("simplification", "rewrite_steps"),
        "high_water_total": ("search_funnel", "high_water_total"),
        "maximum_resident_pages": (
            "resources",
            "maximum_resident_pages",
        ),
    }
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
    }
    paired_ratios: dict[str, float | None] = {}
    for metric_name, path in metric_paths.items():
        ratios = []
        for problem_id in sorted(common):
            for repetition in range(1, repetitions + 1):
                left_value = metric(
                    indexed[(problem_id, left, repetition)], *path
                )
                right_value = metric(
                    indexed[(problem_id, right, repetition)], *path
                )
                if (
                    left_value is not None
                    and right_value is not None
                    and right_value != 0
                ):
                    ratios.append(left_value / right_value)
        paired_ratios[f"median_{metric_name}_ratio"] = rounded(median(ratios))
    return {
        "left": left,
        "right": right,
        "budget": budget,
        "left_solved": len(left_coverage),
        "right_solved": len(right_coverage),
        "left_only": sorted(left_coverage - right_coverage),
        "right_only": sorted(right_coverage - left_coverage),
        "common_ids": sorted(common),
        "portfolio_union_solved": len(left_coverage | right_coverage),
        **paired_ratios,
    }


def load_selection(path: Path) -> dict[str, Any]:
    selection = json.loads(path.read_text(encoding="utf-8"))
    body = {
        key: value
        for key, value in selection.items()
        if key != "selection_id"
    }
    if hashlib.sha256(canonical_json(body)).hexdigest() != selection.get(
        "selection_id"
    ):
        raise AnalysisError(f"invalid selection ID: {path}")
    return selection


def load_proof_validation(path: Path) -> dict[str, Any]:
    report = json.loads(path.read_text(encoding="utf-8"))
    body = {
        key: value
        for key, value in report.items()
        if key != "report_id"
    }
    if hashlib.sha256(canonical_json(body)).hexdigest() != report.get(
        "report_id"
    ):
        raise AnalysisError("invalid proof-validation report ID")
    return report


def completion_decision(
    larger_comparison: dict[str, Any],
    *,
    contradictory_status_count: int,
    proof_complete: bool,
) -> dict[str, Any]:
    unique_advance = (
        len(larger_comparison["left_only"]) >= 2
        and contradictory_status_count == 0
        and proof_complete
    )
    cpu_ratio = larger_comparison["median_cpu_ratio"]
    memory_ratio = larger_comparison["median_high_water_total_ratio"]
    efficiency_advance = (
        not larger_comparison["right_only"]
        and cpu_ratio is not None
        and cpu_ratio <= 0.90
        and memory_ratio is not None
        and memory_ratio <= 1.05
        and contradictory_status_count == 0
        and proof_complete
    )
    result = (
        "advance_completion_configuration"
        if unique_advance or efficiency_advance
        else "reject_separate_completion_engine"
    )
    reason = (
        "held-out complementarity"
        if unique_advance
        else (
            "held-out non-inferior coverage with at least 10% paired CPU "
            "improvement and bounded search-state memory"
            if efficiency_advance
            else (
                "the selected completion configuration did not produce two "
                "held-out unique solves or a non-inferior 10% paired CPU gain"
            )
        )
    )
    return {
        "result": result,
        "reason": reason,
        "criteria": {
            "unique_solve_threshold": 2,
            "noninferior_cpu_ratio_threshold": 0.90,
            "high_water_ratio_limit": 1.05,
            "requires_all_claimed_proofs_verified": True,
            "requires_zero_contradictory_statuses": True,
        },
        "new_engine_justified": result
        == "advance_completion_configuration",
    }


def analyze(
    experiment_root: Path,
    final_selection_path: Path,
    proof_validation_path: Path,
) -> dict[str, Any]:
    phases: dict[str, tuple[dict[str, Any], list[dict[str, Any]]]] = {
        phase: load_phase(experiment_root, phase)
        for phase in ("calibration", "validation", "test")
    }
    final_selection = load_selection(final_selection_path)
    if final_selection["source_phase"] != "validation":
        raise AnalysisError("final selection is not based on validation")
    chosen = final_selection["selected_strategies"][0]
    test_contract, test_results = phases["test"]
    if test_contract["selection"]["selection_id"] != final_selection["selection_id"]:
        raise AnalysisError("test contract does not pin the final selection")
    proof_validation = load_proof_validation(proof_validation_path)
    if proof_validation["test_contract_id"] != test_contract["contract_id"]:
        raise AnalysisError("proof validation names another test contract")

    phase_summaries: dict[str, Any] = {}
    for phase, (contract, results) in phases.items():
        phase_summaries[phase] = {
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
            "chosen_vs_auto": comparison(
                test_contract,
                test_results,
                chosen,
                "auto_general",
                budget,
            ),
            "chosen_vs_manual": comparison(
                test_contract,
                test_results,
                chosen,
                "manual_general",
                budget,
            ),
        }
        for budget in test_contract["budgets"]
    }
    larger = comparisons["larger"]["chosen_vs_auto"]
    all_results = [
        result for _, results in phases.values() for result in results
    ]
    contradictory_statuses = [
        {
            "phase": result["phase"],
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "budget": result["budget"],
            "repetition": result["repetition"],
            "szs_status": result["szs_status"],
        }
        for result in all_results
        if result["szs_status"] in NON_PROOF_STATUSES
    ]
    proof_complete = (
        proof_validation["all_verified"]
        and proof_validation["verified_cases"]
        == proof_validation["expected_cases"]
    )
    decision = completion_decision(
        larger,
        contradictory_status_count=len(contradictory_statuses),
        proof_complete=proof_complete,
    )
    return {
        "schema_version": 1,
        "contracts": {
            phase: contract["contract_id"]
            for phase, (contract, _) in phases.items()
        },
        "binary_sha256": test_contract["binary_sha256"],
        "selected_completion_strategy": chosen,
        "selected_completion_features": test_contract["strategies"][chosen][
            "features"
        ],
        "problem_counts": {
            phase: len(contract["selected_problem_ids"])
            for phase, (contract, _) in phases.items()
        },
        "run_count": len(all_results),
        "phase_summaries": phase_summaries,
        "test_comparisons": comparisons,
        "proof_validation": proof_validation,
        "contradictory_statuses": contradictory_statuses,
        "decision": decision,
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Unit-equality completion results",
        "",
        f"- Calibration contract: `{summary['contracts']['calibration']}`",
        f"- Validation contract: `{summary['contracts']['validation']}`",
        f"- Test contract: `{summary['contracts']['test']}`",
        f"- Umlaut binary SHA-256: `{summary['binary_sha256']}`",
        (
            "- Problems: "
            f"{summary['problem_counts']['calibration']} calibration, "
            f"{summary['problem_counts']['validation']} validation, "
            f"{summary['problem_counts']['test']} held-out test"
        ),
        f"- Runs: {summary['run_count']}",
        (
            "- Validation-selected completion strategy: "
            f"`{summary['selected_completion_strategy']}`"
        ),
        "",
        "## Strategy results",
        "",
        "| Phase | Budget | Strategy | Solves | Median solved CPU (s) | "
        "Median paramodulations | Median rewrite steps | "
        "Median high-water clauses | Median max RSS pages |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for phase, budgets in summary["phase_summaries"].items():
        for budget, strategies in budgets.items():
            for strategy, values in strategies.items():
                lines.append(
                    f"| {phase} | {budget} | `{strategy}` | "
                    f"{values['reproducible_solved']} | "
                    f"{values['median_solved_cpu_seconds']} | "
                    f"{values['median_solved_paramodulations']} | "
                    f"{values['median_solved_rewrite_steps']} | "
                    f"{values['median_solved_high_water_total']} | "
                    f"{values['median_solved_maximum_resident_pages']} |"
                )
    lines.extend(["", "## Held-out comparisons", ""])
    for budget, comparisons in summary["test_comparisons"].items():
        for name, values in comparisons.items():
            lines.extend(
                [
                    f"### {budget}: {name.replace('_', ' ')}",
                    "",
                    (
                        f"`{values['left']}` solved {values['left_solved']}; "
                        f"`{values['right']}` solved {values['right_solved']}; "
                        f"portfolio union {values['portfolio_union_solved']}."
                    ),
                    "",
                    f"- Left-only: {values['left_only'] or 'none'}",
                    f"- Right-only: {values['right_only'] or 'none'}",
                    f"- Paired median CPU ratio: {values['median_cpu_ratio']}",
                    (
                        "- Paired median generated-clause ratio: "
                        f"{values['median_generated_ratio']}"
                    ),
                    (
                        "- Paired median paramodulation ratio: "
                        f"{values['median_paramodulations_ratio']}"
                    ),
                    (
                        "- Paired median rewrite-step ratio: "
                        f"{values['median_rewrite_steps_ratio']}"
                    ),
                    (
                        "- Paired median high-water ratio: "
                        f"{values['median_high_water_total_ratio']}"
                    ),
                    "",
                ]
            )
    proof = summary["proof_validation"]
    checker_name = proof.get("checker", {}).get("name", "independent checker")
    lines.extend(
        [
            "## Independent proof validation",
            "",
            (
                f"{checker_name} verified {proof['verified_cases']} of "
                f"{proof['expected_cases']} reproducible larger-budget "
                "strategy/problem claims."
            ),
            "",
            "## Decision",
            "",
            f"- Result: `{summary['decision']['result']}`.",
            f"- Reason: {summary['decision']['reason']}.",
            (
                "- Contradictory statuses: "
                f"{len(summary['contradictory_statuses'])}."
            ),
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--final-selection", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        arguments.experiment_root.resolve(),
        arguments.final_selection.resolve(),
        arguments.proof_validation.resolve(),
    )
    arguments.json_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.json_output.write_bytes(canonical_json(summary) + b"\n")
    arguments.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.markdown_output.write_text(
        render_markdown(summary), encoding="utf-8", newline="\n"
    )
    print(
        f"OK: {summary['run_count']} verified runs; "
        f"decision {summary['decision']['result']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AnalysisError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
