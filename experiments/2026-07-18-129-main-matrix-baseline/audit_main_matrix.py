#!/usr/bin/env python3
"""Retain stable compatibility facts from a main-executable matrix report."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def difference_record(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": case["name"],
        "mode": case["mode"],
        "reference_exit_code": case["reference_exit_code"],
        "candidate_exit_code": case["candidate_exit_code"],
        "reference_status": case["reference_status"],
        "candidate_status": case["candidate_status"],
        "mismatches": case["mismatches"],
        "expected_mismatches": case["expected_mismatches"],
    }


def summarize(report: dict[str, Any]) -> dict[str, Any]:
    cases = report["cases"]
    return {
        "schema_version": 1,
        "upstream_commit": report["reference_manifest"]["upstream_commit"],
        "candidate_kind": report["candidate_kind"],
        "case_count": report["case_count"],
        "mismatch_count": report["mismatch_count"],
        "expected_difference_count": report["expected_difference_count"],
        "case_inventory": [
            f"{case['mode']}:{case['name']}" for case in cases
        ],
        "unexpected_differences": [
            difference_record(case)
            for case in cases
            if not case["mismatch_expectation_met"]
        ],
        "expected_differences": [
            difference_record(case)
            for case in cases
            if case["expected_mismatches"]
        ],
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("main report used the wrong archived C commit")
    if summary["candidate_kind"] != "windows-rust":
        raise SystemExit("main report did not use the native Windows Rust executable")
    if summary["case_count"] != 50:
        raise SystemExit("main comparison inventory changed")
    if len(summary["case_inventory"]) != summary["case_count"]:
        raise SystemExit("main report case count does not match its inventory")
    inventory = set(summary["case_inventory"])
    if len(inventory) != summary["case_count"]:
        raise SystemExit("main report contains duplicate case identities")
    if summary["mismatch_count"] != 4:
        raise SystemExit("main unexpected-mismatch count changed")
    if len(summary["unexpected_differences"]) != summary["mismatch_count"]:
        raise SystemExit("main report mismatch count does not match its cases")
    if summary["expected_difference_count"] != 1:
        raise SystemExit("main expected-difference count changed")
    if len(summary["expected_differences"]) != summary["expected_difference_count"]:
        raise SystemExit("main expected-difference count does not match its cases")
    if any(
        difference["mismatches"] != difference["expected_mismatches"]
        for difference in summary["expected_differences"]
    ):
        raise SystemExit("declared main comparison difference changed shape")


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
            raise SystemExit("stable main-matrix summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
