#!/usr/bin/env python3
"""Verify and summarize the frozen inference-gap search experiment."""

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
    / "2026-07-28-008-stronger-redundancy"
    / "analyze.py"
)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"CounterSatisfiable", "Satisfiable"}
PAIR_METRICS = {
    "cpu": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "final_total": ("search_funnel", "final_total"),
    "term_storage_bytes": ("terms", "storage_estimate_bytes"),
    "rewrite_steps": ("simplification", "rewrite_steps"),
    "maximum_resident_pages": ("resources", "maximum_resident_pages"),
}
BEHAVIOR_PATHS = (
    ("search_funnel", "generated"),
    ("search_funnel", "processed"),
    ("search_funnel", "high_water_total"),
    ("search_funnel", "final_total"),
    ("simplification", "rewrite_steps"),
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PRIOR = load_module("inference_gap_prior_analyze", PRIOR_ANALYZE_PATH)
BASE = PRIOR.BASE
AnalysisError = BASE.AnalysisError


def median(values: Iterable[float]) -> float | None:
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
    solved = BASE.reproducible_coverage(
        results, strategy, budget, contract["repetitions"]
    )
    categories: dict[str, set[str]] = {}
    for result in results:
        if result["problem_id"] in solved:
            categories.setdefault(result["category"], set()).add(
                result["problem_id"]
            )
    return {
        category: len(problem_ids)
        for category, problem_ids in sorted(categories.items())
    }


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
    }


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
    left: str,
    right: str,
    budget: str,
    problem_ids: set[str] | None = None,
) -> dict[str, Any]:
    indexed = indexed_results(results, budget)
    coordinates = sorted(
        {
            (problem_id, repetition)
            for problem_id, strategy, repetition in indexed
            if strategy == left
            and (problem_id, right, repetition) in indexed
            and (problem_ids is None or problem_id in problem_ids)
        }
    )
    report: dict[str, Any] = {"paired_coordinates": len(coordinates)}
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
        report[f"median_{name}_ratio"] = median(ratios)
    return report


def maximum_rss_ratio(
    results: Sequence[dict[str, Any]],
    left: str,
    right: str,
    budget: str,
) -> dict[str, Any]:
    maxima: dict[str, float] = {}
    for strategy in (left, right):
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
            raise AnalysisError(f"{strategy}/{budget} has no RSS telemetry")
        maxima[strategy] = float(max(values))
    return {
        "left_maximum_resident_pages": int(maxima[left]),
        "right_maximum_resident_pages": int(maxima[right]),
        "maximum_rss_ratio": round(maxima[left] / maxima[right], 6),
    }


def behavior_effects(
    results: Sequence[dict[str, Any]], budget: str
) -> list[dict[str, Any]]:
    indexed = indexed_results(results, budget)
    effects = []
    for problem_id, strategy, repetition in sorted(indexed):
        if strategy != "local_rw":
            continue
        candidate = indexed[(problem_id, strategy, repetition)]
        baseline = indexed[(problem_id, "baseline", repetition)]
        changed = [
            ".".join(path)
            for path in BEHAVIOR_PATHS
            if BASE.metric(candidate, *path) != BASE.metric(baseline, *path)
        ]
        if changed:
            effects.append(
                {
                    "problem_id": problem_id,
                    "repetition": repetition,
                    "changed_metrics": changed,
                }
            )
    return effects


