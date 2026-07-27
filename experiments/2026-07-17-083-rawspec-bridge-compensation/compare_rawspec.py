#!/usr/bin/env python3
"""Compare raw feature vectors for represented formula-owner inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


CASES = (
    ("represented_fof", "represented_fof.p"),
    ("represented_thf", "represented_thf.p"),
)


def run(command: list[str], stdin: bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        input=stdin,
        check=False,
        capture_output=True,
        timeout=120,
    )


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()


def summarize(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    return {
        "exit_code": process.returncode,
        "stdout_bytes": len(process.stdout),
        "stdout_sha256": digest(process.stdout),
        "stderr_bytes": len(process.stderr),
        "stderr_sha256": digest(process.stderr),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    directory = Path(__file__).resolve().parent
    comparisons = []
    for name, fixture_name in CASES:
        stdin = (directory / fixture_name).read_bytes()
        case_args = ["--raw-class", "--tstp-format", "-"]
        c_process = run(
            [
                "wsl.exe",
                "-d",
                "Ubuntu-24.04",
                "--exec",
                args.c_exe,
                *case_args,
            ],
            stdin,
        )
        rust_process = run([str(args.rust_exe.resolve()), *case_args], stdin)
        exact = (
            c_process.returncode == rust_process.returncode
            and c_process.stdout == rust_process.stdout
            and c_process.stderr == rust_process.stderr
        )
        comparison: dict[str, Any] = {
            "case": name,
            "args": case_args,
            "exact": exact,
            "c": summarize(c_process),
            "rust": summarize(rust_process),
        }
        if not exact:
            comparison["mismatch"] = {
                "c_stdout": c_process.stdout.decode("utf-8", errors="backslashreplace"),
                "c_stderr": c_process.stderr.decode("utf-8", errors="backslashreplace"),
                "rust_stdout": rust_process.stdout.decode(
                    "utf-8", errors="backslashreplace"
                ),
                "rust_stderr": rust_process.stderr.decode(
                    "utf-8", errors="backslashreplace"
                ),
            }
        comparisons.append(comparison)

    exact_count = sum(bool(comparison["exact"]) for comparison in comparisons)
    report = {
        "schema_version": 1,
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "case_count": len(comparisons),
        "exact_count": exact_count,
        "comparisons": comparisons,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if exact_count != len(comparisons):
        print("rawspec comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("rawspec reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
