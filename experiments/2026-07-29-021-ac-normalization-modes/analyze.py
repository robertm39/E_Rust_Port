#!/usr/bin/env python3
"""Validate and summarize the preregistered explicit-AC mode experiment."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import statistics
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "analyze.py"
)
MODES = ("none", "discard_all", "keep_units", "keep_orientable")
NON_BASELINE_MODES = MODES[1:]
METRICS = {
    "cpu_seconds": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "paramodulations": ("inferences", "paramodulations"),
    "rewrite_steps": ("simplification", "rewrite_steps"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "maximum_resident_pages": ("resources", "maximum_resident_pages"),
    "ac_checks": ("ac", "equality_checks"),
    "ac_hits": ("ac", "equality_hits"),
    "ac_normalizations": ("ac", "normalizations"),
    "ac_input_nodes": ("ac", "input_nodes"),
    "ac_normalized_nodes": ("ac", "normalized_nodes"),
    "ac_flattened_nodes": ("ac", "flattened_nodes"),
}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("ac_mode_base_analyzer", BASE_PATH)


class AnalysisError(RuntimeError):
    """The experiment is incomplete or violates its fixed contract."""


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 6)


def metric(result: dict[str, Any], path: tuple[str, ...]) -> int | float | None:
    value: Any = result["_telemetry"]
    for key in path:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return value
    return None


def reproducible_coverage(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    mode: str,
    budget: str,
) -> set[str]:
    return BASE.reproducible_coverage(
        results, mode, budget, contract["repetitions"]
    )


def aggregate(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    mode: str,
    budget: str,
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == mode and result["budget"] == budget
    ]
    coverage = reproducible_coverage(contract, results, mode, budget)
    solved = [
        result for result in selected if result["problem_id"] in coverage
    ]
    totals = {
        name: sum(
            value
            for result in selected
            if (value := metric(result, path)) is not None
        )
        for name, path in METRICS.items()
    }
    medians = {
        f"median_{name}": rounded(
            median(
                value
                for result in selected
                if (value := metric(result, path)) is not None
            )
        )
        for name, path in METRICS.items()
    }
    solved_medians = {
        f"median_solved_{name}": rounded(
            median(
                value
                for result in solved
                if (value := metric(result, path)) is not None
            )
        )
        for name, path in METRICS.items()
        if name
        in {
            "cpu_seconds",
            "generated",
            "processed",
            "paramodulations",
            "rewrite_steps",
            "high_water_total",
            "maximum_resident_pages",
        }
    }
    checks = totals["ac_checks"]
    input_nodes = totals["ac_input_nodes"]
    return {
        "runs": len(selected),
        "reproducible_solved": len(coverage),
        "reproducible_solved_ids": sorted(coverage),
        "telemetry_records": sum(
            result["_telemetry"] is not None for result in selected
        ),
        "no_status": sum(result["szs_status"] is None for result in selected),
        "external_timeouts": sum(result["external_timeout"] for result in selected),
        "ac_hit_rate": rounded(totals["ac_hits"] / checks) if checks else None,
        "ac_flattened_fraction": (
            rounded(totals["ac_flattened_nodes"] / input_nodes)
            if input_nodes
            else None
        ),
        "totals": totals,
        **medians,
        **solved_medians,
    }


def paired_ratio(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    mode: str,
    budget: str,
    path: tuple[str, ...],
) -> float | None:
    baseline = reproducible_coverage(contract, results, "none", budget)
    candidate = reproducible_coverage(contract, results, mode, budget)
    common = baseline & candidate
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
    }
    ratios = []
    for problem_id in sorted(common):
        for repetition in range(1, contract["repetitions"] + 1):
            candidate_value = metric(
                indexed[(problem_id, mode, repetition)], path
            )
            baseline_value = metric(
                indexed[(problem_id, "none", repetition)], path
            )
            if (
                candidate_value is not None
                and baseline_value is not None
                and baseline_value != 0
            ):
                ratios.append(candidate_value / baseline_value)
    return rounded(median(ratios))


def compare_with_baseline(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    mode: str,
    budget: str,
) -> dict[str, Any]:
    baseline = reproducible_coverage(contract, results, "none", budget)
    candidate = reproducible_coverage(contract, results, mode, budget)
    candidate_runs = [
        result
        for result in results
        if result["strategy"] == mode and result["budget"] == budget
    ]
    ac_hits = sum(
        value
        for result in candidate_runs
        if (value := metric(result, METRICS["ac_hits"])) is not None
    )
    flattened_nodes = sum(
        value
        for result in candidate_runs
        if (value := metric(result, METRICS["ac_flattened_nodes"])) is not None
    )
    return {
        "mode": mode,
        "budget": budget,
        "baseline_solved": len(baseline),
        "mode_solved": len(candidate),
        "mode_only": sorted(candidate - baseline),
        "baseline_only": sorted(baseline - candidate),
        "common": sorted(candidate & baseline),
        "paired_median_cpu_ratio": paired_ratio(
            contract, results, mode, budget, METRICS["cpu_seconds"]
        ),
        "paired_median_generated_ratio": paired_ratio(
            contract, results, mode, budget, METRICS["generated"]
        ),
        "paired_median_processed_ratio": paired_ratio(
            contract, results, mode, budget, METRICS["processed"]
        ),
        "ac_hits": ac_hits,
        "ac_flattened_nodes": flattened_nodes,
    }


def decision(
    comparisons: dict[str, dict[str, Any]],
    proof_validation: dict[str, Any],
    contradictory_statuses: list[dict[str, Any]],
) -> dict[str, Any]:
    advancing = []
    reasons: dict[str, list[str]] = {}
    proof_complete = (
        proof_validation.get("all_verified") is True
        and proof_validation.get("verified_cases")
        == proof_validation.get("expected_cases")
    )
    for mode, comparison in comparisons.items():
        mode_reasons = []
        if comparison["mode_only"]:
            mode_reasons.append("held_out_unique_solve")
        no_loss = not comparison["baseline_only"]
        activation = (
            comparison["ac_hits"] > 0
            and comparison["ac_flattened_nodes"] > 0
        )
        cpu_ratio = comparison["paired_median_cpu_ratio"]
        generated_ratio = comparison["paired_median_generated_ratio"]
        efficiency = (
            (cpu_ratio is not None and cpu_ratio <= 0.90)
            or (generated_ratio is not None and generated_ratio <= 0.90)
        )
        if no_loss and activation and efficiency:
            mode_reasons.append("noninferior_with_ten_percent_efficiency_gain")
        if mode_reasons and proof_complete and not contradictory_statuses:
            advancing.append(mode)
        reasons[mode] = mode_reasons
    return {
        "result": (
            "justify_further_ac_work"
            if advancing
            else "defer_ac_indexing_and_joinability"
        ),
        "advancing_modes": advancing,
        "mode_reasons": reasons,
        "proof_complete": proof_complete,
        "contradictory_status_count": len(contradictory_statuses),
        "exploratory_due_to_four_problem_test": True,
    }


def analyze(
    experiment_root: Path, proof_validation_path: Path
) -> dict[str, Any]:
    phases = {
        phase: BASE.load_phase(experiment_root, phase)
        for phase in ("calibration", "validation", "test")
    }
    expected_counts = {"calibration": 21, "validation": 16, "test": 4}
    for phase, (contract, _) in phases.items():
        if len(contract["selected_problem_ids"]) != expected_counts[phase]:
            raise AnalysisError(f"{phase}: unexpected problem count")
        if set(contract["strategies"]) != set(MODES):
            raise AnalysisError(f"{phase}: unexpected modes")
    binary_hashes = {contract["binary_sha256"] for contract, _ in phases.values()}
    if len(binary_hashes) != 1:
        raise AnalysisError("phases used different binaries")

    summaries = {
        phase: {
            budget: {
                mode: aggregate(contract, results, mode, budget)
                for mode in MODES
            }
            for budget in contract["budgets"]
        }
        for phase, (contract, results) in phases.items()
    }
    test_contract, test_results = phases["test"]
    comparisons = {
        budget: {
            mode: compare_with_baseline(
                test_contract, test_results, mode, budget
            )
            for mode in NON_BASELINE_MODES
        }
        for budget in test_contract["budgets"]
    }
    all_results = [
        result for _, results in phases.values() for result in results
    ]
    contradictory_statuses = [
        {
            "phase": result["phase"],
            "problem_id": result["problem_id"],
            "mode": result["strategy"],
            "budget": result["budget"],
            "repetition": result["repetition"],
            "szs_status": result["szs_status"],
        }
        for result in all_results
        if result["szs_status"] in BASE.NON_PROOF_STATUSES
    ]
    proof_validation = json.loads(
        proof_validation_path.read_text(encoding="utf-8")
    )
    if proof_validation["test_contract_id"] != test_contract["contract_id"]:
        raise AnalysisError("proof validation names another test contract")
    body = {
        "schema_version": 1,
        "contracts": {
            phase: contract["contract_id"]
            for phase, (contract, _) in phases.items()
        },
        "binary_sha256": next(iter(binary_hashes)),
        "problem_counts": expected_counts,
        "run_count": len(all_results),
        "summaries": summaries,
        "test_comparisons": comparisons,
        "proof_validation": proof_validation,
        "contradictory_statuses": contradictory_statuses,
        "decision": decision(
            comparisons["larger"],
            proof_validation,
            contradictory_statuses,
        ),
    }
    return {
        **body,
        "summary_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }


def display(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float) and not math.isfinite(value):
        return "-"
    return str(value)


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# AC normalization mode results",
        "",
        f"- Runs: {summary['run_count']}",
        f"- Binary SHA-256: `{summary['binary_sha256']}`",
        (
            "- Proof claims: "
            f"{summary['proof_validation']['verified_cases']}/"
            f"{summary['proof_validation']['expected_cases']} verified"
        ),
        f"- Decision: `{summary['decision']['result']}`",
        "",
        "## Aggregate results",
        "",
        "| Phase | Budget | Mode | Solves | AC checks | AC hits | "
        "Flattened nodes | Median solved CPU | Median solved generated |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for phase, budgets in summary["summaries"].items():
        for budget, modes in budgets.items():
            for mode, values in modes.items():
                lines.append(
                    f"| {phase} | {budget} | {mode} | "
                    f"{values['reproducible_solved']} | "
                    f"{values['totals']['ac_checks']} | "
                    f"{values['totals']['ac_hits']} | "
                    f"{values['totals']['ac_flattened_nodes']} | "
                    f"{display(values['median_solved_cpu_seconds'])} | "
                    f"{display(values['median_solved_generated'])} |"
                )
    lines.extend(
        [
            "",
            "## Held-out comparisons against `none`",
            "",
            "| Budget | Mode | Solves | Mode-only | Baseline-only | "
            "CPU ratio | Generated ratio | AC hits | Flattened nodes |",
            "| --- | --- | ---: | --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for budget, modes in summary["test_comparisons"].items():
        for mode, values in modes.items():
            lines.append(
                f"| {budget} | {mode} | {values['mode_solved']} | "
                f"{', '.join(values['mode_only']) or '-'} | "
                f"{', '.join(values['baseline_only']) or '-'} | "
                f"{display(values['paired_median_cpu_ratio'])} | "
                f"{display(values['paired_median_generated_ratio'])} | "
                f"{values['ac_hits']} | {values['ac_flattened_nodes']} |"
            )
    lines.extend(
        [
            "",
            "The ratios are candidate divided by the `none` baseline on "
            "common reproducible solves; lower is better.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        arguments.experiment_root.resolve(),
        arguments.proof_validation.resolve(),
    )
    arguments.output.resolve().write_bytes(canonical_json(summary) + b"\n")
    arguments.markdown.resolve().write_text(
        render_markdown(summary), encoding="utf-8"
    )
    print(
        f"OK: {summary['run_count']} runs; "
        f"{summary['summary_id']}; {summary['decision']['result']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        AnalysisError,
        BASE.AnalysisError,
        OSError,
        KeyError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
