#!/usr/bin/env python3
"""Compare C/Rust classifier equality-definition caller boundaries."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


FIXTURES = ("cnf-eqdef.p", "fof-eqdef.p")


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def classification(stdout: str) -> str:
    lines = stdout.splitlines()
    if len(lines) != 1 or " : " not in lines[0]:
        raise ValueError(f"expected one classifier result line, got {stdout!r}")
    return lines[0].split(" : ", 1)[1]


def result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    return {
        "classification": classification(stdout),
        "exit_code": process.returncode,
        "stderr": process.stderr.decode("utf-8"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parent
    cases: dict[str, Any] = {}
    all_exact = True
    for fixture_name in FIXTURES:
        fixture = fixture_dir / fixture_name
        c_process = run(
            [
                "wsl.exe",
                "-d",
                "Ubuntu-24.04",
                "--exec",
                args.c_exe,
                "--tstp-format",
                windows_to_wsl(fixture),
            ]
        )
        rust_process = run(
            [str(args.rust_exe.resolve()), "--tstp-format", str(fixture.resolve())]
        )
        c_result = result(c_process)
        rust_result = result(rust_process)
        exact = c_result == rust_result
        all_exact = all_exact and exact
        cases[fixture.stem] = {
            "c": c_result,
            "exact": exact,
            "rust": rust_result,
        }

    report = {
        "schema_version": 1,
        "display_args": ["--tstp-format", "$FIXTURE"],
        "cases": cases,
        "all_exact": all_exact,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not all_exact:
        print("classifier equality-definition comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("classifier equality-definition reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
