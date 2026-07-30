#!/usr/bin/env python3
"""Run production Umlaut and pinned Vampire arms on selected source problems."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import time
from pathlib import Path
from typing import Any


SZS_PATTERN = re.compile(r"SZS status\s+(\S+)", re.IGNORECASE)


def run(
    command: list[str],
    *,
    repository: Path,
    environment: dict[str, str],
    timeout_seconds: float,
) -> dict[str, Any]:
    started = time.perf_counter_ns()
    try:
        result = subprocess.run(
            command,
            cwd=repository,
            env=environment,
            capture_output=True,
            check=False,
            text=True,
            timeout=timeout_seconds,
        )
        timed_out = False
        returncode = result.returncode
        stdout = result.stdout
        stderr = result.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        returncode = None
        stdout = error.stdout.decode() if isinstance(error.stdout, bytes) else (error.stdout or "")
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else (error.stderr or "")
    match = SZS_PATTERN.search(stdout)
    status = match.group(1) if match else None
    return {
        "command": command,
        "returncode": returncode,
        "timed_out": timed_out,
        "elapsed_ms": (time.perf_counter_ns() - started) / 1_000_000,
        "szs_status": status,
        "stdout": stdout,
        "stderr": stderr,
        "stdout_sha256": hashlib.sha256(stdout.encode("utf-8")).hexdigest(),
        "stderr_sha256": hashlib.sha256(stderr.encode("utf-8")).hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--limit-seconds", type=float, default=5.0)
    arguments = parser.parse_args()
    repository = arguments.repository.resolve()
    selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
    environment = os.environ.copy()
    environment["TPTP"] = str(repository / "problems/casc_2025")
    reports: list[dict[str, Any]] = []
    for source in selection["selected"]:
        problem = repository / source["path"]
        common_vampire = [
            str(arguments.vampire.resolve()),
            "--mode",
            "vampire",
            "--time_limit",
            str(arguments.limit_seconds),
            "--random_seed",
            "0",
            "--proof",
            "off",
        ]
        arms = {
            "production_umlaut": [
                str(arguments.umlaut.resolve()),
                "--auto",
                "--output-level=1",
                f"--cpu-limit={max(1, int(arguments.limit_seconds))}",
                "--memory-limit=2048",
                str(problem),
            ],
            "vampire_theory_axioms": [
                *common_vampire,
                "--abstracting_linear_arithmetic_superposition_calculus",
                "off",
                "--theory_axioms",
                "on",
                str(problem),
            ],
            "vampire_alasca_no_viras": [
                *common_vampire,
                "--abstracting_linear_arithmetic_superposition_calculus",
                "on",
                "--virtual_integer_real_arithmetic_substitution",
                "off",
                "--theory_axioms",
                "off",
                str(problem),
            ],
        }
        reports.append(
            {
                **source,
                "arms": {
                    name: run(
                        command,
                        repository=repository,
                        environment=environment,
                        timeout_seconds=arguments.limit_seconds + 2,
                    )
                    for name, command in arms.items()
                },
            }
        )
    summary = {
        name: {
            "solved": sum(
                report["arms"][name]["szs_status"]
                in {"Theorem", "Unsatisfiable", "CounterSatisfiable", "Satisfiable"}
                for report in reports
            ),
            "theorem_or_unsat": sum(
                report["arms"][name]["szs_status"] in {"Theorem", "Unsatisfiable"}
                for report in reports
            ),
            "timed_out": sum(
                report["arms"][name]["timed_out"] for report in reports
            ),
        }
        for name in (
            "production_umlaut",
            "vampire_theory_axioms",
            "vampire_alasca_no_viras",
        )
    }
    result = {"summary": summary, "sources": reports}
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
