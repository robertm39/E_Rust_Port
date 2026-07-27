#!/usr/bin/env python3
from __future__ import annotations

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: analyze.py RESULTS.csv", file=sys.stderr)
        return 2

    samples: dict[tuple[str, int], list[tuple[float, float, int]]] = defaultdict(list)
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        for row in csv.reader(handle):
            if len(row) != 8:
                raise ValueError(f"unexpected result row: {row}")
            implementation, owners, _run, exit_code, wall, user, system, rss = (
                row[0],
                row[1],
                row[2],
                row[3],
                row[4],
                row[5],
                row[6],
                row[7],
            )
            if exit_code != "0":
                raise ValueError(
                    f"{implementation} {owners} owners exited with {exit_code}"
                )
            samples[implementation, int(owners)].append(
                (float(wall), float(user) + float(system), int(rss))
            )

    for key, values in samples.items():
        if len(values) != 5:
            raise ValueError(f"expected five samples for {key}, found {len(values)}")

    print("implementation,owners,wall_median,cpu_median,rss_median")
    for implementation in ("c", "baseline", "current"):
        for owners in sorted(owner_count for impl, owner_count in samples if impl == implementation):
            values = samples[implementation, owners]
            medians = tuple(statistics.median(column) for column in zip(*values))
            print(
                f"{implementation},{owners},{medians[0]:.3f},"
                f"{medians[1]:.3f},{medians[2]:.0f}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
