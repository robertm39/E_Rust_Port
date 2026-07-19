#!/usr/bin/env python3
"""Retain the stable compatibility facts from a support-tool matrix report."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def summarize(report: dict[str, Any]) -> dict[str, Any]:
    cases = report["cases"]
    counts = Counter(case["tool"] for case in cases)
    expected_differences = [
        {
            "name": case["name"],
            "reference_mode": case["reference_mode"],
            "mismatches": case["mismatches"],
        }
        for case in cases
        if case["expected_mismatches"]
    ]
    return {
        "schema_version": 1,
        "upstream_commit": report["reference_manifest"]["upstream_commit"],
        "candidate_kind": report["candidate_kind"],
        "case_count": report["case_count"],
        "tool_count": len(counts),
        "mismatch_count": report["mismatch_count"],
        "expected_difference_count": report["expected_difference_count"],
        "tool_case_counts": dict(sorted(counts.items())),
        "expected_differences": expected_differences,
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("support-tool report used the wrong archived C commit")
    if summary["candidate_kind"] != "windows-rust-tools":
        raise SystemExit("support-tool report did not use native Windows Rust tools")
    if summary["case_count"] != 216 or summary["tool_count"] != 25:
        raise SystemExit("support-tool matrix inventory changed")
    if summary["mismatch_count"] != 0:
        raise SystemExit("support-tool report contains unexpected mismatches")
    if summary["expected_difference_count"] != 8:
        raise SystemExit("support-tool expected-difference inventory changed")
    if any(
        difference["mismatches"] == []
        for difference in summary["expected_differences"]
    ):
        raise SystemExit("declared support-tool difference did not occur")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    report = json.loads(args.report.read_text(encoding="utf-8"))
    summary = summarize(report)
    validate(summary)

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if summary != expected:
            raise SystemExit("stable support-tool summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
