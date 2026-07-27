#!/usr/bin/env python3
"""Run the two maintained Linux allocation-boundary cases."""

from __future__ import annotations

import argparse
import hashlib
import json
import resource
import subprocess
import time
from pathlib import Path


def run_case(binary: Path, problem: Path, output_dir: Path) -> dict[str, object]:
    started = time.perf_counter()
    completed = subprocess.run(
        [
            binary,
            problem,
            "--auto",
            "--silent",
            "--cpu-limit=60",
            "--memory-limit=2048",
            "--detsort-rw",
            "--detsort-new",
            "--proof-object=1",
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    wall_seconds = time.perf_counter() - started
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    stem = problem.stem
    (output_dir / f"{stem}.stdout").write_bytes(completed.stdout)
    (output_dir / f"{stem}.stderr").write_bytes(completed.stderr)
    return {
        "problem": problem.name,
        "status": completed.returncode,
        "wall_seconds": wall_seconds,
        "child_max_rss_kib": usage.ru_maxrss,
        "stdout_len": len(completed.stdout),
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "stdout_contains_resource_out": b"SZS status ResourceOut" in completed.stdout,
        "stderr_len": len(completed.stderr),
        "stderr_sha256": hashlib.sha256(completed.stderr).hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()

    arguments.output_dir.mkdir(parents=True, exist_ok=True)
    problems = [
        arguments.source_root
        / "eprover"
        / "EXAMPLE_PROBLEMS"
        / "SMOKETEST"
        / "BOO020-1.p",
        arguments.source_root
        / "eprover"
        / "EXAMPLE_PROBLEMS"
        / "TPTP"
        / "SWV851-1.p",
    ]
    results = [
        run_case(arguments.binary, problem, arguments.output_dir)
        for problem in problems
    ]
    serialized = json.dumps(results, indent=2, sort_keys=True) + "\n"
    (arguments.output_dir / "summary.json").write_text(serialized, encoding="utf-8")
    print(serialized, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
