#!/usr/bin/env python3
"""Run bounded native comparison arms and independently replay certificates."""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import Any, Iterable

from fm_core import (
    CERTIFICATE_SCHEMA,
    Bounds,
    FmError,
    load_corpus,
    saturate,
)
from fm_replay import replay


def percentile(values: Iterable[float], probability: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    index = max(0, min(len(ordered) - 1, int(probability * len(ordered) + 0.999999) - 1))
    return ordered[index]


def error_certificate(
    workload: dict[str, Any],
    mode: str,
    error: Exception,
) -> dict[str, Any]:
    return {
        "schema": CERTIFICATE_SCHEMA,
        "workload_id": workload.get("id", "<malformed>"),
        "mode": mode,
        "outcome": "unknown",
        "empty_clause_id": None,
        "records": [],
        "metrics": {
            "input_clauses": len(workload.get("clauses", [])),
            "retained_clauses": 0,
            "peak_clauses": 0,
            "generated": {
                "propositional_resolution": 0,
                "fourier_motzkin": 0,
            },
            "attempts": {
                "propositional_resolution": 0,
                "fourier_motzkin": 0,
            },
            "subsumed": 0,
            "elapsed_ns": 0,
            "crossed_bound": "malformed_input",
            "error": f"{type(error).__name__}: {error}",
            "max_coefficient_bits": 0,
        },
    }


def safe_saturate(
    workload: dict[str, Any],
    *,
    enable_fm: bool,
    bounds: Bounds,
) -> dict[str, Any]:
    mode = "native_fm" if enable_fm else "normalize_resolution"
    try:
        return saturate(workload, enable_fm=enable_fm, bounds=bounds)
    except (FmError, KeyError, TypeError, ValueError) as error:
        return error_certificate(workload, mode, error)


def expected_native_outcome(workload: dict[str, Any]) -> str | None:
    if workload["expected"] == "diagnostic":
        return None
    return "unsat" if workload["expected"] == "unsat" else "unknown"


def run_workload(
    workload: dict[str, Any],
    *,
    bounds: Bounds,
    warmups: int,
    repetitions: int,
    certificate_directory: Path,
) -> dict[str, Any]:
    arms = {
        "normalize_resolution": False,
        "native_fm": True,
    }
    arm_reports: dict[str, Any] = {}
    for mode, enable_fm in arms.items():
        for _ in range(warmups):
            safe_saturate(workload, enable_fm=enable_fm, bounds=bounds)
        certificates = [
            safe_saturate(workload, enable_fm=enable_fm, bounds=bounds)
            for _ in range(repetitions)
        ]
        outcomes = {certificate["outcome"] for certificate in certificates}
        if len(outcomes) != 1:
            raise FmError(f"{workload['id']} {mode} outcome is nondeterministic")
        replay_reports = [
            replay(workload, certificate)
            for certificate in certificates
            if "error" not in certificate["metrics"]
        ]
        if len(replay_reports) != len(certificates):
            if any(
                certificate["outcome"] == "unsat"
                for certificate in certificates
            ):
                raise FmError(f"{workload['id']} trusted an error certificate")
        representative = certificates[0]
        certificate_path = certificate_directory / f"{workload['id']}.{mode}.json"
        certificate_path.write_text(
            json.dumps(representative, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        elapsed_ms = [
            certificate["metrics"]["elapsed_ns"] / 1_000_000
            for certificate in certificates
        ]
        arm_reports[mode] = {
            "outcome": representative["outcome"],
            "certificate": str(certificate_path),
            "replay_count": len(replay_reports),
            "median_ms": statistics.median(elapsed_ms),
            "p95_ms": percentile(elapsed_ms, 0.95),
            "retained_clauses": representative["metrics"]["retained_clauses"],
            "peak_clauses": representative["metrics"]["peak_clauses"],
            "generated": representative["metrics"]["generated"],
            "attempts": representative["metrics"]["attempts"],
            "subsumed": representative["metrics"]["subsumed"],
            "crossed_bound": representative["metrics"]["crossed_bound"],
            "max_coefficient_bits": representative["metrics"][
                "max_coefficient_bits"
            ],
        }
    baseline = arm_reports["normalize_resolution"]
    native = arm_reports["native_fm"]
    expected = expected_native_outcome(workload)
    return {
        "id": workload["id"],
        "partition": workload["partition"],
        "template_family": workload["template_family"],
        "expected": workload["expected"],
        "expected_native": expected,
        "supported": workload.get("supported", True),
        "arms": arm_reports,
        "native_matches_expected": (
            True if expected is None else native["outcome"] == expected
        ),
        "baseline_matches_expected": (
            True
            if expected is None
            else
            baseline["outcome"] == "unknown"
            if workload["expected"] != "unsat"
            else baseline["outcome"] in {"unknown", "unsat"}
        ),
        "unique_native_closure": (
            native["outcome"] == "unsat"
            and baseline["outcome"] != "unsat"
        ),
        "retained_growth_ratio": (
            native["retained_clauses"]
            / max(1, baseline["retained_clauses"])
        ),
    }


def summarize(reports: list[dict[str, Any]]) -> dict[str, Any]:
    supported = [report for report in reports if report["supported"]]
    growth = [report["retained_growth_ratio"] for report in supported]
    native_times = [
        report["arms"]["native_fm"]["p95_ms"] for report in supported
    ]
    return {
        "workloads": len(reports),
        "supported": len(supported),
        "native_expected_matches": sum(
            report["native_matches_expected"] for report in reports
        ),
        "baseline_expected_matches": sum(
            report["baseline_matches_expected"] for report in reports
        ),
        "unique_native_closures": sum(
            report["unique_native_closure"] for report in reports
        ),
        "replayed_certificates": sum(
            arm["replay_count"]
            for report in reports
            for arm in report["arms"].values()
        ),
        "median_retained_growth_ratio": statistics.median(growth) if growth else 0,
        "p95_retained_growth_ratio": percentile(growth, 0.95),
        "native_p95_ms": percentile(native_times, 0.95),
        "crossed_bounds": sorted(
            {
                arm["crossed_bound"]
                for report in reports
                for arm in report["arms"].values()
                if arm["crossed_bound"] is not None
            }
        ),
        "all_native_expected": all(
            report["native_matches_expected"] for report in reports
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", type=Path)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument(
        "--partition",
        action="append",
        choices=["train", "validation", "test"],
    )
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--repetitions", type=int, default=5)
    arguments = parser.parse_args()
    loaded = json.loads(arguments.corpus.read_text(encoding="utf-8"))
    corpus = load_corpus(loaded.get("corpus", loaded))
    selected = set(arguments.partition or ["train", "validation", "test"])
    workloads = [
        workload
        for workload in corpus["workloads"]
        if workload["partition"] in selected
    ]
    arguments.output_directory.mkdir(parents=True, exist_ok=True)
    certificate_directory = arguments.output_directory / "certificates"
    certificate_directory.mkdir(parents=True, exist_ok=True)
    bounds = Bounds()
    reports = [
        run_workload(
            workload,
            bounds=bounds,
            warmups=arguments.warmups,
            repetitions=arguments.repetitions,
            certificate_directory=certificate_directory,
        )
        for workload in workloads
    ]
    result = {
        "corpus": str(arguments.corpus),
        "partitions": sorted(selected),
        "bounds": {
            key: value
            for key, value in vars(bounds).items()
        },
        "warmups": arguments.warmups,
        "repetitions": arguments.repetitions,
        "summary": summarize(reports),
        "workloads": reports,
    }
    report_path = arguments.output_directory / "native_report.json"
    report_path.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(result["summary"], indent=2, sort_keys=True))
    return 0 if result["summary"]["all_native_expected"] else 1


if __name__ == "__main__":
    sys.exit(main())
