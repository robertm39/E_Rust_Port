#!/usr/bin/env python3
"""Analyze the frozen periodic ground-SAT trigger evaluation."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


STRATEGIES = ("off", "step5000", "step10000", "size10000")
CANDIDATES = STRATEGIES[1:]
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"Satisfiable", "CounterSatisfiable"}


class AnalysisError(RuntimeError):
    """Raised when raw evidence violates the frozen experiment contract."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def median(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def ratio(numerator: float, denominator: float) -> float | None:
    return numerator / denominator if denominator else None


def relative_range(values: list[float]) -> float | None:
    if not values:
        return None
    center = statistics.median(values)
    return (max(values) - min(values)) / center if center else 0.0


def polarity(status: str | None) -> str:
    if status in PROOF_STATUSES:
        return "proof"
    if status in MODEL_STATUSES:
        return "model"
    return "other"


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AnalysisError(f"expected JSON object: {path}")
    return value


def load_results(run_root: Path) -> list[dict[str, Any]]:
    result_paths = sorted((run_root / "heldout").rglob("result.json"))
    if len(result_paths) != 192:
        raise AnalysisError(
            f"found {len(result_paths)} results, expected 192"
        )
    results: list[dict[str, Any]] = []
    for path in result_paths:
        result = load_json(path)
        telemetry_path = path.parent / "telemetry.json"
        stdout_path = path.parent / "stdout.txt"
        stderr_path = path.parent / "stderr.txt"
        for artifact, field in (
            (stdout_path, "stdout_sha256"),
            (stderr_path, "stderr_sha256"),
        ):
            if not artifact.is_file():
                raise AnalysisError(f"missing raw artifact: {artifact}")
            if sha256_file(artifact) != result[field]:
                raise AnalysisError(f"hash mismatch: {artifact}")
        telemetry = None
        if result["telemetry_present"]:
            if not telemetry_path.is_file():
                raise AnalysisError(f"missing telemetry artifact: {path}")
            if sha256_file(telemetry_path) != result["telemetry_sha256"]:
                raise AnalysisError(f"telemetry hash mismatch: {path}")
            telemetry = load_json(telemetry_path)
            if telemetry.get("schema") != "umlaut.search-telemetry":
                raise AnalysisError(f"unexpected telemetry schema: {path}")
        elif (
            result["szs_status"] != "ResourceOut"
            or result["telemetry_sha256"] is not None
            or result["telemetry_error"] is not None
        ):
            raise AnalysisError(
                f"telemetry absent without a clean hard ResourceOut: {path}"
            )
        result["_path"] = str(path)
        result["_telemetry"] = telemetry
        results.append(result)

    strategies = Counter(result["strategy"] for result in results)
    if strategies != Counter({strategy: 48 for strategy in STRATEGIES}):
        raise AnalysisError(f"unexpected strategy counts: {strategies}")
    if len({result["contract_id"] for result in results}) != 1:
        raise AnalysisError("results contain multiple contract ids")
    if len({result["binary_sha256"] for result in results}) != 1:
        raise AnalysisError("results contain multiple binary hashes")
    return results


def key(result: dict[str, Any]) -> tuple[str, int]:
    return str(result["problem_id"]), int(result["repetition"])


def by_strategy_and_key(
    results: list[dict[str, Any]],
) -> dict[str, dict[tuple[str, int], dict[str, Any]]]:
    indexed: dict[str, dict[tuple[str, int], dict[str, Any]]] = {
        strategy: {} for strategy in STRATEGIES
    }
    for result in results:
        coordinate = key(result)
        strategy = str(result["strategy"])
        if coordinate in indexed[strategy]:
            raise AnalysisError(
                f"duplicate {strategy} coordinate: {coordinate}"
            )
        indexed[strategy][coordinate] = result
    expected = set(indexed["off"])
    if len(expected) != 48:
        raise AnalysisError("baseline does not contain 48 coordinates")
    for strategy in STRATEGIES:
        if set(indexed[strategy]) != expected:
            raise AnalysisError(f"unpaired coordinates for {strategy}")
    return indexed


