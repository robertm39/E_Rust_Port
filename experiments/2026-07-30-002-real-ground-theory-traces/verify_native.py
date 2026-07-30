#!/usr/bin/env python3
"""Verify Rust-native decisions against the frozen query corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence

from native_protocol import parse_results
from reference_search import verify_model, verify_negative_cycle
from trace_model import canonical_json


class VerificationError(RuntimeError):
    """Native results are missing, disagreeing, or unverifiable."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def percentile(values: Sequence[int], quantile: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    position = (len(ordered) - 1) * quantile
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def write_certificate(
    path: Path,
    backend: str,
    records: Sequence[tuple[dict[str, Any], dict[str, Any]]],
) -> None:
    lines = ["UMLAUT_GROUND_THEORY_CERT_V1"]
    for workload, result in records:
        constraints = workload["branches"][0]["constraints"]
        lines.append(
            "\t".join(
                [
                    "DECISION",
                    backend,
                    workload["id"],
                    "branch",
                    workload["sort"],
                    result["status"],
                ]
            )
        )
        for constraint in constraints:
            bound = Fraction(constraint["bound"])
            lines.append(
                "\t".join(
                    [
                        "CONSTRAINT",
                        constraint["label"],
                        constraint["lhs"],
                        constraint["rhs"],
                        str(bound.numerator),
                        str(bound.denominator),
                    ]
                )
            )
        if result["status"] == "unsat":
            lines.append("CORE\t" + ",".join(result["core"]))
        else:
            for variable, raw_value in sorted(result["model"].items()):
                value = Fraction(raw_value)
                lines.append(
                    "\t".join(
                        [
                            "MODEL",
                            variable,
                            str(value.numerator),
                            str(value.denominator),
                        ]
                    )
                )
        lines.append("END_DECISION")
    lines.append("END")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--results", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--certificate", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    corpus = json.loads(arguments.corpus.read_text(encoding="utf-8"))
    results, metadata = parse_results(arguments.results)
    workloads = {workload["id"]: workload for workload in corpus["workloads"]}
    if len(workloads) != len(corpus["workloads"]):
        raise VerificationError("duplicate workload ID")
    result_index = {result["id"]: result for result in results}
    if len(result_index) != len(results):
        raise VerificationError("duplicate result ID")
    if set(result_index) != set(workloads):
        raise VerificationError("native result IDs do not match corpus")

    verified: list[tuple[dict[str, Any], dict[str, Any]]] = []
    mismatches = []
    evidence_failures = []
    counts = {"sat": 0, "unsat": 0}
    for workload in corpus["workloads"]:
        result = result_index[workload["id"]]
        expected = workload["branches"][0]["expected"]
        constraints = workload["branches"][0]["constraints"]
        if result["status"] != expected:
            mismatches.append(
                {
                    "id": workload["id"],
                    "expected": expected,
                    "actual": result["status"],
                }
            )
            continue
        if result["status"] == "unsat":
            valid = verify_negative_cycle(constraints, result["core"])
        elif result["status"] == "sat":
            valid = (
                set(result["model"]) == set(workload["variables"])
                and verify_model(constraints, result["model"])
            )
        else:
            raise VerificationError(f"unexpected status {result['status']!r}")
        if not valid:
            evidence_failures.append(workload["id"])
            continue
        counts[result["status"]] += 1
        verified.append((workload, result))
    if mismatches or evidence_failures:
        raise VerificationError(
            f"mismatches={mismatches[:5]}, evidence_failures={evidence_failures[:5]}"
        )
    write_certificate(arguments.certificate, "native", verified)
    timings = [result["elapsed_ns"] for result in results]
    report = {
        "schema": "umlaut-real-ground-native-verification-v1",
        "corpus_sha256": sha256_file(arguments.corpus),
        "results_sha256": sha256_file(arguments.results),
        "certificate_sha256": sha256_file(arguments.certificate),
        "metadata": metadata,
        "query_count": len(results),
        "verified_count": len(verified),
        "counts": counts,
        "timing": {
            "total_ns": sum(timings),
            "median_ns": statistics.median(timings) if timings else 0,
            "p95_ns": percentile(timings, 0.95),
            "minimum_ns": min(timings) if timings else 0,
            "maximum_ns": max(timings) if timings else 0,
        },
    }
    arguments.report.write_text(canonical_json(report) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
