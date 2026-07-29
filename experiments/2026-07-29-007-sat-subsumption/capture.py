#!/usr/bin/env python3
"""Capture real Umlaut subsumption calls on the frozen CASC-30 split."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
SELECTION_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-008-stronger-redundancy"
    / "run.py"
)
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)")
RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
PHASES = {
    "calibration": {
        "split": "train",
        "count": 24,
        "soft_seconds": 5,
        "hard_seconds": 7,
    },
    "validation": {
        "split": "validation",
        "count": 24,
        "soft_seconds": 5,
        "hard_seconds": 7,
    },
    "test": {
        "split": "test",
        "count": 20,
        "soft_seconds": 10,
        "hard_seconds": 13,
    },
}
COMMON_ARGS = [
    "--expert-heuristic=(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))",
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
]


class CaptureError(RuntimeError):
    """A corpus, contract, execution, or capture failure."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CaptureError(f"cannot load selection helper: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SELECTION = load_module("sat_subsumption_selection", SELECTION_PATH)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json(value) + b"\n")


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows or rows[0].get("record_type") != "manifest":
        raise CaptureError(f"invalid manifest: {path}")
    if rows[0].get("problem_count") != len(rows) - 1:
        raise CaptureError("manifest problem count mismatch")
    return rows[0], rows[1:]


def verify_problem(problem_root: Path, record: dict[str, Any]) -> None:
    problem = problem_root / record["path"]
    if not problem.is_file() or sha256_file(problem) != record["sha256"]:
        raise CaptureError(f"problem hash mismatch: {record['problem_id']}")
    for include in record["includes"]:
        include_path = (
            problem_root / "problems" / "casc_2025" / include
        )
        if not include_path.is_file():
            raise CaptureError(f"missing include: {include_path}")


def parse_status(stdout: bytes) -> str | None:
    statuses = SZS_RE.findall(stdout.decode("utf-8", errors="replace"))
    return statuses[-1] if statuses else None


def parse_rss(path: Path) -> int | None:
    if not path.is_file():
        return None
    match = RSS_RE.search(path.read_text(encoding="utf-8", errors="replace"))
    return int(match.group(1)) if match else None


def validate_capture(path: Path, problem_id: str) -> tuple[int, str | None]:
    if not path.is_file():
        return 0, None
    count = 0
    digests: dict[str, tuple[str, str]] = {}
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, start=1):
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                raise CaptureError(
                    f"malformed capture {path}:{line_number}: {error}"
                ) from error
            if record.get("schema_version") != 1:
                raise CaptureError(f"unsupported capture schema: {path}")
            if record.get("problem") != problem_id:
                raise CaptureError(f"capture problem mismatch: {path}")
            digest = record.get("digest")
            payload = (record.get("side"), record.get("main"))
            if not isinstance(digest, str) or not all(
                isinstance(value, str) for value in payload
            ):
                raise CaptureError(f"invalid capture payload: {path}")
            previous = digests.setdefault(digest, payload)
            if previous != payload:
                raise CaptureError(f"digest collision with different payload: {path}")
            count += 1
    return count, sha256_file(path)


