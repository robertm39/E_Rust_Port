#!/usr/bin/env python3
"""Compare mixed clause/formula relevance pruning and accounting."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
LABEL_RE = re.compile(rb"^(?:cnf|fof|tff|tcf|thf)\(([^,]+),", re.MULTILINE)
STAT_LABELS = (
    "Parsed axioms",
    "Removed by relevancy pruning/SinE",
    "Initial clauses",
    "Initial clauses in saturation",
    "Current number of unprocessed clauses",
    "Current number of archived formulas",
)


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def labels(result: dict[str, object]) -> list[str]:
    return [
        match.decode("utf-8")
        for match in LABEL_RE.findall(bytes(result["stdout"]))
    ]


def statistics(result: dict[str, object]) -> dict[str, int]:
    text = bytes(result["stdout"]).decode("utf-8", errors="replace")
    parsed: dict[str, int] = {}
    for label in STAT_LABELS:
        match = re.search(rf"^% {re.escape(label)}\s*: (\d+)$", text, re.MULTILINE)
        if match is not None:
            parsed[label] = int(match.group(1))
    return parsed


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = bytes(result["stdout"])
    stderr = bytes(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "labels": labels(result),
        "statistics": statistics(result),
        "stdout_bytes": len(stdout),
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_bytes": len(stderr),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    fixture = Path(__file__).resolve().parent / "mixed.p"
    common = ["--tstp-in", "--tstp-out", "--rel-pruning-level=3"]
    cases = (
        (
            "prune_owner_order",
            ["--prune", "--output-level=2", *common],
            ["goal", "formula_bridge", "clause_bridge"],
            {},
        ),
        (
            "cnf_owner_accounting",
            ["--cnf", "--output-level=1", "--print-statistics", *common],
            [],
            {
                "Parsed axioms": 5,
                "Removed by relevancy pruning/SinE": 2,
                "Initial clauses": 3,
                "Initial clauses in saturation": 3,
                "Current number of unprocessed clauses": 3,
                "Current number of archived formulas": 0,
            },
        ),
    )

    results: list[dict[str, object]] = []
    for case_name, options, expected_labels, expected_stats in cases:
        rust = run([str(args.rust_exe.resolve()), *options, str(fixture)])
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *options,
                wsl_path(fixture),
            ]
        )
        rust_labels = labels(rust)
        c_labels = labels(c)
        rust_stats = statistics(rust)
        c_stats = statistics(c)
        labels_match = not expected_labels or (
            rust_labels == expected_labels and c_labels == expected_labels
        )
        stats_match = not expected_stats or (
            rust_stats == expected_stats and c_stats == expected_stats
        )
        status_match = rust["exit_code"] == 0 and c["exit_code"] == 0
        stderr_match = bytes(rust["stderr"]) == b"" and bytes(c["stderr"]) == b""
        results.append(
            {
                "case": case_name,
                "labels_match": labels_match,
                "statistics_match": stats_match,
                "status_match": status_match,
                "stderr_match": stderr_match,
                "rust": summarize(rust),
                "c": summarize(c),
            }
        )

    rendered = json.dumps(
        {
            "reference_commit": REFERENCE_COMMIT,
            "case_count": len(results),
            "matching_cases": sum(
                all(
                    bool(result[key])
                    for key in (
                        "labels_match",
                        "statistics_match",
                        "status_match",
                        "stderr_match",
                    )
                )
                for result in results
            ),
            "results": results,
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
