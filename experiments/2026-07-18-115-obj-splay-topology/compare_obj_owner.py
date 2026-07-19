#!/usr/bin/env python3
"""Compare a fingerprint-index ObjTree workload against unchanged C."""

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
OPTIONS = ["--auto", "--silent"]


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
    stdout = result["stdout"]
    return {
        "exit_code": result["exit_code"],
        "stderr": result["stderr"],
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stdout_sha256": sha256_bytes(stdout.encode("utf-8")),
        "proof_found": "% Proof found!" in stdout,
        "theorem_status": "% SZS status Theorem" in stdout,
    }


def first_difference(c_stdout: str, rust_stdout: str) -> dict[str, Any] | None:
    c_lines = c_stdout.splitlines()
    rust_lines = rust_stdout.splitlines()
    for index in range(max(len(c_lines), len(rust_lines))):
        c_line = c_lines[index] if index < len(c_lines) else None
        rust_line = rust_lines[index] if index < len(rust_lines) else None
        if c_line != rust_line:
            return {"line": index + 1, "c": c_line, "rust": rust_line}
    return None


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
    exact = c_raw == rust_raw
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
            "input_transport": "stdin",
            "options": OPTIONS,
            "owner": "fingerprint-index leaf ObjTree payloads",
        },
        "c": summarize(c_raw),
        "rust_result": summarize(rust_raw),
        "first_difference": first_difference(c_raw["stdout"], rust_raw["stdout"]),
        "exact": exact,
    }


def main() -> int:
    args = parse_args()
    result = collect(args)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if rendered != expected:
            print(f"owner comparison mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"object owner comparison: exact={result['exact']}")
    return 0 if result["exact"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
