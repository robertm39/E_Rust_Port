#!/usr/bin/env python3
"""Run bounded proving probes for J13 THF syntax failures from an audit."""

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

from audit_j13_thf_syntax import canonical_json, load_thf_records, sha256_file


def decoded_output(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value or ""


def terminal_statuses(stdout: str, stderr: str) -> list[str]:
    return re.findall(r"\bSZS\s+status\s+([A-Za-z]+)", f"{stdout}\n{stderr}")


def classify(
    return_code: int | None,
    stdout: str,
    stderr: str,
    statuses: list[str],
) -> str:
    output = f"{stdout}\n{stderr}"
    if return_code is None:
        return "external_timeout"
    if return_code == 3 or "Too many arguments applied to the term" in output:
        return "parser_failure"
    if statuses:
        return "entered_proving"
    return "no_terminal_status"


def run_problem(
    *,
    binary: Path,
    problem: Path,
    problem_id: str,
    cpu_limit_seconds: int,
    memory_limit_mib: int,
    timeout_seconds: float,
    environment: dict[str, str],
) -> dict[str, Any]:
    command = [
        str(binary),
        "--auto-schedule=1",
        "--silent",
        "--resources-info",
        "--proof-object",
        "--tstp-format",
        f"--cpu-limit={cpu_limit_seconds}",
        f"--memory-limit={memory_limit_mib}",
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
    statuses = terminal_statuses(stdout, stderr)
    return {
        "classification": classify(return_code, stdout, stderr, statuses),
        "command": command,
        "problem_id": problem_id,
        "return_code": return_code,
        "statuses": statuses,
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
    parser.add_argument("--selection-audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cpu-limit-seconds", type=int, default=1)
    parser.add_argument("--memory-limit-mib", type=int, default=2048)
    parser.add_argument("--timeout-seconds", type=float, default=15.0)
    parser.add_argument("--source-snapshot-sha256", required=True)
    parser.add_argument("--require-all-entered", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.cpu_limit_seconds <= 0:
        raise ValueError("CPU limit must be positive")
    if arguments.memory_limit_mib < 20:
        raise ValueError("memory limit must be at least 20 MiB")
    if arguments.timeout_seconds <= arguments.cpu_limit_seconds:
        raise ValueError("external timeout must exceed the CPU limit")
    if re.fullmatch(r"[0-9a-f]{64}", arguments.source_snapshot_sha256) is None:
        raise ValueError("source snapshot SHA-256 must be 64 lowercase hex digits")

    binary = arguments.binary.resolve(strict=True)
    manifest = arguments.manifest.resolve(strict=True)
    problem_root = arguments.problem_root.resolve(strict=True)
    selection_path = arguments.selection_audit.resolve(strict=True)
    probe_script = Path(__file__).resolve(strict=True)
    if not binary.is_file():
        raise ValueError(f"binary is not a regular file: {binary}")
    if not manifest.is_file():
        raise ValueError(f"manifest is not a regular file: {manifest}")
    if not selection_path.is_file():
        raise ValueError(f"selection audit is not a regular file: {selection_path}")

    metadata, records = load_thf_records(manifest)
    records_by_id = {record["problem_id"]: record for record in records}
    if len(records_by_id) != len(records):
        raise ValueError("manifest contains duplicate THF problem identifiers")
    selection = json.loads(selection_path.read_text(encoding="utf-8"))
    if selection.get("kind") != "umlaut-j13-thf-syntax-audit":
        raise ValueError("selection input is not a J13 THF syntax audit")
    if selection.get("manifest", {}).get("sha256") != sha256_file(manifest):
        raise ValueError("selection audit manifest hash does not match")
    selected_ids = [
        result["problem_id"]
        for result in selection.get("results", [])
        if result.get("classification") == "too_many_arguments"
    ]
    if not selected_ids:
        raise ValueError("selection audit contains no overapplication failures")
    if len(set(selected_ids)) != len(selected_ids):
        raise ValueError("selection audit contains duplicate selected identifiers")

    corpus_root = (problem_root / metadata["sources"]["corpus_root"]).resolve(
        strict=True
    )
    if not corpus_root.is_relative_to(problem_root):
        raise ValueError(f"corpus root escapes problem root: {corpus_root}")
    environment = os.environ.copy()
    environment["TPTP"] = str(corpus_root)

    results = []
    for index, problem_id in enumerate(selected_ids, start=1):
        record = records_by_id.get(problem_id)
        if record is None:
            raise ValueError(f"selected problem is not a manifest THF record: {problem_id}")
        problem = (problem_root / record["path"]).resolve(strict=True)
        if not problem.is_relative_to(problem_root):
            raise ValueError(f"problem escapes problem root: {record['path']}")
        problem_sha256 = sha256_file(problem)
        if problem_sha256 != record["sha256"]:
            raise ValueError(f"problem hash mismatch: {problem_id}")
        result = run_problem(
            binary=binary,
            problem=problem,
            problem_id=problem_id,
            cpu_limit_seconds=arguments.cpu_limit_seconds,
            memory_limit_mib=arguments.memory_limit_mib,
            timeout_seconds=arguments.timeout_seconds,
            environment=environment,
        )
        result["path"] = record["path"]
        result["problem_sha256"] = problem_sha256
        results.append(result)
        print(
            f"{index:03d}/{len(selected_ids)} {problem_id} "
            f"{result['classification']} {result['wall_seconds']:.3f}s",
            flush=True,
        )

    counts = collections.Counter(result["classification"] for result in results)
    evidence = {
        "binary": {"path": str(binary), "sha256": sha256_file(binary)},
        "captured_at": datetime.now(UTC).isoformat(timespec="seconds").replace(
            "+00:00", "Z"
        ),
        "classification_counts": dict(sorted(counts.items())),
        "cpu_limit_seconds": arguments.cpu_limit_seconds,
        "host": {"platform": platform.platform(), "python": sys.version},
        "kind": "umlaut-j13-thf-proving-probe",
        "manifest": {"path": str(manifest), "sha256": sha256_file(manifest)},
        "memory_limit_mib": arguments.memory_limit_mib,
        "probe_count": len(results),
        "probe_script": {"path": str(probe_script), "sha256": sha256_file(probe_script)},
        "results": results,
        "schema_version": 1,
        "selection_audit": {
            "path": str(selection_path),
            "sha256": sha256_file(selection_path),
        },
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
    if arguments.require_all_entered and not (
        counts.get("entered_proving") == len(results) and len(counts) == 1
    ):
        print("error: not every selected problem entered proving", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
