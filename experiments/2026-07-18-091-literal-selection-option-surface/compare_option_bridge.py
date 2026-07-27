#!/usr/bin/env python3
"""Compare C and Rust executable option-to-heuristic bridges."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


STRATEGY_CASES = {
    "literal_selection_limits": [
        "--print-strategy",
        "--no-preprocessing",
        "--literal-selection-strategy=SelectMinInfpos",
        "--select-on-processing-only",
        "--inherit-paramod-literals",
        "--inherit-goal-pm-literals",
        "--inherit-conjecture-pm-literals",
        "--selection-pos-min=1",
        "--selection-pos-max=2",
        "--selection-neg-min=3",
        "--selection-neg-max=4",
        "--selection-all-min=5",
        "--selection-all-max=6",
        "--selection-weight-min=7",
    ],
    "expert_heuristic": [
        "--print-strategy",
        "--no-preprocessing",
        "--expert-heuristic=FIFO",
    ],
    "no_generation_override": [
        "--print-strategy",
        "--no-preprocessing",
        "--literal-selection-strategy=SelectMinInfpos",
        "--no-generation",
    ],
    "inference_and_splitting": [
        "--print-strategy",
        "--no-preprocessing",
        "--disable-eq-factoring",
        "--disable-paramod-into-neg-units",
        "--condense-aggressive",
        "--disable-given-clause-fw-contraction",
        "--oriented-supersimul-paramod",
        "--split-clauses=3",
        "--split-method=2",
        "--split-aggressive",
        "--split-reuse-defs",
        "--disequality-decomposition=5",
        "--disequality-decomp-maxarity=4",
    ],
    "simul_paramod": ["--print-strategy", "--simul-paramod"],
    "oriented_simul_paramod": ["--print-strategy", "--oriented-simul-paramod"],
    "supersimul_paramod": ["--print-strategy", "--supersimul-paramod"],
    "oriented_supersimul_paramod": [
        "--print-strategy",
        "--oriented-supersimul-paramod",
    ],
}

STATUS_CASES = {
    "assume_incompleteness": [
        "--output-level=1",
        "--no-preprocessing",
        "--no-generation",
        "--assume-incompleteness",
    ]
}


def normalized_text(data: bytes) -> str:
    return data.decode("utf-8").replace("\r\n", "\n")


def run_case(exe: str, fixture: str, options: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        [exe, *options, fixture],
        check=False,
        capture_output=True,
        timeout=60,
    )
    return {
        "exit": completed.returncode,
        "stdout": normalized_text(completed.stdout),
        "stderr": normalized_text(completed.stderr),
    }


def summarize_status(result: dict[str, Any]) -> dict[str, Any]:
    statuses = re.findall(r"^% SZS status (\S+)", result["stdout"], re.MULTILINE)
    return {
        "exit": result["exit"],
        "statuses": statuses,
        "stderr": result["stderr"],
    }


def digest(result: dict[str, Any]) -> str:
    payload = json.dumps(result, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def run_worker(args: argparse.Namespace) -> int:
    jobs = json.loads(sys.stdin.read())
    results = {
        name: run_case(args.exe, args.fixture, options)
        for name, options in jobs.items()
    }
    sys.stdout.write(json.dumps(results, sort_keys=True))
    return 0


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture")
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    if args.worker:
        if args.exe is None or args.fixture is None:
            parser.error("--worker requires --exe and --fixture")
        return run_worker(args)

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")

    fixture = Path(__file__).resolve().parent / "selection.p"
    jobs = {**STRATEGY_CASES, **STATUS_CASES}
    rust_results = {
        name: run_case(str(args.rust_exe.resolve()), str(fixture), options)
        for name, options in jobs.items()
    }
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture",
            windows_to_wsl(fixture),
        ],
        input=json.dumps(jobs).encode("utf-8"),
        check=False,
        capture_output=True,
        timeout=180,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))

    cases = []
    for name in jobs:
        c_result = c_results[name]
        rust_result = rust_results[name]
        if name in STATUS_CASES:
            c_result = summarize_status(c_result)
            rust_result = summarize_status(rust_result)
        cases.append(
            {
                "name": name,
                "exact": c_result == rust_result,
                "c_sha256": digest(c_result),
                "rust_sha256": digest(rust_result),
                "exit": rust_result["exit"],
            }
        )

    report = {
        "schema_version": 1,
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "cases": cases,
        "exact_count": sum(case["exact"] for case in cases),
        "total": len(cases),
        "all_exact": all(case["exact"] for case in cases),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("option report does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_exact"]:
        failed = [case["name"] for case in cases if not case["exact"]]
        print(f"option bridge differs: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
