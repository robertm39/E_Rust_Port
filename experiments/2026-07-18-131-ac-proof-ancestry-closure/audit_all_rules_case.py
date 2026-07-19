#!/usr/bin/env python3
"""Retain the stable ALL_RULES compatibility result from a main report."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def summarize(report: dict[str, Any]) -> dict[str, Any]:
    matching = [case for case in report["cases"] if case["name"] == "ALL_RULES.p"]
    if len(matching) != 1:
        raise SystemExit("main report does not contain exactly one ALL_RULES.p case")
    case = matching[0]
    return {
        "schema_version": 1,
        "upstream_commit": report["reference_manifest"]["upstream_commit"],
        "candidate_kind": report["candidate_kind"],
        "name": case["name"],
        "scenario": case["scenario"],
        "mode": case["mode"],
        "arguments": case["arguments"],
        "reference_status": case["reference_status"],
        "candidate_status": case["candidate_status"],
        "reference_exit_code": case["reference_exit_code"],
        "candidate_exit_code": case["candidate_exit_code"],
        "normalized_output_equal": case["normalized_output_equal"],
        "mismatches": case["mismatches"],
        "expected_mismatches": case["expected_mismatches"],
        "mismatch_expectation_met": case["mismatch_expectation_met"],
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("ALL_RULES report used the wrong archived C commit")
    if summary["candidate_kind"] != "windows-rust":
        raise SystemExit("ALL_RULES report did not use native Windows Rust")
    if summary["mode"] != "fol" or summary["scenario"] != "file":
        raise SystemExit("ALL_RULES comparison mode changed")
    if summary["reference_status"] != "Theorem":
        raise SystemExit("archived C ALL_RULES outcome changed")
    if summary["candidate_status"] != summary["reference_status"]:
        raise SystemExit("Rust ALL_RULES outcome differs from archived C")
    if summary["reference_exit_code"] != 0 or summary["candidate_exit_code"] != 0:
        raise SystemExit("ALL_RULES comparison did not exit successfully")
    if not summary["normalized_output_equal"]:
        raise SystemExit("ALL_RULES normalized proof output differs")
    if summary["mismatches"] or summary["expected_mismatches"]:
        raise SystemExit("ALL_RULES must remain exact, not declared different")
    if not summary["mismatch_expectation_met"]:
        raise SystemExit("ALL_RULES exact-match expectation failed")


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
            raise SystemExit("stable ALL_RULES summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
