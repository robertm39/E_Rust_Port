#!/usr/bin/env python3
"""Verify and summarize the staged stronger-redundancy experiment."""

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
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"CounterSatisfiable", "Satisfiable"}
MECHANISM_PATHS = {
    "forward_subsumed": ("search_funnel", "forward_subsumed"),
    "aggressively_forward_subsumed": (
        "simplification",
        "aggressively_forward_subsumed",
    ),
    "contextual_simplify_reflections": (
        "simplification",
        "contextual_simplify_reflections",
    ),
    "backward_subsumed": ("simplification", "backward_subsumed"),
    "backward_rewritten": ("simplification", "backward_rewritten"),
    "condensation_attempts": (
        "simplification",
        "condensation_attempts",
    ),
    "condensation_successes": (
        "simplification",
        "condensation_successes",
    ),
    "rewrite_steps": ("simplification", "rewrite_steps"),
    "clause_subsumption_calls": (
        "indices",
        "clause_subsumption_calls",
    ),
    "clause_subsumption_successes": (
        "indices",
        "clause_subsumption_successes",
    ),
    "unit_subsumption_calls": ("indices", "unit_subsumption_calls"),
}
PAIR_METRICS = {
    "cpu": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "final_total": ("search_funnel", "final_total"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "maximum_resident_pages": (
        "resources",
        "maximum_resident_pages",
    ),
}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("stronger_redundancy_base_analyze", BASE_ANALYZE_PATH)
AnalysisError = BASE.AnalysisError


def rounded_median(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return (
        None
        if not materialized
        else round(statistics.median(materialized), 6)
    )


def status_polarity(status: str | None) -> str | None:
    if status in PROOF_STATUSES:
        return "proof"
    if status in MODEL_STATUSES:
        return "model"
    return None


def category_coverage(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, int]:
    coverage = BASE.reproducible_coverage(
        results, strategy, budget, contract["repetitions"]
    )
    by_category: dict[str, set[str]] = defaultdict(set)
    for result in results:
        if result["problem_id"] in coverage:
            by_category[result["category"]].add(result["problem_id"])
    return {
        category: len(problem_ids)
        for category, problem_ids in sorted(by_category.items())
    }


def mechanism_summary(
    results: Sequence[dict[str, Any]], strategy: str, budget: str
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == strategy and result["budget"] == budget
    ]
    summary: dict[str, Any] = {}
    for name, path in MECHANISM_PATHS.items():
        values = [
            value
            for result in selected
            if (value := BASE.metric(result, *path)) is not None
        ]
        summary[name] = {
            "records": len(values),
            "positive_records": sum(value > 0 for value in values),
            "sum": sum(values),
            "median": rounded_median(float(value) for value in values),
        }
    return summary


def aggregate_strategy(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, Any]:
    return {
        **BASE.aggregate_strategy(
            contract, results, strategy, budget
        ),
        "reproducible_solved_by_category": category_coverage(
            contract, results, strategy, budget
        ),
        "mechanisms": mechanism_summary(results, strategy, budget),
    }


def paired_all_run_ratios(
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    budget: str,
) -> dict[str, float | None]:
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
    }
    coordinates = sorted(
        {
            (problem_id, repetition)
            for problem_id, strategy, repetition in indexed
            if strategy == left
            and (problem_id, right, repetition) in indexed
        }
    )
    output: dict[str, float | None] = {}
    for name, path in PAIR_METRICS.items():
        ratios = []
        for problem_id, repetition in coordinates:
            left_value = BASE.metric(
                indexed[(problem_id, left, repetition)], *path
            )
            right_value = BASE.metric(
                indexed[(problem_id, right, repetition)], *path
            )
            if (
                left_value is not None
                and right_value is not None
                and right_value != 0
            ):
                ratios.append(float(left_value) / float(right_value))
        output[f"median_{name}_ratio"] = rounded_median(ratios)
    output["paired_coordinates"] = len(coordinates)
    return output


def direct_reference_audit(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    indexed_name: str,
    direct_name: str,
) -> dict[str, Any]:
    indexed = {
        (
            result["problem_id"],
            result["budget"],
            result["repetition"],
        ): result
        for result in results
        if result["strategy"] == indexed_name
    }
    direct = {
        (
            result["problem_id"],
            result["budget"],
            result["repetition"],
        ): result
        for result in results
        if result["strategy"] == direct_name
    }
    if indexed.keys() != direct.keys():
        raise AnalysisError(
            f"indexed/direct coordinates differ for {indexed_name}"
        )
    disagreements = []
    terminal_pairs = 0
    exact_status_pairs = 0
    for coordinate in sorted(indexed):
        indexed_status = indexed[coordinate]["szs_status"]
        direct_status = direct[coordinate]["szs_status"]
        indexed_polarity = status_polarity(indexed_status)
        direct_polarity = status_polarity(direct_status)
        exact_status_pairs += int(indexed_status == direct_status)
        if indexed_polarity is not None and direct_polarity is not None:
            terminal_pairs += 1
            if indexed_polarity != direct_polarity:
                disagreements.append(
                    {
                        "problem_id": coordinate[0],
                        "budget": coordinate[1],
                        "repetition": coordinate[2],
                        "indexed_status": indexed_status,
                        "direct_status": direct_status,
                    }
                )
    comparisons = {
        budget: BASE.comparison(
            contract,
            results,
            indexed_name,
            direct_name,
            budget,
        )
        for budget in contract["budgets"]
    }
    return {
        "indexed": indexed_name,
        "direct": direct_name,
        "coordinates": len(indexed),
        "terminal_pairs": terminal_pairs,
        "exact_status_pairs": exact_status_pairs,
        "polarity_disagreements": disagreements,
        "comparisons": comparisons,
    }


def behavior_effect_count(
    results: Sequence[dict[str, Any]],
    selected: str,
    baseline: str,
    budget: str,
) -> int:
    paths = [
        ("search_funnel", "forward_subsumed"),
        ("search_funnel", "generated"),
        ("search_funnel", "high_water_total"),
        ("simplification", "aggressively_forward_subsumed"),
        ("simplification", "contextual_simplify_reflections"),
        ("simplification", "condensation_successes"),
        ("simplification", "rewrite_steps"),
    ]
    indexed = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
    }
    affected = 0
    for problem_id, strategy, repetition in sorted(indexed):
        if strategy != selected:
            continue
        baseline_key = (problem_id, baseline, repetition)
        if baseline_key not in indexed:
            continue
        if any(
            BASE.metric(indexed[(problem_id, selected, repetition)], *path)
            != BASE.metric(indexed[baseline_key], *path)
            for path in paths
        ):
            affected += 1
    return affected


