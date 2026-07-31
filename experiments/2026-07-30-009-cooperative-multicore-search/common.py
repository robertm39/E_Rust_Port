#!/usr/bin/env python3
"""Shared helpers for the cooperative multicore experiment."""

from __future__ import annotations

import hashlib
import json
import os
import re
import tempfile
from pathlib import Path
from typing import Any, Iterable, Sequence


SOURCE_REVISION = "77a42527467d01f17a6045852f57d3498d93de23"
CORPUS_SHA256 = (
    "28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1"
)
MEMORY_MIB = 1536
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
BAD_STATUSES = {"Satisfiable", "CounterSatisfiable"}
SZS_STATUS = re.compile(r"SZS status\s+([A-Za-z]+)")
SATURATED_CLAUSE = re.compile(
    r"^\s*cnf\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*(.+)\)\.\s*"
    r"(?:%\s*info\(([^)]*)\))?\s*$"
)
PCL_STEP = re.compile(r"^\s*-?\d+\s*:")
TSTP_STEP = re.compile(r"^\s*(?:cnf|fof|tff|tcf|thf)\s*\(", re.IGNORECASE)


class ExperimentError(RuntimeError):
    """Raised when a frozen experiment contract is violated."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode("utf-8")


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(canonical_json(value))
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except BaseException:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


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
    if not rows or rows[0].get("record_type") != "manifest":
        raise ExperimentError("corpus manifest record is missing")
    problems = rows[1:]
    if len(problems) != 32:
        raise ExperimentError(f"corpus has {len(problems)} problems, expected 32")
    expected = {"train": 16, "validation": 8, "test": 8}
    observed = {
        split: sum(row.get("experiment_split") == split for row in problems)
        for split in expected
    }
    if observed != expected:
        raise ExperimentError(f"corpus split mismatch: {observed}")
    families = {
        split: {
            str(row["family"])
            for row in problems
            if row["experiment_split"] == split
        }
        for split in expected
    }
    if any(
        families[left] & families[right]
        for left in families
        for right in families
        if left < right
    ):
        raise ExperimentError("corpus source families are not split-disjoint")
    return rows[0], problems


def normalize_tptp(text: str) -> str:
    """Remove whitespace outside quoted identifiers."""
    result: list[str] = []
    quote: str | None = None
    escaped = False
    for char in text.strip():
        if quote is not None:
            result.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char in {"'", '"'}:
            quote = char
            result.append(char)
        elif not char.isspace():
            result.append(char)
    if quote is not None:
        raise ExperimentError("unterminated quote in TPTP text")
    return "".join(result)


def strip_outer_parens(text: str) -> str:
    value = text.strip()
    while value.startswith("(") and value.endswith(")"):
        depth = 0
        quote: str | None = None
        escaped = False
        encloses_all = True
        for index, char in enumerate(value):
            if quote is not None:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    quote = None
                continue
            if char in {"'", '"'}:
                quote = char
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0 and index != len(value) - 1:
                    encloses_all = False
                    break
                if depth < 0:
                    raise ExperimentError("unbalanced TPTP parentheses")
        if not encloses_all or depth != 0:
            break
        value = value[1:-1].strip()
    return value


def literal_count(body: str) -> int:
    value = strip_outer_parens(body)
    if normalize_tptp(value) in {"$false", "($false)"}:
        return 0
    depth = 0
    count = 1
    quote: str | None = None
    escaped = False
    for char in value:
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                raise ExperimentError("unbalanced TPTP clause")
        elif char == "|" and depth == 0:
            count += 1
    if quote is not None or depth != 0:
        raise ExperimentError("malformed TPTP clause")
    return count


def final_status(text: str) -> str | None:
    statuses = SZS_STATUS.findall(text)
    return statuses[-1] if statuses else None


def proof_step_count(text: str) -> int:
    return sum(
        bool(PCL_STEP.match(line) or TSTP_STEP.match(line))
        for line in text.splitlines()
    )


def parse_info(raw: str | None) -> dict[str, int] | None:
    if raw is None:
        return None
    try:
        fields = [int(field.strip()) for field in raw.split(",")]
    except ValueError:
        return None
    if len(fields) != 8:
        return None
    names = (
        "identifier",
        "proof_depth",
        "proof_size",
        "symbol_count",
        "term_depth",
        "literal_count",
        "variable_occurrences",
        "distinct_variables",
    )
    return dict(zip(names, fields, strict=True))


def parse_saturated_clauses(
    text: str,
    *,
    producer: int,
    wave: int,
    original_bodies: set[str],
) -> tuple[list[dict[str, Any]], list[str]]:
    clauses: list[dict[str, Any]] = []
    errors: list[str] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line.lstrip().startswith("cnf("):
            continue
        match = SATURATED_CLAUSE.match(line)
        if match is None:
            errors.append(f"line {line_number}: malformed cnf record")
            continue
        name, role, body, raw_info = match.groups()
        try:
            normalized = normalize_tptp(body)
            literals = literal_count(body)
        except ExperimentError as error:
            errors.append(f"line {line_number}: {error}")
            continue
        if raw_info is None:
            # Ordinary initial/problem output precedes the explicitly
            # annotated saturated set and is not exchange material.
            continue
        info = parse_info(raw_info)
        if info is None:
            errors.append(f"line {line_number}: missing or malformed info record")
            continue
        if info["literal_count"] != literals:
            errors.append(
                f"line {line_number}: literal count {literals} != "
                f"info {info['literal_count']}"
            )
            continue
        if literals == 0 or normalized in original_bodies:
            continue
        clauses.append(
            {
                "body": body.strip(),
                "body_normalized": normalized,
                "body_sha256": sha256_bytes(normalized.encode("utf-8")),
                "info": info,
                "line_number": line_number,
                "name": name.strip(),
                "producer": producer,
                "role": role.strip(),
                "wave": wave,
            }
        )
    return clauses, errors


def rank_peer_clauses(
    pools: Iterable[Sequence[dict[str, Any]]],
    *,
    recipient: int,
    cap: int,
) -> list[dict[str, Any]]:
    by_body: dict[str, list[dict[str, Any]]] = {}
    for pool in pools:
        for clause in pool:
            if int(clause["producer"]) == recipient:
                continue
            by_body.setdefault(str(clause["body_normalized"]), []).append(clause)
    ranked: list[dict[str, Any]] = []
    for body, occurrences in by_body.items():
        producers = sorted({int(item["producer"]) for item in occurrences})
        representative = min(
            occurrences,
            key=lambda item: (
                int(item["info"]["literal_count"]),
                int(item["info"]["symbol_count"]),
                int(item["info"]["proof_depth"]),
                int(item["producer"]),
            ),
        )
        ranked.append(
            {
                "body": representative["body"],
                "body_normalized": body,
                "body_sha256": representative["body_sha256"],
                "literal_count": int(representative["info"]["literal_count"]),
                "peer_coverage": len(producers),
                "producers": producers,
                "proof_depth": int(representative["info"]["proof_depth"]),
                "source_wave": int(representative["wave"]),
                "symbol_count": int(representative["info"]["symbol_count"]),
            }
        )
    ranked.sort(
        key=lambda item: (
            item["literal_count"],
            item["symbol_count"],
            item["proof_depth"],
            -item["peer_coverage"],
            item["body_sha256"],
            item["producers"],
        )
    )
    return ranked[:cap]


def render_wrapper(
    original: Path,
    clauses: Sequence[dict[str, Any]],
    *,
    wave: int,
    recipient: int,
) -> str:
    include_path = str(original.resolve()).replace("\\", "/").replace("'", "\\'")
    lines = [f"include('{include_path}')."]
    for index, clause in enumerate(clauses):
        lines.append(
            f"cnf(coop_w{wave}_r{recipient}_{index}, watchlist, "
            f"{clause['body']})."
        )
    return "\n".join(lines) + "\n"


def verify_files(entries: Sequence[dict[str, Any]], root: Path) -> None:
    for entry in entries:
        path = root / str(entry["path"])
        if not path.is_file():
            raise ExperimentError(f"missing corpus file: {path}")
        if sha256_file(path) != entry["sha256"]:
            raise ExperimentError(f"corpus file hash mismatch: {path}")


def stable_hash(items: Iterable[str]) -> str:
    digest = hashlib.sha256()
    for item in sorted(items):
        digest.update(item.encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()
