#!/usr/bin/env python3
"""Compare focused LFHO inference traces under all six release orderings."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
ORDERINGS = ("KBO", "KBO6", "LPO", "LPOCopy", "LPO4", "LPO4Copy")
FIXTURES = ("eta-wrapper.p", "flex-flex.p", "rigid-prefix.p")
COUNTER_PREFIXES = (
    b"% Processed clauses",
    b"% Generated clauses",
    b"% Paramodulations",
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


def joined(lines: list[bytes]) -> bytes:
    return b"\n".join(lines) + (b"\n" if lines else b"")


def inference_trace(stdout: bytes) -> bytes:
    lines = [line for line in stdout.splitlines() if b"inference(" in line]
    return joined([re.sub(rb"c_0_[0-9]+", b"c_0_N", line) for line in lines])


def counters(stdout: bytes) -> bytes:
    return joined(
        [line for line in stdout.splitlines() if line.startswith(COUNTER_PREFIXES)]
    )


def summarized(data: bytes) -> dict[str, Any]:
    return {"bytes": len(data), "sha256": digest(data)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-eprover", required=True)
    parser.add_argument("--rust-eprover", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    fixtures = Path(__file__).resolve().parents[1] / "2026-07-15-003-lfho-paramod-direct-mgu"
    common = [
        "--literal-selection-strategy=NoSelection",
        "--pm-from-index=NoIndex",
        "--pm-into-index=NoIndex",
        "--processed-clauses-limit=2",
        "--output-level=2",
        "--print-statistics",
    ]
    case_names: list[str] = []
    failures: list[dict[str, Any]] = []
    combined_c = bytearray()
    combined_rust = bytearray()
    for fixture_name in FIXTURES:
        fixture = fixtures / fixture_name
        for ordering in ORDERINGS:
            options = [f"--term-ordering={ordering}", *common]
            c_process = run(
                [
                    "wsl.exe",
                    "-d",
                    "Ubuntu-24.04",
                    "--exec",
                    args.c_eprover,
                    *options,
                    wsl_path(fixture),
                ]
            )
            rust_process = run(
                [str(args.rust_eprover.resolve()), *options, str(fixture)]
            )
            c_trace = inference_trace(c_process.stdout)
            rust_trace = inference_trace(rust_process.stdout)
            c_counters = counters(c_process.stdout)
            rust_counters = counters(rust_process.stdout)
            exact = (
                c_process.returncode == 9
                and rust_process.returncode == 9
                and c_process.stderr == rust_process.stderr
                and c_trace == rust_trace
                and c_counters == rust_counters
                and b"% Paramodulations" in rust_counters
                and not re.search(rb"% Paramodulations\s*:\s*0\s*$", rust_counters, re.MULTILINE)
            )
            case_name = f"{fixture.stem}/{ordering}"
            case_names.append(case_name)
            for combined, process, trace, case_counters in (
                (combined_c, c_process, c_trace, c_counters),
                (combined_rust, rust_process, rust_trace, rust_counters),
            ):
                combined.extend(case_name.encode("utf-8"))
                combined.extend(b"\0")
                combined.extend(str(process.returncode).encode("ascii"))
                combined.extend(b"\0")
                combined.extend(process.stderr)
                combined.extend(b"\0")
                combined.extend(trace)
                combined.extend(b"\0")
                combined.extend(case_counters)
                combined.extend(b"\0")
            if not exact:
                failures.append(
                    {
                        "case": case_name,
                        "rust": {
                            "exit_code": rust_process.returncode,
                            "stderr": summarized(rust_process.stderr),
                            "inference_trace": summarized(rust_trace),
                            "counters": summarized(rust_counters),
                        },
                        "c": {
                            "exit_code": c_process.returncode,
                            "stderr": summarized(c_process.stderr),
                            "inference_trace": summarized(c_trace),
                            "counters": summarized(c_counters),
                        },
                    }
                )

    exact_count = len(case_names) - len(failures)
    report = {
        "schema_version": 1,
        "reference_commit": REFERENCE_COMMIT,
        "case_count": len(case_names),
        "exact_count": exact_count,
        "cases": case_names,
        "combined": summarized(bytes(combined_rust)),
        "failures": failures,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if exact_count != len(case_names) or combined_c != combined_rust:
        print("higher-order ordering comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("higher-order ordering reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
