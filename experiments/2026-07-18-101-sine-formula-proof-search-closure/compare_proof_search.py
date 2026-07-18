#!/usr/bin/env python3
"""Compare C and Rust SInE formula-owner pruning through proof search."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


STAT_LABELS = (
    "Parsed axioms",
    "Removed by relevancy pruning/SinE",
    "Initial clauses",
    "Initial clauses in saturation",
    "Processed clauses",
    "Current number of processed clauses",
    "Current number of unprocessed clauses",
    "Current number of archived formulas",
)

CASES = (
    {
        "name": "threshold",
        "fixture": "threshold.p",
        "filter": "Threshold(1)",
        "input_names": ("first", "second"),
    },
    {
        "name": "gsine",
        "fixture": "gsine.p",
        "filter": "GSinE(CountTerms,,false,10.0,,2,10,1.0)",
        "input_names": ("goal", "link", "far", "irrelevant"),
    },
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def extract_result(
    process: subprocess.CompletedProcess[bytes], input_names: tuple[str, ...]
) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    statistics: dict[str, int] = {}
    for label in STAT_LABELS:
        match = re.search(
            rf"^% {re.escape(label)}\s*:\s*(\d+)$", stdout, re.MULTILINE
        )
        if match is None:
            raise ValueError(f"missing statistic {label!r}")
        statistics[label] = int(match.group(1))
    strategy = next(
        (line for line in stdout.splitlines() if line.startswith("% SinE strategy is ")),
        None,
    )
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    completion = next(
        (
            line
            for line in stdout.splitlines()
            if line in {
                "% Clause set closed under restricted calculus!",
                "% Proof found!",
            }
        ),
        None,
    )
    if strategy is None or status is None or completion is None:
        raise ValueError("missing SInE or proof-search completion surface")
    input_pattern = re.compile(
        rf"^fof\(({'|'.join(re.escape(name) for name in input_names)}),",
        re.MULTILINE,
    )
    surviving_formula_owners = input_pattern.findall(stdout)
    initialization_index = stdout.index("% Initializing proof state")
    formula_docs_before_initialization = all(
        stdout.index(f"fof({name},") < initialization_index
        for name in surviving_formula_owners
    )
    return {
        "completion": completion,
        "exit_code": process.returncode,
        "formula_docs_before_initialization": formula_docs_before_initialization,
        "statistics": statistics,
        "status": status,
        "stderr": process.stderr.decode("utf-8"),
        "strategy": strategy,
        "surviving_formula_owners": surviving_formula_owners,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent
    reports: list[dict[str, Any]] = []
    for case in CASES:
        fixture = root / str(case["fixture"])
        common_args = [
            "--tstp-in",
            "--tstp-out",
            "--no-generation",
            "--print-statistics",
            "--output-level=4",
            f"--sine={case['filter']}",
        ]
        c_process = run(
            [
                "wsl.exe",
                "-d",
                args.distro,
                "--exec",
                args.c_exe,
                *common_args,
                windows_to_wsl(fixture),
            ]
        )
        rust_process = run(
            [str(args.rust_exe.resolve()), *common_args, str(fixture.resolve())]
        )
        input_names = tuple(str(name) for name in case["input_names"])
        c_result = extract_result(c_process, input_names)
        rust_result = extract_result(rust_process, input_names)
        reports.append(
            {
                "name": case["name"],
                "display_args": [*common_args, "$FIXTURE"],
                "c": c_result,
                "rust": rust_result,
                "all_exact": c_result == rust_result,
            }
        )

    report = {
        "schema_version": 1,
        "case_count": len(reports),
        "matching_cases": sum(case["all_exact"] for case in reports),
        "all_exact": all(case["all_exact"] for case in reports),
        "cases": reports,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not report["all_exact"]:
        print("SInE formula-owner proof-search comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("SInE formula-owner proof-search reference changed", file=sys.stderr)
            return 1
    print(f"matched {report['matching_cases']}/{report['case_count']} cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
