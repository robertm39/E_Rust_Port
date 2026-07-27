#!/usr/bin/env python3
"""Compare main eprover stdout, stderr, and configured-output ownership."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


DIRECT_HARD_TIMEOUT = (
    b"\n%% Failure: Resource limit exceeded (time)\n"
    b"%% SZS status ResourceOut\n"
)
HARD_TIMEOUT_STDERR = "eprover: CPU time limit exceeded, terminating\n"

SAT_INPUT = b"p(a).\n"
PROOF_INPUT = b"a!=a.\n"
AC_INPUT = b"f(f(X,Y),Z)=f(X,f(Y,Z)).\nf(X,Y)=f(Y,X).\n"
WATCHLIST_INPUT = (
    b"tcf(watch, watchlist, p(a)).\n"
    b"cnf(input, axiom, p(a)).\n"
)
MALFORMED_INPUT = b"p(.\n"

EXACT_CASES = (
    ("satisfiable", ["--lop-in"], SAT_INPUT),
    ("proof_found", ["--lop-in"], PROOF_INPUT),
    ("ac_cnf", ["--lop-in", "--cnf"], AC_INPUT),
    ("statistics", ["--lop-in", "--print-statistics"], SAT_INPUT),
    (
        "proof_object",
        ["--lop-in", "--tstp-out", "--proof-object=1"],
        PROOF_INPUT,
    ),
    (
        "inline_watchlist",
        [
            "--tstp-in",
            "--no-generation",
            "--watchlist=Use inline watchlist type",
        ],
        WATCHLIST_INPUT,
    ),
    ("malformed_input", ["--lop-in"], MALFORMED_INPUT),
    ("print_info", ["--lop-in", "--print-version"], SAT_INPUT),
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str], input_bytes: bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        check=False,
        capture_output=True,
        input=input_bytes,
        timeout=120,
    )


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def captured_result(
    process: subprocess.CompletedProcess[bytes], output_path: Path
) -> dict[str, Any]:
    output_created = output_path.is_file()
    configured_output = output_path.read_bytes() if output_created else b""
    return {
        "exit_code": process.returncode,
        "stdout_bytes": len(process.stdout),
        "stdout_sha256": sha256(process.stdout),
        "stderr_bytes": len(process.stderr),
        "stderr_sha256": sha256(process.stderr),
        "configured_output_created": output_created,
        "configured_output_bytes": len(configured_output),
        "configured_output_sha256": sha256(configured_output),
    }


def output_paths(base: Path, name: str) -> tuple[Path, Path]:
    parent = base.resolve().parent
    return (
        parent / f"output-routing-{name}-c.out",
        parent / f"output-routing-{name}-rust.out",
    )


def run_pair(
    args: argparse.Namespace,
    name: str,
    common_args: list[str],
    input_bytes: bytes,
) -> tuple[
    subprocess.CompletedProcess[bytes],
    Path,
    subprocess.CompletedProcess[bytes],
    Path,
]:
    c_output, rust_output = output_paths(args.output, name)
    c_output.unlink(missing_ok=True)
    rust_output.unlink(missing_ok=True)
    c_process = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *common_args,
            f"--output-file={windows_to_wsl(c_output)}",
        ],
        input_bytes,
    )
    rust_process = run(
        [
            str(args.rust_exe.resolve()),
            *common_args,
            f"--output-file={rust_output.resolve()}",
        ],
        input_bytes,
    )
    return c_process, c_output, rust_process, rust_output


def normalize_resource_values(output: bytes) -> bytes:
    normalized = re.sub(
        rb"(?m)^(% Preprocessing time\s+:) [0-9]+\.[0-9]{3} s$",
        rb"\1 <seconds>",
        output,
    )
    normalized = re.sub(
        rb"(?m)^(% (?:User|System|Total) time\s+:) [0-9]+\.[0-9]{3} s$",
        rb"\1 <seconds>",
        normalized,
    )
    return re.sub(
        rb"(?m)^(% Maximum resident set size:) [0-9]+ pages$",
        rb"\1 <host-value>",
        normalized,
    )


def normalized_resource_result(
    process: subprocess.CompletedProcess[bytes], output_path: Path
) -> dict[str, Any]:
    configured_output = normalize_resource_values(output_path.read_bytes())
    return {
        "exit_code": process.returncode,
        "stdout_bytes": len(process.stdout),
        "stdout_sha256": sha256(process.stdout),
        "stderr": process.stderr.decode("utf-8"),
        "normalized_output_bytes": len(configured_output),
        "normalized_output_sha256": sha256(configured_output),
        "preprocessing_time_normalized": (
            b"% Preprocessing time       : <seconds>" in configured_output
        ),
        "resource_time_count": len(
            re.findall(
                rb"(?m)^% (?:User|System|Total) time\s+: <seconds>$",
                configured_output,
            )
        ),
        "resident_value_normalized": (
            b"% Maximum resident set size: <host-value>" in configured_output
        ),
    }


def hard_output_projection(
    process: subprocess.CompletedProcess[bytes], output_path: Path
) -> dict[str, Any]:
    configured_output = output_path.read_bytes() if output_path.is_file() else b""
    return {
        "exit_code": process.returncode,
        "configured_output_created": output_path.is_file(),
        "direct_banner_count": configured_output.count(DIRECT_HARD_TIMEOUT),
        "contains_soft_failure": b"User resource limit exceeded" in configured_output,
        "stderr": process.stderr.decode("utf-8"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    exact_reports = []
    for name, common_args, input_bytes in EXACT_CASES:
        c_process, c_output, rust_process, rust_output = run_pair(
            args, name, common_args, input_bytes
        )
        c_result = captured_result(c_process, c_output)
        rust_result = captured_result(rust_process, rust_output)
        exact_reports.append(
            {
                "name": name,
                "display_args": [*common_args, "--output-file=$OUTPUT", "< $STDIN"],
                "c": c_result,
                "rust": rust_result,
                "all_exact": c_result == rust_result,
            }
        )

    resource_args = ["--lop-in", "--resources-info"]
    c_resources, c_resource_output, rust_resources, rust_resource_output = run_pair(
        args,
        "resources_info",
        resource_args,
        SAT_INPUT,
    )
    c_resource_result = normalized_resource_result(c_resources, c_resource_output)
    rust_resource_result = normalized_resource_result(
        rust_resources, rust_resource_output
    )
    resource_report = {
        "name": "resources_info",
        "display_args": [*resource_args, "--output-file=$OUTPUT", "< $STDIN"],
        "normalization": "preprocessing/resource seconds and target-dependent resident value",
        "c": c_resource_result,
        "rust": rust_resource_result,
        "all_exact": c_resource_result == rust_resource_result,
    }

    hard_input = (
        Path(__file__).resolve().parents[2]
        / "eprover"
        / "EXAMPLE_PROBLEMS"
        / "SMOKETEST"
        / "LUSK6.lop"
    ).read_bytes()
    hard_args = ["--lop-in", "--auto", "--cpu-limit=0", "--memory-limit=0"]
    c_hard, c_hard_output, rust_hard, rust_hard_output = run_pair(
        args,
        "hard_timeout",
        hard_args,
        hard_input,
    )
    c_hard_result = hard_output_projection(c_hard, c_hard_output)
    rust_hard_result = hard_output_projection(rust_hard, rust_hard_output)
    hard_report = {
        "name": "hard_timeout",
        "display_args": [*hard_args, "--output-file=$OUTPUT", "< $LUSK6_STDIN"],
        "projection": "asynchronous/cooperative stop phase and stdout side-channel prefix excluded",
        "c": c_hard_result,
        "rust": rust_hard_result,
        "all_exact": (
            c_hard_result == rust_hard_result
            and c_hard_result["exit_code"] == 8
            and c_hard_result["configured_output_created"]
            and c_hard_result["direct_banner_count"] == 1
            and not c_hard_result["contains_soft_failure"]
            and c_hard_result["stderr"] == HARD_TIMEOUT_STDERR
        ),
    }

    report = {
        "schema_version": 1,
        "upstream_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "exact_case_count": len(exact_reports),
        "exact_matching_cases": sum(case["all_exact"] for case in exact_reports),
        "exact_cases": exact_reports,
        "normalized_case": resource_report,
        "projected_case": hard_report,
        "all_exact": (
            all(case["all_exact"] for case in exact_reports)
            and resource_report["all_exact"]
            and hard_report["all_exact"]
        ),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not report["all_exact"]:
        print("main output-routing comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("main output-routing reference changed", file=sys.stderr)
            return 1
    print(
        f"matched {report['exact_matching_cases']}/{report['exact_case_count']} "
        "exact cases plus normalized resource and projected hard-timeout cases"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
