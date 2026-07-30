#!/usr/bin/env python3
"""Capture deterministic production Umlaut CNF transcripts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any, Sequence


class CaptureError(RuntimeError):
    """CNF capture failed or violated source identity."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("utf-8")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True, type=Path)
    parser.add_argument("--selection", required=True, type=Path)
    parser.add_argument("--umlaut", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    binary = arguments.umlaut.resolve()
    selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["TPTP"] = str(repo_root / "problems/casc_2025")
    records = []
    for source in selection["sources"]:
        path = repo_root / source["path"]
        source_bytes = path.read_bytes()
        if sha256_bytes(source_bytes) != source["source_sha256"]:
            raise CaptureError(f"source hash mismatch for {source['problem_id']}")
        command = [str(binary), "--cnf", "--tstp-out", str(path)]
        started = time.perf_counter_ns()
        try:
            completed = subprocess.run(
                command,
                cwd=repo_root,
                check=False,
                capture_output=True,
                env=environment,
                timeout=arguments.timeout_seconds,
            )
            timed_out = False
        except subprocess.TimeoutExpired as error:
            completed = error
            timed_out = True
        elapsed_ns = time.perf_counter_ns() - started
        stdout = completed.stdout or b""
        stderr = completed.stderr or b""
        source_root = arguments.output_root / source["problem_id"]
        source_root.mkdir(parents=True, exist_ok=True)
        (source_root / "stdout.txt").write_bytes(stdout)
        (source_root / "stderr.txt").write_bytes(stderr)
        return_code = None if timed_out else completed.returncode
        record = {
            **source,
            "command": ["umlaut", "--cnf", "--tstp-out", source["path"]],
            "elapsed_ns": elapsed_ns,
            "return_code": return_code,
            "timed_out": timed_out,
            "stdout_bytes": len(stdout),
            "stdout_sha256": sha256_bytes(stdout),
            "stderr_bytes": len(stderr),
            "stderr_sha256": sha256_bytes(stderr),
        }
        (source_root / "metadata.json").write_bytes(canonical_bytes(record))
        records.append(record)
    report = {
        "schema": "umlaut-real-ground-cnf-capture-v1",
        "selection_sha256": sha256_file(arguments.selection),
        "umlaut_sha256": sha256_file(binary),
        "records": records,
    }
    (arguments.output_root / "capture.json").write_bytes(canonical_bytes(report))
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
