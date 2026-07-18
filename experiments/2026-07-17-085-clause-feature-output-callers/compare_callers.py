#!/usr/bin/env python3
"""Compare live clause-feature print callers across every output family."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def wsl_path(path: Path) -> str:
    absolute = path.resolve().as_posix()
    if len(absolute) < 3 or absolute[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {absolute}")
    return f"/mnt/{absolute[0].lower()}{absolute[2:]}"


def run(command: list[str], stdin: bytes | None = None) -> subprocess.CompletedProcess[bytes]:
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
    parser.add_argument("--c-eprover", required=True)
    parser.add_argument("--c-epclanalyse", required=True)
    parser.add_argument("--rust-eprover", required=True, type=Path)
    parser.add_argument("--rust-epclanalyse", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    directory = Path(__file__).resolve().parent
    saturated = directory / "saturated.lop"
    protocol = (directory / "protocol.pcl").read_bytes()
    common = [
        "--lop-in",
        "--no-generation",
        "--processed-clauses-limit=0",
        "--print-saturated=eigEIG",
        "--print-sat-info",
    ]
    cases: list[tuple[str, list[str], list[str], bytes | None]] = []
    for name, format_args in (
        ("eprover_lop", []),
        ("eprover_tptp", ["--tptp-out"]),
        ("eprover_tstp", ["--tstp-out"]),
    ):
        cases.append(
            (
                name,
                [str(args.rust_eprover.resolve()), *common, *format_args, str(saturated)],
                [
                    "wsl.exe",
                    "-d",
                    "Ubuntu-24.04",
                    "--exec",
                    args.c_eprover,
                    *common,
                    *format_args,
                    wsl_path(saturated),
                ],
                None,
            )
        )
    cases.append(
        (
            "epclanalyse_pcl",
            [str(args.rust_epclanalyse.resolve())],
            [
                "wsl.exe",
                "-d",
                "Ubuntu-24.04",
                "--exec",
                args.c_epclanalyse,
            ],
            protocol,
        )
    )

    comparisons = []
    for name, rust_command, c_command, stdin in cases:
        c_process = run(c_command, stdin)
        rust_process = run(rust_command, stdin)
        exact = (
            c_process.returncode == rust_process.returncode
            and c_process.stdout == rust_process.stdout
            and c_process.stderr == rust_process.stderr
        )
        comparison: dict[str, Any] = {
            "case": name,
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
        print("clause-feature caller comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("clause-feature caller reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
