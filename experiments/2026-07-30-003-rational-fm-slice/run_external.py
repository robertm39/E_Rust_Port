#!/usr/bin/env python3
"""Run pinned Z3 and Vampire controls on rendered experiment workloads."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from pathlib import Path
from typing import Any


SZS_PATTERN = re.compile(r"SZS status\s+(\S+)", re.IGNORECASE)


def run_command(command: list[str], timeout_seconds: float) -> dict[str, Any]:
    started = time.perf_counter_ns()
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            check=False,
            text=True,
            timeout=timeout_seconds,
        )
        return {
            "command": command,
            "returncode": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "elapsed_ms": (time.perf_counter_ns() - started) / 1_000_000,
            "timed_out": False,
        }
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout.decode() if isinstance(error.stdout, bytes) else (error.stdout or "")
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else (error.stderr or "")
        return {
            "command": command,
            "returncode": None,
            "stdout": stdout,
            "stderr": stderr,
            "elapsed_ms": (time.perf_counter_ns() - started) / 1_000_000,
            "timed_out": True,
        }


def z3_outcome(result: dict[str, Any]) -> str:
    for line in result["stdout"].splitlines():
        value = line.strip().lower()
        if value in {"sat", "unsat", "unknown"}:
            return value
    return "unknown"


def vampire_outcome(result: dict[str, Any]) -> tuple[str, str | None]:
    match = SZS_PATTERN.search(result["stdout"])
    status = match.group(1) if match else None
    if status in {"Unsatisfiable", "Theorem"}:
        return "unsat", status
    if status in {"Satisfiable", "CounterSatisfiable"}:
        return "sat", status
    return "unknown", status


def compact(result: dict[str, Any], outcome: str) -> dict[str, Any]:
    return {
        **result,
        "outcome": outcome,
        "stdout_sha256": __import__("hashlib").sha256(
            result["stdout"].encode("utf-8")
        ).hexdigest(),
        "stderr_sha256": __import__("hashlib").sha256(
            result["stderr"].encode("utf-8")
        ).hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--z3", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=5.0)
    arguments = parser.parse_args()
    base = arguments.manifest.parent
    manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
    reports: list[dict[str, Any]] = []
    for workload in manifest["workloads"]:
        smt_path = base / workload["smt2"]
        tptp_path = base / workload["tptp"]
        z3_result = run_command(
            [
                str(arguments.z3),
                f"-T:{max(1, int(arguments.timeout_seconds))}",
                "-smt2",
                str(smt_path),
            ],
            arguments.timeout_seconds + 1,
        )
        theory_result = run_command(
            [
                str(arguments.vampire),
                "--mode",
                "vampire",
                "--time_limit",
                str(arguments.timeout_seconds),
                "--random_seed",
                "0",
                "--proof",
                "off",
                "--abstracting_linear_arithmetic_superposition_calculus",
                "off",
                "--theory_axioms",
                "on",
                str(tptp_path),
            ],
            arguments.timeout_seconds + 1,
        )
        alasca_result = run_command(
            [
                str(arguments.vampire),
                "--mode",
                "vampire",
                "--time_limit",
                str(arguments.timeout_seconds),
                "--random_seed",
                "0",
                "--proof",
                "off",
                "--abstracting_linear_arithmetic_superposition_calculus",
                "on",
                "--virtual_integer_real_arithmetic_substitution",
                "off",
                "--theory_axioms",
                "off",
                str(tptp_path),
            ],
            arguments.timeout_seconds + 1,
        )
        theory_outcome, theory_status = vampire_outcome(theory_result)
        alasca_outcome, alasca_status = vampire_outcome(alasca_result)
        reports.append(
            {
                "id": workload["id"],
                "partition": workload["partition"],
                "expected": workload["expected"],
                "z3": compact(z3_result, z3_outcome(z3_result)),
                "vampire_theory_axioms": {
                    **compact(theory_result, theory_outcome),
                    "szs_status": theory_status,
                },
                "vampire_alasca_no_viras": {
                    **compact(alasca_result, alasca_outcome),
                    "szs_status": alasca_status,
                },
            }
        )
    summary = {
        arm: {
            outcome: sum(
                report[arm]["outcome"] == outcome for report in reports
            )
            for outcome in ("sat", "unsat", "unknown")
        }
        for arm in (
            "z3",
            "vampire_theory_axioms",
            "vampire_alasca_no_viras",
        )
    }
    expected_reports = [
        report
        for report in reports
        if report["expected"] in {"sat", "unsat", "unknown"}
    ]
    summary["z3_expected_total"] = len(expected_reports)
    summary["z3_expected_matches"] = sum(
        report["z3"]["outcome"] == report["expected"]
        for report in expected_reports
    )
    result = {"summary": summary, "workloads": reports}
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    return (
        0
        if summary["z3_expected_matches"] == summary["z3_expected_total"]
        else 1
    )


if __name__ == "__main__":
    raise SystemExit(main())