def reproducible_solves(
    strategy_results: dict[tuple[str, int], dict[str, Any]]
) -> set[str]:
    statuses: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for (problem_id, _repetition), result in strategy_results.items():
        statuses[problem_id].append(result)
    return {
        problem_id
        for problem_id, problem_results in statuses.items()
        if len(problem_results) == 2
        and all(
            result["expected_status_match"]
            and result["szs_status"] in PROOF_STATUSES
            for result in problem_results
        )
    }


def metric(result: dict[str, Any], name: str) -> float:
    telemetry = result["_telemetry"]
    if telemetry is None:
        raise AnalysisError(
            f"metric {name} is unavailable for {result['_path']}"
        )
    if name == "cpu":
        return float(telemetry["resources"]["total_cpu_seconds"])
    if name == "generated":
        return float(telemetry["search_funnel"]["generated"])
    if name == "processed":
        return float(telemetry["search_funnel"]["processed"])
    if name == "high_water":
        return float(telemetry["search_funnel"]["high_water_total"])
    if name == "term_storage":
        return float(telemetry["terms"]["storage_estimate_bytes"])
    if name == "rss":
        return float(telemetry["resources"]["maximum_resident_pages"])
    raise AnalysisError(f"unknown metric: {name}")


def paired_metric_summary(
    baseline: dict[tuple[str, int], dict[str, Any]],
    candidate: dict[tuple[str, int], dict[str, Any]],
    problems: set[str],
    name: str,
) -> dict[str, Any]:
    ratios: list[float] = []
    per_problem: dict[str, dict[str, Any]] = {}
    for problem_id in sorted(problems):
        baseline_values: list[float] = []
        candidate_values: list[float] = []
        paired_ratios: list[float] = []
        for repetition in (1, 2):
            coordinate = problem_id, repetition
            baseline_value = metric(baseline[coordinate], name)
            candidate_value = metric(candidate[coordinate], name)
            observed_ratio = ratio(candidate_value, baseline_value)
            baseline_values.append(baseline_value)
            candidate_values.append(candidate_value)
            if observed_ratio is not None:
                paired_ratios.append(observed_ratio)
                ratios.append(observed_ratio)
        per_problem[problem_id] = {
            "baseline": baseline_values,
            "candidate": candidate_values,
            "ratios": paired_ratios,
            "baseline_relative_range": relative_range(baseline_values),
            "candidate_relative_range": relative_range(candidate_values),
            "paired_ratio_relative_range": relative_range(paired_ratios),
        }
    baseline_noise = [
        value["baseline_relative_range"]
        for value in per_problem.values()
        if value["baseline_relative_range"] is not None
    ]
    candidate_noise = [
        value["candidate_relative_range"]
        for value in per_problem.values()
        if value["candidate_relative_range"] is not None
    ]
    paired_noise = [
        value["paired_ratio_relative_range"]
        for value in per_problem.values()
        if value["paired_ratio_relative_range"] is not None
    ]
    return {
        "paired_coordinates": len(ratios),
        "median_candidate_over_baseline": median(ratios),
        "maximum_candidate_over_baseline": max(ratios) if ratios else None,
        "noise": {
            "method": "within-problem two-repetition relative range",
            "problem_count": len(per_problem),
            "baseline_median_relative_range": median(baseline_noise),
            "baseline_max_relative_range": (
                max(baseline_noise) if baseline_noise else None
            ),
            "candidate_median_relative_range": median(candidate_noise),
            "candidate_max_relative_range": (
                max(candidate_noise) if candidate_noise else None
            ),
            "paired_ratio_median_relative_range": median(paired_noise),
            "paired_ratio_max_relative_range": (
                max(paired_noise) if paired_noise else None
            ),
        },
        "per_problem": per_problem,
    }


