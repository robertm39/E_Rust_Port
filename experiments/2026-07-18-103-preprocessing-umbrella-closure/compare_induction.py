#!/usr/bin/env python3
"""Compare the production induction-preinstantiation effect in C and Rust."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


STAT_LABELS = (
    "Parsed axioms",
    "Initial clauses",
    "Removed in clause preprocessing",
    "Initial clauses in saturation",
    "Current number of unprocessed clauses",
    "...number of literals in the above",
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def normalize_clause(line: str) -> str:
    return re.sub(r"c_0_-?\d+", "c_0_N", line.strip())


def extract_statistic(stdout: str, label: str) -> int:
    match = re.search(
        rf"^% {re.escape(label)}\s*:\s*(-?\d+)$", stdout, re.MULTILINE
    )
    if match is None:
        raise ValueError(f"missing statistic {label!r}")
    return int(match.group(1))


def extract_result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = process.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    marker = "% CNFization successful!"
    if marker not in stdout:
        raise ValueError("missing CNF success marker")
    final_output = stdout.split(marker, maxsplit=1)[1]
    final_output = final_output.split("% Parsed axioms", maxsplit=1)[0]
    clauses = [
        normalize_clause(line)
        for line in final_output.splitlines()
        if line.startswith(("cnf(", "fof(", "thf("))
        and not line.startswith("thf(decl_")
    ]
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    if status is None:
        raise ValueError("missing SZS status")
    return {
        "exit_code": process.returncode,
        "stderr": stderr,
        "status": status,
        "clauses": clauses,
        "statistics": {label: extract_statistic(stdout, label) for label in STAT_LABELS},
        "induction_instance_present": any(
            "((g @ (f @ b))=(g @ b))" in clause for clause in clauses
        ),
    }


def run_cases(exe: str, fixture: str) -> dict[str, dict[str, Any]]:
    common = ["--cnf", "--output-level=2", "--print-statistics"]
    cases = {
        "disabled": [*common, "--preinstantiate-induction=false", fixture],
        "enabled": [*common, "--preinstantiate-induction=true", fixture],
    }
    return {
        name: extract_result(
            subprocess.run(
                [exe, *arguments],
                check=False,
                capture_output=True,
                timeout=120,
            )
        )
        for name, arguments in cases.items()
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture")
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture = Path(__file__).resolve().with_name("induction.p")
    if args.worker:
        if args.exe is None or args.fixture is None:
            parser.error("--worker requires --exe and --fixture")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.fixture), sort_keys=True))
        return 0
    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")

    rust_results = run_cases(str(args.rust_exe.resolve()), str(fixture))
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture",
            windows_to_wsl(fixture),
        ],
        check=False,
        capture_output=True,
        timeout=600,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    all_exact = c_results == rust_results
    effect_observed = (
        not rust_results["disabled"]["induction_instance_present"]
        and rust_results["enabled"]["induction_instance_present"]
        and rust_results["disabled"]["statistics"]["Initial clauses in saturation"] == 2
        and rust_results["enabled"]["statistics"]["Initial clauses in saturation"] == 3
    )
    report = {
        "schema_version": 1,
        "display_args": [
            "--cnf",
            "--output-level=2",
            "--print-statistics",
            "--preinstantiate-induction=$BOOL",
            "$FIXTURE",
        ],
        "case_count": 2,
        "c": c_results,
        "rust": rust_results,
        "all_exact": all_exact,
        "effect_observed": effect_observed,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")
    if args.expected is not None and encoded != args.expected.read_text(encoding="utf-8"):
        print("induction reference changed", file=sys.stderr)
        return 1
    if not all_exact or not effect_observed:
        print("induction comparison failed", file=sys.stderr)
        return 1
    print("validated 2/2 exact C/Rust induction-preinstantiation cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