def run_one(
    *,
    binary: Path,
    binary_sha256: str,
    problem_root: Path,
    phase_root: Path,
    contract_id: str,
    record: dict[str, Any],
    phase: str,
    soft_seconds: int,
    hard_seconds: int,
    memory_mib: int,
    capture_limit: int,
) -> dict[str, Any]:
    run_root = phase_root / "runs" / record["family"] / record["problem_id"]
    run_root.mkdir(parents=True, exist_ok=True)
    capture_path = run_root / "capture.jsonl"
    telemetry_path = run_root / "telemetry.json"
    stdout_path = run_root / "stdout.txt"
    stderr_path = run_root / "stderr.txt"
    time_path = run_root / "time.txt"
    for path in (capture_path, telemetry_path, stdout_path, stderr_path, time_path):
        path.unlink(missing_ok=True)

    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(time_path),
        str(binary),
        *COMMON_ARGS,
        f"--soft-cpu-limit={soft_seconds}",
        f"--cpu-limit={hard_seconds}",
        f"--memory-limit={memory_mib}",
        f"--search-telemetry={telemetry_path}",
        str(problem_root / record["path"]),
    ]
    environment = os.environ.copy()
    environment.update(
        {
            "TPTP": str(problem_root / "problems" / "casc_2025"),
            "UMLAUT_SAT_SUBSUMPTION_CAPTURE": str(capture_path),
            "UMLAUT_SAT_SUBSUMPTION_CAPTURE_LIMIT": str(capture_limit),
            "UMLAUT_SAT_SUBSUMPTION_PROBLEM": record["problem_id"],
        }
    )
    started_at = datetime.now(UTC).isoformat(timespec="seconds")
    started = time.monotonic()
    external_timeout = False
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=hard_seconds + 15,
            check=False,
        )
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        external_timeout = True
        return_code = None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    wall_seconds = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    capture_count, capture_sha256 = validate_capture(
        capture_path, record["problem_id"]
    )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "phase": phase,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "family": record["family"],
        "category": record["category"],
        "holdout_split": record["holdout_split"],
        "binary_sha256": binary_sha256,
        "command": command,
        "started_at": started_at,
        "return_code": return_code,
        "external_timeout": external_timeout,
        "wall_seconds": wall_seconds,
        "szs_status": parse_status(stdout),
        "capture_count": capture_count,
        "capture_sha256": capture_sha256,
        "telemetry_sha256": (
            sha256_file(telemetry_path) if telemetry_path.is_file() else None
        ),
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
        "maximum_rss_kib": parse_rss(time_path),
    }
    write_json(run_root / "result.json", result)
    return result


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=tuple(PHASES), required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--memory-mib", type=int, default=1_536)
    parser.add_argument("--capture-limit", type=int, default=2_048)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.workers < 1 or arguments.capture_limit < 1:
        raise CaptureError("workers and capture limit must be positive")
    phase = PHASES[arguments.phase]
    manifest = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    output_root = arguments.output_root.resolve()
    if sys.platform != "linux":
        raise CaptureError("prover capture may run only on Linux")
    if not binary.is_file():
        raise CaptureError(f"missing binary: {binary}")

    metadata, records = load_manifest(manifest)
    selected = SELECTION.select_records(
        records, phase["split"], phase["count"]
    )
    for record in selected:
        verify_problem(problem_root, record)

    contract_body = {
        "schema_version": 1,
        "phase": arguments.phase,
        "split": phase["split"],
        "problem_ids": [record["problem_id"] for record in selected],
        "problem_hashes": [record["sha256"] for record in selected],
        "manifest_sha256": sha256_file(manifest),
        "manifest_problem_count": metadata["problem_count"],
        "selection_helper_sha256": sha256_file(SELECTION_PATH),
        "binary_sha256": sha256_file(binary),
        "common_args": COMMON_ARGS,
        "soft_seconds": phase["soft_seconds"],
        "hard_seconds": phase["hard_seconds"],
        "memory_mib": arguments.memory_mib,
        "capture_limit": arguments.capture_limit,
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    contract = {**contract_body, "contract_id": contract_id}
    phase_root = output_root / arguments.phase
    phase_root.mkdir(parents=True, exist_ok=True)
    write_json(phase_root / "contract.json", contract)

    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=arguments.workers
    ) as executor:
        futures = [
            executor.submit(
                run_one,
                binary=binary,
                binary_sha256=contract_body["binary_sha256"],
                problem_root=problem_root,
                phase_root=phase_root,
                contract_id=contract_id,
                record=record,
                phase=arguments.phase,
                soft_seconds=phase["soft_seconds"],
                hard_seconds=phase["hard_seconds"],
                memory_mib=arguments.memory_mib,
                capture_limit=arguments.capture_limit,
            )
            for record in selected
        ]
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            results.append(result)
            print(
                f"{result['problem_id']}: status={result['szs_status']} "
                f"capture={result['capture_count']} "
                f"wall={result['wall_seconds']:.3f}s",
                flush=True,
            )
    results.sort(key=lambda result: result["problem_id"])
    results_path = phase_root / "results.jsonl"
    results_path.write_bytes(
        b"".join(canonical_json(result) + b"\n" for result in results)
    )
    if not any(result["capture_count"] for result in results):
        raise CaptureError("phase produced no subsumption captures")
    summary = {
        "schema_version": 1,
        "phase": arguments.phase,
        "contract_id": contract_id,
        "problems": len(results),
        "records": sum(result["capture_count"] for result in results),
        "external_timeouts": sum(
            int(result["external_timeout"]) for result in results
        ),
        "maximum_rss_kib": max(
            (
                result["maximum_rss_kib"]
                for result in results
                if result["maximum_rss_kib"] is not None
            ),
            default=None,
        ),
        "results_sha256": sha256_file(results_path),
    }
    write_json(phase_root / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CaptureError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
