#!/usr/bin/env python3
"""Capture production Umlaut CNF for the preregistered source selection."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True, type=Path)
    parser.add_argument("--selection", required=True, type=Path)
    parser.add_argument("--umlaut", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    arguments = parser.parse_args()
    repository = arguments.repo_root.resolve()
    binary = arguments.umlaut.resolve()
    selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["TPTP"] = str(repository / "problems/casc_2025")
    records: list[dict[str, Any]] = []
    for source in selection["selected"]:
        path = repository / source["path"]
        if sha256_file(path) != source["sha256"]:
            raise ValueError(f"source hash mismatch for {source['problem_id']}")
        command = [str(binary), "--cnf", "--tstp-out", str(path)]
        started = time.perf_counter_ns()
        try:
            result = subprocess.run(
                command,
                cwd=repository,
                capture_output=True,
                check=False,
                env=environment,
                timeout=arguments.timeout_seconds,
            )
            timed_out = False
            return_code = result.returncode
            stdout = result.stdout
            stderr = result.stderr
        except subprocess.TimeoutExpired as error:
            timed_out = True
            return_code = None
            stdout = error.stdout or b""
            stderr = error.stderr or b""
        directory = arguments.output_root / source["problem_id"]
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "stdout.txt").write_bytes(stdout)
        (directory / "stderr.txt").write_bytes(stderr)
        record = {
            **source,
            "command": ["umlaut", "--cnf", "--tstp-out", source["path"]],
            "elapsed_ns": time.perf_counter_ns() - started,
            "return_code": return_code,
            "timed_out": timed_out,
            "stdout_bytes": len(stdout),
            "stdout_sha256": sha256_bytes(stdout),
            "stderr_bytes": len(stderr),
            "stderr_sha256": sha256_bytes(stderr),
        }
        (directory / "metadata.json").write_text(
            json.dumps(record, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        records.append(record)
    report = {
        "schema": "umlaut-rational-fm-cnf-capture-v1",
        "selection_sha256": sha256_file(arguments.selection),
        "umlaut_sha256": sha256_file(binary),
        "records": records,
    }
    (arguments.output_root / "capture.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    failed = [
        record["problem_id"]
        for record in records
        if record["timed_out"] or record["return_code"] != 0
    ]
    print(
        json.dumps(
            {
                "sources": len(records),
                "failed": failed,
                "stdout_bytes": sum(record["stdout_bytes"] for record in records),
            },
            sort_keys=True,
        )
    )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