def sat_summary(results: Iterable[dict[str, Any]]) -> dict[str, Any]:
    materialized = list(results)
    telemetry_results = [
        result for result in materialized if result["_telemetry"] is not None
    ]
    missing = [
        {
            "problem_id": result["problem_id"],
            "repetition": result["repetition"],
            "status": result["szs_status"],
        }
        for result in materialized
        if result["_telemetry"] is None
    ]
    reached = [
        result
        for result in telemetry_results
        if int(result["_telemetry"]["sat"]["checks"]) > 0
    ]
    sat_rows = [
        result["_telemetry"]["sat"] for result in telemetry_results
    ]
    checks = sum(int(row["checks"]) for row in sat_rows)
    satisfiable = sum(int(row["satisfiable"]) for row in sat_rows)
    unsatisfiable = sum(int(row["unsatisfiable"]) for row in sat_rows)
    preprocessing = sum(
        float(row["preprocessing_cpu_seconds"]) for row in sat_rows
    )
    encoding = sum(float(row["encoding_cpu_seconds"]) for row in sat_rows)
    solver = sum(float(row["solver_cpu_seconds"]) for row in sat_rows)
    combined = preprocessing + encoding + solver
    reached_cpu = sum(metric(result, "cpu") for result in reached)
    core_sizes = [
        int(result["_telemetry"]["sat"]["unsat_core_clauses"])
        for result in telemetry_results
        if int(result["_telemetry"]["sat"]["unsatisfiable"]) > 0
    ]
    return {
        "coordinates": len(materialized),
        "telemetry_coordinates": len(telemetry_results),
        "missing_telemetry_resourceouts": missing,
        "reached_coordinates": len(reached),
        "reached_problems": sorted(
            {str(result["problem_id"]) for result in reached}
        ),
        "checks": checks,
        "satisfiable": satisfiable,
        "unsatisfiable": unsatisfiable,
        "limited_or_unknown": checks - satisfiable - unsatisfiable,
        "calls_per_reached_coordinate": ratio(checks, len(reached)),
        "input_clauses": sum(int(row["input_clauses"]) for row in sat_rows),
        "post_purity_clauses": sum(
            int(row["post_purity_clauses"]) for row in sat_rows
        ),
        "unsat_core_clauses": sum(
            int(row["unsat_core_clauses"]) for row in sat_rows
        ),
        "terminal_core_sizes": core_sizes,
        "terminal_core_size_median": median(core_sizes),
        "preprocessing_cpu_seconds": preprocessing,
        "encoding_cpu_seconds": encoding,
        "solver_cpu_seconds": solver,
        "combined_cpu_seconds": combined,
        "combined_cpu_seconds_per_call": ratio(combined, checks),
        "combined_fraction_of_reached_cpu": ratio(combined, reached_cpu),
    }


def status_audit(
    indexed: dict[str, dict[tuple[str, int], dict[str, Any]]]
) -> dict[str, Any]:
    comparisons: dict[str, Any] = {}
    for candidate in CANDIDATES:
        exact_mismatches = []
        polarity_disagreements = []
        for coordinate, baseline in indexed["off"].items():
            other = indexed[candidate][coordinate]
            if baseline["szs_status"] != other["szs_status"]:
                exact_mismatches.append(
                    {
                        "problem_id": coordinate[0],
                        "repetition": coordinate[1],
                        "baseline": baseline["szs_status"],
                        "candidate": other["szs_status"],
                    }
                )
            if polarity(baseline["szs_status"]) != polarity(
                other["szs_status"]
            ):
                polarity_disagreements.append(exact_mismatches[-1])
        comparisons[candidate] = {
            "paired_coordinates": 48,
            "exact_matches": 48 - len(exact_mismatches),
            "exact_mismatches": exact_mismatches,
            "polarity_disagreements": polarity_disagreements,
        }
    return comparisons


