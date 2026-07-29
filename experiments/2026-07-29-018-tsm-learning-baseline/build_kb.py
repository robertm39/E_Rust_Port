#!/usr/bin/env python3
"""Build a proof-derived TSM KB from the frozen training split on Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence


PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
BASE_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
FIFO = "FIFOWeight(ConstPrio)"
TRAINING_ARGS = (
    f"--expert-heuristic=(5*{BASE_WEIGHT},1*{FIFO})",
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
    "--pcl-out",
    "--record-gcs",
    "--proof-object=1",
    "--force-deriv=2",
)


class ExperimentError(RuntimeError):
    """Raised when the frozen training contract cannot be satisfied."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(value) + b"\n")
    temporary.replace(path)


def utc_now() -> str:
    return datetime.now(UTC).isoformat()


def final_status(*texts: str) -> str | None:
    matches: list[str] = []
    for text in texts:
        matches.extend(match.group(1) for match in SZS_RE.finditer(text))
    return matches[-1] if matches else None


def load_corpus(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    if not rows or rows[0].get("record_type") != "manifest":
        raise ExperimentError("invalid frozen corpus")
    records = [
        record for record in rows[1:] if record["experiment_split"] == "train"
    ]
    if len(records) != 16:
        raise ExperimentError(f"expected 16 training records, found {len(records)}")
    return rows[0], records


def verify_inputs(
    problem_root: Path, records: Sequence[dict[str, Any]]
) -> None:
    for record in records:
        problem = problem_root / record["path"]
        if not problem.is_file() or sha256_file(problem) != record["sha256"]:
            raise ExperimentError(f"problem mismatch: {record['problem_id']}")
        for include in record["includes"]:
            include_path = problem_root / "problems" / "casc_2025" / include
            if not include_path.is_file():
                raise ExperimentError(f"missing include: {include_path}")


def run_command(
    command: Sequence[str],
    *,
    cwd: Path,
    environment: dict[str, str],
    timeout: int,
) -> subprocess.CompletedProcess[bytes]:
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        return subprocess.CompletedProcess(
            command,
            124,
            stdout=error.stdout or b"",
            stderr=error.stderr or b"",
        )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--kb-create", type=Path, required=True)
    parser.add_argument("--kb-ginsert", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--soft-cpu-seconds", type=int, default=8)
    parser.add_argument("--hard-cpu-seconds", type=int, default=10)
    parser.add_argument("--memory-mib", type=int, default=1536)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("prover training may run only on Linux")
    if arguments.source_revision != "812323618aaa42d0f5e24bba8a0ef146ff1757cd":
        raise ExperimentError("source revision differs from preregistration")

    corpus = arguments.corpus.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    kb_create = arguments.kb_create.resolve()
    kb_ginsert = arguments.kb_ginsert.resolve()
    output_root = arguments.output_root.resolve()
    for executable in (binary, kb_create, kb_ginsert):
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ExperimentError(f"missing executable: {executable}")
    metadata, records = load_corpus(corpus)
    verify_inputs(problem_root, records)

    contract_body = {
        "schema_version": 1,
        "source_revision": arguments.source_revision,
        "corpus_sha256": sha256_file(corpus),
        "problem_ids": [record["problem_id"] for record in records],
        "training_families": metadata["selected_families"]["train"],
        "binary_sha256": sha256_file(binary),
        "kb_create_sha256": sha256_file(kb_create),
        "kb_ginsert_sha256": sha256_file(kb_ginsert),
        "training_args": list(TRAINING_ARGS),
        "soft_cpu_seconds": arguments.soft_cpu_seconds,
        "hard_cpu_seconds": arguments.hard_cpu_seconds,
        "memory_mib": arguments.memory_mib,
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    contract_path = output_root / "contract.json"
    if contract_path.is_file():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        if {
            key: value
            for key, value in existing.items()
            if key != "created_at"
        } != {**contract_body, "contract_id": contract_id}:
            raise ExperimentError("output root contains an incompatible contract")
    else:
        write_json(
            contract_path,
            {
                **contract_body,
                "contract_id": contract_id,
                "created_at": utc_now(),
            },
        )

    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    output_root.mkdir(parents=True, exist_ok=True)
    kb_path = output_root / "E_KNOWLEDGE"
    if not kb_path.exists():
        created = run_command(
            [str(kb_create), "E_KNOWLEDGE"],
            cwd=output_root,
            environment=environment,
            timeout=30,
        )
        (output_root / "kb-create.stdout").write_bytes(created.stdout)
        (output_root / "kb-create.stderr").write_bytes(created.stderr)
        if created.returncode != 0:
            raise ExperimentError("umlaut-kb-create failed")

    solved = 0
    inserted = 0
    solved_categories: set[str] = set()
    for index, record in enumerate(records, start=1):
        run_dir = output_root / "runs" / record["category"] / record["problem_id"]
        run_dir.mkdir(parents=True, exist_ok=True)
        trace_path = run_dir / "trace.pcl"
        command = [
            str(binary),
            *TRAINING_ARGS,
            f"--soft-cpu-limit={arguments.soft_cpu_seconds}",
            f"--cpu-limit={arguments.hard_cpu_seconds}",
            f"--memory-limit={arguments.memory_mib}",
            f"--output-file={trace_path}",
            str(problem_root / record["path"]),
        ]
        started = time.monotonic()
        completed = run_command(
            command,
            cwd=output_root,
            environment=environment,
            timeout=arguments.hard_cpu_seconds + 10,
        )
        wall_seconds = time.monotonic() - started
        stdout_path = run_dir / "stdout.txt"
        stderr_path = run_dir / "stderr.txt"
        stdout_path.write_bytes(completed.stdout)
        stderr_path.write_bytes(completed.stderr)
        trace_text = (
            trace_path.read_text(encoding="utf-8", errors="replace")
            if trace_path.is_file()
            else ""
        )
        status = final_status(
            completed.stdout.decode("utf-8", errors="replace"),
            trace_text,
        )
        is_proof = status in PROOF_STATUSES
        insert_return_code: int | None = None
        if is_proof:
            solved += 1
            solved_categories.add(str(record["category"]))
            inserted_process = run_command(
                [
                    str(kb_ginsert),
                    "-k",
                    "E_KNOWLEDGE",
                    "-n",
                    str(record["problem_id"]),
                    str(trace_path),
                ],
                cwd=output_root,
                environment=environment,
                timeout=60,
            )
            insert_return_code = inserted_process.returncode
            (run_dir / "insert.stdout").write_bytes(inserted_process.stdout)
            (run_dir / "insert.stderr").write_bytes(inserted_process.stderr)
            if insert_return_code != 0:
                raise ExperimentError(
                    f"KB insertion failed for {record['problem_id']}"
                )
            inserted += 1
        write_json(
            run_dir / "result.json",
            {
                "schema_version": 1,
                "contract_id": contract_id,
                "problem_id": record["problem_id"],
                "problem_sha256": record["sha256"],
                "family": record["family"],
                "category": record["category"],
                "command": command,
                "return_code": completed.returncode,
                "wall_seconds": wall_seconds,
                "szs_status": status,
                "proof": is_proof,
                "insert_return_code": insert_return_code,
                "trace_sha256": sha256_file(trace_path)
                if trace_path.is_file()
                else None,
                "stdout_sha256": sha256_file(stdout_path),
                "stderr_sha256": sha256_file(stderr_path),
            },
        )
        print(
            f"training: {index}/{len(records)} {record['problem_id']} "
            f"{status or 'NO_STATUS'}",
            flush=True,
        )

    if solved < 4 or len(solved_categories) < 2:
        raise ExperimentError(
            f"training proof gate failed: {solved} proofs, "
            f"{len(solved_categories)} categories"
        )
    clausepatterns = kb_path / "clausepatterns"
    summary = {
        "schema_version": 1,
        "contract_id": contract_id,
        "attempted": len(records),
        "solved": solved,
        "inserted": inserted,
        "solved_categories": sorted(solved_categories),
        "knowledge_base": str(kb_path),
        "clausepatterns_sha256": sha256_file(clausepatterns),
        "clausepatterns_bytes": clausepatterns.stat().st_size,
    }
    write_json(output_root / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
