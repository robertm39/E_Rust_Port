#!/usr/bin/env python3
"""Shared helpers for the proof-lemma/watchlist transfer experiment."""

from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any, Iterable


PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
PCL_STEP_RE = re.compile(r"^\s*\d+\s*:", re.MULTILINE)
ANNOTATED_HEAD_RE = re.compile(r"^\s*(cnf|tcf|fof)\s*\(", re.IGNORECASE)
DROP_TARGET_ROLES = {"conjecture", "negated_conjecture", "question"}


class ExperimentError(RuntimeError):
    """A frozen experiment contract or input was violated."""


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


def final_status(*texts: str) -> str | None:
    statuses: list[str] = []
    for text in texts:
        statuses.extend(match.group(1) for match in SZS_RE.finditer(text))
    return statuses[-1] if statuses else None


def proof_step_count(text: str) -> int:
    return len(PCL_STEP_RE.findall(text))


def _strip_comments(text: str) -> str:
    """Remove TPTP line/block comments without touching quoted strings."""
    result: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(text):
        char = text[index]
        if quote is not None:
            result.append(char)
            if char == "\\" and index + 1 < len(text):
                index += 1
                result.append(text[index])
            elif char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
            result.append(char)
            index += 1
            continue
        if char == "%":
            while index < len(text) and text[index] not in "\r\n":
                index += 1
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            if end < 0:
                raise ExperimentError("unterminated TPTP block comment")
            index = end + 2
            continue
        result.append(char)
        index += 1
    return "".join(result)


def split_tptp_records(text: str) -> list[str]:
    """Split TPTP text at top-level record terminators."""
    clean = _strip_comments(text)
    records: list[str] = []
    start: int | None = None
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(clean):
        char = clean[index]
        if start is None:
            if char.isspace():
                index += 1
                continue
            start = index
        if quote is not None:
            if char == "\\" and index + 1 < len(clean):
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                raise ExperimentError("unbalanced TPTP record")
        elif char == "." and depth == 0:
            record = clean[start : index + 1].strip()
            if record:
                records.append(record)
            start = None
        index += 1
    if quote is not None or depth != 0:
        raise ExperimentError("unterminated TPTP record")
    if start is not None and clean[start:].strip():
        # Selector progress text is permitted before its first TPTP record.
        tail = clean[start:].strip()
        if ANNOTATED_HEAD_RE.match(tail) or tail.lower().startswith("include("):
            raise ExperimentError("TPTP record has no final period")
    return records


def split_top_level(text: str) -> list[str]:
    fields: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(text):
        char = text[index]
        if quote is not None:
            if char == "\\" and index + 1 < len(text):
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in {"'", '"'}:
            quote = char
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            fields.append(text[start:index].strip())
            start = index + 1
        index += 1
    fields.append(text[start:].strip())
    return fields


def annotated_record(record: str) -> dict[str, str] | None:
    match = ANNOTATED_HEAD_RE.match(record)
    if match is None:
        return None
    kind = match.group(1).lower()
    open_paren = record.find("(", match.start(1) + len(kind))
    if open_paren < 0 or not record.endswith("."):
        raise ExperimentError(f"malformed annotated record: {record[:80]}")
    close_paren = record.rfind(")")
    if close_paren < open_paren:
        raise ExperimentError(f"malformed annotated record: {record[:80]}")
    fields = split_top_level(record[open_paren + 1 : close_paren])
    if len(fields) < 3:
        raise ExperimentError(f"annotated record has fewer than three fields: {record}")
    return {
        "kind": kind,
        "name": fields[0],
        "role": fields[1].strip().lower(),
        "body": fields[2].strip(),
    }


def canonical_body(body: str) -> str:
    return " ".join(body.split())


def free_variable_names(body: str) -> list[str]:
    """Return unquoted TPTP variables in first-occurrence order."""
    unquoted: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(body):
        char = body[index]
        if quote is not None:
            if char == "\\" and index + 1 < len(body):
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
            index += 1
            continue
        unquoted.append(char)
        index += 1
    observed: list[str] = []
    seen: set[str] = set()
    for match in re.finditer(r"\b[A-Z][A-Za-z0-9_]*\b", "".join(unquoted)):
        variable = match.group(0)
        if variable not in seen:
            seen.add(variable)
            observed.append(variable)
    return observed


def is_empty_clause(body: str) -> bool:
    normalized = canonical_body(body).replace(" ", "").lower()
    while normalized.startswith("(") and normalized.endswith(")"):
        normalized = normalized[1:-1]
    return normalized in {"$false", "false", "[]"}


def axiom_only_target(text: str) -> str:
    retained: list[str] = []
    for record in split_tptp_records(text):
        parsed = annotated_record(record)
        if parsed is not None and parsed["role"] in DROP_TARGET_ROLES:
            continue
        retained.append(record)
    return "\n".join(retained) + "\n"


def render_annotated(
    *, kind: str, name: str, role: str, body: str
) -> str:
    if kind not in {"cnf", "tcf", "fof"}:
        raise ExperimentError(f"unsupported candidate kind: {kind}")
    return f"{kind}({name},{role},{body})."


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
