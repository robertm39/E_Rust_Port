#!/usr/bin/env python3
"""Measure adapter-level UNSAT proof generation overhead."""

from __future__ import annotations

import argparse
import json
import math
import random
import statistics
import subprocess
import tempfile
import time
from collections import defaultdict
from pathlib import Path

from benchmark import Backend, parse_backend


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def run_one(
    backend: Backend,
    session: Path,
    proof: bool,
    repetition: int,
    timeout_seconds: float,
    cpu: int | None,
    temp: Path,
) -> dict[str, object]:
    proof_path = temp / f"{backend.name}-{repetition}.proof"
    rss_path = temp / f"{backend.name}-{repetition}.rss"
    adapter = [str(backend.executable), str(session)]
    if proof:
        adapter.append(str(proof_path))
    if cpu is not None:
        adapter = ["taskset", "-c", str(cpu), *adapter]
    command = ["/usr/bin/time", "-f", "%M", "-o", str(rss_path), *adapter]
    started = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout_seconds,
    )
    wall_ns = time.perf_counter_ns() - started
    try:
        peak_rss_kib = int(rss_path.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        peak_rss_kib = None
    records = [
        json.loads(line) for line in completed.stdout.splitlines() if line.strip()
    ]
    result: dict[str, object] = {
        "backend": backend.name,
        "proof": proof,
        "repetition": repetition,
        "returncode": completed.returncode,
        "process_wall_ns": wall_ns,
        "peak_rss_kib": peak_rss_kib,
        "proof_bytes": proof_path.stat().st_size if proof_path.exists() else 0,
    }
    if len(records) == 1:
        result.update(
            {
                "status": records[0].get("status"),
                "elapsed_ns": int(records[0].get("elapsed_ns", 0)),
            }
        )
    else:
        result.update(
            {
                "status": "malformed_output",
                "records": len(records),
                "stderr": completed.stderr[-500:],
            }
        )
    proof_path.unlink(missing_ok=True)
    rss_path.unlink(missing_ok=True)
    return result


def distribution(records: list[dict[str, object]], field: str) -> dict[str, object]:
    values = [int(record[field]) for record in records if record.get(field) is not None]
    return {
        "count": len(values),
        "median": statistics.median(values),
        "p95": percentile(values, 0.95),
        "min": min(values),
        "max": max(values),
    }


def summarize(records: list[dict[str, object]]) -> dict[str, object]:
    groups: dict[tuple[str, bool], list[dict[str, object]]] = defaultdict(list)
    for record in records:
        groups[(str(record["backend"]), bool(record["proof"]))].append(record)
    backends: dict[str, object] = {}
    for backend in sorted({key[0] for key in groups}):
        off = groups[(backend, False)]
        on = groups[(backend, True)]
        off_median = statistics.median(int(record["elapsed_ns"]) for record in off)
        on_median = statistics.median(int(record["elapsed_ns"]) for record in on)
        backends[backend] = {
            "without_proof": {
                "solve_ns": distribution(off, "elapsed_ns"),
                "wall_ns": distribution(off, "process_wall_ns"),
                "peak_rss_kib": distribution(off, "peak_rss_kib"),
            },
            "with_proof": {
                "solve_ns": distribution(on, "elapsed_ns"),
                "wall_ns": distribution(on, "process_wall_ns"),
                "peak_rss_kib": distribution(on, "peak_rss_kib"),
                "proof_bytes": distribution(on, "proof_bytes"),
            },
            "median_solve_overhead_ratio": on_median / off_median,
        }
    failures = [
        record
        for record in records
        if record.get("returncode") != 0 or record.get("status") != "unsat"
    ]
    return {
        "schema": 1,
        "records": len(records),
        "failures": failures,
        "backends": backends,
        "valid": not failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--backend",
        action="append",
        type=parse_backend,
        required=True,
    )
    parser.add_argument("--session", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=20)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--cpu", type=int, default=0)
    parser.add_argument("--seed", type=int, default=20_260_728)
    arguments = parser.parse_args()

    jobs = [
        (backend, proof)
        for backend in arguments.backend
        for proof in (False, True)
    ]
    records: list[dict[str, object]] = []
    rng = random.Random(arguments.seed)
    with tempfile.TemporaryDirectory(prefix="sat-proof-overhead-") as raw_temp:
        temp = Path(raw_temp)
        for warmup in range(arguments.warmups):
            for backend, proof in jobs:
                run_one(
                    backend,
                    arguments.session,
                    proof,
                    -(warmup + 1),
                    arguments.timeout_seconds,
                    arguments.cpu,
                    temp,
                )
        for repetition in range(arguments.repetitions):
            shuffled = jobs.copy()
            rng.shuffle(shuffled)
            for backend, proof in shuffled:
                records.append(
                    run_one(
                        backend,
                        arguments.session,
                        proof,
                        repetition,
                        arguments.timeout_seconds,
                        arguments.cpu,
                        temp,
                    )
                )

    arguments.output.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
    )
    summary = summarize(records)
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    arguments.summary.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if summary["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
