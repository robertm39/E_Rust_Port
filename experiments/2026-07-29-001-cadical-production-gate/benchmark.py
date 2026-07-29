#!/usr/bin/env python3
"""Benchmark both modes of the production SAT service in frozen random order."""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import time
from pathlib import Path

SEED = 20_260_729


def output_prefix(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")[:500]
    return (value or "")[:500]


def discover(roots: list[Path]) -> list[Path]:
    sessions: set[Path] = set()
    for root in roots:
        if root.is_file():
            sessions.add(root.resolve())
        else:
            sessions.update(path.resolve() for path in root.rglob("*.isat"))
    return sorted(sessions)


def run_one(
    executable: Path,
    backend: str,
    session: Path,
    repetition: int,
    order: int,
    timeout: float,
    cpu: int,
) -> list[dict[str, object]]:
    command = [
        "taskset",
        "-c",
        str(cpu),
        str(executable),
        backend,
        str(session),
    ]
    started = time.perf_counter_ns()
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        return [
            {
                "record_type": "process",
                "backend": backend,
                "session": str(session),
                "repetition": repetition,
                "order": order,
                "outcome": "timeout",
                "process_wall_ns": time.perf_counter_ns() - started,
                "stdout_prefix": output_prefix(error.stdout),
                "stderr_prefix": output_prefix(error.stderr),
            }
        ]
    wall_ns = time.perf_counter_ns() - started
    records: list[dict[str, object]] = []
    parse_error = None
    for line_number, line in enumerate(completed.stdout.splitlines(), 1):
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            parse_error = f"line {line_number}: {error}"
            break
        record.update(
            {
                "record_type": "query",
                "repetition": repetition,
                "order": order,
                "process_wall_ns": wall_ns,
            }
        )
        records.append(record)
    if completed.returncode != 0 or parse_error is not None or not records:
        return [
            {
                "record_type": "process",
                "backend": backend,
                "session": str(session),
                "repetition": repetition,
                "order": order,
                "outcome": "error",
                "returncode": completed.returncode,
                "process_wall_ns": wall_ns,
                "parse_error": parse_error,
                "stdout_prefix": completed.stdout[:500],
                "stderr_prefix": completed.stderr[:500],
            }
        ]
    return records


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("--sessions", action="append", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--cpu", type=int, default=0)
    parser.add_argument("--seed", type=int, default=SEED)
    arguments = parser.parse_args()
    sessions = discover(arguments.sessions)
    if not sessions:
        parser.error("no .isat sessions found")
    jobs = [(backend, session) for session in sessions for backend in ("internal", "cadical")]

    for _warmup in range(arguments.warmups):
        for backend, session in jobs:
            run_one(
                arguments.executable,
                backend,
                session,
                -1,
                -1,
                arguments.timeout_seconds,
                arguments.cpu,
            )

    rng = random.Random(arguments.seed)
    failures = 0
    records_written = 0
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("w", encoding="utf-8", newline="\n") as output:
        order = 0
        for repetition in range(arguments.repetitions):
            shuffled = jobs.copy()
            rng.shuffle(shuffled)
            for backend, session in shuffled:
                records = run_one(
                    arguments.executable,
                    backend,
                    session,
                    repetition,
                    order,
                    arguments.timeout_seconds,
                    arguments.cpu,
                )
                order += 1
                for record in records:
                    output.write(json.dumps(record, sort_keys=True) + "\n")
                    records_written += 1
                    failures += int(record["record_type"] == "process")
                output.flush()
                os.fsync(output.fileno())
    print(
        json.dumps(
            {
                "sessions": len(sessions),
                "repetitions": arguments.repetitions,
                "records": records_written,
                "process_failures": failures,
                "seed": arguments.seed,
            },
            sort_keys=True,
        )
    )
    return int(failures != 0)


if __name__ == "__main__":
    raise SystemExit(main())
