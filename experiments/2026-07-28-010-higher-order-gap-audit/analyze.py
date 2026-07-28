#!/usr/bin/env python3
"""Verify and summarize the higher-order gap audit and staged experiment."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import statistics
import sys
from collections import Counter, defaultdict
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
PAIR_METRICS = {
    "cpu": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "term_storage_bytes": ("terms", "storage_estimate_bytes"),
    "maximum_resident_pages": ("resources", "maximum_resident_pages"),
}
INFERENCE_METRICS = {
    "positive_extensionality": (
        "inferences",
        "positive_extensionality",
    ),
    "negative_extensionality": (
        "inferences",
        "negative_extensionality",
    ),
    "paramodulations": ("inferences", "paramodulations"),
    "factorizations": ("inferences", "factorizations"),
    "equation_resolutions": ("inferences", "equation_resolutions"),
    "disequality_decompositions": (
        "inferences",
        "disequality_decompositions",
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


BASE = load_module("higher_order_gap_analyze_base", BASE_ANALYZE_PATH)
AnalysisError = BASE.AnalysisError


def rounded_median(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return (
        None
        if not materialized
        else round(statistics.median(materialized), 6)
    )


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
    for name, path in INFERENCE_METRICS.items():
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
    terms = [
        value
        for result in selected
        if (value := BASE.metric(
            result, "terms", "storage_estimate_bytes"
        ))
        is not None
    ]
    summary["term_storage_bytes"] = {
        "records": len(terms),
        "median": rounded_median(float(value) for value in terms),
        "maximum": max(terms) if terms else None,
    }
    return summary


def aggregate_strategy(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
    budget: str,
) -> dict[str, Any]:
    return {
        **BASE.aggregate_strategy(contract, results, strategy, budget),
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
) -> dict[str, float | int | None]:
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
    output: dict[str, float | int | None] = {}
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


def checked_json(path: Path, id_field: str) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    body = {key: item for key, item in value.items() if key != id_field}
    expected = hashlib.sha256(BASE.canonical_json(body)).hexdigest()
    if value.get(id_field) != expected:
        raise AnalysisError(f"invalid {id_field}: {path}")
    return value


def proof_taxonomy(report: dict[str, Any]) -> dict[str, Any]:
    counts: Counter[str] = Counter()
    held_out_verified = 0
    focused_verified = False
    for case in report["cases"]:
        reasons = " ".join(case.get("gate_reasons", []))
        if (
            case.get("gate_verdict") == "verified"
            and case.get("gate_returncode") == 0
        ):
            label = "verified"
        elif "AdapterError" in reasons:
            label = "adapter_coverage_gap"
        elif "recognized positive verdict" in reasons:
            label = "checker_implementation_gap"
        elif "VerifiedBad" in reasons:
            label = "checker_rejected"
        else:
            label = "other_unverified"
        counts[label] += 1
        if case["scope"] == "held_out_larger_budget" and label == "verified":
            held_out_verified += 1
        if case["scope"] == "focused_positive_extensionality":
            focused_verified = label == "verified"
    return {
        "counts": dict(sorted(counts.items())),
        "held_out_expected": report["expected_held_out_cases"],
        "held_out_verified": held_out_verified,
        "focused_verified": focused_verified,
        "all_verified": report["all_verified"],
    }


def contradictory_statuses(
    phases: dict[
        str, tuple[dict[str, Any], Sequence[dict[str, Any]]]
    ],
) -> list[dict[str, Any]]:
    contradictions = []
    for phase, (_, results) in phases.items():
        for result in results:
            status = result["szs_status"]
            if (
                status in PROOF_STATUSES | MODEL_STATUSES
                and not result["expected_status_match"]
            ):
                contradictions.append(
                    {
                        "phase": phase,
                        "problem_id": result["problem_id"],
                        "strategy": result["strategy"],
                        "budget": result["budget"],
                        "repetition": result["repetition"],
                        "status": status,
                        "expected_class": result["expected_class"],
                    }
                )
    return contradictions


def default_decision(
    *,
    comparison: dict[str, Any],
    fof_ratios: dict[str, Any],
    proof_complete: bool,
    contradictions: int,
) -> dict[str, Any]:
    cpu = comparison["median_cpu_ratio"]
    generated = comparison["median_generated_ratio"]
    high_water = comparison["median_high_water_total_ratio"]
    fof_cpu = fof_ratios["median_cpu_ratio"]
    fof_high_water = fof_ratios["median_high_water_total_ratio"]
    fof_rss = fof_ratios["median_maximum_resident_pages_ratio"]
    validity = proof_complete and contradictions == 0
    overhead = (
        fof_cpu is not None
        and fof_cpu <= 1.02
        and fof_high_water is not None
        and fof_high_water <= 1.02
        and fof_rss is not None
        and fof_rss <= 1.05
    )
    no_losses = not comparison["right_only"]
    unique = len(comparison["left_only"]) >= 2 and no_losses
    efficient = (
        no_losses
        and cpu is not None
        and cpu <= 0.95
        and generated is not None
        and generated <= 1.02
        and high_water is not None
        and high_water <= 1.02
    )
    advances = validity and overhead and (unique or efficient)
    return {
        "result": (
            "enable_selected_higher_order_mechanism_by_default"
            if advances
            else "retain_existing_higher_order_defaults"
        ),
        "advances": advances,
        "validity_gate_passed": validity,
        "fof_overhead_gate_passed": overhead,
        "unique_solve_gate_passed": unique,
        "efficiency_gate_passed": efficient,
        "criteria": {
            "requires_all_held_out_proof_claims_verified": True,
            "requires_zero_contradictory_statuses": True,
            "fof_cpu_ratio_limit": 1.02,
            "fof_high_water_ratio_limit": 1.02,
            "fof_rss_ratio_limit": 1.05,
            "selected_only_solve_threshold": 2,
            "common_solved_cpu_ratio_limit": 0.95,
            "common_solved_generated_ratio_limit": 1.02,
            "common_solved_high_water_ratio_limit": 1.02,
        },
    }


def phase_summary(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> dict[str, Any]:
    return {
        budget: {
            strategy: aggregate_strategy(
                contract, results, strategy, budget
            )
            for strategy in contract["strategies"]
        }
        for budget in contract["budgets"]
    }


def analyze(
    *,
    experiment_root: Path,
    pos_ext_root: Path,
    audit_summary_path: Path,
    final_selection_path: Path,
    proof_validation_path: Path,
) -> dict[str, Any]:
    staged = {
        phase: BASE.load_phase(experiment_root, phase)
        for phase in ("calibration", "validation", "test", "fof")
    }
    secondary = {
        phase: BASE.load_phase(pos_ext_root, phase)
        for phase in ("pos_ext_holdout", "pos_ext_fof")
    }
    phases = {**staged, **secondary}
    final_selection = BASE.load_selection(final_selection_path)
    chosen = final_selection["selected_strategies"][0]
    test_contract, test_results = staged["test"]
    if (
        test_contract["selection"]["selection_id"]
        != final_selection["selection_id"]
    ):
        raise AnalysisError("test contract pins another final selection")
    audit = checked_json(audit_summary_path, "report_id")
    proof_validation = BASE.load_proof_validation(proof_validation_path)
    if proof_validation["test_contract_id"] != test_contract["contract_id"]:
        raise AnalysisError("proof validation names another test contract")

    test_comparisons = {
        budget: BASE.comparison(
            test_contract,
            test_results,
            chosen,
            "baseline_auto",
            budget,
        )
        for budget in test_contract["budgets"]
    }
    test_ratios = {
        budget: paired_all_run_ratios(
            test_results, chosen, "baseline_auto", budget
        )
        for budget in test_contract["budgets"]
    }
    fof_contract, fof_results = staged["fof"]
    fof_comparison = BASE.comparison(
        fof_contract, fof_results, chosen, "baseline_auto", "fof"
    )
    fof_ratios = paired_all_run_ratios(
        fof_results, chosen, "baseline_auto", "fof"
    )
    pos_contract, pos_results = secondary["pos_ext_holdout"]
    pos_comparison = BASE.comparison(
        pos_contract,
        pos_results,
        "pos_ext_all",
        "baseline_auto",
        "holdout",
    )
    pos_ratios = paired_all_run_ratios(
        pos_results, "pos_ext_all", "baseline_auto", "holdout"
    )
    pos_fof_contract, pos_fof_results = secondary["pos_ext_fof"]
    pos_fof_comparison = BASE.comparison(
        pos_fof_contract,
        pos_fof_results,
        "pos_ext_all",
        "baseline_auto",
        "fof",
    )
    pos_fof_ratios = paired_all_run_ratios(
        pos_fof_results, "pos_ext_all", "baseline_auto", "fof"
    )

    contradictions = contradictory_statuses(phases)
    proof_summary = proof_taxonomy(proof_validation)
    decision = default_decision(
        comparison=test_comparisons["larger"],
        fof_ratios=fof_ratios,
        proof_complete=(
            proof_summary["held_out_verified"]
            == proof_summary["held_out_expected"]
        ),
        contradictions=len(contradictions),
    )
    pos_ext_inferences = mechanism_summary(
        pos_results, "pos_ext_all", "holdout"
    )["positive_extensionality"]
    option_fix_valid = (
        proof_summary["focused_verified"]
        and not contradictions
        and pos_ext_inferences["positive_records"] > 0
    )
    return {
        "schema_version": 1,
        "binary_sha256": test_contract["binary_sha256"],
        "contracts": {
            phase: contract["contract_id"]
            for phase, (contract, _) in phases.items()
        },
        "audit": audit,
        "selected_strategy": chosen,
        "selected_features": test_contract["strategies"][chosen]["features"],
        "problem_counts": {
            phase: len(contract["selected_problem_ids"])
            for phase, (contract, _) in phases.items()
        },
        "run_count": sum(len(results) for _, results in phases.values()),
        "phase_summaries": {
            phase: phase_summary(contract, results)
            for phase, (contract, results) in phases.items()
        },
        "test_comparisons": test_comparisons,
        "test_all_run_ratios": test_ratios,
        "fof_comparison": fof_comparison,
        "fof_all_run_ratios": fof_ratios,
        "positive_extensionality_holdout": {
            "comparison": pos_comparison,
            "all_run_ratios": pos_ratios,
            "fof_comparison": pos_fof_comparison,
            "fof_all_run_ratios": pos_fof_ratios,
            "positive_extensionality": pos_ext_inferences,
        },
        "proof_validation": proof_validation,
        "proof_taxonomy": proof_summary,
        "contradictory_statuses": contradictions,
        "option_fix_decision": {
            "result": (
                "retain_positive_extensionality_option_fix"
                if option_fix_valid
                else "reject_positive_extensionality_option_fix"
            ),
            "valid": option_fix_valid,
            "default_unchanged": True,
        },
        "default_decision": decision,
    }


def render_comparison(
    title: str,
    comparison: dict[str, Any],
    ratios: dict[str, Any],
) -> list[str]:
    return [
        f"## {title}",
        "",
        (
            f"- Coverage: {comparison['left']} "
            f"{comparison['left_solved']}, {comparison['right']} "
            f"{comparison['right_solved']}."
        ),
        f"- Left-only: {comparison['left_only'] or 'none'}.",
        f"- Right-only: {comparison['right_only'] or 'none'}.",
        f"- Paired all-run CPU ratio: {ratios['median_cpu_ratio']}.",
        (
            "- Paired all-run generated ratio: "
            f"{ratios['median_generated_ratio']}."
        ),
        (
            "- Paired all-run high-water ratio: "
            f"{ratios['median_high_water_total_ratio']}."
        ),
        (
            "- Paired all-run term-storage ratio: "
            f"{ratios['median_term_storage_bytes_ratio']}."
        ),
        (
            "- Paired all-run max-RSS ratio: "
            f"{ratios['median_maximum_resident_pages_ratio']}."
        ),
        "",
    ]


def render_markdown(summary: dict[str, Any]) -> str:
    audit = summary["audit"]
    lines = [
        "# Higher-order gap audit results",
        "",
        f"- Selected staged strategy: `{summary['selected_strategy']}`.",
        f"- Umlaut binary SHA-256: `{summary['binary_sha256']}`.",
        f"- Controlled search runs: {summary['run_count']}.",
        f"- Full-corpus audit: {audit['problem_count']} THF problems.",
        "",
        "## Full-corpus failure taxonomy",
        "",
        "| Classification | Problems |",
        "| --- | ---: |",
    ]
    for name, count in sorted(audit["taxonomy"].items()):
        lines.append(f"| `{name}` | {count} |")
    lines.append("")
    lines.extend(
        render_comparison(
            "Held-out staged winner versus baseline (larger)",
            summary["test_comparisons"]["larger"],
            summary["test_all_run_ratios"]["larger"],
        )
    )
    lines.extend(
        render_comparison(
            "Staged winner FOF control",
            summary["fof_comparison"],
            summary["fof_all_run_ratios"],
        )
    )
    pos = summary["positive_extensionality_holdout"]
    lines.extend(
        render_comparison(
            "Direct positive-extensionality THF holdout",
            pos["comparison"],
            pos["all_run_ratios"],
        )
    )
    lines.extend(
        render_comparison(
            "Direct positive-extensionality FOF control",
            pos["fof_comparison"],
            pos["fof_all_run_ratios"],
        )
    )
    inference = pos["positive_extensionality"]
    proof = summary["proof_taxonomy"]
    lines.extend(
        [
            "## Inference and proof audit",
            "",
            (
                "- Positive extensionality fired in "
                f"{inference['positive_records']} held-out run records "
                f"({inference['sum']} total inferences)."
            ),
            (
                "- Nörgler verified "
                f"{proof['held_out_verified']}/{proof['held_out_expected']} "
                "reproducible larger-budget held-out proof claims."
            ),
            (
                "- The focused PosExt=1, NegExt=0 refutation was "
                f"{'verified' if proof['focused_verified'] else 'not verified'}."
            ),
            f"- Checker taxonomy: `{proof['counts']}`.",
            (
                "- Contradictory terminal statuses: "
                f"{len(summary['contradictory_statuses'])}."
            ),
            "",
            "## Decisions",
            "",
            (
                "- Option correction: "
                f"`{summary['option_fix_decision']['result']}`."
            ),
            (
                "- Default schedule: "
                f"`{summary['default_decision']['result']}`."
            ),
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--pos-ext-root", type=Path, required=True)
    parser.add_argument("--audit-summary", type=Path, required=True)
    parser.add_argument("--final-selection", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    summary = analyze(
        experiment_root=arguments.experiment_root.resolve(),
        pos_ext_root=arguments.pos_ext_root.resolve(),
        audit_summary_path=arguments.audit_summary.resolve(),
        final_selection_path=arguments.final_selection.resolve(),
        proof_validation_path=arguments.proof_validation.resolve(),
    )
    arguments.json_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.json_output.write_bytes(BASE.canonical_json(summary) + b"\n")
    arguments.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    arguments.markdown_output.write_text(
        render_markdown(summary), encoding="utf-8", newline="\n"
    )
    print(
        f"OK: {summary['run_count']} verified runs; "
        f"decision {summary['default_decision']['result']}"
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
