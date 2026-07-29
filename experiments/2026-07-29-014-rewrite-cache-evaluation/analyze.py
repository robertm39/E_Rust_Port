#!/usr/bin/env python3
"""Verify and summarize the shared rewrite-cache evaluation."""

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
    / "2026-07-29-013-inference-gap-audit"
    / "analyze.py"
)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"CounterSatisfiable", "Satisfiable"}
PAIR_METRICS = {
    "cpu": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "term_storage_bytes": ("terms", "storage_estimate_bytes"),
    "rewrite_steps": ("simplification", "rewrite_steps"),
    "uncached_links": ("simplification", "rewrite_uncached_links"),
    "link_lookups": (
        "simplification",
        "rewrite_cache",
        "link_lookups",
    ),
    "link_hits": ("simplification", "rewrite_cache", "link_hits"),
    "link_edges": (
        "simplification",
        "rewrite_cache",
        "link_edges_followed",
    ),
    "nf_date_checks": (
        "simplification",
        "rewrite_cache",
        "normal_form_date_checks",
    ),
    "nf_date_hits": (
        "simplification",
        "rewrite_cache",
        "normal_form_date_hits",
    ),
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


PRIOR = load_module("rewrite_cache_prior_analyze", PRIOR_ANALYZE_PATH)
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
    problem_ids: set[str] | None = None,
) -> dict[str, Any]:
    indexed = indexed_results(results, budget)
    coordinates = sorted(
        {
            (problem_id, repetition)
            for problem_id, strategy, repetition in indexed
            if strategy == "cache"
            and (problem_id, "recompute", repetition) in indexed
            and (problem_ids is None or problem_id in problem_ids)
        }
    )
    report: dict[str, Any] = {"paired_coordinates": len(coordinates)}
    for name, path in PAIR_METRICS.items():
        ratios = []
        for problem_id, repetition in coordinates:
            left = BASE.metric(
                indexed[(problem_id, "cache", repetition)], *path
            )
            right = BASE.metric(
                indexed[(problem_id, "recompute", repetition)], *path
            )
            ratio = safe_ratio(left, right)
            if ratio is not None:
                ratios.append(ratio)
        report[f"median_{name}_ratio"] = median(ratios)
    return report


