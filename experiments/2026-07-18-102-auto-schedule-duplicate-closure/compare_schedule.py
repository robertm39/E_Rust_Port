#!/usr/bin/env python3
"""Compare C and Rust auto-schedule coordinator projections."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, TypeVar


T = TypeVar("T")

CASES = (
    ("proof_search", []),
    ("cnf_nested_preprocessing_proof", ["--cnf"]),
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def unique(values: list[T]) -> list[T]:
    result: list[T] = []
    for value in values:
        if value not in result:
            result.append(value)
    return result


def extract_result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    schedule_summaries = unique(
        [
            tuple(int(value) for value in match)
            for match in re.findall(
                r"^% Scheduled (\d+) strats onto (\d+) cores with (\d+) seconds \((\d+) total\)$",
                stdout,
                re.MULTILINE,
            )
        ]
    )
    total_times = [
        float(value)
        for value in re.findall(
            r"^% Total time\s+: ([0-9.]+) s$", stdout, re.MULTILINE
        )
    ]
    proof_position = stdout.index("% Proof found!")
    resource_positions = [
        match.start()
        for match in re.finditer(r"^% User time\s+:", stdout, re.MULTILINE)
    ]
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    if status is None:
        raise ValueError("missing SZS status")
    return {
        "exit_code": process.returncode,
        "preprocessing_classes": unique(
            re.findall(r"^% Preprocessing class: ([^.]+)\.$", stdout, re.MULTILINE)
        ),
        "search_classes": unique(
            re.findall(r"^% Search class: (\S+)$", stdout, re.MULTILINE)
        ),
        "schedule_summaries": schedule_summaries,
        "started_strategies": unique(
            re.findall(r"^% Starting (\S+) with ", stdout, re.MULTILINE)
        ),
        "winning_strategies": unique(
            re.findall(r"^% Result found by (\S+)$", stdout, re.MULTILINE)
        ),
        "preprocessing_time_reported": "% Preprocessing time       : " in stdout,
        "completion": "% Proof found!",
        "status": status,
        "resource_footer_count": len(resource_positions),
        "resource_footers_after_proof": all(
            position > proof_position for position in resource_positions
        ),
        "resource_totals_nondecreasing": all(
            later >= earlier for earlier, later in zip(total_times, total_times[1:])
        ),
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

    fixture = Path(__file__).resolve().parent / "false.p"
    reports: list[dict[str, Any]] = []
    for name, extra_args in CASES:
        common_args = [
            "--auto-schedule=1",
            "--resources-info",
            *extra_args,
            "--tstp-in",
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
        reports.append(
            {
                "name": name,
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
        print("auto-schedule coordinator comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("auto-schedule coordinator reference changed", file=sys.stderr)
            return 1
    print(f"matched {report['matching_cases']}/{report['case_count']} scheduler cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
