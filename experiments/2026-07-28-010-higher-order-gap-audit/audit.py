#!/usr/bin/env python3
"""Classify all CASC-30 THF outcomes against a pinned Vampire reference."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from collections import Counter, defaultdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence


SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NON_PROOF_STATUSES = {"CounterSatisfiable", "Satisfiable"}
THF_CATEGORIES = {"TEQ", "TNE"}
TAG_PATTERNS = {
    "lambda": re.compile(r"\^"),
    "explicit_application": re.compile(r"\s@\s"),
    "applied_variable": re.compile(r"\b[A-Z][A-Za-z0-9_]*\s*@"),
    "choice": re.compile(r"@\+|\$choice|\bchoice\b", re.IGNORECASE),
    "equality": re.compile(r"(?<![<~!])=(?!=|>)"),
    "boolean_sort": re.compile(r"\$o\b"),
    "conditional": re.compile(r"\$ite\b|\$ite_f\b"),
}


class AuditError(RuntimeError):
    """An invalid audit contract, corpus, or execution."""


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


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


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(value) + b"\n")
    temporary.replace(path)


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows or rows[0].get("record_type") != "manifest":
        raise AuditError(f"invalid manifest header: {path}")
    records = [
        record
        for record in rows[1:]
        if record.get("category") in THF_CATEGORIES
    ]
    if len(records) != 500:
        raise AuditError(f"expected 500 THF records, found {len(records)}")
    return rows[0], records


def final_status(output: bytes) -> str | None:
    statuses = SZS_RE.findall(output.decode("utf-8", errors="replace"))
    return statuses[-1] if statuses else None


def run_command(
    command: list[str],
    environment: dict[str, str],
    timeout_seconds: int,
) -> dict[str, Any]:
    started = time.monotonic()
    external_timeout = False
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=timeout_seconds,
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
    return {
        "command": command,
        "return_code": return_code,
        "external_timeout": external_timeout,
        "wall_seconds": time.monotonic() - started,
        "stdout": stdout,
        "stderr": stderr,
        "szs_status": final_status(stdout),
    }


def source_tags(
    record: dict[str, Any], problem_root: Path
) -> dict[str, bool]:
    paths = [problem_root / record["path"]]
    paths.extend(
        problem_root / "problems" / "casc_2025" / include
        for include in record["includes"]
    )
    source = "\n".join(
        path.read_text(encoding="utf-8", errors="replace") for path in paths
    )
    return {
        name: pattern.search(source) is not None
        for name, pattern in TAG_PATTERNS.items()
    }


def classify(
    syntax: dict[str, Any],
    umlaut: dict[str, Any],
    vampire: dict[str, Any],
) -> str:
    syntax_text = syntax["stdout"].decode("utf-8", errors="replace")
    syntax_ok = (
        syntax["return_code"] == 0
        and not syntax["external_timeout"]
        and "Parsing successful!" in syntax_text
    )
    if not syntax_ok:
        return "syntax_or_typing_rejection"
    if umlaut["szs_status"] in NON_PROOF_STATUSES:
        return "contradictory_or_unsupported_status"
    if umlaut["szs_status"] in PROOF_STATUSES:
        return "umlaut_solved"
    if (
        not umlaut["external_timeout"]
        and umlaut["return_code"] not in (0, None)
        and umlaut["szs_status"] != "ResourceOut"
    ):
        return "preprocessing_or_initialization_diagnostic"
    if vampire["szs_status"] in PROOF_STATUSES:
        return "search_limited_reference_solved"
    if umlaut["external_timeout"] or vampire["external_timeout"]:
        return "externally_timed_out"
    return "jointly_search_limited"


def result_is_resumable(
    result_path: Path,
    *,
    contract_id: str,
    problem_sha256: str,
    umlaut_sha256: str,
    vampire_sha256: str,
) -> bool:
    if not result_path.is_file():
        return False
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
        run_dir = result_path.parent
        return (
            result["contract_id"] == contract_id
            and result["problem_sha256"] == problem_sha256
            and result["umlaut_sha256"] == umlaut_sha256
            and result["vampire_sha256"] == vampire_sha256
            and all(
                sha256_file(run_dir / filename) == expected
                for filename, expected in result["artifact_sha256"].items()
            )
        )
    except (KeyError, OSError, ValueError, json.JSONDecodeError):
        return False


def run_one(
    *,
    record: dict[str, Any],
    problem_root: Path,
    output_root: Path,
    contract_id: str,
    umlaut_binary: Path,
    umlaut_sha256: str,
    vampire_binary: Path,
    vampire_sha256: str,
    soft_seconds: int,
    hard_seconds: int,
    memory_mib: int,
) -> dict[str, Any]:
    run_dir = (
        output_root
        / "runs"
        / record["category"]
        / record["family"]
        / record["problem_id"]
    )
    result_path = run_dir / "result.json"
    if result_is_resumable(
        result_path,
        contract_id=contract_id,
        problem_sha256=record["sha256"],
        umlaut_sha256=umlaut_sha256,
        vampire_sha256=vampire_sha256,
    ):
        return {"resumed": True, "result_path": str(result_path)}
    run_dir.mkdir(parents=True, exist_ok=True)
    problem_path = problem_root / record["path"]
    if not problem_path.is_file() or sha256_file(problem_path) != record["sha256"]:
        raise AuditError(f"problem hash mismatch: {record['problem_id']}")
    environment = os.environ.copy()
    environment["LC_ALL"] = "C"
    environment["TPTP"] = str(
        problem_root / "problems" / "casc_2025"
    )
    syntax = run_command(
        [str(umlaut_binary), "--syntax-only", str(problem_path)],
        environment,
        15,
    )
    umlaut = run_command(
        [
            str(umlaut_binary),
            "--auto",
            "--silent",
            f"--soft-cpu-limit={soft_seconds}",
            f"--cpu-limit={hard_seconds}",
            f"--memory-limit={memory_mib}",
            str(problem_path),
        ],
        environment,
        hard_seconds + 10,
    )
    vampire = run_command(
        [
            str(vampire_binary),
            "--mode",
            "casc",
            "--time_limit",
            str(soft_seconds),
            "--memory_limit",
            str(memory_mib),
            str(problem_path),
        ],
        environment,
        hard_seconds + 10,
    )
    artifact_sha256: dict[str, str] = {}
    for name, execution in (
        ("syntax", syntax),
        ("umlaut", umlaut),
        ("vampire", vampire),
    ):
        for stream in ("stdout", "stderr"):
            filename = f"{name}-{stream}.txt"
            path = run_dir / filename
            path.write_bytes(execution[stream])
            artifact_sha256[filename] = sha256_file(path)
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "problem_id": record["problem_id"],
        "problem_path": record["path"],
        "problem_sha256": record["sha256"],
        "category": record["category"],
        "family": record["family"],
        "holdout_split": record["holdout_split"],
        "difficulty_band": record["difficulty_band"],
        "expected_class": record["expected_class"],
        "source_tags": source_tags(record, problem_root),
        "taxonomy": classify(syntax, umlaut, vampire),
        "umlaut_sha256": umlaut_sha256,
        "vampire_sha256": vampire_sha256,
        "syntax": {
            key: value
            for key, value in syntax.items()
            if key not in {"stdout", "stderr"}
        },
        "umlaut": {
            key: value
            for key, value in umlaut.items()
            if key not in {"stdout", "stderr"}
        },
        "vampire": {
            key: value
            for key, value in vampire.items()
            if key not in {"stdout", "stderr"}
        },
        "artifact_sha256": artifact_sha256,
        "completed_at": utc_now(),
    }
    atomic_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path)}


def summarize(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> dict[str, Any]:
    taxonomy = Counter(result["taxonomy"] for result in results)
    by_category: dict[str, Counter[str]] = defaultdict(Counter)
    by_split: dict[str, Counter[str]] = defaultdict(Counter)
    tag_counts: dict[str, Counter[str]] = defaultdict(Counter)
    for result in results:
        by_category[result["category"]][result["taxonomy"]] += 1
        by_split[result["holdout_split"]][result["taxonomy"]] += 1
        for tag, present in result["source_tags"].items():
            if present:
                tag_counts[tag][result["taxonomy"]] += 1
    body = {
        "schema_version": 1,
        "contract_id": contract["contract_id"],
        "problem_count": len(results),
        "taxonomy": dict(sorted(taxonomy.items())),
        "by_category": {
            key: dict(sorted(value.items()))
            for key, value in sorted(by_category.items())
        },
        "by_split": {
            key: dict(sorted(value.items()))
            for key, value in sorted(by_split.items())
        },
        "source_tag_taxonomy": {
            key: dict(sorted(value.items()))
            for key, value in sorted(tag_counts.items())
        },
        "umlaut_solved_ids": sorted(
            result["problem_id"]
            for result in results
            if result["taxonomy"] == "umlaut_solved"
        ),
        "reference_only_ids": sorted(
            result["problem_id"]
            for result in results
            if result["taxonomy"] == "search_limited_reference_solved"
        ),
    }
    return {
        **body,
        "report_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--soft-seconds", type=int, default=2)
    parser.add_argument("--hard-seconds", type=int, default=4)
    parser.add_argument("--memory-mib", type=int, default=1536)
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="audit only the first N manifest THF records; zero means all",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise AuditError("prover audits may run only on Linux")
    if arguments.workers < 1:
        raise AuditError("--workers must be positive")
    if arguments.soft_seconds < 1:
        raise AuditError("--soft-seconds must be positive")
    if arguments.hard_seconds < arguments.soft_seconds:
        raise AuditError("--hard-seconds must be at least --soft-seconds")
    if arguments.limit < 0:
        raise AuditError("--limit must be nonnegative")
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    umlaut_binary = arguments.umlaut.resolve()
    vampire_binary = arguments.vampire.resolve()
    output_root = arguments.output_root.resolve()
    for binary in (umlaut_binary, vampire_binary):
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise AuditError(f"missing executable: {binary}")
    metadata, records = load_manifest(manifest_path)
    if arguments.limit:
        records = records[: arguments.limit]
    umlaut_sha256 = sha256_file(umlaut_binary)
    vampire_sha256 = sha256_file(vampire_binary)
    controller_sha256 = sha256_file(Path(__file__).resolve())
    contract_body = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest_path),
        "manifest_problem_archive_sha256": metadata["sources"][
            "problem_archive_sha256"
        ],
        "presentation_id": metadata["presentation"]["id"],
        "problem_ids": [record["problem_id"] for record in records],
        "problem_sha256": {
            record["problem_id"]: record["sha256"] for record in records
        },
        "umlaut_sha256": umlaut_sha256,
        "vampire_sha256": vampire_sha256,
        "controller_sha256": controller_sha256,
        "soft_seconds": arguments.soft_seconds,
        "hard_seconds": arguments.hard_seconds,
        "memory_mib": arguments.memory_mib,
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    contract = {
        **contract_body,
        "contract_id": contract_id,
        "created_at": utc_now(),
        "host": {
            "hostname": socket.gethostname(),
            "platform": platform.platform(),
            "python": platform.python_version(),
        },
    }
    output_root.mkdir(parents=True, exist_ok=True)
    contract_path = output_root / "contract.json"
    if contract_path.is_file():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        existing_body = {
            key: value
            for key, value in existing.items()
            if key not in {"created_at", "host"}
        }
        current_body = {
            key: value
            for key, value in contract.items()
            if key not in {"created_at", "host"}
        }
        if existing_body != current_body:
            raise AuditError("output contains an incompatible audit contract")
    else:
        atomic_json(contract_path, contract)
    records.sort(
        key=lambda record: hashlib.sha256(
            f"{contract_id}:{record['problem_id']}".encode()
        ).digest()
    )
    completed = 0
    resumed = 0
    result_paths: list[Path] = []
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=arguments.workers
    ) as executor:
        futures = [
            executor.submit(
                run_one,
                record=record,
                problem_root=problem_root,
                output_root=output_root,
                contract_id=contract_id,
                umlaut_binary=umlaut_binary,
                umlaut_sha256=umlaut_sha256,
                vampire_binary=vampire_binary,
                vampire_sha256=vampire_sha256,
                soft_seconds=arguments.soft_seconds,
                hard_seconds=arguments.hard_seconds,
                memory_mib=arguments.memory_mib,
            )
            for record in records
        ]
        for future in concurrent.futures.as_completed(futures):
            outcome = future.result()
            completed += 1
            resumed += int(outcome["resumed"])
            result_paths.append(Path(outcome["result_path"]))
            if completed % 25 == 0 or completed == len(records):
                print(
                    f"audit: {completed}/{len(records)} complete "
                    f"({resumed} resumed)",
                    flush=True,
                )
    results = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(result_paths)
    ]
    summary = summarize(contract, results)
    atomic_json(output_root / "summary.json", summary)
    print(
        f"OK: audit contract {contract_id}; {len(results)} problems; "
        f"{resumed} resumed; report {summary['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        AuditError,
        OSError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
