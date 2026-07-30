#!/usr/bin/env python3
"""Audit the outcome-blind restricted integer-induction trigger."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any, Sequence

from schema import SchemaError, extract_target, generate_schema


PREDECESSOR_RE = re.compile(
    r"\$difference\s*\(\s*[A-Z][A-Za-z0-9_]*\s*,\s*1\s*\)"
)
SUCCESSOR_RE = re.compile(
    r"\$sum\s*\(\s*[A-Z][A-Za-z0-9_]*\s*,\s*1\s*\)"
)
TFF_CONJECTURE_RE = re.compile(
    r"(?is)\btff\s*\(\s*(?:'[^']*'|[A-Za-z0-9_$]+)\s*,\s*conjecture\s*,"
)


class AuditError(RuntimeError):
    """The immutable corpus or audit contract is invalid."""


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not rows or rows[0].get("record_type") != "manifest":
        raise AuditError("invalid CASC manifest")
    return rows[0], rows[1:]


def combined_source(problem_root: Path, record: dict[str, Any]) -> str:
    paths = [problem_root / record["path"]]
    include_root = problem_root / "problems" / "casc_2025"
    paths.extend(include_root / include for include in record.get("includes", []))
    return "\n".join(path.read_text(encoding="utf-8") for path in paths)


def rejection_reason(error: SchemaError) -> str:
    message = str(error)
    stable = {
        "expected one conjecture": "conjecture_count",
        "expected a TFF conjecture": "not_tff",
        "conjecture is neither": "unsupported_outer_form",
        "universal target": "unsupported_universal",
        "negated target": "unsupported_negation",
        "negated existential": "unsupported_existential",
        "bound variable": "variable_absent",
        "nested quantifier": "nested_quantifier",
        "quantifier is not": "unsupported_binder",
    }
    for prefix, reason in stable.items():
        if prefix in message:
            return reason
    return "other_rejection"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    metadata, records = load_manifest(manifest_path)
    accepted: list[dict[str, Any]] = []
    rejections: Counter[str] = Counter()

    for record in records:
        problem_path = problem_root / record["path"]
        problem_text = problem_path.read_text(encoding="utf-8")
        # Tokenizing large THF and untyped files is both unnecessary and
        # expensive. This conservative textual gate can only reject a file
        # that lacks either an integer sort or a TFF conjecture declaration.
        if "$int" not in problem_text or TFF_CONJECTURE_RE.search(problem_text) is None:
            rejections["not_tff_integer_conjecture"] += 1
            continue
        try:
            target = extract_target(problem_text)
            schema = generate_schema(problem_text)
        except SchemaError as error:
            rejections[rejection_reason(error)] += 1
            continue
        source = combined_source(problem_root, record)
        predecessor_terms = len(PREDECESSOR_RE.findall(source))
        successor_terms = len(SUCCESSOR_RE.findall(source))
        accepted.append(
            {
                "problem_id": record["problem_id"],
                "path": record["path"],
                "sha256": record["sha256"],
                "category": record["category"],
                "division": record["division"],
                "family": record["family"],
                "holdout_split": record["holdout_split"],
                "conjecture_name": target.conjecture_name,
                "source_form": target.source_form,
                "variable": target.variable,
                "bound": target.bound,
                "property": target.property,
                "schema_name": schema.name,
                "schema_id": schema.schema_id,
                "predecessor_terms": predecessor_terms,
                "successor_terms": successor_terms,
                "recurrence_proxy": predecessor_terms + successor_terms > 0,
            }
        )

    accepted.sort(key=lambda value: value["problem_id"])
    proxy_positive = sum(record["recurrence_proxy"] for record in accepted)
    body = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest_path),
        "manifest_problem_count": metadata["problem_count"],
        "accepted_count": len(accepted),
        "rejected_count": len(records) - len(accepted),
        "rejection_counts": dict(sorted(rejections.items())),
        "recurrence_proxy_positive": proxy_positive,
        "recurrence_proxy_precision": (
            proxy_positive / len(accepted) if accepted else None
        ),
        "records": accepted,
    }
    report = {
        **body,
        "report_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(report) + b"\n")
    print(
        f"OK: accepted {len(accepted)} of {len(records)} problems; "
        f"{proxy_positive} recurrence-proxy positive; "
        f"report {report['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        AuditError,
        OSError,
        KeyError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
