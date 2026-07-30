#!/usr/bin/env python3
"""Build frozen abstractions and no-theory traces from CNF captures."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path
from typing import Any, Sequence

from trace_model import (
    TraceError,
    build_abstraction,
    build_no_theory_trace,
    canonical_json,
    parse_transcript,
)


class BuildError(RuntimeError):
    """Capture identity or trace construction failed."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(canonical_json(value) + "\n", encoding="utf-8")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", required=True, type=Path)
    parser.add_argument("--capture-root", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
    capture = json.loads(
        (arguments.capture_root / "capture.json").read_text(encoding="utf-8")
    )
    expected_selection_sha256 = sha256_file(arguments.selection)
    if capture["selection_sha256"] != expected_selection_sha256:
        raise BuildError("capture does not match source selection")
    selection_by_id = {
        source["problem_id"]: source for source in selection["sources"]
    }
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, Any]] = []
    for capture_record in capture["records"]:
        source = selection_by_id[capture_record["problem_id"]]
        common = {
            "problem_id": source["problem_id"],
            "family": source["family"],
            "partition": source["partition"],
            "source_sha256": source["source_sha256"],
            "capture_return_code": capture_record["return_code"],
            "capture_timed_out": capture_record["timed_out"],
            "capture_stdout_sha256": capture_record["stdout_sha256"],
        }
        if capture_record["timed_out"] or capture_record["return_code"] != 0:
            records.append({**common, "status": "capture_failed"})
            continue
        transcript_path = (
            arguments.capture_root / source["problem_id"] / "stdout.txt"
        )
        if sha256_file(transcript_path) != capture_record["stdout_sha256"]:
            raise BuildError(f"capture hash mismatch for {source['problem_id']}")
        try:
            parsed = parse_transcript(transcript_path.read_text(encoding="utf-8"))
            abstraction = build_abstraction(
                parsed,
                source_id=source["problem_id"],
                source_sha256=source["source_sha256"],
                family=source["family"],
                partition=source["partition"],
            )
            trace = build_no_theory_trace(abstraction)
        except (OSError, UnicodeError, TraceError) as error:
            records.append(
                {
                    **common,
                    "status": "trace_failed",
                    "error": str(error),
                }
            )
            continue
        problem_root = arguments.output_root / source["problem_id"]
        problem_root.mkdir(parents=True, exist_ok=True)
        write_json(problem_root / "abstraction.json", abstraction)
        write_json(problem_root / "trace.json", trace)
        records.append(
            {
                **common,
                "status": "traced",
                "transcript_sha256": abstraction["transcript_sha256"],
                "parsed_clauses": abstraction["parsed_clause_count"],
                "canonical_clauses": abstraction["canonical_clause_count"],
                "atoms": abstraction["atom_count"],
                "arithmetic_atoms": sum(
                    atom["arithmetic"] for atom in abstraction["atoms"]
                ),
                "abstraction_bounds": abstraction["bounds_crossed"],
                "trace_status": trace["status"],
                "trace_bounds": trace["bounds_crossed"],
                "nodes": trace["nodes"],
                "leaves": trace["leaves"],
                "eligible_queries": trace["eligible_queries"],
                "unsupported_contexts": trace["unsupported_contexts"],
                "abstraction_sha256": sha256_file(
                    problem_root / "abstraction.json"
                ),
                "trace_sha256": sha256_file(problem_root / "trace.json"),
            }
        )

    families: dict[str, collections.Counter[str]] = collections.defaultdict(
        collections.Counter
    )
    for record in records:
        families[record["family"]][record["status"]] += 1
        families[record["family"]]["eligible_queries"] += record.get(
            "eligible_queries", 0
        )
    report = {
        "schema": "umlaut-real-ground-trace-build-v1",
        "selection_sha256": expected_selection_sha256,
        "capture_sha256": sha256_file(arguments.capture_root / "capture.json"),
        "records": records,
        "families": {
            family: dict(counter) for family, counter in sorted(families.items())
        },
        "totals": {
            "sources": len(records),
            "captured": sum(record["status"] != "capture_failed" for record in records),
            "traced": sum(record["status"] == "traced" for record in records),
            "eligible_queries": sum(
                record.get("eligible_queries", 0) for record in records
            ),
            "eligible_families": sorted(
                {
                    record["family"]
                    for record in records
                    if record.get("eligible_queries", 0) > 0
                }
            ),
            "complete_traces": sum(
                record.get("trace_status") == "complete" for record in records
            ),
            "bounded_traces": sum(
                record.get("trace_status") == "bound" for record in records
            ),
        },
    }
    write_json(arguments.output_root / "trace-build.json", report)
    print(json.dumps(report["totals"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
