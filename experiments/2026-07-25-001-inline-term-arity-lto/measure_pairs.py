#!/usr/bin/env python3
"""Run alternating parent/candidate LUSK6 proof-search measurements."""

from __future__ import annotations

import argparse
import csv
import hashlib
import resource
import subprocess
import time
from pathlib import Path


def run_once(
    *,
    phase: str,
    pair: int,
    position: int,
    label: str,
    binary: Path,
    problem: Path,
) -> dict[str, object]:
    usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    completed = subprocess.run(
        [
            binary,
            problem,
            "--auto",
            "--silent",
            "--cpu-limit=600",
            "--memory-limit=2048",
            "--detsort-rw",
            "--detsort-new",
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    wall_seconds = time.perf_counter() - started
    usage_after = resource.getrusage(resource.RUSAGE_CHILDREN)
    user_seconds = usage_after.ru_utime - usage_before.ru_utime
    system_seconds = usage_after.ru_stime - usage_before.ru_stime
    return {
        "phase": phase,
        "pair": pair,
        "position": position,
        "label": label,
        "wall_seconds": f"{wall_seconds:.9f}",
        "cpu_seconds": f"{user_seconds + system_seconds:.9f}",
        "user_seconds": f"{user_seconds:.9f}",
        "system_seconds": f"{system_seconds:.9f}",
        "status": completed.returncode,
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "stdout_len": len(completed.stdout),
        "stderr_len": len(completed.stderr),
    }


def run_phase(
    *,
    writer: csv.DictWriter,
    output,
    phase: str,
    pairs: int,
    parent: Path,
    candidate: Path,
    problem: Path,
) -> None:
    for pair in range(1, pairs + 1):
        order = (
            (("parent", parent), ("candidate", candidate))
            if pair % 2
            else (("candidate", candidate), ("parent", parent))
        )
        for position, (label, binary) in enumerate(order, start=1):
            row = run_once(
                phase=phase,
                pair=pair,
                position=position,
                label=label,
                binary=binary,
                problem=problem,
            )
            writer.writerow(row)
            output.flush()
            if (
                row["status"] != 0
                or row["stderr_len"] != 0
                or row["stdout_len"] == 0
            ):
                raise RuntimeError(f"{phase} pair {pair} {label} did not prove cleanly")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--parent", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--warmups", type=int, default=4)
    parser.add_argument("--pairs", type=int, default=64)
    parser.add_argument("--warmup-csv", type=Path, required=True)
    parser.add_argument("--measurement-csv", type=Path, required=True)
    arguments = parser.parse_args()

    fieldnames = [
        "phase",
        "pair",
        "position",
        "label",
        "wall_seconds",
        "cpu_seconds",
        "user_seconds",
        "system_seconds",
        "status",
        "stdout_sha256",
        "stdout_len",
        "stderr_len",
    ]
    for phase, pairs, destination in (
        ("warmup", arguments.warmups, arguments.warmup_csv),
        ("measurement", arguments.pairs, arguments.measurement_csv),
    ):
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("w", encoding="utf-8", newline="") as output:
            writer = csv.DictWriter(output, fieldnames=fieldnames)
            writer.writeheader()
            run_phase(
                writer=writer,
                output=output,
                phase=phase,
                pairs=pairs,
                parent=arguments.parent,
                candidate=arguments.candidate,
                problem=arguments.problem,
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
