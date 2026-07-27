#!/usr/bin/env python3
"""Compare a live comparison-cache workload against unchanged C."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
FIXTURE = Path("eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop")
OPTIONS = ["--silent"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def run(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
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


def wsl_path(path: Path, distro: str) -> str:
    return checked_stdout(
        ["wsl.exe", "-d", distro, "--exec", "wslpath", "-a", str(path.resolve())]
    ).strip()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    fixture = repo / FIXTURE
    rust_exe = args.rust_exe.resolve()
    c_result = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *OPTIONS,
            wsl_path(fixture, args.distro),
        ]
    )
    rust_result = run([str(rust_exe), *OPTIONS, str(fixture.resolve())])
    c_hash = checked_stdout(
        ["wsl.exe", "-d", args.distro, "--exec", "sha256sum", args.c_exe]
    ).split()[0]
    exact = c_result == rust_result
    return {
        "schema_version": 1,
        "reference": {
            "commit": REFERENCE_COMMIT,
            "executable_sha256": c_hash,
            "platform": "Linux under WSL 2",
        },
        "rust": {
            "executable_sha256": sha256(rust_exe),
            "platform": "native Windows",
        },
        "workload": {
            "fixture": FIXTURE.as_posix(),
            "fixture_sha256": sha256(fixture),
            "options": OPTIONS,
        },
        "c": c_result,
        "rust_result": rust_result,
        "exact": exact,
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo, args)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if rendered != expected:
            print(f"comparison mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"comparison-cache workload: exact={result['exact']}")
    return 0 if result["exact"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
