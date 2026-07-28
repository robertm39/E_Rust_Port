#!/usr/bin/env python3
"""Capture exact production SATCheck DIMACS workloads on a selected corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import time
from pathlib import Path

SZS = re.compile(r"SZS status ([A-Za-z]+)")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def dimacs_shape(path: Path) -> tuple[int, int]:
    with path.open("r", encoding="ascii") as stream:
        header = stream.readline().split()
    if len(header) != 4 or header[:2] != ["p", "cnf"]:
        raise ValueError(f"{path}: invalid DIMACS header")
    return int(header[2]), int(header[3])


def safe_label(record: dict[str, object]) -> str:
    raw = (
        f"{record['holdout_split']}-{record['category']}-"
        f"{record['problem_id']}"
    )
    return "".join(
        character if character.isalnum() or character in "-_." else "_"
        for character in raw
    )


def run_problem(
    record: dict[str, object],
    corpus_root: Path,
    executable: Path,
    capture_root: Path,
    cpu_seconds: int,
    wall_seconds: float,
    capture_max: int,
    cpu: int | None,
) -> dict[str, object]:
    problem = corpus_root / str(record["path"])
    if not problem.is_file():
        raise FileNotFoundError(problem)
    actual_hash = sha256(problem)
    if actual_hash != record["sha256"]:
        raise ValueError(f"{problem}: hash {actual_hash} does not match manifest")
    label = safe_label(record)
    label_root = capture_root / label
    if label_root.exists() and any(label_root.iterdir()):
        raise FileExistsError(f"capture output already exists: {label_root}")

    command = [
        str(executable),
        str(problem),
        "--auto",
        "--silent",
        f"--cpu-limit={cpu_seconds}",
        "--memory-limit=4096",
        "--satcheck=ConjMinMinFreq",
        "--satcheck-proc-interval=50",
        "--satcheck-gen-interval=500",
        "--satcheck-ttinsert-interval=500",
        "--satcheck-decision-limit=10000",
    ]
    if cpu is not None:
        command = ["taskset", "-c", str(cpu), *command]
    environment = os.environ.copy()
    environment.update(
        {
            "TPTP": str(corpus_root / "problems" / "casc_2025"),
            "UMLAUT_SAT_CAPTURE_DIR": str(capture_root),
            "UMLAUT_SAT_CAPTURE_LABEL": label,
            "UMLAUT_SAT_CAPTURE_MAX": str(capture_max),
        }
    )
    started = time.perf_counter_ns()
    outcome = "completed"
    try:
        completed = subprocess.run(
            command,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=wall_seconds,
            start_new_session=True,
            check=False,
        )
        returncode = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        outcome = "wall_timeout"
        returncode = None
        stdout = error.stdout or ""
        stderr = error.stderr or ""
    elapsed_ns = time.perf_counter_ns() - started

    captures = []
    if label_root.is_dir():
        for capture in sorted(label_root.glob("*.cnf")):
            variables, clauses = dimacs_shape(capture)
            captures.append(
                {
                    "path": str(capture.relative_to(capture_root)),
                    "sha256": sha256(capture),
                    "bytes": capture.stat().st_size,
                    "variables": variables,
                    "clauses": clauses,
                }
            )
    statuses = SZS.findall(stdout)
    return {
        "record_type": "capture",
        "problem_id": record["problem_id"],
        "category": record["category"],
        "division": record["division"],
        "holdout_split": record["holdout_split"],
        "family": record["family"],
        "problem_path": str(problem),
        "problem_sha256": actual_hash,
        "label": label,
        "command": command,
        "cpu_seconds": cpu_seconds,
        "wall_seconds": wall_seconds,
        "capture_max": capture_max,
        "outcome": outcome,
        "returncode": returncode,
        "elapsed_ns": elapsed_ns,
        "szs_statuses": statuses,
        "stdout_sha256": hashlib.sha256(stdout.encode()).hexdigest(),
        "stderr_sha256": hashlib.sha256(stderr.encode()).hexdigest(),
        "stdout_prefix": stdout[:500],
        "stderr_prefix": stderr[:500],
        "captures": captures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("selection", type=Path)
    parser.add_argument("corpus_root", type=Path)
    parser.add_argument("executable", type=Path)
    parser.add_argument("capture_root", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cpu-seconds", type=int, default=2)
    parser.add_argument("--wall-seconds", type=float, default=5.0)
    parser.add_argument("--capture-max", type=int, default=5)
    parser.add_argument("--cpu", type=int, default=0)
    arguments = parser.parse_args()

    records = [
        json.loads(line)
        for line in arguments.selection.read_text(encoding="utf-8").splitlines()
        if line
    ]
    arguments.capture_root.mkdir(parents=True, exist_ok=True)
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("w", encoding="utf-8", newline="\n") as output:
        for record in records:
            result = run_problem(
                record,
                arguments.corpus_root,
                arguments.executable,
                arguments.capture_root,
                arguments.cpu_seconds,
                arguments.wall_seconds,
                arguments.capture_max,
                arguments.cpu,
            )
            output.write(json.dumps(result, sort_keys=True) + "\n")
            output.flush()
            os.fsync(output.fileno())
            print(
                json.dumps(
                    {
                        "problem": result["problem_id"],
                        "outcome": result["outcome"],
                        "captures": len(result["captures"]),
                    },
                    sort_keys=True,
                ),
                flush=True,
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
