#!/usr/bin/env python3
"""Validate and summarize the restricted integer-induction experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from pathlib import Path
from typing import Any, Iterable, Sequence


PHASES = ("calibration", "validation", "test", "transfer")
STRATEGIES = ("baseline", "induction")
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
CONTRADICTORY_STATUSES = {"Satisfiable", "CounterSatisfiable"}
METRICS = {
    "cpu_seconds": ("resources", "total_cpu_seconds"),
    "generated": ("search_funnel", "generated"),
    "processed": ("search_funnel", "processed"),
    "raw_clauses": ("input_funnel", "raw_clauses"),
    "preprocessing_removed": ("input_funnel", "preprocessing_removed"),
    "paramodulations": ("inferences", "paramodulations"),
    "rewrite_steps": ("simplification", "rewrite_steps"),
    "high_water_total": ("search_funnel", "high_water_total"),
    "maximum_resident_pages": ("resources", "maximum_resident_pages"),
}


class AnalysisError(RuntimeError):
    """The experiment is incomplete or violates a frozen contract."""


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{id(value)}.tmp")
    temporary.write_bytes(canonical_json(value) + b"\n")
    temporary.replace(path)


def metric(result: dict[str, Any], path: tuple[str, ...]) -> int | float | None:
    value: Any = result.get("_telemetry")
    for key in path:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return value
    return None


def rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 6)


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def load_phase(
    experiment_root: Path, phase: str
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    phase_root = experiment_root / phase
    contract_path = phase_root / "contract.json"
    if not contract_path.is_file():
        raise AnalysisError(f"missing contract: {contract_path}")
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    results = []
    for result_path in sorted((phase_root / "runs").rglob("result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result["contract_id"] != contract["contract_id"]:
            raise AnalysisError(f"contract mismatch: {result_path}")
        run_dir = result_path.parent
        if sha256_file(run_dir / "stdout.txt") != result["stdout_sha256"]:
            raise AnalysisError(f"stdout hash mismatch: {result_path}")
        if sha256_file(run_dir / "stderr.txt") != result["stderr_sha256"]:
            raise AnalysisError(f"stderr hash mismatch: {result_path}")
        telemetry_path = run_dir / "telemetry.json"
        telemetry = None
        if result["telemetry_sha256"] is not None:
            if sha256_file(telemetry_path) != result["telemetry_sha256"]:
                raise AnalysisError(f"telemetry hash mismatch: {result_path}")
            telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
        result["_path"] = str(result_path.relative_to(phase_root))
        result["_telemetry"] = telemetry
        results.append(result)

    expected = (
        len(contract["selected_problem_ids"])
        * len(contract["strategies"])
        * contract["phase_config"]["repetitions"]
    )
    if len(results) != expected:
        raise AnalysisError(
            f"{phase}: expected {expected} results, found {len(results)}"
        )
    coordinates = {
        (result["problem_id"], result["strategy"], result["repetition"])
        for result in results
    }
    if len(coordinates) != expected:
        raise AnalysisError(f"{phase}: duplicate or missing coordinate")
    return contract, results


def reproducible_coverage(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
) -> set[str]:
    repetitions = contract["phase_config"]["repetitions"]
    by_problem: dict[str, list[dict[str, Any]]] = {}
    for result in results:
        if result["strategy"] == strategy:
            by_problem.setdefault(result["problem_id"], []).append(result)
    return {
        problem_id
        for problem_id, values in by_problem.items()
        if len(values) == repetitions
        and all(result["szs_status"] in PROOF_STATUSES for result in values)
    }


def aggregate(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    strategy: str,
) -> dict[str, Any]:
    selected = [result for result in results if result["strategy"] == strategy]
    coverage = reproducible_coverage(contract, results, strategy)
    return {
        "runs": len(selected),
        "reproducible_solved": len(coverage),
        "reproducible_solved_ids": sorted(coverage),
        "status_counts": {
            status: sum(str(result["szs_status"]) == status for result in selected)
            for status in sorted({str(result["szs_status"]) for result in selected})
        },
        "external_timeouts": sum(result["external_timeout"] for result in selected),
        "missing_status": sum(result["szs_status"] is None for result in selected),
        "missing_telemetry": sum(
            result["_telemetry"] is None for result in selected
        ),
        "medians": {
            name: rounded(
                median(
                    value
                    for result in selected
                    if (value := metric(result, path)) is not None
                )
            )
            for name, path in METRICS.items()
        },
        "totals": {
            name: sum(
                value
                for result in selected
                if (value := metric(result, path)) is not None
            )
            for name, path in METRICS.items()
        },
    }


def paired_ratio(
    results: Sequence[dict[str, Any]], path: tuple[str, ...]
) -> float | None:
    indexed = {
        (result["problem_id"], result["strategy"], result["repetition"]): result
        for result in results
    }
    ratios = []
    coordinates = sorted(
        {
            (result["problem_id"], result["repetition"])
            for result in results
        }
    )
    for problem_id, repetition in coordinates:
        if (problem_id, "baseline", repetition) not in indexed:
            continue
        if (problem_id, "induction", repetition) not in indexed:
            continue
        baseline = metric(indexed[(problem_id, "baseline", repetition)], path)
        induction = metric(indexed[(problem_id, "induction", repetition)], path)
        if baseline not in {None, 0} and induction is not None:
            ratios.append(induction / baseline)
    return rounded(median(ratios))


def clause_growth(contract: dict[str, Any]) -> list[dict[str, Any]]:
    values = []
    for problem_id in contract["selected_problem_ids"]:
        materialized = contract["materialized_inputs"][problem_id]
        baseline = materialized["baseline"]["clausified_clauses"]
        induction = materialized["induction"]["clausified_clauses"]
        values.append(
            {
                "problem_id": problem_id,
                "baseline": baseline,
                "induction": induction,
                "delta": induction - baseline,
                "ratio": rounded(induction / baseline) if baseline else None,
            }
        )
    return values


def analyze_phase(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> dict[str, Any]:
    baseline = reproducible_coverage(contract, results, "baseline")
    induction = reproducible_coverage(contract, results, "induction")
    return {
        "contract_id": contract["contract_id"],
        "binary_sha256": contract["binary_sha256"],
        "source_snapshot_sha256": contract["source_snapshot_sha256"],
        "selected_problem_ids": contract["selected_problem_ids"],
        "strategies": {
            strategy: aggregate(contract, results, strategy)
            for strategy in STRATEGIES
        },
        "induction_only": sorted(induction - baseline),
        "baseline_only": sorted(baseline - induction),
        "common": sorted(baseline & induction),
        "paired_median_ratios": {
            name: paired_ratio(results, path) for name, path in METRICS.items()
        },
        "clause_growth": clause_growth(contract),
    }


def load_proof_validation(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    report = json.loads(path.resolve().read_text(encoding="utf-8"))
    body = {key: value for key, value in report.items() if key != "report_id"}
    expected_id = hashlib.sha256(canonical_json(body)).hexdigest()
    if report.get("report_id") != expected_id:
        raise AnalysisError("proof-validation report ID mismatch")
    return report


def decide(
    phases: dict[str, dict[str, Any]],
    contradictory: list[dict[str, Any]],
    proof_validation: dict[str, Any] | None,
) -> dict[str, Any]:
    test_selected = set(phases["test"]["selected_problem_ids"])
    test_advances = set(phases["test"]["induction_only"])
    transfer_advances = phases["transfer"]["induction_only"]
    transfer_ratios = phases["transfer"]["paired_median_ratios"]
    proof_ok = (
        proof_validation is not None
        and proof_validation["verified_cases"]
        == proof_validation["expected_cases"]
    )
    transfer_efficiency = (
        not phases["transfer"]["baseline_only"]
        and transfer_ratios["generated"] is not None
        and transfer_ratios["generated"] <= 0.8
        and transfer_ratios["cpu_seconds"] is not None
        and transfer_ratios["cpu_seconds"] <= 1.0
    )
    gates = {
        "all_targeted_test_examples_advance": test_advances == test_selected,
        "all_claimed_proofs_verify": proof_ok,
        "no_contradictory_status": not contradictory,
        "no_lost_baseline_solve": not any(
            phases[phase]["baseline_only"] for phase in PHASES
        ),
        "real_transfer_or_efficiency": bool(transfer_advances)
        or transfer_efficiency,
    }
    advance = all(gates.values())
    return {
        "verdict": (
            "advance_production_integer_induction"
            if advance
            else "defer_production_integer_induction"
        ),
        "advance": advance,
        "gates": gates,
        "targeted_test_advance_count": len(test_advances),
        "targeted_test_count": len(test_selected),
        "transfer_advance_count": len(transfer_advances),
        "transfer_efficiency_gate": transfer_efficiency,
    }


def markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Restricted integer-induction results",
        "",
        f"Decision: `{summary['decision']['verdict']}`.",
        "",
        "| Phase | Baseline solves | Induction solves | Induction-only |",
        "| --- | ---: | ---: | --- |",
    ]
    for phase in PHASES:
        value = summary["phases"][phase]
        lines.append(
            f"| {phase} | "
            f"{value['strategies']['baseline']['reproducible_solved']} | "
            f"{value['strategies']['induction']['reproducible_solved']} | "
            f"{', '.join(value['induction_only']) or '-'} |"
        )
    lines.extend(
        [
            "",
            "The five-problem CASC transfer set is train-split SWC and is not a",
            "family-held-out efficacy claim.",
            "",
            f"Summary ID: `{summary['summary_id']}`.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--proof-validation", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    experiment_root = arguments.experiment_root.resolve()
    audit = json.loads(arguments.audit.resolve().read_text(encoding="utf-8"))
    if audit["accepted_count"] != 5 or audit["recurrence_proxy_positive"] != 5:
        raise AnalysisError("audit population differs from the frozen five problems")
    phase_results: dict[str, dict[str, Any]] = {}
    all_results = []
    for phase in PHASES:
        contract, results = load_phase(experiment_root, phase)
        phase_results[phase] = analyze_phase(contract, results)
        all_results.extend(results)
    contradictory = [
        {
            "phase": result["phase"],
            "problem_id": result["problem_id"],
            "strategy": result["strategy"],
            "repetition": result["repetition"],
            "status": result["szs_status"],
        }
        for result in all_results
        if result["szs_status"] in CONTRADICTORY_STATUSES
    ]
    proof_validation = load_proof_validation(arguments.proof_validation)
    body = {
        "schema_version": 1,
        "audit_report_id": audit["report_id"],
        "trigger": {
            "accepted": audit["accepted_count"],
            "population": audit["manifest_problem_count"],
            "recurrence_proxy_positive": audit["recurrence_proxy_positive"],
            "recurrence_proxy_precision": audit["recurrence_proxy_precision"],
        },
        "total_runs": len(all_results),
        "phases": phase_results,
        "contradictory_statuses": contradictory,
        "proof_validation": proof_validation,
        "decision": decide(
            phase_results,
            contradictory,
            proof_validation,
        ),
    }
    summary = {
        **body,
        "summary_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }
    atomic_json(arguments.output.resolve(), summary)
    markdown_path = arguments.markdown.resolve()
    markdown_path.parent.mkdir(parents=True, exist_ok=True)
    markdown_path.write_text(markdown(summary), encoding="utf-8")
    print(
        f"OK: {len(all_results)} runs; "
        f"{summary['decision']['verdict']}; "
        f"summary {summary['summary_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        AnalysisError,
        OSError,
        KeyError,
        ValueError,
        json.JSONDecodeError,
        statistics.StatisticsError,
    ) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