def candidate_comparison(
    name: str,
    indexed: dict[str, dict[tuple[str, int], dict[str, Any]]],
    solves: dict[str, set[str]],
    status: dict[str, Any],
) -> dict[str, Any]:
    common = solves["off"] & solves[name]
    metrics = {
        metric_name: paired_metric_summary(
            indexed["off"], indexed[name], common, metric_name
        )
        for metric_name in (
            "cpu",
            "generated",
            "processed",
            "high_water",
            "term_storage",
            "rss",
        )
    }
    sat = sat_summary(indexed[name].values())
    candidate_only = sorted(solves[name] - solves["off"])
    baseline_only = sorted(solves["off"] - solves[name])
    correctness = (
        not status[name]["exact_mismatches"]
        and not status[name]["polarity_disagreements"]
    )
    enough_reach = (
        sat["reached_coordinates"] >= 8
        and len(sat["reached_problems"]) >= 4
    )
    cpu_ratio = metrics["cpu"]["median_candidate_over_baseline"]
    generated_ratio = metrics["generated"]["median_candidate_over_baseline"]
    high_water_ratio = metrics["high_water"][
        "median_candidate_over_baseline"
    ]
    rss_ratio = metrics["rss"]["median_candidate_over_baseline"]
    cost_fraction = sat["combined_fraction_of_reached_cpu"]
    benefit = bool(candidate_only) or (
        cpu_ratio is not None
        and cpu_ratio <= 0.95
        and generated_ratio is not None
        and generated_ratio <= 1.02
        and high_water_ratio is not None
        and high_water_ratio <= 1.02
        and rss_ratio is not None
        and rss_ratio <= 1.05
        and cost_fraction is not None
        and cost_fraction <= 0.03
    )
    material_regression = bool(baseline_only) or any(
        value is not None and value > 1.05
        for value in (
            cpu_ratio,
            generated_ratio,
            high_water_ratio,
            rss_ratio,
        )
    )
    if not correctness:
        decision = "inconclusive"
    elif material_regression:
        decision = "reject"
    elif enough_reach and benefit:
        decision = "promote_to_schedule_followup"
    else:
        decision = "keep_default_off"
    return {
        "candidate": name,
        "reproducible_solves": sorted(solves[name]),
        "common_reproducible_solves": sorted(common),
        "candidate_only": candidate_only,
        "baseline_only": baseline_only,
        "status_audit": status[name],
        "sat": sat,
        "common_solve_metrics": metrics,
        "gates": {
            "correctness": correctness,
            "enough_reach": enough_reach,
            "benefit": benefit,
            "material_regression": material_regression,
        },
        "decision": decision,
    }