def polarity_audit(
    results: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    by_coordinate = {
        (
            result["problem_id"],
            result["budget"],
            result["repetition"],
            result["strategy"],
        ): result
        for result in results
    }
    exact_status_pairs = 0
    terminal_pairs = 0
    disagreements = []
    for coordinate, candidate in sorted(by_coordinate.items()):
        if coordinate[3] != "local_rw":
            continue
        problem_id, budget, repetition, _ = coordinate
        baseline = by_coordinate[
            (problem_id, budget, repetition, "baseline")
        ]
        exact_status_pairs += int(
            candidate["szs_status"] == baseline["szs_status"]
        )
        candidate_polarity = status_polarity(candidate["szs_status"])
        baseline_polarity = status_polarity(baseline["szs_status"])
        if candidate_polarity is not None and baseline_polarity is not None:
            terminal_pairs += 1
            if candidate_polarity != baseline_polarity:
                disagreements.append(
                    {
                        "problem_id": problem_id,
                        "budget": budget,
                        "repetition": repetition,
                        "candidate_status": candidate["szs_status"],
                        "baseline_status": baseline["szs_status"],
                    }
                )
    return {
        "paired_coordinates": len(results) // 2,
        "exact_status_pairs": exact_status_pairs,
        "terminal_pairs": terminal_pairs,
        "polarity_disagreements": disagreements,
    }


def local_rw_proof_records(
    results: Sequence[dict[str, Any]],
) -> list[dict[str, Any]]:
    records = []
    for result in results:
        if result["strategy"] != "local_rw":
            continue
        stdout = Path(result["_path"]).parent / "stdout.txt"
        text = stdout.read_text(encoding="utf-8", errors="replace")
        if "inference(local_rw" in text or "'local_rw" in text:
            records.append(
                {
                    "problem_id": result["problem_id"],
                    "budget": result["budget"],
                    "repetition": result["repetition"],
                    "szs_status": result["szs_status"],
                    "stdout_sha256": result["stdout_sha256"],
                }
            )
    return records


def load_proof_validation(
    path: Path | None, contract: dict[str, Any]
) -> dict[str, Any] | None:
    if path is None:
        return None
    report = json.loads(path.read_text(encoding="utf-8"))
    if report.get("test_contract_id") != contract["contract_id"]:
        raise AnalysisError("proof validation names another search contract")
    if report.get("test_binary_sha256") != contract["binary_sha256"]:
        raise AnalysisError("proof validation names another binary")
    if report.get("verified_cases") != report.get("expected_cases"):
        raise AnalysisError("proof validation is incomplete")
    if not report.get("all_verified"):
        raise AnalysisError("proof validation did not verify every claim")
    return report


def analyze(
    experiment_root: Path,
    matrix_audit_path: Path,
    proof_validation_path: Path | None,
) -> dict[str, Any]:
    contract, results = BASE.load_phase(experiment_root, "audit")
    if set(contract["strategies"]) != {"baseline", "local_rw"}:
        raise AnalysisError("unexpected audit strategies")
    matrix_audit = json.loads(matrix_audit_path.read_text(encoding="utf-8"))
    if not matrix_audit.get("focused_tests_run"):
        raise AnalysisError("matrix audit did not run focused tests")
    if len(matrix_audit.get("focused_test_results", [])) != 13:
        raise AnalysisError("matrix audit lacks 13 focused test results")
    proof_validation = load_proof_validation(
        proof_validation_path, contract
    )

    budgets: dict[str, Any] = {}
    all_effects = []
    for budget in contract["budgets"]:
        comparison = BASE.comparison(
            contract,
            results,
            "local_rw",
            "baseline",
            budget,
        )
        common = set(comparison["common_ids"])
        effects = behavior_effects(results, budget)
        all_effects.extend(
            {"budget": budget, **effect} for effect in effects
        )
        budgets[budget] = {
            "strategies": {
                strategy: aggregate_strategy(
                    contract, results, strategy, budget
                )
                for strategy in contract["strategies"]
            },
            "coverage_comparison": comparison,
            "all_run_ratios": paired_ratios(
                results, "local_rw", "baseline", budget
            ),
            "common_solved_ratios": paired_ratios(
                results,
                "local_rw",
                "baseline",
                budget,
                common,
            ),
            "maximum_rss": maximum_rss_ratio(
                results, "local_rw", "baseline", budget
            ),
            "behavior_effect_coordinates": len(effects),
        }

    larger = budgets["larger"]
    comparison = larger["coverage_comparison"]
    common_ratios = larger["common_solved_ratios"]
    rss = larger["maximum_rss"]
    validity = (
        proof_validation is not None
        and not polarity_audit(results)["polarity_disagreements"]
        and bool(all_effects)
    )
    unique = (
        len(comparison["left_only"]) >= 2
        and not comparison["right_only"]
    )
    efficient = (
        len(comparison["common_ids"]) >= 4
        and not comparison["right_only"]
        and common_ratios["median_generated_ratio"] is not None
        and common_ratios["median_generated_ratio"] <= 0.90
        and common_ratios["median_cpu_ratio"] is not None
        and common_ratios["median_cpu_ratio"] <= 1.02
        and common_ratios["median_high_water_total_ratio"] is not None
        and common_ratios["median_high_water_total_ratio"] <= 1.02
    )
    rss_passed = rss["maximum_rss_ratio"] <= 1.05
    advances = validity and rss_passed and (unique or efficient)
    proof_records = local_rw_proof_records(results)
    summary_body = {
        "schema_version": 1,
        "contract_id": contract["contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "matrix_audit": {
            "report_id": matrix_audit["report_id"],
            "matrix_sha256": matrix_audit["matrix_sha256"],
            "row_count": matrix_audit["row_count"],
            "status_counts": matrix_audit["status_counts"],
            "focused_test_count": matrix_audit["focused_test_count"],
        },
        "run_count": len(results),
        "polarity_audit": polarity_audit(results),
        "budgets": budgets,
        "behavior_effect_coordinates": len(all_effects),
        "local_rw_proof_records": proof_records,
        "proof_validation": proof_validation,
        "decision": {
            "result": (
                "advance_selective_local_rewriting"
                if advances
                else "retain_local_rewriting_as_default_off"
            ),
            "advances": advances,
            "validity_gate_passed": validity,
            "maximum_rss_gate_passed": rss_passed,
            "unique_solve_gate_passed": unique,
            "efficiency_gate_passed": efficient,
            "criteria": {
                "candidate_only_solves_required": 2,
                "common_solved_required": 4,
                "median_generated_ratio": 0.90,
                "median_cpu_ratio": 1.02,
                "median_high_water_ratio": 1.02,
                "maximum_rss_ratio": 1.05,
                "observable_behavior_effect_required": True,
                "all_proof_claims_verified_required": True,
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
        "# Inference-gap audit results",
        "",
        f"- Search contract: `{summary['contract_id']}`",
        f"- Binary SHA-256: `{summary['binary_sha256']}`",
        f"- Matrix report: `{summary['matrix_audit']['report_id']}`",
        f"- Search runs: {summary['run_count']}",
        (
            "- Independent proof claims: "
            f"{summary['proof_validation']['verified_cases']}/"
            f"{summary['proof_validation']['expected_cases']} verified"
            if summary["proof_validation"] is not None
            else "- Independent proof claims: pending"
        ),
        "",
        "| Budget | Baseline solves | Local-rw solves | Candidate-only | "
        "Baseline-only | Common CPU | Common generated | Common high-water | "
        "All-run generated | Max RSS | Effect coordinates |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | "
        "---: | ---: |",
    ]
    for budget, report in summary["budgets"].items():
        comparison = report["coverage_comparison"]
        common = report["common_solved_ratios"]
        all_runs = report["all_run_ratios"]
        rss = report["maximum_rss"]
        lines.append(
            f"| {budget} | {comparison['right_solved']} | "
            f"{comparison['left_solved']} | {len(comparison['left_only'])} | "
            f"{len(comparison['right_only'])} | "
            f"{common['median_cpu_ratio']} | "
            f"{common['median_generated_ratio']} | "
            f"{common['median_high_water_total_ratio']} | "
            f"{all_runs['median_generated_ratio']} | "
            f"{rss['maximum_rss_ratio']} | "
            f"{report['behavior_effect_coordinates']} |"
        )
    decision = summary["decision"]
    lines.extend(
        [
            "",
            f"Decision: `{decision['result']}`.",
            "",
            (
                "No checked proof emitted a `local_rw` step."
                if not summary["local_rw_proof_records"]
                else (
                    f"{len(summary['local_rw_proof_records'])} proof records "
                    "emitted a `local_rw` step."
                )
            ),
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--matrix-audit", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        arguments.experiment_root.resolve(),
        arguments.matrix_audit.resolve(),
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

