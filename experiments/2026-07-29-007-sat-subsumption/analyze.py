#!/usr/bin/env python3
"""Analyze SAT-subsumption captures and freeze crossover decisions."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Callable, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent


class AnalysisError(RuntimeError):
    """A capture, correctness, or selection-contract failure."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def quantile(values: Sequence[float], probability: float) -> float:
    if not values:
        raise AnalysisError("cannot take a quantile of an empty sequence")
    ordered = sorted(values)
    index = max(0, math.ceil(probability * len(ordered)) - 1)
    return float(ordered[index])


def distribution(values: Sequence[float]) -> dict[str, float]:
    if not values:
        return {}
    return {
        "median": float(statistics.median(values)),
        "p90": quantile(values, 0.90),
        "p95": quantile(values, 0.95),
        "p99": quantile(values, 0.99),
        "maximum": float(max(values)),
        "sum": float(sum(values)),
    }


def load_results(phase_root: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    contract_path = phase_root / "contract.json"
    results_path = phase_root / "results.jsonl"
    if not contract_path.is_file() or not results_path.is_file():
        raise AnalysisError(f"incomplete phase output: {phase_root}")
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    body = {key: value for key, value in contract.items() if key != "contract_id"}
    expected = hashlib.sha256(canonical_json(body)).hexdigest()
    if contract.get("contract_id") != expected:
        raise AnalysisError(f"invalid phase contract ID: {phase_root}")
    results: dict[str, dict[str, Any]] = {}
    with results_path.open(encoding="utf-8") as stream:
        for line in stream:
            result = json.loads(line)
            if result.get("contract_id") != expected:
                raise AnalysisError("result/contract ID mismatch")
            problem = result["problem_id"]
            if problem in results:
                raise AnalysisError(f"duplicate problem result: {problem}")
            results[problem] = result
    if set(results) != set(contract["problem_ids"]):
        raise AnalysisError("result problem coordinates do not match contract")
    return contract, results


def load_records(
    phase_root: Path, results: dict[str, dict[str, Any]]
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    digest_payloads: dict[str, tuple[str, str]] = {}
    for problem, result in results.items():
        capture_path = (
            phase_root / "runs" / result["family"] / problem / "capture.jsonl"
        )
        expected_count = result["capture_count"]
        if expected_count == 0:
            continue
        if not capture_path.is_file():
            raise AnalysisError(f"missing capture: {capture_path}")
        if sha256_file(capture_path) != result["capture_sha256"]:
            raise AnalysisError(f"capture hash mismatch: {capture_path}")
        count = 0
        with capture_path.open(encoding="utf-8") as stream:
            for line in stream:
                record = json.loads(line)
                if record.get("problem") != problem:
                    raise AnalysisError(f"capture problem mismatch: {capture_path}")
                payload = (record["side"], record["main"])
                previous = digest_payloads.setdefault(record["digest"], payload)
                if previous != payload:
                    raise AnalysisError(
                        f"digest collision with different payload: {record['digest']}"
                    )
                enriched = {
                    **record,
                    "category": result["category"],
                    "family": result["family"],
                }
                enriched["sat_ns"] = (
                    enriched["match_ns"] + enriched["ordinary_solve_ns"]
                )
                enriched["estimated_bytes"] = (
                    128
                    * (
                        enriched["positive_choices"]
                        + enriched["negative_choices"]
                    )
                    + 32 * enriched["binding_count"]
                    + 24
                    * (
                        enriched["ordinary_clause_count"]
                        + enriched["resolution_clause_count"]
                    )
                    + 4
                    * (
                        enriched["ordinary_literal_count"]
                        + enriched["resolution_literal_count"]
                    )
                )
                records.append(enriched)
                count += 1
        if count != expected_count:
            raise AnalysisError(
                f"capture count mismatch for {problem}: {count} != {expected_count}"
            )
    if not records:
        raise AnalysisError("analysis has no capture records")
    return records


def summarize_records(records: Sequence[dict[str, Any]]) -> dict[str, Any]:
    disagreements = [
        record for record in records if record["baseline"] != record["ordinary"]
    ]
    if disagreements:
        first = disagreements[0]
        raise AnalysisError(
            "ordinary SAT disagreement: "
            f"{first['problem']} ordinal {first['ordinal']} digest {first['digest']}"
        )
    by_problem: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        by_problem[record["problem"]].append(record)
    problem_ratios = {
        problem: statistics.median(
            record["sat_ns"] for record in problem_records
        )
        / statistics.median(
            record["baseline_ns"] for record in problem_records
        )
        for problem, problem_records in by_problem.items()
    }
    return {
        "records": len(records),
        "unique_pairs": len({record["digest"] for record in records}),
        "problems": len(by_problem),
        "families": len({record["family"] for record in records}),
        "categories": dict(sorted(Counter(record["category"] for record in records).items())),
        "baseline_true": sum(int(record["baseline"]) for record in records),
        "ordinary_disagreements": 0,
        "resolution_true": sum(int(record["resolution"]) for record in records),
        "resolution_unique_pairs": len(
            {record["digest"] for record in records if record["resolution"]}
        ),
        "baseline_ns": distribution([record["baseline_ns"] for record in records]),
        "sat_ns": distribution([record["sat_ns"] for record in records]),
        "match_ns": distribution([record["match_ns"] for record in records]),
        "resolution_solve_ns": distribution(
            [record["resolution_solve_ns"] for record in records]
        ),
        "aggregate_sat_ratio": sum(record["sat_ns"] for record in records)
        / sum(record["baseline_ns"] for record in records),
        "p95_sat_ratio": quantile(
            [record["sat_ns"] for record in records], 0.95
        )
        / quantile([record["baseline_ns"] for record in records], 0.95),
        "problem_balanced_median_ratio": float(
            statistics.median(problem_ratios.values())
        ),
        "maximum_problem_median_ratio": max(problem_ratios.values()),
        "maximum_estimated_bytes": max(
            record["estimated_bytes"] for record in records
        ),
        "side_literals": distribution(
            [record["side_literals"] for record in records]
        ),
        "main_literals": distribution(
            [record["main_literals"] for record in records]
        ),
        "positive_choices": distribution(
            [record["positive_choices"] for record in records]
        ),
        "negative_choices": distribution(
            [record["negative_choices"] for record in records]
        ),
    }


def policy_records(
    records: Sequence[dict[str, Any]], policy: dict[str, int]
) -> list[dict[str, Any]]:
    return [
        record
        for record in records
        if record["side_literals"] >= policy["min_side_literals"]
        and record["main_literals"] >= policy["min_main_literals"]
        and record["positive_choices"] >= policy["min_positive_choices"]
    ]


def policy_metrics(records: Sequence[dict[str, Any]]) -> dict[str, Any]:
    by_problem: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        by_problem[record["problem"]].append(record)
    problem_ratios = [
        statistics.median(item["sat_ns"] for item in values)
        / statistics.median(item["baseline_ns"] for item in values)
        for values in by_problem.values()
    ]
    return {
        "records": len(records),
        "problems": len(by_problem),
        "families": len({record["family"] for record in records}),
        "aggregate_ratio": sum(record["sat_ns"] for record in records)
        / sum(record["baseline_ns"] for record in records),
        "p95_ratio": quantile(
            [record["sat_ns"] for record in records], 0.95
        )
        / quantile([record["baseline_ns"] for record in records], 0.95),
        "maximum_problem_median_ratio": max(problem_ratios),
        "problem_balanced_median_ratio": float(
            statistics.median(problem_ratios)
        ),
        "maximum_estimated_bytes": max(
            record["estimated_bytes"] for record in records
        ),
    }


def candidate_policies(records: Sequence[dict[str, Any]]) -> list[dict[str, Any]]:
    candidates: list[dict[str, Any]] = []
    for min_side in range(2, 9):
        for min_main in range(2, 13):
            for min_choices in (0, 4, 8, 16, 32, 64):
                policy = {
                    "min_side_literals": min_side,
                    "min_main_literals": min_main,
                    "min_positive_choices": min_choices,
                }
                selected = policy_records(records, policy)
                if len(selected) < 200 or len(
                    {record["problem"] for record in selected}
                ) < 6:
                    continue
                metrics = policy_metrics(selected)
                if (
                    metrics["aggregate_ratio"] <= 0.80
                    and metrics["p95_ratio"] <= 0.90
                    and metrics["maximum_estimated_bytes"] < 256 * 1024
                ):
                    candidates.append({"policy": policy, "metrics": metrics})
    candidates.sort(
        key=lambda candidate: (
            candidate["metrics"]["aggregate_ratio"],
            candidate["metrics"]["p95_ratio"],
            -candidate["metrics"]["records"],
            canonical_json(candidate["policy"]),
        )
    )
    return candidates


def make_selection(
    records: Sequence[dict[str, Any]], contract_id: str
) -> dict[str, Any]:
    candidates = candidate_policies(records)
    body = {
        "schema_version": 1,
        "source_phase": "calibration",
        "source_contract_id": contract_id,
        "selected_policy": candidates[0]["policy"] if candidates else None,
        "selected_metrics": candidates[0]["metrics"] if candidates else None,
        "eligible_policy_count": len(candidates),
        "decision": "advance" if candidates else "no-calibration-policy",
    }
    return {
        **body,
        "selection_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }


def load_selection(path: Path) -> dict[str, Any]:
    selection = json.loads(path.read_text(encoding="utf-8"))
    body = {
        key: value for key, value in selection.items() if key != "selection_id"
    }
    expected = hashlib.sha256(canonical_json(body)).hexdigest()
    if selection.get("selection_id") != expected:
        raise AnalysisError("invalid selection ID")
    if selection.get("source_phase") != "calibration":
        raise AnalysisError("selection is not from calibration")
    return selection


def validate_policy(
    records: Sequence[dict[str, Any]], selection: dict[str, Any]
) -> dict[str, Any]:
    policy = selection["selected_policy"]
    if policy is None:
        return {
            "decision": "no-calibration-policy",
            "policy": None,
            "metrics": None,
        }
    selected = policy_records(records, policy)
    if not selected:
        return {
            "decision": "reject-empty-validation-regime",
            "policy": policy,
            "metrics": None,
        }
    metrics = policy_metrics(selected)
    advances = (
        metrics["records"] >= 200
        and metrics["problems"] >= 6
        and metrics["aggregate_ratio"] <= 0.90
        and metrics["p95_ratio"] <= 0.95
        and metrics["maximum_problem_median_ratio"] <= 1.10
        and metrics["maximum_estimated_bytes"] < 256 * 1024
    )
    return {
        "decision": "advance" if advances else "reject-validation",
        "policy": policy,
        "metrics": metrics,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase-root", type=Path, required=True)
    parser.add_argument("--phase", choices=("calibration", "validation", "test"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--selection-output", type=Path)
    parser.add_argument("--selection", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    phase_root = arguments.phase_root.resolve()
    contract, results = load_results(phase_root)
    if contract.get("phase") != arguments.phase:
        raise AnalysisError("requested phase does not match capture contract")
    records = load_records(phase_root, results)
    summary = summarize_records(records)
    maximum_rss = max(
        (
            result["maximum_rss_kib"]
            for result in results.values()
            if result["maximum_rss_kib"] is not None
        ),
        default=None,
    )
    decision: dict[str, Any]
    if arguments.phase == "calibration":
        selection = make_selection(records, contract["contract_id"])
        decision = {
            "decision": selection["decision"],
            "policy": selection["selected_policy"],
            "metrics": selection["selected_metrics"],
        }
        if arguments.selection_output is None:
            raise AnalysisError("calibration requires --selection-output")
        arguments.selection_output.parent.mkdir(parents=True, exist_ok=True)
        arguments.selection_output.write_bytes(canonical_json(selection) + b"\n")
    else:
        if arguments.selection is None:
            raise AnalysisError(f"{arguments.phase} requires --selection")
        selection = load_selection(arguments.selection)
        decision = validate_policy(records, selection)

    report = {
        "schema_version": 1,
        "phase": arguments.phase,
        "contract_id": contract["contract_id"],
        "summary": summary,
        "decision": decision,
        "process_maximum_rss_kib": maximum_rss,
        "prototype": {
            "path": str(EXPERIMENT_ROOT / "sat_subsumption.rs"),
            "bytes": (EXPERIMENT_ROOT / "sat_subsumption.rs").stat().st_size,
            "sha256": sha256_file(EXPERIMENT_ROOT / "sat_subsumption.rs"),
        },
        "analyzer_sha256": sha256_file(Path(__file__)),
    }
    report_body = canonical_json(report)
    report["report_id"] = hashlib.sha256(report_body).hexdigest()
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes(canonical_json(report) + b"\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AnalysisError, OSError, ValueError, ZeroDivisionError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
