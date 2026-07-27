#!/usr/bin/env python3
"""Benchmark fresh allocation against the typed RegMem scratch policy."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import statistics
import subprocess
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=int, default=400_000)
    parser.add_argument("--rounds", type=int, default=7)
    parser.add_argument("--minimum-speedup", type=float, default=1.10)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--build-dir", type=Path, default=Path("target/regmem-typed-scratch")
    )
    return parser.parse_args()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command: list[str], *, cwd: Path) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout


def parse_measurements(output: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for row in csv.DictReader(io.StringIO(output)):
        rows.append(
            {
                "mode": row["mode"],
                "round": int(row["round"]),
                "nanoseconds": int(row["nanoseconds"]),
                "checksum": int(row["checksum"]),
                "growths": int(row["growths"]),
            }
        )
    return rows


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    experiment = Path(__file__).resolve().parent
    build_dir = (repo / args.build_dir).resolve()
    build_dir.mkdir(parents=True, exist_ok=True)
    executable = build_dir / "scratch_bench.exe"
    source = experiment / "scratch_bench.rs"
    run(
        [
            "rustc",
            "--edition=2021",
            "-C",
            "opt-level=3",
            str(source),
            "-o",
            str(executable),
        ],
        cwd=repo,
    )
    measurements = parse_measurements(
        run([str(executable), str(args.iterations), str(args.rounds)], cwd=repo)
    )

    expected_rows = args.rounds * 2
    checksums = {row["checksum"] for row in measurements}
    fresh = [row for row in measurements if row["mode"] == "fresh"]
    reused = [row for row in measurements if row["mode"] == "reused"]
    fresh_median = statistics.median(row["nanoseconds"] for row in fresh)
    reused_median = statistics.median(row["nanoseconds"] for row in reused)
    speedup = fresh_median / reused_median
    deterministic_exact = (
        len(measurements) == expected_rows
        and len(fresh) == args.rounds
        and len(reused) == args.rounds
        and len(checksums) == 1
        and all(row["growths"] == args.iterations for row in fresh)
        and all(row["growths"] == 3 for row in reused)
    )
    accepted = deterministic_exact and speedup >= args.minimum_speedup
    return {
        "schema_version": 1,
        "workload": {
            "iterations_per_round": args.iterations,
            "rounds": args.rounds,
            "required_lengths": [256, 1024, 4096, 512, 2048, 384, 1536, 768],
            "touched_slots_per_iteration": 12,
        },
        "sources": {
            "benchmark_sha256": sha256(source),
            "regmem_rs_sha256": sha256(repo / "src/basics/regmem.rs"),
            "freqvectors_rs_sha256": sha256(repo / "src/clauses/freqvectors.rs"),
        },
        "measurements": measurements,
        "summary": {
            "fresh_median_nanoseconds": fresh_median,
            "reused_median_nanoseconds": reused_median,
            "median_speedup": speedup,
            "fresh_allocations_per_round": args.iterations,
            "reused_growths_per_round": 3,
            "deterministic_checks_exact": deterministic_exact,
            "minimum_speedup": args.minimum_speedup,
            "accepted": accepted,
        },
    }


def main() -> int:
    args = parse_args()
    if args.iterations <= 0 or args.rounds <= 0:
        raise ValueError("iterations and rounds must be positive")
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo, args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    summary = result["summary"]
    print(
        "typed scratch benchmark: "
        f"{summary['median_speedup']:.3f}x median speedup; "
        f"accepted={summary['accepted']}"
    )
    return 0 if summary["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
