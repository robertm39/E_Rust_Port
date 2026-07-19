#!/usr/bin/env python3
"""Compare nested selector and repeated-include behavior with unchanged C."""

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


def wsl_path(path: Path, distro: str) -> str:
    completed = run(
        ["wsl.exe", "-d", distro, "--exec", "wslpath", "-a", "-u", str(path)]
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr)
    return completed.stdout.strip()


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
    fixture = Path(__file__).resolve().parent / "fixtures" / "main.p"
    c_completed = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_binary,
            "--print-formulas",
            "--tstp-in",
            wsl_path(fixture, args.distro),
        ]
    )
    rust_completed = run(
        [str(rust_binary), "--print-formulas", "--tstp-in", str(fixture)]
    )
    c_record = process_record(c_completed)
    rust_record = process_record(rust_completed)
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "unchanged C under WSL 2; Rust on Windows",
            "c_binary_sha256": wsl_sha256(args.c_binary, args.distro),
            "c_formula_parser_sha256": sha256(
                c_repo / "CLAUSES/ccl_formulafunc.c"
            ),
            "rust_scanner_sha256": sha256(repo / "src/inout/scanner.rs"),
            "rust_general_parser_sha256": sha256(repo / "src/prover/eprover.rs"),
            "rust_batch_parser_sha256": sha256(repo / "src/control/batch_spec.rs"),
            "fixture_sha256": sha256(fixture),
        },
        "case": "nested inner/outer selectors plus repeated include",
        "c": c_record,
        "rust": rust_record,
        "accepted": c_record == rust_record and c_record["exit_code"] == 0,
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
            print(f"nested include mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"nested include comparison: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
