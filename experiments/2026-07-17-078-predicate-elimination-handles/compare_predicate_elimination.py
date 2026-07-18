#!/usr/bin/env python3
"""Compare C, Rust-internal, and Rust runtime-PicoSAT gate elimination."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


PICOSAT_ENV = "E_RUST_PORT_PICOSAT_LIBRARY"
STAT_LABELS = (
    "Parsed axioms",
    "Initial clauses",
    "Initial clauses in saturation",
    "Processed clauses",
    "Current number of archived clauses",
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(
    command: list[str], env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        check=False,
        capture_output=True,
        env=env,
        timeout=120,
    )


def compile_mock(rustc: str, source: Path, output: Path) -> None:
    process = run(
        [
            rustc,
            "--edition",
            "2021",
            "--crate-type",
            "cdylib",
            str(source.resolve()),
            "-o",
            str(output.resolve()),
        ]
    )
    if process.returncode != 0:
        sys.stderr.buffer.write(process.stderr)
        raise RuntimeError("could not compile the PicoSAT ABI fixture")


def extract_result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8")
    pe_lines = [line for line in stdout.splitlines() if line.startswith("% PE ")]
    clauses = []
    for line in stdout.splitlines():
        match = re.fullmatch(r"%cnf\([^,]+, plain, (.+)\)\.", line)
        if match is not None:
            clauses.append(match.group(1))
    statistics = {}
    for label in STAT_LABELS:
        match = re.search(rf"^% {re.escape(label)}\s*:\s*(-?\d+)$", stdout, re.MULTILINE)
        if match is None:
            raise ValueError(f"missing statistic {label!r}")
        statistics[label] = int(match.group(1))
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    if status is None:
        raise ValueError("missing SZS status")
    return {
        "clauses_after_preprocessing": clauses,
        "exit_code": process.returncode,
        "pe_lines": pe_lines,
        "statistics": statistics,
        "status": status,
        "stderr": process.stderr.decode("utf-8"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--mock-library", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parent
    fixture = fixture_dir / "gate.p"
    compile_mock(args.rustc, fixture_dir / "mock_picosat.rs", args.mock_library)

    common_args = [
        "--pred-elim=true",
        "--pred-elim-recognize-gates=true",
        "--print-statistics",
    ]
    c_process = run(
        [
            "wsl.exe",
            "-d",
            "Ubuntu-24.04",
            "--exec",
            args.c_exe,
            *common_args,
            windows_to_wsl(fixture),
        ]
    )

    internal_env = os.environ.copy()
    internal_env.pop(PICOSAT_ENV, None)
    rust_internal = run(
        [str(args.rust_exe.resolve()), *common_args, str(fixture.resolve())],
        internal_env,
    )
    runtime_env = internal_env.copy()
    runtime_env[PICOSAT_ENV] = str(args.mock_library.resolve())
    rust_runtime = run(
        [str(args.rust_exe.resolve()), *common_args, str(fixture.resolve())],
        runtime_env,
    )

    results = {
        "c": extract_result(c_process),
        "rust_internal": extract_result(rust_internal),
        "rust_runtime_picosat": extract_result(rust_runtime),
    }
    all_exact = results["c"] == results["rust_internal"] == results["rust_runtime_picosat"]
    report = {
        "schema_version": 1,
        "display_args": [*common_args, "$FIXTURE"],
        "results": results,
        "all_exact": all_exact,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not all_exact:
        print("predicate-elimination comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("predicate-elimination reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
