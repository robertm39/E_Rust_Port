#!/usr/bin/env python3
"""Compare the shared Rust top-level fatal path with unchanged C eprover."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
DEFAULT_C_BINARY = (
    "/home/rober/.cache/e-rust-port/bin/"
    f"{REFERENCE_COMMIT}/fol/eprover"
)
INVALID_OPTION = "--definitely-invalid-option"
EXPECTED_STDERR = (
    "eprover: Unknown Option: --definitely-invalid-option "
    "(Use -h for a list of valid options)\n"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-binary", default=DEFAULT_C_BINARY)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument(
        "--rust-binary", type=Path, default=Path("target/release/eprover.exe")
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def run(command: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wsl_sha256(path: str, distro: str) -> str:
    completed = run(["wsl.exe", "-d", distro, "--exec", "sha256sum", path])
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr)
    return completed.stdout.split()[0]


def process_record(completed: subprocess.CompletedProcess[str]) -> dict[str, int | str]:
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    c_repo = repo / "eprover"
    commit = run(["git", "rev-parse", "HEAD"], cwd=c_repo).stdout.strip()
    if commit != REFERENCE_COMMIT:
        raise RuntimeError(f"expected C commit {REFERENCE_COMMIT}, found {commit}")

    rust_binary = (repo / args.rust_binary).resolve()
    if not rust_binary.is_file():
        raise RuntimeError(f"Rust release binary not found: {rust_binary}")
    c_completed = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_binary,
            INVALID_OPTION,
        ]
    )
    rust_completed = run([str(rust_binary), INVALID_OPTION])
    c_record = process_record(c_completed)
    rust_record = process_record(rust_completed)
    expected_record = {
        "exit_code": 5,
        "stdout": "",
        "stderr": EXPECTED_STDERR,
    }
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "unchanged C under WSL 2; Rust on Windows",
            "c_binary_sha256": wsl_sha256(args.c_binary, args.distro),
            "clb_error_h_sha256": sha256(c_repo / "BASICS/clb_error.h"),
            "clb_error_c_sha256": sha256(c_repo / "BASICS/clb_error.c"),
            "c_eprover_sha256": sha256(c_repo / "PROVER/eprover.c"),
            "rust_error_sha256": sha256(repo / "src/basics/error.rs"),
            "rust_eprover_entry_sha256": sha256(repo / "src/bin/eprover.rs"),
            "rust_runtime_test_sha256": sha256(
                repo / "tests/executable_diagnostics.rs"
            ),
        },
        "invalid_option": INVALID_OPTION,
        "c": c_record,
        "rust": rust_record,
        "accepted": c_record == expected_record and rust_record == expected_record,
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
            print(f"fatal diagnostic mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"top-level fatal diagnostic comparison: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