def markdown_report(summary: dict[str, Any]) -> str:
    lines = [
        "# Ground-SAT trigger results",
        "",
        "| Strategy | Reached coords/problems | Calls | SAT / UNSAT / limited | SAT CPU/call | SAT CPU share | Common solves | CPU ratio | Generated | High-water | RSS | Decision |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    for strategy in STRATEGIES:
        sat = summary["strategies"][strategy]["sat"]
        comparison = summary["comparisons"].get(strategy)
        if comparison is None:
            common = len(summary["strategies"][strategy]["reproducible_solves"])
            metrics = {}
            decision = "baseline"
        else:
            common = len(comparison["common_reproducible_solves"])
            metrics = comparison["common_solve_metrics"]
            decision = comparison["decision"]

        def value(metric_name: str) -> str:
            observed = (
                metrics.get(metric_name, {})
                .get("median_candidate_over_baseline")
            )
            return "n/a" if observed is None else f"{observed:.6f}"

        per_call = sat["combined_cpu_seconds_per_call"]
        share = sat["combined_fraction_of_reached_cpu"]
        lines.append(
            f"| {strategy} | {sat['reached_coordinates']}/"
            f"{len(sat['reached_problems'])} | {sat['checks']} | "
            f"{sat['satisfiable']} / {sat['unsatisfiable']} / "
            f"{sat['limited_or_unknown']} | "
            f"{'n/a' if per_call is None else f'{per_call:.6f}'} | "
            f"{'n/a' if share is None else f'{share:.4%}'} | "
            f"{common} | {value('cpu')} | {value('generated')} | "
            f"{value('high_water')} | {value('rss')} | {decision} |"
        )
    lines.extend(
        [
            "",
            "Ratios are candidate/baseline medians on paired coordinates for",
            "common reproducible solves. Full detail is in",
            "`results-summary.json`.",
            "",
        ]
    )
    return "\n".join(lines)


def analyze(
    run_root: Path, reuse_path: Path, proof_validation_path: Path
) -> dict[str, Any]:
    results = load_results(run_root)
    indexed = by_strategy_and_key(results)
    solves = {
        strategy: reproducible_solves(indexed[strategy])
        for strategy in STRATEGIES
    }
    statuses = status_audit(indexed)
    strategy_summaries = {
        strategy: {
            "reproducible_solves": sorted(solves[strategy]),
            "status_counts": dict(
                sorted(
                    Counter(
                        str(result["szs_status"])
                        for result in indexed[strategy].values()
                    ).items()
                )
            ),
            "sat": sat_summary(indexed[strategy].values()),
        }
        for strategy in STRATEGIES
    }
    comparisons = {
        candidate: candidate_comparison(
            candidate, indexed, solves, statuses
        )
        for candidate in CANDIDATES
    }
    reuse = load_json(reuse_path)
    proof_validation = load_json(proof_validation_path)
    if (
        proof_validation.get("kind")
        != "umlaut-ground-sat-proof-validation"
        or proof_validation.get("cadical", {}).get("result")
        != "unsatisfiable"
        or proof_validation.get("proofcheck", {}).get("gate_verdict")
        != "verified"
    ):
        raise AnalysisError("proof-only validation did not pass")
    overall = (
        "promote_to_schedule_followup"
        if any(
            comparison["decision"] == "promote_to_schedule_followup"
            for comparison in comparisons.values()
        )
        else "leave_default_off"
    )
    return {
        "schema_version": 1,
        "kind": "umlaut-ground-sat-trigger-analysis",
        "source": {
            "run_root": str(run_root),
            "contract_id": results[0]["contract_id"],
            "binary_sha256": results[0]["binary_sha256"],
            "result_count": len(results),
            "reuse_analysis_sha256": sha256_file(reuse_path),
            "proof_validation_sha256": sha256_file(
                proof_validation_path
            ),
        },
        "strategies": strategy_summaries,
        "comparisons": comparisons,
        "incremental_reuse": {
            key: reuse[key]
            for key in (
                "source_archive_sha256",
                "sessions",
                "capture_groups",
                "consecutive_pairs",
                "monotonic_add_only_pairs",
                "identical_pairs",
                "retained_from_previous",
                "reusable_in_current",
                "interpretation",
            )
        },
        "proof_reconstruction": {
            "heldout_satcheck_refutation_coordinates": sum(
                summary["sat"]["unsatisfiable"]
                for summary in strategy_summaries.values()
            ),
            "terminal_core_sizes": {
                strategy: strategy_summaries[strategy]["sat"][
                    "terminal_core_sizes"
                ]
                for strategy in STRATEGIES
            },
            "targeted_witness": {
                "report_id": proof_validation["report_id"],
                "proof_sha256": proof_validation["proof_sha256"],
                "core_parents": proof_validation["core_parents"],
                "core_size": proof_validation["telemetry_sat"][
                    "unsat_core_clauses"
                ],
                "cadical_result": proof_validation["cadical"]["result"],
                "proofcheck_verdict": proof_validation["proofcheck"][
                    "gate_verdict"
                ],
            },
            "independent_validation": "pass",
        },
        "overall_decision": overall,
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_root", type=Path)
    parser.add_argument("--reuse-analysis", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path, required=True)
    arguments = parser.parse_args(argv)
    summary = analyze(
        arguments.run_root.resolve(),
        arguments.reuse_analysis.resolve(),
        arguments.proof_validation.resolve(),
    )
    write_json(arguments.output.resolve(), summary)
    arguments.markdown.resolve().write_text(
        markdown_report(summary),
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "contract_id": summary["source"]["contract_id"],
                "overall_decision": summary["overall_decision"],
                "results": summary["source"]["result_count"],
                "decisions": {
                    candidate: summary["comparisons"][candidate]["decision"]
                    for candidate in CANDIDATES
                },
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AnalysisError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
