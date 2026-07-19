#!/usr/bin/env python3
"""Compare the optional performance-counter surface against unchanged C."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
FIXTURE = Path("eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop")
OPTIONS = ["--silent", "--print-statistics"]
EXPECTED_COUNTERS = [
    "MguTimer",
    "SatTimer",
    "ParamodTimer",
    "PMIndexTimer",
    "IndexUnifTimer",
    "BWRWTimer",
    "BWRWIndexTimer",
    "IndexMatchTimer",
    "FreqVecTimer",
    "FVIndexTimer",
    "SubsumeTimer",
    "SetSubsumeTimer",
    "ClauseEvalTimer",
]
COUNTER_RE = re.compile(r"^% PC\(([^)]+)\)\s+: ([0-9]+\.[0-9]{6})$", re.MULTILINE)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def run(command: list[str], input_text: str) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        input=input_text,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def checked_stdout(command: list[str]) -> str:
    completed = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def summarize(result: dict[str, Any]) -> dict[str, Any]:
    counters = COUNTER_RE.findall(result["stdout"])
    return {
        "exit_code": result["exit_code"],
        "stderr": result["stderr"],
        "stdout_sha256": sha256_bytes(result["stdout"].encode("utf-8")),
        "proof_found": "% Proof found!" in result["stdout"],
        "counter_names": [name for name, _value in counters],
        "counter_values_seconds": {
            name: value for name, value in counters
        },
    }


def valid_summary(summary: dict[str, Any]) -> bool:
    values = summary["counter_values_seconds"]
    return (
        summary["exit_code"] == 0
        and summary["stderr"] == ""
        and summary["proof_found"]
        and summary["counter_names"] == EXPECTED_COUNTERS
        and len(values) == len(EXPECTED_COUNTERS)
        and all(float(value) >= 0.0 for value in values.values())
        and float(values["SatTimer"]) > 0.0
    )


def stable_contract(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "reference": result["reference"],
        "rust": result["rust"],
        "workload": result["workload"],
        "c": {
            key: result["c"][key]
            for key in ("exit_code", "stderr", "proof_found", "counter_names")
        },
        "rust_result": {
            key: result["rust_result"][key]
            for key in ("exit_code", "stderr", "proof_found", "counter_names")
        },
        "accepted": result["accepted"],
    }


def collect(args: argparse.Namespace) -> dict[str, Any]:
    repo = Path(__file__).resolve().parents[2]
    fixture = repo / FIXTURE
    input_text = fixture.read_text(encoding="utf-8")
    rust_exe = args.rust_exe.resolve()
    c_raw = run(
        ["wsl.exe", "-d", args.distro, "--exec", args.c_exe, *OPTIONS],
        input_text,
    )
    rust_raw = run([str(rust_exe), *OPTIONS], input_text)
    c_hash = checked_stdout(
        ["wsl.exe", "-d", args.distro, "--exec", "sha256sum", args.c_exe]
    ).split()[0]
    c_summary = summarize(c_raw)
    rust_summary = summarize(rust_raw)
    result = {
        "schema_version": 1,
        "reference": {
            "commit": REFERENCE_COMMIT,
            "executable_sha256": c_hash,
            "platform": "Linux under WSL 2",
            "build_flag": "-DINSTRUMENT_PERF_CTR",
        },
        "rust": {
            "executable_sha256": sha256(rust_exe),
            "platform": "native Windows",
            "cargo_feature": "instrument-perf-ctr",
        },
        "workload": {
            "fixture": FIXTURE.as_posix(),
            "fixture_sha256": sha256(fixture),
            "input_transport": "stdin",
            "options": OPTIONS,
        },
        "c": c_summary,
        "rust_result": rust_summary,
        "accepted": valid_summary(c_summary) and valid_summary(rust_summary),
    }
    result["stable_contract"] = stable_contract(result)
    return result


def main() -> int:
    args = parse_args()
    result = collect(args)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if result["stable_contract"] != expected.get("stable_contract"):
            print(
                f"stable comparison mismatch: {args.output} != {args.expected}",
                file=sys.stderr,
            )
            return 1
    print(f"performance-counter comparison: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
