#!/usr/bin/env python3
"""Summarize paired tsm_classify timing CSV files."""

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
        grouped: dict[str, list[dict[str, str]]] = {"reference": [], "rust": []}
        with path.open(newline="", encoding="utf-8") as handle:
            for row in csv.DictReader(handle):
                grouped[row["implementation"]].append(row)
        summaries = {name: summarize(rows) for name, rows in grouped.items()}
        print(path)
        for name in ("reference", "rust"):
            values = summaries[name]
            print(
                f"  {name}: wall={values['wall']:.3f}s "
                f"cpu={values['cpu']:.3f}s rss={values['rss']:.0f}KiB"
            )
        reference = summaries["reference"]
        rust = summaries["rust"]
        print(
            f"  rust/reference: wall={rust['wall'] / reference['wall']:.3f}x "
            f"cpu={rust['cpu'] / reference['cpu']:.3f}x "
            f"rss={rust['rss'] / reference['rss']:.3f}x"
        )


def summarize(rows: list[dict[str, str]]) -> dict[str, float]:
    if not rows:
        raise ValueError("each CSV must contain both implementations")
    return {
        "wall": statistics.median(float(row["wall_seconds"]) for row in rows),
        "cpu": statistics.median(
            float(row["user_seconds"]) + float(row["system_seconds"])
            for row in rows
        ),
        "rss": statistics.median(float(row["max_rss_kib"]) for row in rows),
    }


if __name__ == "__main__":
    main()
