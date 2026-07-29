#!/usr/bin/env python3
"""Run and time the frozen proof-derived TSM classifier workloads."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import resource
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


class ExperimentError(RuntimeError):
    """Raised when a classifier workload fails."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def child_cpu_seconds() -> float:
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def classify_once(
    binary: Path, input_path: Path
) -> tuple[subprocess.CompletedProcess[bytes], float, float]:
    cpu_before = child_cpu_seconds()
    started = time.monotonic()
    completed = subprocess.run(
        [
            str(binary),
            "-l",
            "1",
            "-i",
            "Identity",
            "-d",
            "100000",
            "-t",
            "Flat",
            str(input_path),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=120,
        check=False,
    )
    wall_seconds = time.monotonic() - started
    cpu_seconds = child_cpu_seconds() - cpu_before
    return completed, wall_seconds, cpu_seconds


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--input-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=5)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("classifier execution may run only on Linux")
    binary = arguments.binary.resolve()
    input_root = arguments.input_root.resolve()
    output_root = arguments.output_root.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(f"missing executable: {binary}")
    if arguments.repetitions != 5:
        raise ExperimentError("the preregistered repetition count is five")
    output_root.mkdir(parents=True, exist_ok=True)

    summary: dict[str, Any] = {
        "schema_version": 1,
        "binary_sha256": sha256_file(binary),
        "workloads": {},
    }
    for name in ("train-self", "validation", "test"):
        input_path = input_root / f"{name}.tsm"
        if not input_path.is_file():
            raise ExperimentError(f"missing classifier input: {input_path}")
        completed, _wall, _cpu = classify_once(binary, input_path)
        if completed.returncode != 0:
            raise ExperimentError(f"classifier warm-up failed: {name}")
        (output_root / f"{name}.stdout").write_bytes(completed.stdout)
        (output_root / f"{name}.stderr").write_bytes(completed.stderr)

        timings = []
        for repetition in range(1, arguments.repetitions + 1):
            measured, wall_seconds, cpu_seconds = classify_once(
                binary, input_path
            )
            if measured.returncode != 0:
                raise ExperimentError(
                    f"classifier repetition failed: {name}/{repetition}"
                )
            timings.append(
                {
                    "repetition": repetition,
                    "wall_seconds": wall_seconds,
                    "cpu_seconds": cpu_seconds,
                    "stdout_sha256": hashlib.sha256(measured.stdout).hexdigest(),
                    "stderr_sha256": hashlib.sha256(measured.stderr).hexdigest(),
                }
            )
        if len({timing["stdout_sha256"] for timing in timings}) != 1:
            raise ExperimentError(f"classifier output changed across {name}")
        summary["workloads"][name] = {
            "input_sha256": sha256_file(input_path),
            "output_sha256": sha256_file(output_root / f"{name}.stdout"),
            "timings": timings,
        }
    write_json(output_root / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
