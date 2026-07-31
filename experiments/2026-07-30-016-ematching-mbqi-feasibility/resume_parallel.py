#!/usr/bin/env python3
"""Run the three remaining treatment shards concurrently, then merge."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    arguments = parser.parse_args()
    here = Path(__file__).resolve().parent
    output_root = arguments.output_root.resolve()
    processes: list[
        tuple[str, subprocess.Popen[str], object, object]
    ] = []
    for method in ("clausify", "ematch", "mbqi"):
        stdout = (output_root / f"{method}-shard.stdout.txt").open(
            "a", encoding="utf-8", newline="\n"
        )
        stderr = (output_root / f"{method}-shard.stderr.txt").open(
            "a", encoding="utf-8", newline="\n"
        )
        command = [
            sys.executable,
            str(here / "run_shard.py"),
            "--method",
            method,
            "--repo-root",
            str(arguments.repo_root.resolve()),
            "--problem-root",
            str(arguments.problem_root.resolve()),
            "--corpus",
            str(arguments.corpus.resolve()),
            "--cadical-driver",
            str(arguments.cadical_driver.resolve()),
            "--drat-trim",
            str(arguments.drat_trim.resolve()),
            "--output-root",
            str(output_root),
        ]
        process = subprocess.Popen(
            command, stdout=stdout, stderr=stderr, text=True
        )
        processes.append((method, process, stdout, stderr))

    failures: list[str] = []
    for method, process, stdout, stderr in processes:
        returncode = process.wait()
        stdout.close()
        stderr.close()
        if returncode != 0:
            failures.append(f"{method}:{returncode}")
    if failures:
        raise RuntimeError("shard failures: " + ", ".join(failures))

    subprocess.run(
        [
            sys.executable,
            str(here / "merge_results.py"),
            "--output-root",
            str(output_root),
        ],
        check=True,
    )
    subprocess.run(
        [
            sys.executable,
            str(here / "analyze.py"),
            "--output-root",
            str(output_root),
        ],
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
