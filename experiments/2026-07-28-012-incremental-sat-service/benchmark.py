#!/usr/bin/env python3
"""Run SAT adapters in randomized, resource-guarded experiment order."""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

SEED = 20_260_728


@dataclass(frozen=True)
class Backend:
    name: str
    executable: Path


def parse_backend(value: str) -> Backend:
    if "=" not in value:
        raise argparse.ArgumentTypeError("backend must be NAME=EXECUTABLE")
    name, executable = value.split("=", 1)
    path = Path(executable)
    if not name or not path.is_file():
        raise argparse.ArgumentTypeError(f"invalid backend {value!r}")
    return Backend(name, path)


def adapter_command(backend: Backend, session: Path, cpu: int | None) -> list[str]:
    command = [str(backend.executable), str(session)]
    if cpu is not None:
        command = ["taskset", "-c", str(cpu), *command]
    return command


def run_one(
    backend: Backend,
    session: Path,
    repetition: int,
    order: int,
    timeout_seconds: float,
    cpu: int | None,
) -> list[dict[str, object]]:
    with tempfile.NamedTemporaryFile(prefix="sat-rss-", delete=False) as stream:
        rss_path = Path(stream.name)
    command = [
        "/usr/bin/time",
        "-f",
        "%M",
        "-o",
        str(rss_path),
        *adapter_command(backend, session, cpu),
    ]
    started = time.perf_counter_ns()
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
        wall_ns = time.perf_counter_ns() - started
    except subprocess.TimeoutExpired as error:
        wall_ns = time.perf_counter_ns() - started
        return [
            {
                "record_type": "process",
                "backend": backend.name,
                "session": str(session),
                "repetition": repetition,
                "order": order,
                "outcome": "timeout",
                "process_wall_ns": wall_ns,
                "timeout_seconds": timeout_seconds,
                "stdout_prefix": (error.stdout or "")[:500],
                "stderr_prefix": (error.stderr or "")[:500],
            }
        ]
    finally:
        peak_rss_kib: int | None
        try:
            peak_rss_kib = int(rss_path.read_text(encoding="utf-8").strip())
        except (OSError, ValueError):
            peak_rss_kib = None
        try:
            rss_path.unlink()
        except FileNotFoundError:
            pass

    records: list[dict[str, object]] = []
    parse_error: str | None = None
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
                "peak_rss_kib": peak_rss_kib,
            }
        )
        records.append(record)

    if completed.returncode != 0 or parse_error is not None or not records:
        return [
            {
                "record_type": "process",
                "backend": backend.name,
                "session": str(session),
                "repetition": repetition,
                "order": order,
                "outcome": "error",
                "returncode": completed.returncode,
                "process_wall_ns": wall_ns,
                "peak_rss_kib": peak_rss_kib,
                "parse_error": parse_error,
                "stdout_prefix": completed.stdout[:500],
                "stderr_prefix": completed.stderr[:500],
            }
        ]
    return records


def discover_sessions(roots: list[Path]) -> list[Path]:
    sessions: set[Path] = set()
    for root in roots:
        if root.is_file():
            sessions.add(root.resolve())
        else:
            sessions.update(path.resolve() for path in root.rglob("*.isat"))
    return sorted(sessions)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--backend", action="append", type=parse_backend, required=True)
    parser.add_argument("--sessions", action="append", type=Path, required=True)
    parser.add_argument("--prefix", action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    parser.add_argument("--cpu", type=int, default=0)
    parser.add_argument("--seed", type=int, default=SEED)
    arguments = parser.parse_args()
    if arguments.repetitions < 1 or arguments.warmups < 0:
        parser.error("repetitions must be positive and warmups nonnegative")

    sessions = discover_sessions(arguments.sessions)
    if arguments.prefix:
        sessions = [
            session
            for session in sessions
            if any(session.name.startswith(prefix) for prefix in arguments.prefix)
        ]
    if not sessions:
        parser.error("no .isat sessions found")
    backends = list(arguments.backend)
    jobs = [
        (backend, session)
        for session in sessions
        for backend in backends
    ]

    for warmup in range(arguments.warmups):
        for backend, session in jobs:
            run_one(
                backend,
                session,
                -(warmup + 1),
                -1,
                arguments.timeout_seconds,
                arguments.cpu,
            )

    rng = random.Random(arguments.seed)
    output_records = 0
    failures = 0
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("w", encoding="utf-8", newline="\n") as output:
        order = 0
        for repetition in range(arguments.repetitions):
            shuffled = jobs.copy()
            rng.shuffle(shuffled)
            for backend, session in shuffled:
                records = run_one(
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
                    output_records += 1
                    if record["record_type"] == "process":
                        failures += 1
                output.flush()
                os.fsync(output.fileno())

    print(
        json.dumps(
            {
                "backends": [backend.name for backend in backends],
                "sessions": len(sessions),
                "repetitions": arguments.repetitions,
                "records": output_records,
                "process_failures": failures,
                "seed": arguments.seed,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
