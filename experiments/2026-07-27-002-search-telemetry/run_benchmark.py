#!/usr/bin/env python3
"""Measure search-telemetry overhead and reproduce a limit diagnosis on Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import resource
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


CPU_OVERHEAD_BUDGET_PERCENT = 5.0
WALL_OVERHEAD_BUDGET_PERCENT = 10.0
SCHEMA = "umlaut.search-telemetry"
SCHEMA_VERSION = 1


@dataclass(frozen=True)
class Workload:
    name: str
    relative_problem: str
    processed_limit: int


OVERHEAD_WORKLOADS = (
    Workload("lcl365_limit", "eprover/EXAMPLE_PROBLEMS/TPTP/LCL365-1.p", 20_000),
    Workload("seu027_limit", "eprover/EXAMPLE_PROBLEMS/TPTP/SEU027+1.p", 20_000),
    Workload("swv851_limit", "eprover/EXAMPLE_PROBLEMS/TPTP/SWV851-1.p", 20_000),
)
DIAGNOSIS_PROBLEM = "eprover/EXAMPLE_PROBLEMS/TPTP/SYN190-1.p"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def child_cpu_seconds() -> float:
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def validate_telemetry(record: dict[str, Any], returncode: int) -> None:
    if record.get("schema") != SCHEMA or record.get("schema_version") != SCHEMA_VERSION:
        raise RuntimeError("telemetry schema identity is not version 1")
    outcome = record["outcome"]
    if outcome["exit_status"] != returncode:
        raise RuntimeError("telemetry exit status differs from the process return code")
    if outcome["kind"] not in {"returned", "stopped"}:
        raise RuntimeError("telemetry outcome kind is not recognized")
    if outcome["processed_steps"] < 0:
        raise RuntimeError("telemetry reports a negative processed-step count")
    funnel = record["search_funnel"]
    for suffix in ("processed", "unprocessed", "total", "archived"):
        if funnel[f"high_water_{suffix}"] < funnel[f"final_{suffix}"]:
            raise RuntimeError(f"high-water {suffix} is below its final value")
    if funnel["high_water_total"] < funnel["high_water_processed"]:
        raise RuntimeError("total clause high water is below the processed high water")
    if funnel["high_water_total"] < funnel["high_water_unprocessed"]:
        raise RuntimeError("total clause high water is below the unprocessed high water")
    for group in (
        "input_funnel",
        "search_funnel",
        "inferences",
        "simplification",
        "indices",
        "sat",
        "terms",
        "proof",
        "resources",
    ):
        if group not in record:
            raise RuntimeError(f"telemetry group {group!r} is absent")


def run_once(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
    workload: Workload,
    run_id: str,
    telemetry_enabled: bool,
) -> dict[str, Any]:
    problem = repo / workload.relative_problem
    telemetry_path = artifact_dir / "telemetry" / f"{run_id}.json"
    stdout_path = artifact_dir / "stdout" / f"{run_id}.txt"
    stderr_path = artifact_dir / "stderr" / f"{run_id}.txt"
    args = [
        str(binary),
        "--output-level=1",
        f"--processed-clauses-limit={workload.processed_limit}",
    ]
    if telemetry_enabled:
        args.append(f"--search-telemetry={telemetry_path}")
    args.append(str(problem))

    cpu_before = child_cpu_seconds()
    wall_before = time.perf_counter()
    completed = subprocess.run(
        args,
        cwd=repo,
        check=False,
        capture_output=True,
        timeout=60,
    )
    wall_seconds = time.perf_counter() - wall_before
    cpu_seconds = child_cpu_seconds() - cpu_before
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)

    telemetry = None
    if telemetry_enabled:
        if not telemetry_path.is_file():
            raise RuntimeError(f"{run_id} did not write telemetry")
        telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
        validate_telemetry(telemetry, completed.returncode)

    return {
        "run_id": run_id,
        "workload": workload.name,
        "processed_limit": workload.processed_limit,
        "telemetry_enabled": telemetry_enabled,
        "returncode": completed.returncode,
        "wall_seconds": wall_seconds,
        "child_cpu_seconds": cpu_seconds,
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
        "telemetry": telemetry,
    }


def overhead_percent(enabled: float, control: float) -> float:
    if control <= 0.0:
        raise RuntimeError("control duration must be positive")
    return (enabled / control - 1.0) * 100.0


def summarize_overhead(runs: list[dict[str, Any]]) -> dict[str, Any]:
    measured = [run for run in runs if not run["run_id"].startswith("warmup-")]
    control = [run for run in measured if not run["telemetry_enabled"]]
    enabled = [run for run in measured if run["telemetry_enabled"]]
    control_cpu = sum(run["child_cpu_seconds"] for run in control)
    enabled_cpu = sum(run["child_cpu_seconds"] for run in enabled)
    control_wall = sum(run["wall_seconds"] for run in control)
    enabled_wall = sum(run["wall_seconds"] for run in enabled)
    summary: dict[str, Any] = {
        "budgets_percent": {
            "aggregate_child_cpu": CPU_OVERHEAD_BUDGET_PERCENT,
            "aggregate_wall": WALL_OVERHEAD_BUDGET_PERCENT,
        },
        "aggregate": {
            "control_child_cpu_seconds": control_cpu,
            "enabled_child_cpu_seconds": enabled_cpu,
            "child_cpu_overhead_percent": overhead_percent(enabled_cpu, control_cpu),
            "control_wall_seconds": control_wall,
            "enabled_wall_seconds": enabled_wall,
            "wall_overhead_percent": overhead_percent(enabled_wall, control_wall),
        },
        "workloads": {},
    }
    for workload in OVERHEAD_WORKLOADS:
        workload_control = [run for run in control if run["workload"] == workload.name]
        workload_enabled = [run for run in enabled if run["workload"] == workload.name]
        summary["workloads"][workload.name] = {
            "control_child_cpu_median_seconds": statistics.median(
                run["child_cpu_seconds"] for run in workload_control
            ),
            "enabled_child_cpu_median_seconds": statistics.median(
                run["child_cpu_seconds"] for run in workload_enabled
            ),
            "control_wall_median_seconds": statistics.median(
                run["wall_seconds"] for run in workload_control
            ),
            "enabled_wall_median_seconds": statistics.median(
                run["wall_seconds"] for run in workload_enabled
            ),
        }
    aggregate = summary["aggregate"]
    summary["passed"] = (
        aggregate["child_cpu_overhead_percent"] <= CPU_OVERHEAD_BUDGET_PERCENT
        and aggregate["wall_overhead_percent"] <= WALL_OVERHEAD_BUDGET_PERCENT
    )
    return summary


def run_diagnosis(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
) -> dict[str, Any]:
    low = Workload("syn190_low_limit", DIAGNOSIS_PROBLEM, 1_000)
    high = Workload("syn190_high_limit", DIAGNOSIS_PROBLEM, 10_000)
    reproductions = []
    for repetition in range(2):
        low_run = run_once(
            repo=repo,
            binary=binary,
            artifact_dir=artifact_dir,
            workload=low,
            run_id=f"diagnosis-{repetition}-low",
            telemetry_enabled=True,
        )
        high_run = run_once(
            repo=repo,
            binary=binary,
            artifact_dir=artifact_dir,
            workload=high,
            run_id=f"diagnosis-{repetition}-high",
            telemetry_enabled=True,
        )
        low_outcome = low_run["telemetry"]["outcome"]
        high_outcome = high_run["telemetry"]["outcome"]
        if low_outcome["reason"] != "step_limit" or low_outcome["processed_steps"] != 1_000:
            raise RuntimeError("low-limit telemetry did not diagnose the imposed limit")
        if high_outcome["kind"] != "returned" or high_outcome["processed_steps"] <= 1_000:
            raise RuntimeError("high-limit telemetry did not reproduce the solved search")
        reproductions.append(
            {
                "low": {
                    "outcome": low_outcome,
                    "high_water": low_run["telemetry"]["search_funnel"],
                },
                "high": {
                    "outcome": high_outcome,
                    "high_water": high_run["telemetry"]["search_funnel"],
                },
            }
        )
    return {
        "diagnosis": (
            "The 1,000-step configuration truncates a search that needs more given-clause "
            "steps; increasing the limit to 10,000 permits the proof."
        ),
        "independent_reproductions": reproductions,
        "passed": len(reproductions) == 2,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--binary", type=Path, default=Path("target/release/umlaut"))
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=6)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if sys.platform != "linux":
        raise RuntimeError("this benchmark must run on the Linux authority")
    if args.repetitions < 2:
        raise RuntimeError("at least two repetitions are required")
    repo = args.repo.resolve()
    binary = args.binary if args.binary.is_absolute() else repo / args.binary
    artifact_dir = args.artifact_dir.resolve()
    for child in ("telemetry", "stdout", "stderr"):
        (artifact_dir / child).mkdir(parents=True, exist_ok=True)
    for workload in (*OVERHEAD_WORKLOADS, Workload("diagnosis", DIAGNOSIS_PROBLEM, 1)):
        if not (repo / workload.relative_problem).is_file():
            raise RuntimeError(f"missing workload {workload.relative_problem}")
    if not binary.is_file():
        raise RuntimeError(f"missing release binary {binary}")

    runs: list[dict[str, Any]] = []
    for workload in OVERHEAD_WORKLOADS:
        for enabled in (False, True):
            runs.append(
                run_once(
                    repo=repo,
                    binary=binary,
                    artifact_dir=artifact_dir,
                    workload=workload,
                    run_id=f"warmup-{workload.name}-{'on' if enabled else 'off'}",
                    telemetry_enabled=enabled,
                )
            )
        for repetition in range(args.repetitions):
            modes = (False, True) if repetition % 2 == 0 else (True, False)
            pair: list[dict[str, Any]] = []
            for enabled in modes:
                run = run_once(
                    repo=repo,
                    binary=binary,
                    artifact_dir=artifact_dir,
                    workload=workload,
                    run_id=(
                        f"measure-{workload.name}-{repetition}-"
                        f"{'on' if enabled else 'off'}"
                    ),
                    telemetry_enabled=enabled,
                )
                runs.append(run)
                pair.append(run)
            if pair[0]["returncode"] != pair[1]["returncode"]:
                raise RuntimeError("telemetry changed the paired process exit status")
            if pair[0]["stdout_sha256"] != pair[1]["stdout_sha256"]:
                raise RuntimeError("telemetry changed the paired standard output")
            if pair[0]["stderr_sha256"] != pair[1]["stderr_sha256"]:
                raise RuntimeError("telemetry changed the paired standard error")

    diagnosis = run_diagnosis(repo=repo, binary=binary, artifact_dir=artifact_dir)
    overhead = summarize_overhead(runs)
    raw_path = artifact_dir / "raw-runs.jsonl"
    raw_path.write_text(
        "".join(json.dumps(run, sort_keys=True) + "\n" for run in runs),
        encoding="utf-8",
    )
    summary = {
        "schema": "umlaut.search-telemetry-experiment",
        "schema_version": 1,
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "binary": {
            "path": str(binary.relative_to(repo)),
            "sha256": sha256_file(binary),
        },
        "problems": {
            workload.relative_problem: sha256_file(repo / workload.relative_problem)
            for workload in OVERHEAD_WORKLOADS
        }
        | {DIAGNOSIS_PROBLEM: sha256_file(repo / DIAGNOSIS_PROBLEM)},
        "repetitions": args.repetitions,
        "overhead": overhead,
        "diagnosis": diagnosis,
        "passed": overhead["passed"] and diagnosis["passed"],
    }
    summary_path = artifact_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
