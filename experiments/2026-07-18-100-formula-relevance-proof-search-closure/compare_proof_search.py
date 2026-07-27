#!/usr/bin/env python3
"""Compare C and Rust formula relevance pruning through proof search."""

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


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def extract_result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    statistics: dict[str, int] = {}
    for label in STAT_LABELS:
        match = re.search(
            rf"^% {re.escape(label)}\s*:\s*(\d+)$", stdout, re.MULTILINE
        )
        if match is None:
            raise ValueError(f"missing statistic {label!r}")
        statistics[label] = int(match.group(1))
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    completion = next(
        (
            line
            for line in stdout.splitlines()
            if line == "% Clause set closed under restricted calculus!"
        ),
        None,
    )
    if status is None or completion is None:
        raise ValueError("missing proof-search completion surface")
    return {
        "completion": completion,
        "exit_code": process.returncode,
        "statistics": statistics,
        "status": status,
        "stderr": process.stderr.decode("utf-8"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture = Path(__file__).resolve().parent / "mixed.p"
    common_args = [
        "--tstp-in",
        "--tstp-out",
        "--no-generation",
        "--print-statistics",
        "--rel-pruning-level=3",
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
    c_result = extract_result(c_process)
    rust_result = extract_result(rust_process)
    all_exact = c_result == rust_result
    report = {
        "schema_version": 1,
        "display_args": [*common_args, "$FIXTURE"],
        "c": c_result,
        "rust": rust_result,
        "all_exact": all_exact,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not all_exact:
        print("formula relevance proof-search comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("formula relevance proof-search reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
