#!/usr/bin/env python3
"""Compare focused C/Rust executable goal-definition traces."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


EXISTS_LINE = re.compile(
    r"^cnf\(c_0_-?\d+, (plain|negated_conjecture), \((.+)\),.*\['exists'\]\)\.$"
)
COUNTERS = (
    "% Initial clauses in saturation",
    "% Processed clauses",
    "% Total rewrite steps",
)
COMMON_ARGS = (
    "--output-level=2",
    "--no-generation",
    "--expert-heuristic=(1*FIFOWeight(ConstPrio))",
)
CASES = {
    "all-signs": ("multi-goal.p", ("--goal-defs=All",)),
    "negative-only": ("multi-goal.p", ("--goal-defs=Neg",)),
    "recursive-subterms": (
        "nested-goal.p",
        ("--goal-defs=All", "--goal-subterm-defs"),
    ),
    "formula-origin": ("fof-goal.p", ("--goal-defs=All",)),
}


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def extract_trace(stdout: str) -> list[dict[str, str]]:
    lines = stdout.splitlines()
    start = lines.index("% Scanning for AC axioms") + 1
    trace: list[dict[str, str]] = []
    for line in lines[start:]:
        if not line:
            break
        match = EXISTS_LINE.match(line)
        if match is not None:
            role, clause = match.groups()
            trace.append({"role": role, "clause": clause.replace(" ", "")})
    return trace


def extract_counters(stdout: str) -> dict[str, int]:
    counters: dict[str, int] = {}
    for line in stdout.splitlines():
        for name in COUNTERS:
            if line.startswith(name):
                counters[name.removeprefix("% ")] = int(line.split(":", 1)[1])
    if len(counters) != len(COUNTERS):
        raise ValueError(f"missing counters in output: {counters}")
    return counters


def result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    return {
        "counters": extract_counters(stdout),
        "exit_code": process.returncode,
        "stderr": process.stderr.decode("utf-8"),
        "trace": extract_trace(stdout),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parent
    report_cases: dict[str, Any] = {}
    all_exact = True
    for case_name, (fixture_name, case_args) in CASES.items():
        fixture = fixture_dir / fixture_name
        c_process = run(
            [
                "wsl.exe",
                "-d",
                "Ubuntu-24.04",
                "--exec",
                args.c_exe,
                *COMMON_ARGS,
                *case_args,
                windows_to_wsl(fixture),
            ]
        )
        rust_process = run(
            [
                str(args.rust_exe.resolve()),
                *COMMON_ARGS,
                *case_args,
                str(fixture.resolve()),
            ]
        )
        c_result = result(c_process)
        rust_result = result(rust_process)
        exact = c_result == rust_result
        all_exact = all_exact and exact
        report_cases[case_name] = {
            "args": [*COMMON_ARGS, *case_args, "$FIXTURE"],
            "c": c_result,
            "exact": exact,
            "fixture": fixture_name,
            "rust": rust_result,
        }

    report = {
        "schema_version": 1,
        "all_exact": all_exact,
        "cases": report_cases,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")
    if not all_exact:
        print("goal-definition comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("goal-definition reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
