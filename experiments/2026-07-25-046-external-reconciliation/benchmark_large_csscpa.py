#!/usr/bin/env python3
"""Benchmark streamed C versus eager Rust scanning on large CSSCPA inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


METRIC_PREFIX = "__CSSCPA_METRIC__"
TAUTOLOGY_COMMAND = b"accept: cnf(repeated,axiom,(p(a)|~p(a))).\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-bin", type=Path, required=True)
    parser.add_argument("--rust-bin", type=Path, required=True)
    parser.add_argument("--rust-baseline-bin", type=Path)
    parser.add_argument("--commands", default="1,100000,500000")
    parser.add_argument("--repetitions", type=int, default=3)
    return parser.parse_args()


def run_once(binary: Path, fixture: Path) -> dict[str, object]:
    command = [
        "/usr/bin/time",
        "-f",
        f"{METRIC_PREFIX} %e %M %x",
        str(binary),
        "--silent",
        str(fixture),
    ]
    started = time.perf_counter()
    completed = subprocess.run(
        command,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    measured_wall = time.perf_counter() - started
    metric_line = next(
        (
            line
            for line in reversed(completed.stderr.splitlines())
            if line.startswith(METRIC_PREFIX)
        ),
        None,
    )
    if metric_line is None:
        raise RuntimeError(
            f"missing GNU time metric for {binary}: {completed.stderr[-1000:]}"
        )
    _prefix, elapsed, max_rss, timed_exit = metric_line.split()
    if completed.returncode != 0 or int(timed_exit) != 0:
        raise RuntimeError(
            f"{binary} exited {completed.returncode}: {completed.stderr[-1000:]}"
        )
    return {
        "gnu_elapsed_seconds": float(elapsed),
        "max_rss_kib": int(max_rss),
        "wall_seconds": measured_wall,
    }


def summarize(runs: list[dict[str, object]]) -> dict[str, object]:
    return {
        "max_rss_kib": [run["max_rss_kib"] for run in runs],
        "median_max_rss_kib": statistics.median(
            int(run["max_rss_kib"]) for run in runs
        ),
        "median_wall_seconds": statistics.median(
            float(run["wall_seconds"]) for run in runs
        ),
        "wall_seconds": [run["wall_seconds"] for run in runs],
    }


def main() -> int:
    args = parse_args()
    if args.repetitions < 1:
        raise ValueError("--repetitions must be positive")
    command_counts = [int(value) for value in args.commands.split(",")]
    if any(value < 1 for value in command_counts):
        raise ValueError("--commands values must be positive")
    binaries = {
        "c": args.c_bin.resolve(),
        "rust": args.rust_bin.resolve(),
    }
    if args.rust_baseline_bin is not None:
        binaries["rust_baseline"] = args.rust_baseline_bin.resolve()
    for binary in binaries.values():
        if not binary.is_file():
            raise FileNotFoundError(binary)

    cases: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="csscpa-large-") as temp_name:
        temp_dir = Path(temp_name)
        for command_count in command_counts:
            fixture = temp_dir / f"commands-{command_count}.csscpa"
            with fixture.open("wb") as stream:
                for _index in range(command_count):
                    stream.write(TAUTOLOGY_COMMAND)
            fixture_bytes = fixture.stat().st_size
            fixture_sha256 = hashlib.sha256(fixture.read_bytes()).hexdigest()

            runs: dict[str, list[dict[str, object]]] = {
                implementation: [] for implementation in binaries
            }
            implementations = tuple(binaries)
            for repetition in range(args.repetitions):
                offset = repetition % len(implementations)
                order = implementations[offset:] + implementations[:offset]
                for implementation in order:
                    runs[implementation].append(
                        run_once(binaries[implementation], fixture)
                    )
            cases.append(
                {
                    "command_count": command_count,
                    "fixture_bytes": fixture_bytes,
                    "fixture_sha256": fixture_sha256,
                    "implementations": {
                        implementation: summarize(implementation_runs)
                        for implementation, implementation_runs in runs.items()
                    },
                }
            )

    baseline = cases[0]["implementations"]
    assert isinstance(baseline, dict)
    for case in cases:
        implementations = case["implementations"]
        assert isinstance(implementations, dict)
        for implementation in binaries:
            summary = implementations[implementation]
            base_summary = baseline[implementation]
            assert isinstance(summary, dict)
            assert isinstance(base_summary, dict)
            summary["rss_growth_from_small_kib"] = (
                float(summary["median_max_rss_kib"])
                - float(base_summary["median_max_rss_kib"])
            )
        c_summary = implementations["c"]
        rust_summary = implementations["rust"]
        assert isinstance(c_summary, dict)
        assert isinstance(rust_summary, dict)
        case["rust_over_c_wall_ratio"] = (
            float(rust_summary["median_wall_seconds"])
            / float(c_summary["median_wall_seconds"])
        )
        case["rust_minus_c_rss_kib"] = (
            float(rust_summary["median_max_rss_kib"])
            - float(c_summary["median_max_rss_kib"])
        )
        if "rust_baseline" in implementations:
            baseline_summary = implementations["rust_baseline"]
            assert isinstance(baseline_summary, dict)
            case["candidate_over_baseline_wall_ratio"] = (
                float(rust_summary["median_wall_seconds"])
                / float(baseline_summary["median_wall_seconds"])
            )
            case["candidate_minus_baseline_rss_kib"] = (
                float(rust_summary["median_max_rss_kib"])
                - float(baseline_summary["median_max_rss_kib"])
            )

    report = {
        "binaries": {key: str(value) for key, value in binaries.items()},
        "cases": cases,
        "repetitions": args.repetitions,
        "schema_version": 1,
        "workload_command_sha256": hashlib.sha256(TAUTOLOGY_COMMAND).hexdigest(),
        "workload_command_size": len(TAUTOLOGY_COMMAND),
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
