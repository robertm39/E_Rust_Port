#!/usr/bin/env python3
"""Benchmark the live comparison-cache workload before and after safe splaying."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import subprocess
import time
from pathlib import Path
from typing import Any


BASELINE_COMMIT = "45692b5e49abe236047fbf5f9630630f57aac186"
OPTIONS = ["--silent"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--maximum-ratio", type=float, default=1.10)
    parser.add_argument("--timeout-seconds", type=float, default=60.0)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_once(executable: Path, fixture: Path, timeout: float) -> dict[str, Any]:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        [str(executable), *OPTIONS, str(fixture)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    elapsed = time.perf_counter_ns() - started
    return {
        "wall_nanoseconds": elapsed,
        "exit_code": completed.returncode,
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
        "stdout": completed.stdout.decode("utf-8"),
        "stderr": completed.stderr.decode("utf-8"),
    }


def behavior(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "exit_code": result["exit_code"],
        "stdout_sha256": result["stdout_sha256"],
        "stderr_sha256": result["stderr_sha256"],
        "stdout": result["stdout"],
        "stderr": result["stderr"],
    }


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    if args.rounds <= 0 or args.warmups < 0:
        raise ValueError("rounds must be positive and warmups non-negative")
    baseline = args.baseline_exe.resolve()
    candidate = args.candidate_exe.resolve()
    fixture = repo / "eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"

    for _ in range(args.warmups):
        run_once(baseline, fixture, args.timeout_seconds)
        run_once(candidate, fixture, args.timeout_seconds)

    measurements: list[dict[str, Any]] = []
    for round_index in range(args.rounds):
        order = (
            [("baseline", baseline), ("candidate", candidate)]
            if round_index % 2 == 0
            else [("candidate", candidate), ("baseline", baseline)]
        )
        for name, executable in order:
            result = run_once(executable, fixture, args.timeout_seconds)
            measurements.append(
                {
                    "mode": name,
                    "round": round_index,
                    "wall_nanoseconds": result["wall_nanoseconds"],
                    "exit_code": result["exit_code"],
                    "stdout_sha256": result["stdout_sha256"],
                    "stderr_sha256": result["stderr_sha256"],
                }
            )

    baseline_behavior = behavior(run_once(baseline, fixture, args.timeout_seconds))
    candidate_behavior = behavior(run_once(candidate, fixture, args.timeout_seconds))
    behavior_exact = baseline_behavior == candidate_behavior
    baseline_times = [
        row["wall_nanoseconds"] for row in measurements if row["mode"] == "baseline"
    ]
    candidate_times = [
        row["wall_nanoseconds"] for row in measurements if row["mode"] == "candidate"
    ]
    baseline_median = statistics.median(baseline_times)
    candidate_median = statistics.median(candidate_times)
    ratio = candidate_median / baseline_median
    all_measurements_exact = all(
        row["exit_code"] == baseline_behavior["exit_code"]
        and row["stdout_sha256"] == baseline_behavior["stdout_sha256"]
        and row["stderr_sha256"] == baseline_behavior["stderr_sha256"]
        for row in measurements
    )
    accepted = (
        behavior_exact
        and all_measurements_exact
        and ratio <= args.maximum_ratio
    )
    return {
        "schema_version": 1,
        "workload": {
            "fixture": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop",
            "fixture_sha256": sha256(fixture),
            "options": OPTIONS,
            "rounds": args.rounds,
            "warmups_per_executable": args.warmups,
            "maximum_candidate_baseline_ratio": args.maximum_ratio,
        },
        "executables": {
            "baseline_commit": BASELINE_COMMIT,
            "baseline_sha256": sha256(baseline),
            "candidate_sha256": sha256(candidate),
            "candidate_quadtrees_rs_sha256": sha256(
                repo / "src/basics/quadtrees.rs"
            ),
        },
        "behavior": {
            "baseline": baseline_behavior,
            "candidate": candidate_behavior,
            "exact": behavior_exact,
            "all_measurements_exact": all_measurements_exact,
        },
        "measurements": measurements,
        "summary": {
            "baseline_median_nanoseconds": baseline_median,
            "candidate_median_nanoseconds": candidate_median,
            "candidate_baseline_ratio": ratio,
            "accepted": accepted,
        },
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo, args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    summary = result["summary"]
    print(
        "comparison-cache benchmark: "
        f"ratio={summary['candidate_baseline_ratio']:.3f}; "
        f"behavior_exact={result['behavior']['exact']}; "
        f"accepted={summary['accepted']}"
    )
    return 0 if summary["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
