#!/usr/bin/env python3
"""Compare deterministic and platform-projected eprover resource-limit output."""

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
SOFT_TIMEOUT = (
    b"\n% Failure: User resource limit exceeded!\n"
    b"% SZS status ResourceOut\n"
)
LIMIT_ORDER_ERROR = "eprover: Soft time limit has to be smaller than hardtime limit\n"


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def exact_result(process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    return {
        "exit_code": process.returncode,
        "stdout_bytes": len(process.stdout),
        "stdout_sha256": sha256(process.stdout),
        "stderr_bytes": len(process.stderr),
        "stderr_sha256": sha256(process.stderr),
    }


def normalize_resources(output: bytes) -> bytes:
    normalized = re.sub(
        rb"(?m)^(% (?:User|System|Total) time\s+:) [0-9]+\.[0-9]{3} s$",
        rb"\1 <seconds>",
        output,
    )
    return re.sub(
        rb"(?m)^(% Maximum resident set size:) [0-9]+ pages$",
        rb"\1 <host-value>",
        normalized,
    )


def normalized_resource_result(
    process: subprocess.CompletedProcess[bytes],
) -> dict[str, Any]:
    normalized = normalize_resources(process.stdout)
    return {
        "exit_code": process.returncode,
        "normalized_stdout_bytes": len(normalized),
        "normalized_stdout_sha256": sha256(normalized),
        "resource_line_count": len(
            re.findall(
                rb"(?m)^% (?:User|System|Total) time\s+: <seconds>$",
                normalized,
            )
        ),
        "resident_line_normalized": (
            b"% Maximum resident set size: <host-value>" in normalized
        ),
        "stderr": process.stderr.decode("utf-8"),
    }


def hard_timeout_projection(
    process: subprocess.CompletedProcess[bytes],
) -> dict[str, Any]:
    return {
        "exit_code": process.returncode,
        "direct_banner_is_prefix": process.stdout.startswith(DIRECT_HARD_TIMEOUT),
        "direct_banner_count": process.stdout.count(DIRECT_HARD_TIMEOUT),
        "contains_soft_failure": b"User resource limit exceeded" in process.stdout,
        "stderr": process.stderr.decode("utf-8"),
    }


def soft_timeout_projection(
    process: subprocess.CompletedProcess[bytes],
) -> dict[str, Any]:
    return {
        "exit_code": process.returncode,
        "soft_banner_is_suffix": process.stdout.endswith(SOFT_TIMEOUT),
        "soft_banner_count": process.stdout.count(SOFT_TIMEOUT),
        "contains_hard_failure": b"Failure: Resource limit exceeded (time)"
        in process.stdout,
        "stderr": process.stderr.decode("utf-8"),
    }


def auto_memory_projection(
    process: subprocess.CompletedProcess[bytes],
) -> dict[str, Any]:
    stderr = process.stderr.decode("utf-8")
    match = re.fullmatch(
        r"Physical memory determined as (-?\d+) MB\n"
        r"Memory limit set to (-?\d+) MB\n"
        r"eprover: Soft time limit has to be smaller than hardtime limit\n",
        stderr,
    )
    physical_mb = int(match.group(1)) if match is not None else None
    limit_mb = int(match.group(2)) if match is not None else None
    return {
        "exit_code": process.returncode,
        "two_memory_lines_before_error": match is not None,
        "limit_is_truncated_eighty_percent": (
            physical_mb is not None
            and limit_mb is not None
            and limit_mb == int(physical_mb * 0.8)
        ),
        "stderr_ends_with_limit_order_error": stderr.endswith(LIMIT_ORDER_ERROR),
        "stdout": process.stdout.decode("utf-8"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture = Path(__file__).resolve().parent / "limit.lop"
    c_fixture = windows_to_wsl(fixture)
    rust_fixture = str(fixture.resolve())
    hard_fixture = (
        Path(__file__).resolve().parents[2]
        / "eprover"
        / "EXAMPLE_PROBLEMS"
        / "SMOKETEST"
        / "LUSK6.lop"
    )

    exact_cases = (
        (
            "memory_verbose_before_error",
            [
                "--verbose=1",
                "--memory-limit=0",
                "--cpu-limit=1",
                "--soft-cpu-limit=1",
                "--lop-in",
            ],
        ),
        (
            "memory_before_verbose_error",
            [
                "--memory-limit=0",
                "--verbose=1",
                "--cpu-limit=1",
                "--soft-cpu-limit=1",
                "--lop-in",
            ],
        ),
        (
            "negative_memory_verbose_before_error",
            [
                "--verbose=1",
                "--memory-limit=-1",
                "--cpu-limit=1",
                "--soft-cpu-limit=1",
                "--lop-in",
            ],
        ),
    )
    exact_reports = []
    for name, common_args in exact_cases:
        c_process = run(
            [
                "wsl.exe",
                "-d",
                args.distro,
                "--exec",
                args.c_exe,
                *common_args,
                c_fixture,
            ]
        )
        rust_process = run(
            [str(args.rust_exe.resolve()), *common_args, rust_fixture]
        )
        c_result = exact_result(c_process)
        rust_result = exact_result(rust_process)
        exact_reports.append(
            {
                "name": name,
                "display_args": [*common_args, "$FIXTURE"],
                "c": c_result,
                "rust": rust_result,
                "all_exact": c_result == rust_result,
            }
        )

    resource_args = [
        "--syntax-only",
        "--lop-in",
        "--resources-info",
        "--memory-limit=0",
    ]
    c_resources = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *resource_args,
            c_fixture,
        ]
    )
    rust_resources = run(
        [str(args.rust_exe.resolve()), *resource_args, rust_fixture]
    )
    c_resource_result = normalized_resource_result(c_resources)
    rust_resource_result = normalized_resource_result(rust_resources)
    resource_report = {
        "name": "resources_info_host_values_normalized",
        "display_args": [*resource_args, "$FIXTURE"],
        "normalization": "three CPU seconds and target-dependent resident value",
        "c": c_resource_result,
        "rust": rust_resource_result,
        "all_exact": c_resource_result == rust_resource_result,
    }

    auto_memory_args = [
        "--verbose=1",
        "--memory-limit=Auto",
        "--cpu-limit=1",
        "--soft-cpu-limit=1",
        "--lop-in",
    ]
    c_auto_memory = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *auto_memory_args,
            c_fixture,
        ]
    )
    rust_auto_memory = run(
        [str(args.rust_exe.resolve()), *auto_memory_args, rust_fixture]
    )
    c_auto_memory_result = auto_memory_projection(c_auto_memory)
    rust_auto_memory_result = auto_memory_projection(rust_auto_memory)
    auto_memory_report = {
        "name": "auto_memory_host_projection",
        "display_args": [*auto_memory_args, "$FIXTURE"],
        "projection": "host physical-memory values excluded; line order and 80-percent derivation retained",
        "c": c_auto_memory_result,
        "rust": rust_auto_memory_result,
        "all_exact": (
            c_auto_memory_result == rust_auto_memory_result
            and c_auto_memory_result["exit_code"] == 5
            and c_auto_memory_result["two_memory_lines_before_error"]
            and c_auto_memory_result["limit_is_truncated_eighty_percent"]
            and c_auto_memory_result["stderr_ends_with_limit_order_error"]
            and c_auto_memory_result["stdout"] == ""
        ),
    }

    soft_args = [
        "--lop-in",
        "--auto",
        "--soft-cpu-limit=0",
        "--cpu-limit=1",
        "--memory-limit=0",
    ]
    c_soft = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *soft_args,
            windows_to_wsl(hard_fixture),
        ]
    )
    rust_soft = run(
        [str(args.rust_exe.resolve()), *soft_args, str(hard_fixture.resolve())]
    )
    c_soft_result = soft_timeout_projection(c_soft)
    rust_soft_result = soft_timeout_projection(rust_soft)
    soft_report = {
        "name": "soft_zero_platform_projection",
        "display_args": [*soft_args, "$LUSK6_FIXTURE"],
        "projection": (
            "asynchronous/cooperative stop phase excluded; user-resource suffix, "
            "status, stderr, and exit retained"
        ),
        "c": c_soft_result,
        "rust": rust_soft_result,
        "all_exact": (
            c_soft_result == rust_soft_result
            and c_soft_result["exit_code"] == 9
            and c_soft_result["soft_banner_is_suffix"]
            and c_soft_result["soft_banner_count"] == 1
            and not c_soft_result["contains_hard_failure"]
            and c_soft_result["stderr"] == ""
        ),
    }

    hard_args = ["--lop-in", "--auto", "--cpu-limit=0", "--memory-limit=0"]
    c_hard = run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *hard_args,
            windows_to_wsl(hard_fixture),
        ]
    )
    rust_hard = run(
        [str(args.rust_exe.resolve()), *hard_args, str(hard_fixture.resolve())]
    )
    c_hard_result = hard_timeout_projection(c_hard)
    rust_hard_result = hard_timeout_projection(rust_hard)
    hard_report = {
        "name": "hard_zero_platform_projection",
        "display_args": [*hard_args, "$LUSK6_FIXTURE"],
        "projection": (
            "signal/cooperative stop phase excluded; direct banner, status, "
            "diagnostic, and exit retained"
        ),
        "c": c_hard_result,
        "rust": rust_hard_result,
        "all_exact": (
            c_hard_result == rust_hard_result
            and c_hard_result["exit_code"] == 8
            and c_hard_result["direct_banner_is_prefix"]
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
        "projected_cases": [auto_memory_report, soft_report, hard_report],
        "all_exact": (
            all(case["all_exact"] for case in exact_reports)
            and resource_report["all_exact"]
            and auto_memory_report["all_exact"]
            and soft_report["all_exact"]
            and hard_report["all_exact"]
        ),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not report["all_exact"]:
        print("resource-limit comparison failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("resource-limit reference changed", file=sys.stderr)
            return 1
    print(
        f"matched {report['exact_matching_cases']}/{report['exact_case_count']} "
        "exact cases plus normalized resource and three projected host/timeout cases"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
