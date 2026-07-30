#!/usr/bin/env python3
"""Analyze train-only Umlaut CNF pilot transcripts."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Sequence

from trace_model import (
    build_abstraction,
    build_no_theory_trace,
    canonical_json,
    parse_transcript,
)


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
PILOT_SOURCES = {
    "SWW667_2": ("SWW", "problems/casc_2025/TFI/SWW667_2.p"),
    "ITP348_1": ("ITP", "problems/casc_2025/TFE/ITP348_1.p"),
    "HWV050_6": ("HWV", "problems/casc_2025/TFI/HWV050_6.p"),
    "SYO522_1": ("SYO", "problems/casc_2025/TFI/SYO522_1.p"),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("transcript_root", type=Path)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    records = []
    for source_id, (family, source_relative) in PILOT_SOURCES.items():
        source = REPO_ROOT / source_relative
        transcript_path = arguments.transcript_root / f"{source_id}.stdout"
        parsed = parse_transcript(transcript_path.read_text(encoding="utf-8"))
        abstraction = build_abstraction(
            parsed,
            source_id=source_id,
            source_sha256=sha256(source),
            family=family,
            partition="train",
        )
        trace = build_no_theory_trace(abstraction)
        records.append(
            {
                "source_id": source_id,
                "family": family,
                "source_sha256": abstraction["source_sha256"],
                "transcript_sha256": abstraction["transcript_sha256"],
                "parsed_clauses": abstraction["parsed_clause_count"],
                "canonical_clauses": abstraction["canonical_clause_count"],
                "atoms": abstraction["atom_count"],
                "bounds_crossed": abstraction["bounds_crossed"],
                "arithmetic_atoms": sum(
                    atom["arithmetic"] for atom in abstraction["atoms"]
                ),
                "supported_true_polarities": sum(
                    atom["arithmetic"]
                    and atom["polarities"]["true"]["unsupported_reason"] is None
                    for atom in abstraction["atoms"]
                ),
                "supported_false_polarities": sum(
                    atom["arithmetic"]
                    and atom["polarities"]["false"]["unsupported_reason"] is None
                    for atom in abstraction["atoms"]
                ),
                "trace_status": trace["status"],
                "nodes": trace["nodes"],
                "leaves": trace["leaves"],
                "eligible_queries": trace["eligible_queries"],
                "unsupported_contexts": trace["unsupported_contexts"],
            }
        )
    report = {
        "schema": "umlaut-real-ground-train-pilot-v1",
        "records": records,
        "eligible_queries": sum(record["eligible_queries"] for record in records),
        "eligible_families": sorted(
            {
                record["family"]
                for record in records
                if record["eligible_queries"]
            }
        ),
    }
    payload = canonical_json(report) + "\n"
    if arguments.output is None:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        arguments.output.write_text(payload, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
