#!/usr/bin/env python3
"""Shared helpers for deterministic adaptive-probe evaluation."""

from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any, Iterable


SOURCE_REVISION = "f03259698d81e8fbc25c8b64deb4e7c35e3ffd77"
SOURCE_MANIFEST_SHA256 = (
    "31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d"
)
CORPUS_SHA256 = (
    "5b3b2bf5c86bf6537742705a49a15e224dd1062b9d5ad96d56913e2dfdddc923"
)
SELECTION_SALT = "umlaut-adaptive-probe-observability-v1"
THRESHOLD = 64.0
MIN_PROCESSED = 64
PROCESSED_LIMIT = 256
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
BAD_STATUSES = {"Satisfiable", "CounterSatisfiable", "NonTheorem"}
NO_CLAIM_STATUSES = {
    "GaveUp",
    "Inappropriate",
    "InputError",
    "MemoryOut",
    "NoSuccess",
    "OutputError",
    "ResourceOut",
    "Timeout",
    "Unknown",
}
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
INFO_RE = re.compile(r"%\s*info\(([^)]*)\)")
PROCESSED_RE = re.compile(
    r"^[%#]\s*Processed clauses\s*:\s*(\d+)\s*$",
    re.MULTILINE,
)


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


def load_corpus(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if sha256_file(path) != CORPUS_SHA256:
        raise ExperimentError(f"corpus hash mismatch: {path}")
    rows = read_jsonl(path)
    if (
        not rows
        or rows[0].get("kind")
        != "umlaut-deterministic-adaptive-probe-corpus"
        or rows[0].get("source_revision") != SOURCE_REVISION
    ):
        raise ExperimentError("corpus header violates the frozen contract")
    problems = rows[1:]
    expected = {"train": 8, "validation": 8, "test": 8}
    observed = {
        split: sum(
            record.get("experiment_split") == split for record in problems
        )
        for split in expected
    }
    if observed != expected:
        raise ExperimentError(f"corpus split mismatch: {observed}")
    families = {
        split: {
            str(record["family"])
            for record in problems
            if record["experiment_split"] == split
        }
        for split in expected
    }
    if any(
        families[left] & families[right]
        for left in families
        for right in families
        if left < right
    ):
        raise ExperimentError("corpus families are not split-disjoint")
    return rows[0], problems


def final_status(*texts: str) -> str | None:
    statuses: list[str] = []
    for text in texts:
        statuses.extend(match.group(1) for match in SZS_RE.finditer(text))
    return statuses[-1] if statuses else None


def status_is_acceptable(status: str | None, expected_class: str) -> bool:
    if status in NO_CLAIM_STATUSES:
        return True
    if expected_class == "theorem":
        return status in {"Theorem", "ContradictoryAxioms"}
    if expected_class == "unsatisfiable":
        return status in {"Unsatisfiable", "ContradictoryAxioms"}
    return False


def processed_clause_count(text: str) -> int | None:
    matches = PROCESSED_RE.findall(text)
    return int(matches[-1]) if matches else None


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
    elif telemetry.get("record_kind") not in {"checkpoint", "final"}:
        fallback_reason = "unknown_record_kind"
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


def choose_branch(telemetry: dict[str, Any] | None) -> dict[str, Any]:
    signal = signal_from_telemetry(telemetry)
    branch = (
        "goal"
        if not signal["valid"]
        or float(signal["clause_growth"]) >= THRESHOLD
        else "global"
    )
    return {"threshold": THRESHOLD, "branch": branch, **signal}
