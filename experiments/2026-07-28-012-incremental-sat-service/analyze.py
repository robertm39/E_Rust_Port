#!/usr/bin/env python3
"""Summarize correctness and performance from SAT adapter benchmark records."""

from __future__ import annotations

import argparse
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable


def median(values: Iterable[float | int]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def percentile(values: Iterable[float | int], percent: float) -> float | None:
    ordered = sorted(values)
    if not ordered:
        return None
    index = max(0, math.ceil(percent * len(ordered)) - 1)
    return float(ordered[index])


def distribution(values: Iterable[float | int]) -> dict[str, float | int | None]:
    materialized = list(values)
    return {
        "count": len(materialized),
        "median": median(materialized),
        "p95": percentile(materialized, 0.95),
        "min": min(materialized) if materialized else None,
        "max": max(materialized) if materialized else None,
    }


def load_records(paths: list[Path]) -> list[dict[str, object]]:
    records = []
    for path in paths:
        records.extend(
            json.loads(line)
            for line in path.read_text(encoding="utf-8").splitlines()
            if line
        )
    return records


def summarize(records: list[dict[str, object]]) -> dict[str, object]:
    queries = [
        record
        for record in records
        if record.get("record_type", "query") == "query"
    ]
    process_failures = [
        record for record in records if record.get("record_type") == "process"
    ]
    by_backend: dict[str, list[dict[str, object]]] = defaultdict(list)
    for record in queries:
        by_backend[str(record["backend"])].append(record)

    backend_summary: dict[str, object] = {}
    for backend, backend_records in sorted(by_backend.items()):
        process_samples: dict[tuple[str, int], dict[str, object]] = {}
        for record in backend_records:
            key = (str(record["session"]), int(record.get("repetition", 0)))
            process_samples.setdefault(key, record)
        assumption_unsat = [
            record
            for record in backend_records
            if record["status"] == "unsat" and int(record["assumptions"]) > 0
        ]
        query_latencies: dict[str, list[int]] = defaultdict(list)
        for record in backend_records:
            query_latencies[str(record["query"])].append(int(record["elapsed_ns"]))

        reuse_ratios: list[float] = []
        session_queries: dict[tuple[str, int], dict[str, int]] = defaultdict(dict)
        for record in backend_records:
            key = (str(record["session"]), int(record.get("repetition", 0)))
            session_queries[key][str(record["query"])] = int(record["elapsed_ns"])
        for measured in session_queries.values():
            if all(name in measured for name in ("cold", "warm1", "warm2")):
                warm = statistics.median([measured["warm1"], measured["warm2"]])
                if warm > 0:
                    reuse_ratios.append(measured["cold"] / warm)

        backend_summary[backend] = {
            "queries": len(backend_records),
            "sessions": len({str(record["session"]) for record in backend_records}),
            "statuses": dict(sorted(Counter(str(r["status"]) for r in backend_records).items())),
            "solve_ns": distribution(int(r["elapsed_ns"]) for r in backend_records),
            "solve_ns_by_query": {
                query: distribution(values)
                for query, values in sorted(query_latencies.items())
            },
            "insertion_ns": distribution(
                int(record.get("insertion_ns", 0))
                for record in process_samples.values()
            ),
            "core_ns": distribution(
                int(record.get("core_ns", 0)) for record in assumption_unsat
            ),
            "core_literals": distribution(
                len(record.get("core", [])) for record in assumption_unsat
            ),
            "process_wall_ns": distribution(
                int(record["process_wall_ns"])
                for record in process_samples.values()
                if record.get("process_wall_ns") is not None
            ),
            "peak_rss_kib": distribution(
                int(record["peak_rss_kib"])
                for record in process_samples.values()
                if record.get("peak_rss_kib") is not None
            ),
            "cold_over_warm_ratio": distribution(reuse_ratios),
        }

    indexed: dict[tuple[str, str, int, str], dict[str, object]] = {}
    statuses_by_query: dict[tuple[str, str, int], set[str]] = defaultdict(set)
    for record in queries:
        base_key = (
            str(record["session"]),
            str(record["query"]),
            int(record.get("repetition", 0)),
        )
        indexed[(*base_key, str(record["backend"]))] = record
        statuses_by_query[base_key].add(str(record["status"]))
    disagreements = [
        {
            "session": key[0],
            "query": key[1],
            "repetition": key[2],
            "statuses": sorted(statuses),
        }
        for key, statuses in statuses_by_query.items()
        if len(statuses) > 1
    ]

    relative: dict[str, object] = {}
    if "internal-dpll" in by_backend:
        for backend in sorted(name for name in by_backend if name != "internal-dpll"):
            speedups: list[float] = []
            for key, candidate in indexed.items():
                if key[3] != backend:
                    continue
                internal = indexed.get((*key[:3], "internal-dpll"))
                if internal is None or candidate["status"] != internal["status"]:
                    continue
                candidate_ns = int(candidate["elapsed_ns"])
                if candidate_ns > 0:
                    speedups.append(int(internal["elapsed_ns"]) / candidate_ns)
            relative[backend] = {
                "internal_over_candidate_speedup": distribution(speedups),
                "candidate_faster_fraction": (
                    sum(speedup > 1.0 for speedup in speedups) / len(speedups)
                    if speedups
                    else None
                ),
            }

    return {
        "schema": 1,
        "records": len(records),
        "query_records": len(queries),
        "process_failures": process_failures,
        "status_disagreements": disagreements,
        "backends": backend_summary,
        "relative_to_internal": relative,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path, nargs="+")
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    summary = summarize(load_records(arguments.results))
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
