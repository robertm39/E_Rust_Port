#!/usr/bin/env python3
"""Compare live cleanup progress lines from C and Rust eprover."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


PREFIXES = (
    b"% Deleted ",
    b"% Special forward-contraction ",
    b"% Reweighting unprocessed clauses...",
)


def wsl_path(path: Path) -> str:
    absolute = path.resolve().as_posix()
    if len(absolute) < 3 or absolute[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {absolute}")
    return f"/mnt/{absolute[0].lower()}{absolute[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        check=False,
        capture_output=True,
        timeout=120,
    )


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()


def maintenance_lines(stdout: bytes) -> bytes:
    lines = [line for line in stdout.splitlines() if line.startswith(PREFIXES)]
    return b"\n".join(lines) + (b"\n" if lines else b"")


def summary(process: subprocess.CompletedProcess[bytes], maintenance: bytes) -> dict[str, Any]:
    return {
        "exit_code": process.returncode,
        "stdout_bytes": len(process.stdout),
        "stdout_sha256": digest(process.stdout),
        "stderr_bytes": len(process.stderr),
        "stderr_sha256": digest(process.stderr),
        "maintenance_bytes": len(maintenance),
        "maintenance_sha256": digest(maintenance),
        "maintenance_text": maintenance.decode("utf-8", errors="backslashreplace"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-eprover", required=True)
    parser.add_argument("--rust-eprover", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    problem = Path(__file__).resolve().parent / "cleanup_order.p"
    common = [
        "--tstp-in",
        "--output-level=1",
        "--no-generation",
        "--forward-contract-limit=0",
    ]
    c_process = run(
        [
            "wsl.exe",
            "-d",
            "Ubuntu-24.04",
            "--exec",
            args.c_eprover,
            *common,
            wsl_path(problem),
        ]
    )
    rust_process = run([str(args.rust_eprover.resolve()), *common, str(problem)])
    c_maintenance = maintenance_lines(c_process.stdout)
    rust_maintenance = maintenance_lines(rust_process.stdout)
    maintenance_exact = c_maintenance == rust_maintenance and bool(c_maintenance)
    process_exact = (
        c_process.returncode == rust_process.returncode
        and c_process.stdout == rust_process.stdout
        and c_process.stderr == rust_process.stderr
    )
    report = {
        "schema_version": 1,
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "maintenance_exact": maintenance_exact,
        "process_exact": process_exact,
        "c": summary(c_process, c_maintenance),
        "rust": summary(rust_process, rust_maintenance),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not maintenance_exact or not process_exact:
        print("cleanup maintenance output comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("cleanup maintenance output reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
