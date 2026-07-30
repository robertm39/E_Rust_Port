#!/usr/bin/env python3
"""Protocol helpers for the experiment-only Rust native checker."""

from __future__ import annotations

import json
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable


class NativeProtocolError(ValueError):
    """The native checker protocol is malformed."""


def write_protocol(path: Path, queries: Iterable[dict[str, Any]]) -> None:
    lines = ["UMLAUT_REAL_GROUND_NATIVE_V1"]
    for query in queries:
        lines.append("\t".join(["QUERY", query["id"], query["sort"]]))
        for constraint in query["constraints"]:
            bound = Fraction(constraint["bound"])
            lines.append(
                "\t".join(
                    [
                        "CONSTRAINT",
                        constraint["label"],
                        constraint["lhs"],
                        constraint["rhs"],
                        str(bound.numerator),
                        str(bound.denominator),
                    ]
                )
            )
        lines.append("END_QUERY")
    lines.append("END")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_results(path: Path) -> tuple[list[dict[str, Any]], dict[str, str]]:
    results = []
    metadata: dict[str, str] = {}
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        fields = line.split("\t")
        if fields[0] == "META" and len(fields) == 3:
            metadata[fields[1]] = fields[2]
            continue
        if fields[0] != "RESULT" or len(fields) != 7:
            raise NativeProtocolError(
                f"malformed native result line {line_number}"
            )
        model = {}
        for item in fields[5].split(";"):
            if not item:
                continue
            name, separator, value = item.partition("=")
            if not separator or name in model:
                raise NativeProtocolError(
                    f"malformed native model line {line_number}"
                )
            model[name] = str(Fraction(value))
        results.append(
            {
                "id": fields[1],
                "status": fields[2],
                "elapsed_ns": int(fields[3]),
                "core": [label for label in fields[4].split(",") if label],
                "model": model,
                "reason": fields[6],
            }
        )
    return results, metadata


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
