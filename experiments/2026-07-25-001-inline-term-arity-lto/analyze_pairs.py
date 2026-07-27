#!/usr/bin/env python3
"""Summarize alternating parent/candidate measurement CSV files."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path


def percent_delta(candidate: float, parent: float) -> float:
    return (candidate / parent - 1.0) * 100.0


def summarize(rows: list[dict[str, str]]) -> dict[str, object]:
    by_pair: dict[int, dict[str, dict[str, str]]] = {}
    proof_hashes: set[str] = set()
    for row in rows:
        pair = int(row["pair"])
        by_pair.setdefault(pair, {})[row["label"]] = row
        proof_hashes.add(row["stdout_sha256"])

    pairs = [by_pair[index] for index in sorted(by_pair)]
    if any(set(pair) != {"parent", "candidate"} for pair in pairs):
        raise ValueError("every pair must contain one parent and one candidate row")

    result: dict[str, object] = {
        "pair_count": len(pairs),
        "proof_hashes": sorted(proof_hashes),
    }
    for metric in ("wall_seconds", "cpu_seconds"):
        parent = [float(pair["parent"][metric]) for pair in pairs]
        candidate = [float(pair["candidate"][metric]) for pair in pairs]
        paired = [
            percent_delta(candidate_value, parent_value)
            for parent_value, candidate_value in zip(parent, candidate, strict=True)
        ]
        result[metric] = {
            "parent_mean": statistics.fmean(parent),
            "candidate_mean": statistics.fmean(candidate),
            "mean_delta_percent": percent_delta(
                statistics.fmean(candidate), statistics.fmean(parent)
            ),
            "parent_median": statistics.median(parent),
            "candidate_median": statistics.median(candidate),
            "median_delta_percent": percent_delta(
                statistics.median(candidate), statistics.median(parent)
            ),
            "paired_mean_delta_percent": statistics.fmean(paired),
            "paired_median_delta_percent": statistics.median(paired),
            "candidate_wins": sum(
                candidate_value < parent_value
                for parent_value, candidate_value in zip(
                    parent, candidate, strict=True
                )
            ),
            "ties": sum(
                candidate_value == parent_value
                for parent_value, candidate_value in zip(
                    parent, candidate, strict=True
                )
            ),
        }
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv_paths", type=Path, nargs="+")
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--final-pairs",
        type=int,
        default=32,
        help="also summarize this many pairs from the end",
    )
    arguments = parser.parse_args()

    rows: list[dict[str, str]] = []
    final_rows: list[dict[str, str]] = []
    pair_offset = 0
    for csv_path in arguments.csv_paths:
        with csv_path.open(encoding="utf-8", newline="") as source:
            block_rows = list(csv.DictReader(source))
        pair_numbers = sorted({int(row["pair"]) for row in block_rows})
        final_numbers = set(pair_numbers[-arguments.final_pairs :])
        for row in block_rows:
            adjusted = dict(row)
            adjusted["pair"] = str(pair_offset + int(row["pair"]))
            rows.append(adjusted)
            if int(row["pair"]) in final_numbers:
                final_rows.append(adjusted)
        pair_offset += len(pair_numbers)
    report = {
        "all": summarize(rows),
        "final": summarize(final_rows),
    }
    serialized = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output is None:
        print(serialized, end="")
    else:
        arguments.output.write_text(serialized, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
