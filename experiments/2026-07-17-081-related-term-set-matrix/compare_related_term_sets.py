#!/usr/bin/env python3
"""Compare all conjecture related-term modes across all six C/Rust consumers."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


FAMILIES = (
    (
        "relative",
        "ConjectureRelativeTermWeight(ConstPrio,0,{rel},2.0,10,3,20,1,0,1.0,1.0,1.0)",
    ),
    (
        "prefix",
        "ConjectureTermPrefixWeight(ConstPrio,0,{rel},0.5,5.0,0,1.0,1.0,1.0)",
    ),
    (
        "tfidf",
        "ConjectureTermTfIdfWeight(ConstPrio,0,{rel},0,1.0,0,1.0,1.0,1.0)",
    ),
    (
        "levenshtein",
        "ConjectureLevDistanceWeight(ConstPrio,0,{rel},1,1,5,0,1.0,1.0,1.0)",
    ),
    (
        "tree",
        "ConjectureTreeDistanceWeight(ConstPrio,0,{rel},1,1,5,0,1.0,1.0,1.0)",
    ),
    (
        "structural",
        "ConjectureStrucDistanceWeight(ConstPrio,0,{rel},5.0,10.0,2.0,3.0,0,1.0,1.0,1.0)",
    ),
)
RELATED_TERM_SETS = (
    "conjecture_terms",
    "conjecture_subterms",
    "conjecture_subterms_top_gens",
    "conjecture_subterms_all_gens",
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


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
    parser.add_argument("--c-exe", required=True)
    parser.add_argument(
        "--c-variant",
        choices=("stock", "tfidf_factor_initialized"),
        default="stock",
    )
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    fixture = Path(__file__).resolve().parent / "problem.p"
    common_args = ["--processed-clauses-limit=1", "--output-level=1"]
    comparisons = []
    for rel, related_name in enumerate(RELATED_TERM_SETS):
        for family_name, template in FAMILIES:
            definition = template.format(rel=rel)
            heuristic_arg = f"--expert-heuristic=(1*{definition})"
            case_args = [heuristic_arg, *common_args]
            c_process = run(
                [
                    "wsl.exe",
                    "-d",
                    "Ubuntu-24.04",
                    "--exec",
                    args.c_exe,
                    *case_args,
                    windows_to_wsl(fixture),
                ]
            )
            rust_process = run(
                [str(args.rust_exe.resolve()), *case_args, str(fixture.resolve())]
            )
            exact = (
                c_process.returncode == rust_process.returncode
                and c_process.stdout == rust_process.stdout
                and c_process.stderr == rust_process.stderr
            )
            comparison: dict[str, Any] = {
                "case": f"{family_name}:{related_name}",
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
        "c_variant": args.c_variant,
        "display_args": ["--expert-heuristic=$DEFINITION", *common_args, "$FIXTURE"],
        "case_count": len(comparisons),
        "exact_count": exact_count,
        "comparisons": comparisons,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if exact_count != len(comparisons):
        print("related-term-set comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("related-term-set reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
