#!/usr/bin/env python3
"""Audit syntax-only Umlaut coverage for every J13 THF problem."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def decoded_output(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value or ""


def load_thf_records(manifest: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    values = [
        json.loads(line)
        for line in manifest.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not values or values[0].get("record_type") != "manifest":
        raise ValueError("manifest metadata record is missing")
    records = [
        value
        for value in values[1:]
        if value.get("record_type") == "problem" and value.get("division") == "THF"
    ]
    return values[0], records


def classify(return_code: int | None, stdout: str, stderr: str) -> str:
    if return_code is None:
        return "timeout"
    if return_code == 0:
        return "accepted"
    if "Too many arguments applied to the term" in f"{stdout}\n{stderr}":
        return "too_many_arguments"
    return "error"


def run_problem(
    *,
    binary: Path,
    problem: Path,
    problem_id: str,
    timeout_seconds: float,
    environment: dict[str, str],
) -> dict[str, Any]:
    command = [
        str(binary),
        "--syntax-only",
        "--silent",
        "--",
        str(problem),
    ]
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
            timeout=timeout_seconds,
        )
        return_code: int | None = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        return_code = None
        stdout = decoded_output(error.stdout)
        stderr = decoded_output(error.stderr)
    wall_seconds = time.monotonic() - started
    return {
        "classification": classify(return_code, stdout, stderr),
        "command": command,
        "problem_id": problem_id,
        "return_code": return_code,
        "stderr": stderr,
        "stderr_sha256": hashlib.sha256(stderr.encode()).hexdigest(),
        "stdout": stdout,
        "stdout_sha256": hashlib.sha256(stdout.encode()).hexdigest(),
        "wall_seconds": wall_seconds,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    parser.add_argument("--expected-problems", type=int, default=400)
    parser.add_argument("--source-snapshot-sha256")
    parser.add_argument("--require-no-too-many-arguments", action="store_true")
    parser.add_argument("--require-all-success", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.timeout_seconds <= 0:
        raise ValueError("timeout must be positive")
    if arguments.expected_problems <= 0:
        raise ValueError("expected problem count must be positive")
    if arguments.source_snapshot_sha256 is not None and re.fullmatch(
        r"[0-9a-f]{64}", arguments.source_snapshot_sha256
    ) is None:
        raise ValueError("source snapshot SHA-256 must be 64 lowercase hex digits")
    binary = arguments.binary.resolve(strict=True)
    manifest = arguments.manifest.resolve(strict=True)
    problem_root = arguments.problem_root.resolve(strict=True)
    audit_script = Path(__file__).resolve(strict=True)
    if not binary.is_file():
        raise ValueError(f"binary is not a regular file: {binary}")
    if not manifest.is_file():
        raise ValueError(f"manifest is not a regular file: {manifest}")
    metadata, records = load_thf_records(manifest)
    if len(records) != arguments.expected_problems:
        raise ValueError(
            f"THF problem count mismatch: {len(records)} != {arguments.expected_problems}"
        )

    corpus_root = (problem_root / metadata["sources"]["corpus_root"]).resolve(
        strict=True
    )
    if not corpus_root.is_relative_to(problem_root):
        raise ValueError(f"corpus root escapes problem root: {corpus_root}")
    environment = os.environ.copy()
    environment["TPTP"] = str(corpus_root)
    results = []
    for index, record in enumerate(records, start=1):
        problem = (problem_root / record["path"]).resolve(strict=True)
        if not problem.is_relative_to(problem_root):
            raise ValueError(f"problem escapes problem root: {record['path']}")
        problem_sha256 = sha256_file(problem)
        if problem_sha256 != record["sha256"]:
            raise ValueError(f"problem hash mismatch: {record['problem_id']}")
        result = run_problem(
            binary=binary,
            problem=problem,
            problem_id=record["problem_id"],
            timeout_seconds=arguments.timeout_seconds,
            environment=environment,
        )
        result["path"] = record["path"]
        result["problem_sha256"] = problem_sha256
        results.append(result)
        print(
            f"{index:03d}/{len(records)} {record['problem_id']} "
            f"{result['classification']} {result['wall_seconds']:.3f}s",
            flush=True,
        )

    counts = collections.Counter(result["classification"] for result in results)
    evidence = {
        "audit_script": {
            "path": str(audit_script),
            "sha256": sha256_file(audit_script),
        },
        "binary": {
            "path": str(binary),
            "sha256": sha256_file(binary),
        },
        "captured_at": datetime.now(UTC).isoformat(timespec="seconds").replace(
            "+00:00", "Z"
        ),
        "classification_counts": dict(sorted(counts.items())),
        "kind": "umlaut-j13-thf-syntax-audit",
        "host": {
            "platform": platform.platform(),
            "python": sys.version,
        },
        "manifest": {
            "path": str(manifest),
            "sha256": sha256_file(manifest),
        },
        "problem_count": len(records),
        "results": results,
        "schema_version": 1,
        "source_snapshot_sha256": arguments.source_snapshot_sha256,
        "timeout_seconds": arguments.timeout_seconds,
        "tptp_root": environment["TPTP"],
    }
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(evidence))
    os.replace(temporary, output)
    print(f"OK: {dict(sorted(counts.items()))}; evidence {output}")
    if arguments.require_no_too_many_arguments and counts.get(
        "too_many_arguments", 0
    ):
        print(
            "error: at least one THF problem produced the overapplication diagnostic",
            file=sys.stderr,
        )
        return 1
    if arguments.require_all_success and not (
        counts.get("accepted") == len(records) and len(counts) == 1
    ):
        print("error: not every THF problem passed syntax-only parsing", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
