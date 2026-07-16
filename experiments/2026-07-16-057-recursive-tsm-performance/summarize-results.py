#!/usr/bin/env python3
"""Print median timing and memory summaries for paired TSM CSV files."""

from __future__ import annotations

import argparse
import csv
import statistics
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", nargs="+", type=Path)
    args = parser.parse_args()

    for path in args.csv:
        rows = read_rows(path)
        print(path)
        summaries = {
            implementation: summarize(implementation_rows)
            for implementation, implementation_rows in rows.items()
        }
        for implementation in ("reference", "rust"):
            values = summaries[implementation]
            print(
                f"  {implementation}: wall={values['wall']:.3f}s "
                f"cpu={values['cpu']:.3f}s "
                f"median_rss={values['rss']:.0f}KiB "
                f"peak_rss={values['peak_rss']:.0f}KiB"
            )
        reference = summaries["reference"]
        rust = summaries["rust"]
        print(
            f"  rust/reference: wall={rust['wall'] / reference['wall']:.3f}x "
            f"cpu={rust['cpu'] / reference['cpu']:.3f}x "
            f"rss={rust['rss'] / reference['rss']:.3f}x"
        )


def read_rows(path: Path) -> dict[str, list[dict[str, str]]]:
    grouped: dict[str, list[dict[str, str]]] = {"reference": [], "rust": []}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            grouped[row["implementation"]].append(row)
    if not all(grouped.values()):
        raise ValueError(f"{path} does not contain both implementations")
    return grouped


def summarize(rows: list[dict[str, str]]) -> dict[str, float]:
    wall = [float(row["wall_seconds"]) for row in rows]
    cpu = [
        float(row["user_seconds"]) + float(row["system_seconds"])
        for row in rows
    ]
    rss = [float(row["max_rss_kib"]) for row in rows]
    return {
        "wall": statistics.median(wall),
        "cpu": statistics.median(cpu),
        "rss": statistics.median(rss),
        "peak_rss": max(rss),
    }


if __name__ == "__main__":
    main()
