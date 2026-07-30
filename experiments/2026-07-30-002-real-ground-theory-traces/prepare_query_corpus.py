#!/usr/bin/env python3
"""Prepare backend protocols and reference certificates from exact search."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence

from native_protocol import write_protocol
from trace_model import canonical_json


class PreparationError(RuntimeError):
    """Reference-search identity or evidence is incomplete."""


UNSAFE_SYMBOL_CHARACTER = re.compile(r"[^A-Za-z0-9_]")


def protocol_id(reference_id: str) -> str:
    """Encode a provenance ID as a protocol-safe, deterministic symbol."""
    encoded = UNSAFE_SYMBOL_CHARACTER.sub("_", reference_id)
    if not encoded or not (encoded[0].isalpha() or encoded[0] == "_"):
        encoded = f"q_{encoded}"
    return encoded


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def certificate_lines(
    backend: str,
    queries: Sequence[dict[str, Any]],
) -> list[str]:
    lines = ["UMLAUT_GROUND_THEORY_CERT_V1"]
    for query in queries:
        decision = query["reference"]
        lines.append(
            "\t".join(
                [
                    "DECISION",
                    backend,
                    query["id"],
                    "branch",
                    query["sort"],
                    decision["status"],
                ]
            )
        )
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
        if decision["status"] == "unsat":
            lines.append("CORE\t" + ",".join(decision["core"]))
        else:
            for variable, raw_value in sorted(decision["model"].items()):
                value = Fraction(raw_value)
                lines.append(
                    "\t".join(
                        [
                            "MODEL",
                            variable,
                            str(value.numerator),
                            str(value.denominator),
                        ]
                    )
                )
        lines.append("END_DECISION")
    lines.append("END")
    return lines


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-root", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    batch_path = arguments.reference_root / "reference-batch.json"
    batch = json.loads(batch_path.read_text(encoding="utf-8"))
    queries = []
    query_sources = []
    protocol_ids: set[str] = set()
    for record in batch["records"]:
        if record["status"] != "searched":
            continue
        search_path = (
            arguments.reference_root
            / record["problem_id"]
            / "reference-search.json"
        )
        if sha256_file(search_path) != record["search_sha256"]:
            raise PreparationError(
                f"reference hash mismatch for {record['problem_id']}"
            )
        search = json.loads(search_path.read_text(encoding="utf-8"))
        for query in search["queries"]:
            encoded_id = protocol_id(query["id"])
            if encoded_id in protocol_ids:
                raise PreparationError(
                    f"protocol ID collision for {query['id']!r}: {encoded_id}"
                )
            protocol_ids.add(encoded_id)
            prepared_query = {**query, "id": encoded_id}
            queries.append(prepared_query)
            query_sources.append(
                {
                    "query_id": encoded_id,
                    "reference_query_id": query["id"],
                    "problem_id": record["problem_id"],
                    "family": record["family"],
                    "partition": record["partition"],
                    "node": query["node"],
                    "fingerprint": query["fingerprint"],
                }
            )

    arguments.output_root.mkdir(parents=True, exist_ok=True)
    protocol_path = arguments.output_root / "native-protocol.txt"
    write_protocol(protocol_path, queries)
    certificate_path = arguments.output_root / "reference-certificates.txt"
    certificate_path.write_text(
        "\n".join(certificate_lines("reference", queries)) + "\n",
        encoding="utf-8",
    )
    workloads = []
    for query, source in zip(queries, query_sources, strict=True):
        constraints = [
            {
                key: constraint[key]
                for key in ("kind", "label", "lhs", "rhs", "bound")
            }
            for constraint in query["constraints"]
        ]
        variables = sorted(
            {
                endpoint
                for constraint in constraints
                for endpoint in (constraint["lhs"], constraint["rhs"])
                if endpoint != "zero"
            }
        )
        workloads.append(
            {
                "id": query["id"],
                "partition": source["partition"],
                "cohort": "real_cnf_trace",
                "sort": query["sort"],
                "eligible": True,
                "expected_closed": False,
                "variables": variables,
                "base": [],
                "branches": [
                    {
                        "id": "branch",
                        "constraints": constraints,
                        "expected": query["reference"]["status"],
                    }
                ],
            }
        )
    corpus = {
        "schema": "umlaut-ground-theory-corpus-v1",
        "provenance": {
            "kind": "real-cnf-trace-queries",
            "reference_batch_sha256": sha256_file(batch_path),
        },
        "workloads": workloads,
    }
    corpus_path = arguments.output_root / "query-corpus.json"
    corpus_path.write_text(canonical_json(corpus) + "\n", encoding="utf-8")
    index = {
        "schema": "umlaut-real-ground-query-index-v1",
        "reference_batch_sha256": sha256_file(batch_path),
        "query_count": len(queries),
        "sources": query_sources,
        "files": {},
    }
    for path in (protocol_path, certificate_path, corpus_path):
        index["files"][path.name] = {
            "bytes": path.stat().st_size,
            "sha256": sha256_file(path),
        }
    index_path = arguments.output_root / "query-index.json"
    index_path.write_text(canonical_json(index) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "queries": len(queries),
                "protocol_bytes": protocol_path.stat().st_size,
                "certificate_bytes": certificate_path.stat().st_size,
                "corpus_bytes": corpus_path.stat().st_size,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
