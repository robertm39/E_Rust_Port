#!/usr/bin/env python3
"""Verify and summarize the proof-lemma/watchlist transfer experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Iterable, Sequence

from common import (
    ExperimentError,
    PROOF_STATUSES,
    canonical_json,
    proof_step_count,
    read_jsonl,
    sha256_file,
)


STRATEGIES = (
    "control",
    "watch_same",
    "lemma_same",
    "watch_cross",
    "lemma_cross",
)
TREATMENTS = STRATEGIES[1:]
REPETITIONS = 2
EXPECTED_RESULTS_PER_PHASE = 8 * len(STRATEGIES) * REPETITIONS


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 6)


def metric(result: dict[str, Any], *path: str) -> float | None:
    value: Any = result.get("_telemetry")
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return float(value)
    return None


def load_preparation(
    prepared_root: Path,
) -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, Any]]:
    manifest_path = prepared_root / "prepared-manifest.jsonl"
    rows = read_jsonl(manifest_path)
    if not rows or rows[0].get("problem_count") != 16:
        raise ExperimentError("invalid prepared manifest")
    header, records = rows[0], rows[1:]
    if len(records) != 16:
        raise ExperimentError("prepared manifest record count mismatch")
    contract_path = prepared_root / "preparation-contract.json"
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    body = {
        key: value
        for key, value in contract.items()
        if key not in {"contract_id", "created_at", "host"}
    }
    expected_id = hashlib.sha256(canonical_json(body)).hexdigest()
    if contract.get("contract_id") != expected_id:
        raise ExperimentError("preparation contract ID is invalid")
    if header.get("preparation_contract_id") != expected_id:
        raise ExperimentError("prepared manifest names another contract")
    if header.get("preparation_contract_sha256") != sha256_file(contract_path):
        raise ExperimentError("preparation contract hash mismatch")
    for record in records:
        for name in STRATEGIES:
            variant = record["variants"][name]
            if name == "control":
                continue
            path = prepared_root / variant["path"]
            if sha256_file(path) != variant["sha256"]:
                raise ExperimentError(f"prepared wrapper hash mismatch: {path}")
    selection = json.loads(
        (prepared_root / "selection-summary.json").read_text(encoding="utf-8")
    )
    return header, records, selection


def load_phase(
    results_root: Path, phase: str
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    phase_root = results_root / phase
    contract_path = phase_root / "contract.json"
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    body = {
        key: value
        for key, value in contract.items()
        if key not in {"contract_id", "created_at", "host"}
    }
    expected_id = hashlib.sha256(canonical_json(body)).hexdigest()
    if contract.get("contract_id") != expected_id:
        raise ExperimentError(f"{phase} contract ID is invalid")
    if contract.get("phase") != phase:
        raise ExperimentError(f"{phase} contract names another phase")
    results: list[dict[str, Any]] = []
    for result_path in sorted((phase_root / "runs").rglob("result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result.get("contract_id") != expected_id:
            raise ExperimentError(f"foreign result contract: {result_path}")
        stdout_path = result_path.parent / "stdout.txt"
        stderr_path = result_path.parent / "stderr.txt"
        if sha256_file(stdout_path) != result["stdout_sha256"]:
            raise ExperimentError(f"stdout hash mismatch: {stdout_path}")
        if sha256_file(stderr_path) != result["stderr_sha256"]:
            raise ExperimentError(f"stderr hash mismatch: {stderr_path}")
        telemetry = None
        telemetry_path = result_path.parent / "telemetry.json"
        if result["telemetry_sha256"] is not None:
            if sha256_file(telemetry_path) != result["telemetry_sha256"]:
                raise ExperimentError(f"telemetry hash mismatch: {telemetry_path}")
        if result["telemetry_present"]:
            telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
            if telemetry.get("schema") != "umlaut.search-telemetry":
                raise ExperimentError(f"bad telemetry schema: {telemetry_path}")
        stdout = stdout_path.read_text(encoding="utf-8", errors="replace")
        steps = proof_step_count(stdout)
        if result.get("szs_status") in PROOF_STATUSES and steps == 0:
            raise ExperimentError(f"proof status has no PCL steps: {stdout_path}")
        results.append(
            {
                **result,
                "_path": result_path.as_posix(),
                "_telemetry": telemetry,
                "_proof_steps": steps,
                "_watch_hits": stdout.count("final_subsumes_wl")
                + stdout.count("Watchlist reduced by"),
            }
        )
    if len(results) != EXPECTED_RESULTS_PER_PHASE:
        raise ExperimentError(
            f"{phase}: expected {EXPECTED_RESULTS_PER_PHASE} results, "
            f"found {len(results)}"
        )
    coordinates = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        )
        for result in results
    }
    if len(coordinates) != EXPECTED_RESULTS_PER_PHASE:
        raise ExperimentError(f"{phase}: duplicate result coordinates")
    if set(contract["strategies"]) != set(STRATEGIES):
        raise ExperimentError(f"{phase}: strategy contract mismatch")
    return contract, results


def reproducible_solved(
    results: Sequence[dict[str, Any]], strategy: str
) -> set[str]:
    by_problem: dict[str, list[dict[str, Any]]] = {}
    for result in results:
        if result["strategy"] == strategy:
            by_problem.setdefault(str(result["problem_id"]), []).append(result)
    return {
        problem
        for problem, group in by_problem.items()
        if len(group) == REPETITIONS
        and all(result["szs_status"] in PROOF_STATUSES for result in group)
    }


def paired_ratios(
    results: Sequence[dict[str, Any]], treatment: str
) -> dict[str, Any]:
    by_coordinate = {
        (
            result["problem_id"],
            result["strategy"],
            result["repetition"],
        ): result
        for result in results
    }
    cpu_ratios: list[float] = []
    net_cpu_ratios: list[float] = []
    proof_ratios: list[float] = []
    rss_ratios: list[float] = []
    generated_ratios: list[float] = []
    processed_ratios: list[float] = []
    coordinates: list[dict[str, Any]] = []
    problem_ids = sorted({str(result["problem_id"]) for result in results})
    for problem_id in problem_ids:
        for repetition in range(1, REPETITIONS + 1):
            control = by_coordinate.get((problem_id, "control", repetition))
            candidate = by_coordinate.get((problem_id, treatment, repetition))
            if (
                control is None
                or candidate is None
                or control["szs_status"] not in PROOF_STATUSES
                or candidate["szs_status"] not in PROOF_STATUSES
            ):
                continue
            control_cpu = metric(control, "resources", "total_cpu_seconds")
            candidate_cpu = metric(candidate, "resources", "total_cpu_seconds")
            control_steps = int(control["_proof_steps"])
            candidate_steps = int(candidate["_proof_steps"])
            row: dict[str, Any] = {
                "problem_id": problem_id,
                "repetition": repetition,
                "control_proof_steps": control_steps,
                "treatment_proof_steps": candidate_steps,
            }
            if control_cpu not in {None, 0.0} and candidate_cpu is not None:
                cpu_ratio = candidate_cpu / control_cpu
                preparation_cpu = (
                    float(candidate["admissibility_cpu_seconds"]) / REPETITIONS
                    if treatment.startswith("lemma_")
                    else 0.0
                )
                net_cpu_ratio = (candidate_cpu + preparation_cpu) / control_cpu
                cpu_ratios.append(cpu_ratio)
                net_cpu_ratios.append(net_cpu_ratio)
                row["cpu_ratio"] = cpu_ratio
                row["net_cpu_ratio"] = net_cpu_ratio
            if control_steps > 0:
                proof_ratio = candidate_steps / control_steps
                proof_ratios.append(proof_ratio)
                row["proof_step_ratio"] = proof_ratio
            for name, path, destination in (
                (
                    "rss",
                    ("resources", "maximum_resident_pages"),
                    rss_ratios,
                ),
                ("generated", ("search_funnel", "generated"), generated_ratios),
                ("processed", ("search_funnel", "processed"), processed_ratios),
            ):
                control_value = metric(control, *path)
                candidate_value = metric(candidate, *path)
                if control_value not in {None, 0.0} and candidate_value is not None:
                    ratio = candidate_value / control_value
                    destination.append(ratio)
                    row[f"{name}_ratio"] = ratio
            coordinates.append(row)
    return {
        "common_solved_repetition_coordinates": len(coordinates),
        "median_cpu_ratio": rounded(median(cpu_ratios)),
        "median_net_cpu_ratio": rounded(median(net_cpu_ratios)),
        "median_proof_step_ratio": rounded(median(proof_ratios)),
        "median_maximum_resident_pages_ratio": rounded(median(rss_ratios)),
        "median_generated_ratio": rounded(median(generated_ratios)),
        "median_processed_ratio": rounded(median(processed_ratios)),
        "coordinates": coordinates,
    }


def summarize_strategy(
    records: Sequence[dict[str, Any]],
    results: Sequence[dict[str, Any]],
    strategy: str,
) -> dict[str, Any]:
    selected = [result for result in results if result["strategy"] == strategy]
    solved = reproducible_solved(results, strategy)
    variants = [record["variants"][strategy] for record in records]
    statuses = Counter(
        str(result["szs_status"]) if result["szs_status"] is not None else "None"
        for result in selected
    )
    return {
        "runs": len(selected),
        "status_counts": dict(sorted(statuses.items())),
        "reproducible_solved": len(solved),
        "reproducible_solved_ids": sorted(solved),
        "one_repeat_only_solved_ids": sorted(
            {
                str(result["problem_id"])
                for result in selected
                if result["szs_status"] in PROOF_STATUSES
            }
            - solved
        ),
        "unique_target_guidance_clauses": sum(
            int(variant["guidance_clause_count"]) for variant in variants
        ),
        "unique_target_added_clauses": sum(
            int(variant["added_clause_count"]) for variant in variants
        ),
        "median_guidance_clauses_per_target": rounded(
            median(int(variant["guidance_clause_count"]) for variant in variants)
        ),
        "median_added_clauses_per_target": rounded(
            median(int(variant["added_clause_count"]) for variant in variants)
        ),
        "admissibility_attempts": sum(
            int(variant.get("admissibility_attempt_count", 0))
            for variant in variants
        ),
        "admissibility_rejections": sum(
            int(variant.get("admissibility_rejected_count", 0))
            for variant in variants
        ),
        "admissibility_cpu_seconds": rounded(
            sum(
                float(variant.get("admissibility_cpu_seconds", 0.0))
                for variant in variants
            )
        ),
        "admissibility_wall_seconds": rounded(
            sum(
                float(variant.get("admissibility_wall_seconds", 0.0))
                for variant in variants
            )
        ),
        "watch_hit_markers": sum(int(result["_watch_hits"]) for result in selected),
        "telemetry_records": sum(
            result["_telemetry"] is not None for result in selected
        ),
        "proof_records": sum(
            result["szs_status"] in PROOF_STATUSES
            and result["_proof_steps"] > 0
            for result in selected
        ),
    }


def correctness_failures(results: Sequence[dict[str, Any]]) -> list[str]:
    failures: list[str] = []
    for result in results:
        label = (
            f"{result['strategy']}/{result['problem_id']}/"
            f"rep-{result['repetition']}"
        )
        if result["szs_status"] is None:
            failures.append(f"missing_status:{label}")
        if (
            result["_telemetry"] is None
            and result["szs_status"] != "ResourceOut"
        ):
            failures.append(f"missing_telemetry:{label}")
        if result["szs_status"] in PROOF_STATUSES and result["_proof_steps"] == 0:
            failures.append(f"missing_proof:{label}")
        if result["szs_status"] not in PROOF_STATUSES | {"ResourceOut"}:
            failures.append(f"unexpected_status:{label}:{result['szs_status']}")
    return sorted(failures)


def decide(
    *,
    treatment: str,
    test_summary: dict[str, Any],
    correctness_ok: bool,
    replay_verified: bool,
) -> dict[str, Any]:
    comparison = test_summary["comparisons"][treatment]
    treatment_summary = test_summary["strategies"][treatment]
    lost = comparison["control_only_reproducible_solves"]
    unique = comparison["treatment_only_reproducible_solves"]
    common = comparison["paired"]["common_solved_repetition_coordinates"]
    cpu_ratio = (
        comparison["paired"]["median_net_cpu_ratio"]
        if treatment.startswith("lemma_")
        else comparison["paired"]["median_cpu_ratio"]
    )
    proof_ratio = comparison["paired"]["median_proof_step_ratio"]
    effective_clauses = (
        treatment_summary["unique_target_added_clauses"]
        if treatment.startswith("lemma_")
        else treatment_summary["unique_target_guidance_clauses"]
    )
    if not correctness_ok:
        verdict = "stop"
        reason = "correctness_or_contract_failure"
    elif lost:
        verdict = "stop"
        reason = "reproducible_control_solve_lost"
    elif unique and replay_verified and (
        not treatment.startswith("lemma_") or effective_clauses > 0
    ):
        verdict = "adopt"
        reason = "reproducible_treatment_only_test_solve"
    elif (
        common >= 4
        and cpu_ratio is not None
        and proof_ratio is not None
        and cpu_ratio <= 0.95
        and proof_ratio <= 0.95
        and (not treatment.startswith("lemma_") or effective_clauses > 0)
    ):
        verdict = "adopt"
        reason = "paired_cpu_and_proof_shortening"
    elif (
        effective_clauses == 0
        and not unique
        and (
            common == 0
            or cpu_ratio is None
            or proof_ratio is None
            or cpu_ratio > 0.95
            or proof_ratio > 0.95
        )
    ):
        verdict = "stop_no_value"
        reason = "zero_effective_clauses_and_no_observed_gain"
    elif (
        common >= 4
        and cpu_ratio is not None
        and proof_ratio is not None
        and cpu_ratio >= 1.05
        and proof_ratio >= 1.05
    ):
        verdict = "stop"
        reason = "paired_cpu_and_proof_regression"
    elif unique and not replay_verified:
        verdict = "uncertain"
        reason = "treatment_only_proof_replay_pending_or_failed"
    else:
        verdict = "uncertain"
        reason = "insufficient_or_mixed_evidence"
    return {
        "verdict": verdict,
        "reason": reason,
        "effective_clause_count": effective_clauses,
        "common_solved_repetition_coordinates": common,
        "cpu_ratio_used": cpu_ratio,
        "proof_step_ratio": proof_ratio,
        "treatment_only_reproducible_solves": unique,
        "control_only_reproducible_solves": lost,
        "replay_verified": replay_verified,
    }


def summarize_phase(
    records: Sequence[dict[str, Any]],
    results: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    control_solved = reproducible_solved(results, "control")
    strategies = {
        name: summarize_strategy(records, results, name) for name in STRATEGIES
    }
    comparisons = {}
    for treatment in TREATMENTS:
        treatment_solved = reproducible_solved(results, treatment)
        comparisons[treatment] = {
            "treatment_only_reproducible_solves": sorted(
                treatment_solved - control_solved
            ),
            "control_only_reproducible_solves": sorted(
                control_solved - treatment_solved
            ),
            "common_reproducible_solves": sorted(
                control_solved & treatment_solved
            ),
            "paired": paired_ratios(results, treatment),
        }
    failures = correctness_failures(results)
    return {
        "strategies": strategies,
        "comparisons": comparisons,
        "correctness_failures": failures,
        "correctness_ok": not failures,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepared-root", type=Path, required=True)
    parser.add_argument("--results-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--replay-report", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    prepared_root = arguments.prepared_root.resolve()
    results_root = arguments.results_root.resolve()
    header, all_records, selection = load_preparation(prepared_root)
    phase_summaries: dict[str, Any] = {}
    contracts: dict[str, str] = {}
    binary_hashes: set[str] = set()
    for phase in ("validation", "test"):
        contract, results = load_phase(results_root, phase)
        records = [
            record
            for record in all_records
            if record["experiment_split"] == phase
        ]
        phase_summaries[phase] = summarize_phase(records, results)
        contracts[phase] = contract["contract_id"]
        binary_hashes.add(str(contract["binary_sha256"]))
    if len(binary_hashes) != 1:
        raise ExperimentError("validation and test used different binaries")

    replay_verified: dict[str, bool] = {}
    if arguments.replay_report is not None:
        replay = json.loads(
            arguments.replay_report.read_text(encoding="utf-8")
        )
        replay_verified = {
            name: bool(value)
            for name, value in replay.get("treatments_verified", {}).items()
        }
    decisions = {
        treatment: decide(
            treatment=treatment,
            test_summary=phase_summaries["test"],
            correctness_ok=(
                phase_summaries["validation"]["correctness_ok"]
                and phase_summaries["test"]["correctness_ok"]
            ),
            replay_verified=replay_verified.get(
                treatment,
                not phase_summaries["test"]["comparisons"][treatment][
                    "treatment_only_reproducible_solves"
                ],
            ),
        )
        for treatment in TREATMENTS
    }
    summary = {
        "schema_version": 1,
        "source_revision": header["source_revision"],
        "binary_sha256": next(iter(binary_hashes)),
        "prepared_manifest_sha256": sha256_file(
            prepared_root / "prepared-manifest.jsonl"
        ),
        "preparation_contract_id": header["preparation_contract_id"],
        "contracts": contracts,
        "selection_overhead": {
            "source_traces": selection["source_trace_count"],
            "candidate_clauses": selection["candidate_count"],
            "cpu_seconds": rounded(float(selection["total_cpu_seconds"])),
            "wall_seconds": rounded(float(selection["total_wall_seconds"])),
            "sources": selection["sources"],
        },
        "phases": phase_summaries,
        "decisions": decisions,
    }
    body = dict(summary)
    summary["report_id"] = hashlib.sha256(canonical_json(body)).hexdigest()
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes(canonical_json(summary) + b"\n")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error

