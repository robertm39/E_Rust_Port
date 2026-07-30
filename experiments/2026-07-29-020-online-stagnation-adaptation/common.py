#!/usr/bin/env python3
"""Shared helpers for the online-stagnation adaptation experiment."""

from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any, Iterable


SOURCE_REVISION = "42bfa440729dfe214042020898f7ba87fed7ab4f"
SOURCE_MANIFEST_SHA256 = (
    "31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d"
)
SELECTION_SALT = "umlaut-online-stagnation-v1"
THRESHOLDS = (4.0, 8.0, 16.0, 32.0, 64.0)
MIN_PROCESSED = 64
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
BAD_STATUSES = {"Satisfiable", "CounterSatisfiable", "NonTheorem"}
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
PCL_STEP_RE = re.compile(r"^\s*\d+\s*:", re.MULTILINE)


class ExperimentError(RuntimeError):
    """A frozen experiment contract or evidence invariant was violated."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
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


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        b"".join(canonical_json(row) + b"\n" for row in rows)
    )


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def final_status(*texts: str) -> str | None:
    statuses: list[str] = []
    for text in texts:
        statuses.extend(match.group(1) for match in SZS_RE.finditer(text))
    return statuses[-1] if statuses else None


def proof_step_count(text: str) -> int:
    return len(PCL_STEP_RE.findall(text))


def signal_from_telemetry(
    telemetry: dict[str, Any] | None,
) -> dict[str, Any]:
    fallback_reason: str | None = None
    if telemetry is None:
        fallback_reason = "missing_telemetry"
    elif (
        telemetry.get("schema") != "umlaut.search-telemetry"
        or telemetry.get("schema_version") != 1
    ):
        fallback_reason = "unknown_telemetry_schema"
    if fallback_reason is not None:
        return {
            "valid": False,
            "fallback_reason": fallback_reason,
            "processed_non_trivial": None,
            "generated_non_trivial": None,
            "clause_growth": None,
            "passive_pressure": None,
            "maximum_resident_pages": None,
            "total_cpu_seconds": None,
        }
    try:
        funnel = telemetry["search_funnel"]
        resources = telemetry["resources"]
        processed = int(funnel["processed_non_trivial"])
        generated = int(funnel["generated_non_trivial"])
        passive = int(funnel["high_water_unprocessed"])
        resident = int(resources["maximum_resident_pages"])
        cpu = float(resources["total_cpu_seconds"])
    except (KeyError, TypeError, ValueError):
        return {
            "valid": False,
            "fallback_reason": "incomplete_telemetry",
            "processed_non_trivial": None,
            "generated_non_trivial": None,
            "clause_growth": None,
            "passive_pressure": None,
            "maximum_resident_pages": None,
            "total_cpu_seconds": None,
        }
    denominator = max(processed, 1)
    if processed < MIN_PROCESSED:
        fallback_reason = "insufficient_processed_clauses"
    return {
        "valid": fallback_reason is None,
        "fallback_reason": fallback_reason,
        "processed_non_trivial": processed,
        "generated_non_trivial": generated,
        "clause_growth": generated / denominator,
        "passive_pressure": passive / denominator,
        "maximum_resident_pages": resident,
        "total_cpu_seconds": cpu,
    }


def choose_branch(
    telemetry: dict[str, Any] | None, threshold: float
) -> dict[str, Any]:
    if threshold not in THRESHOLDS:
        raise ExperimentError(f"unregistered threshold: {threshold}")
    signal = signal_from_telemetry(telemetry)
    if not signal["valid"]:
        branch = "goal"
    else:
        branch = (
            "goal"
            if float(signal["clause_growth"]) >= threshold
            else "global"
        )
    return {
        "threshold": threshold,
        "branch": branch,
        **signal,
    }