def decision(
    comparison: dict[str, Any],
    all_run_ratios: dict[str, Any],
    *,
    proof_complete: bool,
    reference_disagreements: int,
    behavior_effects: int,
    contradictory_statuses: int,
) -> dict[str, Any]:
    valid = (
        proof_complete
        and reference_disagreements == 0
        and behavior_effects > 0
        and contradictory_statuses == 0
    )
    unique = (
        len(comparison["left_only"]) >= 2
        and not comparison["right_only"]
    )
    cpu = all_run_ratios["median_cpu_ratio"]
    generated = all_run_ratios["median_generated_ratio"]
    high_water = all_run_ratios["median_high_water_total_ratio"]
    rss = all_run_ratios["median_maximum_resident_pages_ratio"]
    efficient = (
        not comparison["right_only"]
        and cpu is not None
        and cpu <= 0.95
        and generated is not None
        and generated <= 0.90
        and high_water is not None
        and high_water <= 0.95
        and rss is not None
        and rss <= 1.05
    )
    advances = valid and (unique or efficient)
    return {
        "result": (
            "advance_selective_redundancy_dispatch"
            if advances
            else "retain_existing_redundancy_defaults"
        ),
        "advances": advances,
        "validity_gate_passed": valid,
        "unique_solve_gate_passed": unique,
        "efficiency_gate_passed": efficient,
        "criteria": {
            "selected_only_solves": 2,
            "baseline_only_solves": 0,
            "median_cpu_ratio": 0.95,
            "median_generated_ratio": 0.90,
            "median_high_water_ratio": 0.95,
            "median_maximum_resident_pages_ratio": 1.05,
            "requires_observed_behavior_effect": True,
            "requires_all_proof_claims_verified": True,
            "requires_zero_reference_polarity_disagreements": True,
            "requires_zero_contradictory_statuses": True,
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
    final_selection = BASE.load_selection(final_selection_path)
    if final_selection["source_phase"] != "validation":
        raise AnalysisError("final selection is not validation-derived")
    chosen = final_selection["selected_strategies"][0]
    test_contract, test_results = phases["test"]
    if (
        test_contract["selection"]["selection_id"]
        != final_selection["selection_id"]
    ):
        raise AnalysisError("test contract pins another selection")
    proof_validation = BASE.load_proof_validation(proof_validation_path)
    if proof_validation["test_contract_id"] != test_contract["contract_id"]:
        raise AnalysisError("proof validation names another test contract")

    phase_summaries: dict[str, Any] = {}
    all_results: list[dict[str, Any]] = []
    for phase, (contract, results) in phases.items():
        all_results.extend(results)
        phase_summaries[phase] = {
            budget: {
                name: aggregate_strategy(
                    contract, results, name, budget
                )
                for name in contract["strategies"]
            }
            for budget in contract["budgets"]
        }

    comparisons = {
        budget: BASE.comparison(
            test_contract,
            test_results,
            chosen,
            "baseline",
            budget,
        )
        for budget in test_contract["budgets"]
    }
    all_run_ratios = {
        budget: paired_all_run_ratios(
            test_results, chosen, "baseline", budget
        )
        for budget in test_contract["budgets"]
    }
    reference = {
        "baseline": direct_reference_audit(
            test_contract,
            test_results,
            "baseline",
            "baseline_direct",
        ),
        "selected": direct_reference_audit(
            test_contract,
            test_results,
            chosen,
            "selected_direct",
        ),
    }
    contradictory = [
        {
            "phase": result["phase"],
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "budget": result["budget"],
            "repetition": result["repetition"],
            "status": result["szs_status"],
            "expected_class": result["expected_class"],
        }
        for result in all_results
        if status_polarity(result["szs_status"]) is not None
        and not result["expected_status_match"]
    ]
    proof_complete = (
        proof_validation["all_verified"]
        and proof_validation["verified_cases"]
        == proof_validation["expected_cases"]
    )
    behavior_effects = behavior_effect_count(
        test_results, chosen, "baseline", "larger"
    )
    reference_disagreements = sum(
        len(audit["polarity_disagreements"])
        for audit in reference.values()
    )
    final_decision = decision(
        comparisons["larger"],
        all_run_ratios["larger"],
        proof_complete=proof_complete,
        reference_disagreements=reference_disagreements,
        behavior_effects=behavior_effects,
        contradictory_statuses=len(contradictory),
    )
    return {
        "schema_version": 1,
        "contracts": {
            phase: contract["contract_id"]
            for phase, (contract, _) in phases.items()
        },
        "binary_sha256": test_contract["binary_sha256"],
        "selected_strategy": chosen,
        "selected_features": test_contract["strategies"][chosen]["features"],
        "problem_counts": {
            phase: len(contract["selected_problem_ids"])
            for phase, (contract, _) in phases.items()
        },
        "run_count": len(all_results),
        "phase_summaries": phase_summaries,
        "test_comparisons": comparisons,
        "test_all_run_ratios": all_run_ratios,
        "direct_reference": reference,
        "behavior_effect_coordinates": behavior_effects,
        "proof_validation": proof_validation,
        "contradictory_statuses": contradictory,
        "decision": final_decision,
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Stronger redundancy results",
        "",
        f"- Selected strategy: `{summary['selected_strategy']}`",
        f"- Umlaut binary SHA-256: `{summary['binary_sha256']}`",
        (
            "- Problems: "
            f"{summary['problem_counts']['calibration']} calibration, "
            f"{summary['problem_counts']['validation']} validation, "
            f"{summary['problem_counts']['test']} test"
        ),
        f"- Runs: {summary['run_count']}",
        "",
        "## End-to-end results",
        "",
        "| Phase | Budget | Strategy | Solves | By category | "
        "Median CPU (s) | Generated | High-water | Max RSS pages |",
        "| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |",
    ]
    for phase, budgets in summary["phase_summaries"].items():
        for budget, strategies in budgets.items():
            for strategy, values in strategies.items():
                lines.append(
                    f"| {phase} | {budget} | `{strategy}` | "
                    f"{values['reproducible_solved']} | "
                    f"`{values['reproducible_solved_by_category']}` | "
                    f"{values['median_solved_cpu_seconds']} | "
                    f"{values['median_solved_generated']} | "
                    f"{values['median_solved_high_water_total']} | "
                    f"{values['median_solved_maximum_resident_pages']} |"
                )
    lines.extend(["", "## Held-out selected versus baseline", ""])
    for budget, comparison in summary["test_comparisons"].items():
        ratios = summary["test_all_run_ratios"][budget]
        lines.extend(
            [
                f"### {budget}",
                "",
                (
                    f"Selected solved {comparison['left_solved']}; baseline "
                    f"solved {comparison['right_solved']}."
                ),
                "",
                f"- Selected-only: {comparison['left_only'] or 'none'}",
                f"- Baseline-only: {comparison['right_only'] or 'none'}",
                f"- All-run paired CPU ratio: {ratios['median_cpu_ratio']}",
                (
                    "- All-run paired generated ratio: "
                    f"{ratios['median_generated_ratio']}"
                ),
                (
                    "- All-run paired final-clause ratio: "
                    f"{ratios['median_final_total_ratio']}"
                ),
                (
                    "- All-run paired high-water ratio: "
                    f"{ratios['median_high_water_total_ratio']}"
                ),
                (
                    "- All-run paired max-RSS-pages ratio: "
                    f"{ratios['median_maximum_resident_pages_ratio']}"
                ),
                "",
            ]
        )
    lines.extend(["## Slow-reference audit", ""])
    for name, audit in summary["direct_reference"].items():
        lines.append(
            f"- {name}: {audit['terminal_pairs']} terminal pairs, "
            f"{len(audit['polarity_disagreements'])} polarity disagreements."
        )
    proof = summary["proof_validation"]
    lines.extend(
        [
            "",
            "## Independent proof validation",
            "",
            (
                f"ProofCheck verified {proof['verified_cases']} of "
                f"{proof['expected_cases']} reproducible proof claims."
            ),
            "",
            "## Decision",
            "",
            f"- Result: `{summary['decision']['result']}`.",
            (
                "- Observed selected/baseline behavior differences: "
                f"{summary['behavior_effect_coordinates']} coordinates."
            ),
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
    arguments.json_output.write_bytes(
        BASE.canonical_json(summary) + b"\n"
    )
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
    except (
        AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
