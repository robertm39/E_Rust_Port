#!/usr/bin/env python3
"""Verify and summarize the staged fingerprint-index bake-off."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_ANALYZE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "analyze.py"
)
NON_PROOF_STATUSES = {"CounterSatisfiable", "Satisfiable"}
BASELINE = "baseline_fp7"


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("index_retrieval_analysis_base", BASE_ANALYZE_PATH)
AnalysisError = BASE.AnalysisError


def rounded_median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    if not materialized:
        return None
    return round(statistics.median(materialized), 6)


def fingerprint(result: dict[str, Any], key: str) -> int | float | None:
    return BASE.metric(result, "indices", "fingerprint", key)


def structure_total(result: dict[str, Any], key: str) -> int | None:
    telemetry = result.get("_telemetry")
    if telemetry is None:
        return None
    structures = (
        telemetry.get("indices", {})
        .get("fingerprint", {})
        .get("structures", {})
    )
    values = [
        structure.get(key)
        for structure in structures.values()
        if isinstance(structure, dict)
    ]
    if not values or any(not isinstance(value, int) for value in values):
        return None
    return sum(values)


def safe_ratio(numerator: int | float | None, denominator: Any) -> float | None:
    if (
        numerator is None
        or not isinstance(denominator, (int, float))
        or isinstance(denominator, bool)
        or denominator == 0
    ):
        return None
    return numerator / denominator


def derived_metric(
    result: dict[str, Any], name: str
) -> int | float | None:
    if name == "cpu":
        return BASE.metric(result, "resources", "total_cpu_seconds")
    if name == "generated":
        return BASE.metric(result, "search_funnel", "generated")
    if name == "processed":
        return BASE.metric(result, "search_funnel", "processed")
    if name == "high_water":
        return BASE.metric(result, "search_funnel", "high_water_total")
    if name == "rss":
        return BASE.metric(result, "resources", "maximum_resident_pages")
    if name == "term_storage":
        return BASE.metric(result, "terms", "storage_estimate_bytes")
    if name == "unification_queries":
        return fingerprint(result, "unification_queries")
    if name == "unification_candidates":
        return fingerprint(result, "unification_candidates")
    if name == "unification_candidates_per_query":
        return safe_ratio(
            fingerprint(result, "unification_candidates"),
            fingerprint(result, "unification_queries"),
        )
    if name == "paramodulation_precision":
        return safe_ratio(
            fingerprint(result, "paramodulation_unifiable_candidates"),
            fingerprint(result, "paramodulation_candidates"),
        )
    if name == "match_candidates_per_query":
        return safe_ratio(
            fingerprint(result, "match_candidates"),
            fingerprint(result, "match_queries"),
        )
    if name == "backward_rewrite_precision":
        return safe_ratio(
            BASE.metric(
                result, "indices", "backward_rewrite_match_successes"
            ),
            fingerprint(result, "match_candidates"),
        )
    if name == "index_nodes":
        return structure_total(result, "nodes")
    if name == "index_entries":
        return structure_total(result, "entries")
    raise AnalysisError(f"unknown derived metric: {name}")


WORKLOAD_METRICS = (
    "cpu",
    "generated",
    "processed",
    "high_water",
    "rss",
    "term_storage",
    "unification_queries",
    "unification_candidates",
    "unification_candidates_per_query",
    "paramodulation_precision",
    "match_candidates_per_query",
    "backward_rewrite_precision",
    "index_nodes",
    "index_entries",
)


def workload_summary(
    results: Sequence[dict[str, Any]], strategy: str, budget: str
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == strategy
        and result["budget"] == budget
        and result["_telemetry"] is not None
    ]
    by_category: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in selected:
        by_category[result["category"]].append(result)

    def summarize(group: Sequence[dict[str, Any]]) -> dict[str, Any]:
        return {
            metric: rounded_median(
                value
                for result in group
                if (value := derived_metric(result, metric)) is not None
            )
            for metric in WORKLOAD_METRICS
        }

    return {
        "telemetry_records": len(selected),
        "all": summarize(selected),
        "by_category": {
            category: summarize(group)
            for category, group in sorted(by_category.items())
        },
    }


def paired_metric_ratios(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    selected: str,
    baseline: str,
    budget: str,
    *,
    category: str | None = None,
    common_solved_only: bool = False,
) -> dict[str, Any]:
    repetitions = contract["repetitions"]
    selected_coverage = BASE.reproducible_coverage(
        results, selected, budget, repetitions
    )
    baseline_coverage = BASE.reproducible_coverage(
        results, baseline, budget, repetitions
    )
    allowed = (
        selected_coverage & baseline_coverage
        if common_solved_only
        else set(contract["selected_problem_ids"])
    )
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
        and result["strategy"] in {selected, baseline}
        and (category is None or result["category"] == category)
    }
    ratios: dict[str, list[float]] = {
        metric: [] for metric in WORKLOAD_METRICS
    }
    coordinates = 0
    for problem_id in sorted(allowed):
        for repetition in range(1, repetitions + 1):
            left = indexed.get((problem_id, selected, repetition))
            right = indexed.get((problem_id, baseline, repetition))
            if left is None or right is None:
                continue
            coordinates += 1
            for metric in WORKLOAD_METRICS:
                ratio = safe_ratio(
                    derived_metric(left, metric),
                    derived_metric(right, metric),
                )
                if ratio is not None:
                    ratios[metric].append(ratio)
    return {
        "coordinates": coordinates,
        "common_solved_only": common_solved_only,
        "median_selected_over_baseline": {
            metric: rounded_median(values)
            for metric, values in ratios.items()
        },
    }


def polarity_disagreements(
    results: Sequence[dict[str, Any]],
    selected: str,
) -> list[dict[str, Any]]:
    indexed = {
        (
            result["problem_id"],
            result["budget"],
            result["repetition"],
            result["strategy"],
        ): result
        for result in results
        if result["strategy"] in {selected, BASELINE}
    }
    disagreements = []
    coordinates = {
        (problem, budget, repetition)
        for problem, budget, repetition, _strategy in indexed
    }
    for problem, budget, repetition in sorted(coordinates):
        left = indexed[(problem, budget, repetition, selected)]
        right = indexed[(problem, budget, repetition, BASELINE)]
        left_proof = left["szs_status"] in BASE.PROOF_STATUSES
        right_proof = right["szs_status"] in BASE.PROOF_STATUSES
        left_model = left["szs_status"] in NON_PROOF_STATUSES
        right_model = right["szs_status"] in NON_PROOF_STATUSES
        if (left_proof and right_model) or (left_model and right_proof):
            disagreements.append(
                {
                    "problem_id": problem,
                    "budget": budget,
                    "repetition": repetition,
                    "selected_status": left["szs_status"],
                    "baseline_status": right["szs_status"],
                }
            )
    return disagreements


def decision(
    larger: dict[str, Any],
    ratios: dict[str, Any],
    proof_complete: bool,
    disagreements: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    values = ratios["median_selected_over_baseline"]
    unique_gain = (
        len(larger["left_only"]) >= 2
        and not larger["right_only"]
        and proof_complete
        and not disagreements
    )
    efficient = (
        not larger["right_only"]
        and values["cpu"] is not None
        and values["cpu"] <= 0.95
        and values["generated"] is not None
        and values["generated"] <= 1.02
        and values["high_water"] is not None
        and values["high_water"] <= 1.02
        and values["rss"] is not None
        and values["rss"] <= 1.05
        and proof_complete
        and not disagreements
    )
    advance = unique_gain or efficient
    return {
        "result": (
            "adopt_selected_index_default"
            if advance
            else "retain_fp7_default"
        ),
        "advance": advance,
        "unique_solve_condition": unique_gain,
        "efficiency_condition": efficient,
        "criteria": {
            "selected_only_solves": 2,
            "baseline_only_solves": 0,
            "cpu_ratio_max": 0.95,
            "generated_ratio_max": 1.02,
            "high_water_ratio_max": 1.02,
            "rss_ratio_max": 1.05,
            "all_proofs_verified": True,
            "polarity_disagreements": 0,
        },
    }


def analyze(
    experiment_root: Path,
    final_selection_path: Path,
    proof_validation_path: Path,
) -> dict[str, Any]:
    phases = {
        phase: BASE.load_phase(experiment_root, phase)
        for phase in ("calibration", "validation", "test")
    }
    selection = BASE.load_selection(final_selection_path)
    selected = selection["selected_strategies"][0]
    test_contract, test_results = phases["test"]
    if test_contract["selection"]["selection_id"] != selection["selection_id"]:
        raise AnalysisError("test contract does not pin final selection")
    proof_validation = BASE.load_proof_validation(proof_validation_path)
    if proof_validation["test_contract_id"] != test_contract["contract_id"]:
        raise AnalysisError("proof validation names another test contract")
    proof_complete = (
        proof_validation["all_verified"]
        and proof_validation["verified_cases"]
        == proof_validation["expected_cases"]
    )

    phase_summaries = {}
    for phase, (contract, results) in phases.items():
        phase_summaries[phase] = {
            budget: {
                strategy: {
                    "search": BASE.aggregate_strategy(
                        contract, results, strategy, budget
                    ),
                    "workload": workload_summary(
                        results, strategy, budget
                    ),
                }
                for strategy in contract["strategies"]
            }
            for budget in contract["budgets"]
        }

    comparisons = {}
    for budget in test_contract["budgets"]:
        comparisons[budget] = {
            "coverage_and_solved_metrics": BASE.comparison(
                test_contract,
                test_results,
                selected,
                BASELINE,
                budget,
            ),
            "all_coordinates": paired_metric_ratios(
                test_contract,
                test_results,
                selected,
                BASELINE,
                budget,
            ),
            "common_solved": paired_metric_ratios(
                test_contract,
                test_results,
                selected,
                BASELINE,
                budget,
                common_solved_only=True,
            ),
            "by_category": {
                category: paired_metric_ratios(
                    test_contract,
                    test_results,
                    selected,
                    BASELINE,
                    budget,
                    category=category,
                )
                for category in ("FEQ", "FNE", "UEQ")
            },
        }

    disagreements = polarity_disagreements(test_results, selected)
    larger = comparisons["larger"]["coverage_and_solved_metrics"]
    final_decision = decision(
        larger,
        comparisons["larger"]["common_solved"],
        proof_complete,
        disagreements,
    )
    all_results = [
        result for _contract, results in phases.values() for result in results
    ]
    body = {
        "schema_version": 1,
        "contracts": {
            phase: contract["contract_id"]
            for phase, (contract, _results) in phases.items()
        },
        "binary_sha256": test_contract["binary_sha256"],
        "selected_strategy": selected,
        "selected_index_variant": test_contract["strategies"][selected][
            "index_variant"
        ],
        "problem_counts": {
            phase: len(contract["selected_problem_ids"])
            for phase, (contract, _results) in phases.items()
        },
        "run_count": len(all_results),
        "phase_summaries": phase_summaries,
        "test_comparisons": comparisons,
        "proof_validation": proof_validation,
        "polarity_disagreements": disagreements,
        "decision": final_decision,
    }
    return {
        **body,
        "report_id": hashlib.sha256(BASE.canonical_json(body)).hexdigest(),
    }


def render_markdown(summary: dict[str, Any]) -> str:
    selected = summary["selected_strategy"]
    lines = [
        "# Index-retrieval bake-off results",
        "",
        f"Selected validation finalist: `{selected}` "
        f"(`{summary['selected_index_variant']}`).",
        "",
        "| Budget | Selected solves | FP7 solves | Selected-only | "
        "FP7-only | CPU ratio | Generated ratio | High-water ratio | "
        "RSS ratio |",
        "| --- | ---: | ---: | --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for budget, comparison in summary["test_comparisons"].items():
        coverage = comparison["coverage_and_solved_metrics"]
        ratios = comparison["common_solved"][
            "median_selected_over_baseline"
        ]
        lines.append(
            f"| {budget} | {coverage['left_solved']} | "
            f"{coverage['right_solved']} | "
            f"{', '.join(coverage['left_only']) or 'none'} | "
            f"{', '.join(coverage['right_only']) or 'none'} | "
            f"{ratios['cpu']} | {ratios['generated']} | "
            f"{ratios['high_water']} | {ratios['rss']} |"
        )
    lines.extend(
        [
            "",
            "## Larger-budget workload crossovers",
            "",
            "| Category | Coordinates | CPU ratio | Candidates/query ratio | "
            "Paramod precision ratio | Node ratio | Entry ratio |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    by_category = summary["test_comparisons"]["larger"]["by_category"]
    for category, comparison in by_category.items():
        ratios = comparison["median_selected_over_baseline"]
        lines.append(
            f"| {category} | {comparison['coordinates']} | "
            f"{ratios['cpu']} | "
            f"{ratios['unification_candidates_per_query']} | "
            f"{ratios['paramodulation_precision']} | "
            f"{ratios['index_nodes']} | {ratios['index_entries']} |"
        )
    proof = summary["proof_validation"]
    lines.extend(
        [
            "",
            "## Validation and decision",
            "",
            f"- ProofCheck: {proof['verified_cases']}/"
            f"{proof['expected_cases']} claims verified.",
            f"- Polarity disagreements: "
            f"{len(summary['polarity_disagreements'])}.",
            f"- Decision: `{summary['decision']['result']}`.",
            f"- Report ID: `{summary['report_id']}`.",
            "",
        ]
    )
    return "\n".join(lines)


def compact_summary(summary: dict[str, Any]) -> dict[str, Any]:
    validation = summary["phase_summaries"]["validation"]["validation"]
    comparisons = summary["test_comparisons"]
    proof = summary["proof_validation"]
    full_encoded = BASE.canonical_json(summary) + b"\n"
    return {
        "schema_version": 1,
        "report_id": summary["report_id"],
        "full_summary_sha256": hashlib.sha256(full_encoded).hexdigest(),
        "binary_sha256": summary["binary_sha256"],
        "contracts": summary["contracts"],
        "problem_counts": summary["problem_counts"],
        "run_count": summary["run_count"],
        "selected_strategy": summary["selected_strategy"],
        "selected_index_variant": summary["selected_index_variant"],
        "validation": {
            strategy: data["search"]
            for strategy, data in validation.items()
        },
        "test": {
            budget: {
                "coverage_and_solved_metrics": data[
                    "coverage_and_solved_metrics"
                ],
                "common_solved_ratios": data["common_solved"],
            }
            for budget, data in comparisons.items()
        },
        "larger_budget_category_crossovers": comparisons["larger"][
            "by_category"
        ],
        "proof_validation": {
            "report_id": proof["report_id"],
            "expected_cases": proof["expected_cases"],
            "verified_cases": proof["verified_cases"],
            "all_verified": proof["all_verified"],
            "proofcheck_executable_sha256": proof["proofcheck"][
                "executable_sha256"
            ],
        },
        "polarity_disagreements": summary["polarity_disagreements"],
        "decision": summary["decision"],
    }


def write_if_requested(path: Path | None, data: bytes) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--final-selection", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path, required=True)
    parser.add_argument("--output-json", type=Path)
    parser.add_argument("--output-full-json", type=Path)
    parser.add_argument("--output-markdown", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        arguments.experiment_root.resolve(),
        arguments.final_selection.resolve(),
        arguments.proof_validation.resolve(),
    )
    encoded = BASE.canonical_json(compact_summary(summary)) + b"\n"
    full_encoded = BASE.canonical_json(summary) + b"\n"
    markdown = render_markdown(summary).encode()
    write_if_requested(arguments.output_json, encoded)
    write_if_requested(arguments.output_full_json, full_encoded)
    write_if_requested(arguments.output_markdown, markdown)
    print(
        f"OK: {summary['run_count']} runs; "
        f"{summary['decision']['result']}; "
        f"report {summary['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
