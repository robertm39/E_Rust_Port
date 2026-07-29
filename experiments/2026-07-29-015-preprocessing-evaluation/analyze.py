#!/usr/bin/env python3
"""Verify and summarize the frozen preprocessing evaluation."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import statistics
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PRIOR_ANALYZE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-29-014-rewrite-cache-evaluation"
    / "analyze.py"
)
CANDIDATES = ("bce", "predicate", "goal_defs")
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"CounterSatisfiable", "Satisfiable"}
PAIR_METRICS = {
    "cpu": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "term_storage_bytes": ("terms", "storage_estimate_bytes"),
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


PRIOR = load_module("preprocessing_prior_analyze", PRIOR_ANALYZE_PATH)
BASE = PRIOR.BASE
AnalysisError = BASE.AnalysisError


def median(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return (
        None
        if not materialized
        else round(statistics.median(materialized), 6)
    )


def safe_ratio(
    numerator: int | float | None,
    denominator: int | float | None,
) -> float | None:
    if numerator is None or denominator in (None, 0):
        return None
    return float(numerator) / float(denominator)


def indexed_results(
    results: Sequence[dict[str, Any]], budget: str
) -> dict[tuple[str, str, int], dict[str, Any]]:
    return {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
        if result["budget"] == budget
    }


def paired_ratios(
    results: Sequence[dict[str, Any]],
    budget: str,
    candidate: str,
    problem_ids: set[str] | None = None,
) -> dict[str, Any]:
    indexed = indexed_results(results, budget)
    coordinates = sorted(
        {
            (problem_id, repetition)
            for problem_id, strategy, repetition in indexed
            if strategy == candidate
            and (problem_id, "baseline", repetition) in indexed
            and (problem_ids is None or problem_id in problem_ids)
        }
    )
    report: dict[str, Any] = {"paired_coordinates": len(coordinates)}
    for name, path in PAIR_METRICS.items():
        ratios = []
        for problem_id, repetition in coordinates:
            candidate_value = BASE.metric(
                indexed[(problem_id, candidate, repetition)], *path
            )
            baseline_value = BASE.metric(
                indexed[(problem_id, "baseline", repetition)], *path
            )
            ratio = safe_ratio(candidate_value, baseline_value)
            if ratio is not None:
                ratios.append(ratio)
        report[f"median_{name}_ratio"] = median(ratios)
    return report


def transformation_values(
    result: dict[str, Any], strategy: str
) -> tuple[int, int]:
    if result["_telemetry"] is None:
        return 0, 0
    if strategy == "bce":
        removed = BASE.metric(
            result,
            "input_funnel",
            "transformations",
            "blocked_clause_elimination",
            "removed",
        )
        return int(removed or 0), 0
    if strategy == "predicate":
        removed = BASE.metric(
            result,
            "input_funnel",
            "transformations",
            "predicate_elimination",
            "removed",
        )
        generated = BASE.metric(
            result,
            "input_funnel",
            "transformations",
            "predicate_elimination",
            "generated",
        )
        return int(removed or 0), int(generated or 0)
    if strategy == "goal_defs":
        added = BASE.metric(
            result,
            "input_funnel",
            "transformations",
            "goal_definitions",
            "added",
        )
        return 0, int(added or 0)
    return 0, 0


def transformation_activity(
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, Any]:
    selected = [
        result
        for result in results
        if result["strategy"] == strategy
        and result["budget"] == budget
        and result["_telemetry"] is not None
    ]
    active = []
    removed_total = 0
    generated_or_added_total = 0
    for result in selected:
        removed, generated_or_added = transformation_values(
            result, strategy
        )
        removed_total += removed
        generated_or_added_total += generated_or_added
        if removed != 0 or generated_or_added != 0:
            active.append(result)
    return {
        "telemetry_records": len(selected),
        "active_coordinates": len(active),
        "active_problem_ids": sorted(
            {result["problem_id"] for result in active}
        ),
        "removed_total": removed_total,
        "generated_or_added_total": generated_or_added_total,
    }


def maximum_rss_ratio(
    results: Sequence[dict[str, Any]],
    budget: str,
    candidate: str,
) -> dict[str, Any]:
    maxima = {}
    for strategy in ("baseline", candidate):
        values = [
            value
            for result in results
            if result["strategy"] == strategy
            and result["budget"] == budget
            and (
                value := BASE.metric(
                    result,
                    "resources",
                    "maximum_resident_pages",
                )
            )
            is not None
        ]
        if not values:
            raise AnalysisError(
                f"{strategy}/{budget} has no RSS telemetry"
            )
        maxima[strategy] = max(values)
    return {
        "candidate_maximum_resident_pages": maxima[candidate],
        "baseline_maximum_resident_pages": maxima["baseline"],
        "candidate_over_baseline": round(
            maxima[candidate] / maxima["baseline"], 6
        ),
    }


def analyze_candidate(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    budget: str,
    candidate: str,
) -> dict[str, Any]:
    comparison = BASE.comparison(
        contract, results, candidate, "baseline", budget
    )
    common = set(comparison["common_ids"])
    return {
        "strategies": {
            strategy: BASE.aggregate_strategy(
                contract, results, strategy, budget
            )
            for strategy in ("baseline", candidate)
        },
        "coverage_comparison": comparison,
        "transformation_activity": transformation_activity(
            results, candidate, budget
        ),
        "all_run_ratios": paired_ratios(
            results, budget, candidate
        ),
        "common_solved_ratios": paired_ratios(
            results, budget, candidate, common
        ),
        "maximum_rss": maximum_rss_ratio(
            results, budget, candidate
        ),
    }


def analyze_phase(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    if set(contract["strategies"]) != {
        "baseline",
        *CANDIDATES,
    }:
        raise AnalysisError("unexpected preprocessing strategies")
    budgets = {
        budget: {
            candidate: analyze_candidate(
                contract, results, budget, candidate
            )
            for candidate in CANDIDATES
        }
        for budget in contract["budgets"]
    }
    return {
        "contract_id": contract["contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "run_count": len(results),
        "budgets": budgets,
    }


def status_polarity(status: str | None) -> str | None:
    if status in PROOF_STATUSES:
        return "proof"
    if status in MODEL_STATUSES:
        return "model"
    return None


def polarity_audit(
    phase_results: dict[str, Sequence[dict[str, Any]]],
) -> dict[str, Any]:
    paired_coordinates = 0
    exact_status_pairs = 0
    disagreements = []
    for phase, results in sorted(phase_results.items()):
        indexed = {
            (
                result["problem_id"],
                result["budget"],
                result["repetition"],
                result["strategy"],
            ): result
            for result in results
        }
        for candidate in CANDIDATES:
            for coordinate, result in sorted(indexed.items()):
                if coordinate[3] != candidate:
                    continue
                problem_id, budget, repetition, _strategy = coordinate
                baseline = indexed[
                    (problem_id, budget, repetition, "baseline")
                ]
                paired_coordinates += 1
                exact_status_pairs += int(
                    result["szs_status"] == baseline["szs_status"]
                )
                result_polarity = status_polarity(
                    result["szs_status"]
                )
                baseline_polarity = status_polarity(
                    baseline["szs_status"]
                )
                if (
                    result_polarity is not None
                    and baseline_polarity is not None
                    and result_polarity != baseline_polarity
                ):
                    disagreements.append(
                        {
                            "phase": phase,
                            "candidate": candidate,
                            "problem_id": problem_id,
                            "budget": budget,
                            "repetition": repetition,
                            "candidate_status": result["szs_status"],
                            "baseline_status": baseline["szs_status"],
                        }
                    )
    return {
        "paired_coordinates": paired_coordinates,
        "exact_status_pairs": exact_status_pairs,
        "polarity_disagreements": disagreements,
    }


def missing_telemetry(
    phase_results: dict[str, Sequence[dict[str, Any]]],
) -> list[dict[str, Any]]:
    return [
        {
            "phase": phase,
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "budget": result["budget"],
            "repetition": result["repetition"],
            "szs_status": result["szs_status"],
            "return_code": result["return_code"],
        }
        for phase, results in sorted(phase_results.items())
        for result in results
        if result["_telemetry"] is None
    ]


def load_proof_validation(
    path: Path | None,
    contracts: dict[str, dict[str, Any]],
) -> dict[str, Any] | None:
    if path is None:
        return None
    report = json.loads(path.read_text(encoding="utf-8"))
    expected_contracts = {
        phase: contract["contract_id"]
        for phase, contract in contracts.items()
    }
    if report.get("contracts") != expected_contracts:
        raise AnalysisError("proof validation names other contracts")
    expected_binary = contracts["casc"]["binary_sha256"]
    if report.get("binary_sha256") != expected_binary:
        raise AnalysisError("proof validation names another binary")
    if report.get("verified_cases") != report.get("expected_cases"):
        raise AnalysisError("proof validation is incomplete")
    if not report.get("all_verified"):
        raise AnalysisError("proof validation rejected a claim")
    validity = report.get("candidate_validity")
    if not isinstance(validity, dict) or any(
        validity.get(candidate) is not True
        for candidate in CANDIDATES
    ):
        raise AnalysisError("proof validation lacks candidate coverage")
    return report


def exceeds(value: float | None, threshold: float) -> bool:
    return value is not None and value > threshold


def candidate_decision(
    report: dict[str, Any],
    correctness: bool,
) -> dict[str, Any]:
    comparison = report["coverage_comparison"]
    common = report["common_solved_ratios"]
    activity = report["transformation_activity"]
    rss = report["maximum_rss"]["candidate_over_baseline"]
    loss = bool(comparison["right_only"])
    unique = bool(comparison["left_only"])
    benefit = (
        common["median_cpu_ratio"] is not None
        and common["median_cpu_ratio"] <= 0.95
        and common["median_generated_ratio"] is not None
        and common["median_generated_ratio"] <= 1.02
        and common["median_high_water_total_ratio"] is not None
        and common["median_high_water_total_ratio"] <= 1.02
        and rss <= 1.05
    )
    regression = (
        exceeds(common["median_cpu_ratio"], 1.05)
        or exceeds(common["median_generated_ratio"], 1.05)
        or exceeds(common["median_high_water_total_ratio"], 1.05)
        or rss > 1.05
    )
    enough_reach = activity["active_coordinates"] >= 4
    followup = (
        correctness
        and enough_reach
        and not loss
        and (unique or benefit)
    )
    if not correctness:
        result = "inconclusive_retain_default_off"
    elif loss or regression:
        result = "reject_generated_schedule_followup"
    elif followup:
        result = "open_generated_schedule_followup"
    else:
        result = "retain_explicit_default_off"
    return {
        "result": result,
        "correctness_gate_passed": correctness,
        "enough_reach": enough_reach,
        "candidate_only_solve": unique,
        "baseline_only_solve": loss,
        "benefit_gate_passed": benefit,
        "material_regression": regression,
    }


def analyze(
    experiment_root: Path,
    proof_validation_path: Path | None,
) -> dict[str, Any]:
    contracts = {}
    phase_results = {}
    phases = {}
    for phase in ("casc", "differential"):
        contract, results = BASE.load_phase(experiment_root, phase)
        contracts[phase] = contract
        phase_results[phase] = results
        phases[phase] = analyze_phase(contract, results)

    proof_validation = load_proof_validation(
        proof_validation_path, contracts
    )
    polarity = polarity_audit(phase_results)
    missing = missing_telemetry(phase_results)
    unexpected_missing = [
        record
        for record in missing
        if record["szs_status"] != "ResourceOut"
    ]
    decisions = {}
    heldout = phases["casc"]["budgets"]["heldout"]
    for candidate in CANDIDATES:
        candidate_disagreements = [
            disagreement
            for disagreement in polarity["polarity_disagreements"]
            if disagreement["candidate"] == candidate
        ]
        correctness = (
            proof_validation is not None
            and not candidate_disagreements
            and not unexpected_missing
        )
        decisions[candidate] = candidate_decision(
            heldout[candidate], correctness
        )

    body = {
        "schema_version": 1,
        "phases": phases,
        "run_count": sum(
            phase["run_count"] for phase in phases.values()
        ),
        "missing_telemetry": missing,
        "polarity_audit": polarity,
        "proof_validation": proof_validation,
        "decisions": decisions,
        "criteria": {
            "minimum_active_coordinates": 4,
            "benefit_cpu_ratio": 0.95,
            "benefit_generated_ratio": 1.02,
            "benefit_high_water_ratio": 1.02,
            "benefit_maximum_rss_ratio": 1.05,
            "material_regression_ratio": 1.05,
        },
        "production_defaults_changed": False,
    }
    return {
        **body,
        "report_id": hashlib.sha256(
            BASE.canonical_json(body)
        ).hexdigest(),
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Preprocessing evaluation results",
        "",
        f"- Runs: {summary['run_count']}",
        (
            "- Independent proof claims: "
            f"{summary['proof_validation']['verified_cases']}/"
            f"{summary['proof_validation']['expected_cases']} verified"
            if summary["proof_validation"] is not None
            else "- Independent proof claims: pending"
        ),
        "",
        "| Candidate | Held-out active | Candidate solves | "
        "Baseline solves | Candidate-only | Baseline-only | CPU | "
        "Generated | High-water | Max RSS | Decision |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | "
        "---: | ---: | ---: | --- |",
    ]
    heldout = summary["phases"]["casc"]["budgets"]["heldout"]
    for candidate in CANDIDATES:
        report = heldout[candidate]
        comparison = report["coverage_comparison"]
        ratios = report["common_solved_ratios"]
        lines.append(
            f"| {candidate} | "
            f"{report['transformation_activity']['active_coordinates']} | "
            f"{comparison['left_solved']} | "
            f"{comparison['right_solved']} | "
            f"{len(comparison['left_only'])} | "
            f"{len(comparison['right_only'])} | "
            f"{ratios['median_cpu_ratio']} | "
            f"{ratios['median_generated_ratio']} | "
            f"{ratios['median_high_water_total_ratio']} | "
            f"{report['maximum_rss']['candidate_over_baseline']} | "
            f"`{summary['decisions'][candidate]['result']}` |"
        )
    lines.append("")
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        arguments.experiment_root.resolve(),
        (
            arguments.proof_validation.resolve()
            if arguments.proof_validation
            else None
        ),
    )
    json_output = arguments.json_output.resolve()
    markdown_output = arguments.markdown_output.resolve()
    json_output.parent.mkdir(parents=True, exist_ok=True)
    markdown_output.parent.mkdir(parents=True, exist_ok=True)
    json_output.write_bytes(BASE.canonical_json(summary) + b"\n")
    markdown_output.write_text(
        render_markdown(summary), encoding="utf-8"
    )
    decisions = ", ".join(
        f"{candidate}={decision['result']}"
        for candidate, decision in summary["decisions"].items()
    )
    print(
        f"OK: {summary['run_count']} runs; {decisions}; "
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
