#!/usr/bin/env python3
"""Compare executable strategy-parser warnings between C and Rust E."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    suffix = resolved.as_posix()[2:]
    return f"/mnt/{drive[0].lower()}{suffix}"


def run(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        timeout=120,
    )
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", errors="replace"),
        "stderr": completed.stderr.decode("utf-8", errors="replace"),
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
    }


def make_partial_strategy(rust_exe: Path, destination: Path) -> str:
    printed = run([str(rust_exe), "--print-strategy"])
    if printed["exit_code"] != 0 or printed["stderr"]:
        raise RuntimeError(f"could not print the Rust default strategy: {printed}")

    strategy_start = printed["stdout"].find("{\n")
    if strategy_start < 0:
        raise RuntimeError("Rust default strategy has no opening brace")
    strategy = printed["stdout"][strategy_start:]

    removed = {"ordertype": 0, "db_w": 0}
    retained: list[str] = []
    for line in strategy.splitlines(keepends=True):
        field = line.lstrip().split(":", maxsplit=1)[0]
        if field in removed:
            removed[field] += 1
        else:
            retained.append(line)
    if removed != {"ordertype": 1, "db_w": 1}:
        raise RuntimeError(f"unexpected removed-field counts: {removed}")

    partial = "".join(retained)
    destination.write_text(partial, encoding="utf-8", newline="\n")
    return partial


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True, help="Linux path to isolated C eprover")
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    rust_exe = args.rust_exe.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    strategy_path = output_dir / "missing-order-fields.strategy"
    problem_path = output_dir / "problem.lop"
    partial_strategy = make_partial_strategy(rust_exe, strategy_path)
    problem_path.write_text("p(a).\n", encoding="utf-8", newline="\n")

    c_prefix = ["wsl.exe", "-d", args.distro, "--exec", args.c_exe]
    c_strategy = windows_to_wsl(strategy_path)
    c_problem = windows_to_wsl(problem_path)

    cases = {
        "proof_search": {
            "display_args": [
                "--output-level=0",
                "--lop-in",
                "--parse-strategy=$STRATEGY",
                "$PROBLEM",
            ],
            "c": run(
                c_prefix
                + [
                    "--output-level=0",
                    "--lop-in",
                    f"--parse-strategy={c_strategy}",
                    c_problem,
                ]
            ),
            "rust": run(
                [
                    str(rust_exe),
                    "--output-level=0",
                    "--lop-in",
                    f"--parse-strategy={strategy_path}",
                    str(problem_path),
                ]
            ),
        },
        "selection_error": {
            "display_args": [
                "--parse-strategy=$STRATEGY",
                "--select-strategy=Missing",
                "--print-strategy=>current-strategy<",
            ],
            "c": run(
                c_prefix
                + [
                    f"--parse-strategy={c_strategy}",
                    "--select-strategy=Missing",
                    "--print-strategy=>current-strategy<",
                ]
            ),
            "rust": run(
                [
                    str(rust_exe),
                    f"--parse-strategy={strategy_path}",
                    "--select-strategy=Missing",
                    "--print-strategy=>current-strategy<",
                ]
            ),
        },
    }

    all_exact = True
    for case in cases.values():
        case["exact_match"] = case["c"] == case["rust"]
        all_exact = all_exact and case["exact_match"]

    report = {
        "schema_version": 1,
        "fixture_sha256": sha256_bytes(partial_strategy.encode("utf-8")),
        "cases": cases,
        "all_exact": all_exact,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("report does not match retained reference", file=sys.stderr)
            return 1
    if not all_exact:
        print("C and Rust executable results differ", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