def cache_activity(
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

    def total(*path: str) -> int:
        return sum(
            int(value)
            for result in selected
            if (value := BASE.metric(result, *path)) is not None
        )

    rewrite_steps = total("simplification", "rewrite_steps")
    uncached = total("simplification", "rewrite_uncached_links")
    lookups = total(
        "simplification", "rewrite_cache", "link_lookups"
    )
    hits = total("simplification", "rewrite_cache", "link_hits")
    edges = total(
        "simplification", "rewrite_cache", "link_edges_followed"
    )
    date_checks = total(
        "simplification", "rewrite_cache", "normal_form_date_checks"
    )
    date_hits = total(
        "simplification", "rewrite_cache", "normal_form_date_hits"
    )
    cached_steps = max(0, rewrite_steps - uncached)
    return {
        "telemetry_records": len(selected),
        "rewrite_steps": rewrite_steps,
        "uncached_links": uncached,
        "link_lookups": lookups,
        "link_hits": hits,
        "link_edges_followed": edges,
        "normal_form_date_checks": date_checks,
        "normal_form_date_hits": date_hits,
        "link_hit_rate": (
            None if lookups == 0 else round(hits / lookups, 6)
        ),
        "mean_followed_path": (
            None if hits == 0 else round(edges / hits, 6)
        ),
        "normal_form_date_hit_rate": (
            None if date_checks == 0 else round(date_hits / date_checks, 6)
        ),
        "cached_rewrite_fraction": (
            None
            if rewrite_steps == 0
            else round(cached_steps / rewrite_steps, 6)
        ),
        "saved_traversal_proxy": edges + date_hits,
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
    exact_status_pairs = 0
    paired_coordinates = 0
    terminal_pairs = 0
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
        for coordinate, cached in sorted(indexed.items()):
            if coordinate[3] != "cache":
                continue
            problem_id, budget, repetition, _strategy = coordinate
            recompute = indexed[
                (problem_id, budget, repetition, "recompute")
            ]
            paired_coordinates += 1
            exact_status_pairs += int(
                cached["szs_status"] == recompute["szs_status"]
            )
            cached_polarity = status_polarity(cached["szs_status"])
            recompute_polarity = status_polarity(
                recompute["szs_status"]
            )
            if (
                cached_polarity is not None
                and recompute_polarity is not None
            ):
                terminal_pairs += 1
                if cached_polarity != recompute_polarity:
                    disagreements.append(
                        {
                            "phase": phase,
                            "problem_id": problem_id,
                            "budget": budget,
                            "repetition": repetition,
                            "cache_status": cached["szs_status"],
                            "recompute_status": recompute["szs_status"],
                        }
                    )
    return {
        "paired_coordinates": paired_coordinates,
        "exact_status_pairs": exact_status_pairs,
        "terminal_pairs": terminal_pairs,
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


def maximum_rss_ratio(
    results: Sequence[dict[str, Any]], budget: str
) -> dict[str, Any]:
    maxima = {}
    for strategy in ("cache", "recompute"):
        values = [
            value
            for result in results
            if result["strategy"] == strategy
            and result["budget"] == budget
            and (
                value := BASE.metric(
                    result, "resources", "maximum_resident_pages"
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
        "cache_maximum_resident_pages": maxima["cache"],
        "recompute_maximum_resident_pages": maxima["recompute"],
        "cache_over_recompute": round(
            maxima["cache"] / maxima["recompute"], 6
        ),
    }


def aggregate_strategy(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, Any]:
    return {
        **BASE.aggregate_strategy(contract, results, strategy, budget),
        "cache_activity": cache_activity(results, strategy, budget),
    }


def analyze_phase(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    if set(contract["strategies"]) != {"cache", "recompute"}:
        raise AnalysisError("unexpected rewrite-cache strategies")
    budgets = {}
    for budget in contract["budgets"]:
        comparison = BASE.comparison(
            contract, results, "cache", "recompute", budget
        )
        common = set(comparison["common_ids"])
        budgets[budget] = {
            "strategies": {
                strategy: aggregate_strategy(
                    contract, results, strategy, budget
                )
                for strategy in ("cache", "recompute")
            },
            "coverage_comparison": comparison,
            "all_run_ratios": paired_ratios(results, budget),
            "common_solved_ratios": paired_ratios(
                results, budget, common
            ),
            "maximum_rss": maximum_rss_ratio(results, budget),
        }
    return {
        "contract_id": contract["contract_id"],
        "cache_binary_sha256": contract["binary_sha256"],
        "recompute_binary_sha256": contract["strategies"][
            "recompute"
        ]["binary_sha256"],
        "run_count": len(results),
        "budgets": budgets,
    }


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
    expected_binaries = {
        "cache": contracts["casc"]["binary_sha256"],
        "recompute": contracts["casc"]["strategies"]["recompute"][
            "binary_sha256"
        ],
    }
    if report.get("binaries") != expected_binaries:
        raise AnalysisError("proof validation names other binaries")
    if report.get("verified_cases") != report.get("expected_cases"):
        raise AnalysisError("proof validation is incomplete")
    if not report.get("all_verified"):
        raise AnalysisError("proof validation rejected a claim")
    return report


def low_cache_hit_rate(phases: dict[str, Any]) -> bool:
    hits = 0
    lookups = 0
    for phase in phases.values():
        for budget in phase["budgets"].values():
            activity = budget["strategies"]["cache"]["cache_activity"]
            hits += activity["link_hits"]
            lookups += activity["link_lookups"]
    return lookups != 0 and hits / lookups < 0.10


def analyze(
    experiment_root: Path,
    proof_validation_path: Path | None,
) -> dict[str, Any]:
    contracts = {}
    phase_results = {}
    phases = {}
    for phase in ("casc", "targeted"):
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
    casc_larger = phases["casc"]["budgets"]["larger"]
    targeted = phases["targeted"]["budgets"]["targeted"]
    comparison = casc_larger["coverage_comparison"]
    common = casc_larger["common_solved_ratios"]
    rss_ratio = casc_larger["maximum_rss"]["cache_over_recompute"]
    correctness = (
        proof_validation is not None
        and not polarity["polarity_disagreements"]
        and not unexpected_missing
    )
    unique = (
        bool(comparison["left_only"])
        and not comparison["right_only"]
    )
    efficient = (
        not comparison["right_only"]
        and common["median_cpu_ratio"] is not None
        and common["median_cpu_ratio"] <= 0.95
        and common["median_generated_ratio"] is not None
        and common["median_generated_ratio"] <= 1.02
        and common["median_high_water_total_ratio"] is not None
        and common["median_high_water_total_ratio"] <= 1.02
        and rss_ratio <= 1.05
    )
    retention_gate = correctness and (unique or efficient)
    targeted_cpu = targeted["common_solved_ratios"][
        "median_cpu_ratio"
    ]
    selective_trigger = (
        low_cache_hit_rate(phases)
        or rss_ratio > 1.05
        or (
            common["median_term_storage_bytes_ratio"] is not None
            and common["median_term_storage_bytes_ratio"] > 1.05
        )
        or (
            common["median_cpu_ratio"] is not None
            and common["median_cpu_ratio"] > 1.02
        )
        or (targeted_cpu is not None and targeted_cpu > 1.02)
    )
    if retention_gate:
        result = "retain_full_shared_rewrite_cache"
    elif correctness and selective_trigger:
        result = "open_selective_cache_followup"
    elif correctness:
        result = "retain_cache_compatibility_neutral"
    else:
        result = "inconclusive_retain_existing_cache"

    summary_body = {
        "schema_version": 1,
        "phases": phases,
        "run_count": sum(
            phase["run_count"] for phase in phases.values()
        ),
        "missing_telemetry": missing,
        "polarity_audit": polarity,
        "proof_validation": proof_validation,
        "decision": {
            "result": result,
            "production_cache_remains_enabled": True,
            "correctness_gate_passed": correctness,
            "retention_gate_passed": retention_gate,
            "unique_solve_gate_passed": unique,
            "efficiency_gate_passed": efficient,
            "selective_followup_triggered": (
                correctness and not retention_gate and selective_trigger
            ),
            "criteria": {
                "cache_only_larger_solves": 1,
                "median_common_solved_cpu_ratio": 0.95,
                "median_generated_ratio": 1.02,
                "median_high_water_ratio": 1.02,
                "maximum_rss_ratio": 1.05,
                "selective_link_hit_rate": 0.10,
                "selective_cpu_regression_ratio": 1.02,
            },
        },
    }
    return {
        **summary_body,
        "report_id": hashlib.sha256(
            BASE.canonical_json(summary_body)
        ).hexdigest(),
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Shared rewrite-cache evaluation results",
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
        "| Phase | Budget | Cache solves | Recompute solves | "
        "Cache-only | Recompute-only | Common CPU | Generated | "
        "High-water | Term storage | Max RSS | Link hit rate | "
        "NF-date hit rate | Saved traversal proxy |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | "
        "---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for phase_name, phase in summary["phases"].items():
        for budget_name, budget in phase["budgets"].items():
            comparison = budget["coverage_comparison"]
            ratios = budget["common_solved_ratios"]
            activity = budget["strategies"]["cache"]["cache_activity"]
            lines.append(
                f"| {phase_name} | {budget_name} | "
                f"{comparison['left_solved']} | "
                f"{comparison['right_solved']} | "
                f"{len(comparison['left_only'])} | "
                f"{len(comparison['right_only'])} | "
                f"{ratios['median_cpu_ratio']} | "
                f"{ratios['median_generated_ratio']} | "
                f"{ratios['median_high_water_total_ratio']} | "
                f"{ratios['median_term_storage_bytes_ratio']} | "
                f"{budget['maximum_rss']['cache_over_recompute']} | "
                f"{activity['link_hit_rate']} | "
                f"{activity['normal_form_date_hit_rate']} | "
                f"{activity['saved_traversal_proxy']} |"
            )
    lines.extend(
        [
            "",
            f"Decision: `{summary['decision']['result']}`.",
            "",
        ]
    )
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
    print(
        f"OK: {summary['run_count']} runs; "
        f"decision {summary['decision']['result']}; "
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

