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

    samples: dict[tuple[str, str, int], list[tuple[float, float, int]]] = (
        defaultdict(list)
    )
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        for row in csv.reader(handle):
            if len(row) != 9:
                raise ValueError(f"unexpected result row: {row}")
            implementation, shape, owners, _run, exit_code, wall, user, system, rss = row
            if exit_code != "0":
                raise ValueError(
                    f"{implementation} {shape} {owners} owners exited with {exit_code}"
                )
            wall_seconds = float(wall)
            if wall_seconds < 0:
                raise ValueError(
                    f"{implementation} {shape} {owners} owners has negative wall time"
                )
            samples[implementation, shape, int(owners)].append(
                (wall_seconds, float(user) + float(system), int(rss))
            )

    for key, values in samples.items():
        if len(values) != 5:
            raise ValueError(f"expected five samples for {key}, found {len(values)}")

    print("implementation,shape,owners,wall_median,cpu_median,rss_median")
    for implementation in ("c", "baseline", "candidate"):
        for shape in ("repeated", "unique"):
            owner_counts = sorted(
                owners
                for impl, sample_shape, owners in samples
                if impl == implementation and sample_shape == shape
            )
            for owners in owner_counts:
                values = samples[implementation, shape, owners]
                wall, cpu, rss = (
                    statistics.median(column) for column in zip(*values)
                )
                print(
                    f"{implementation},{shape},{owners},{wall:.3f},"
                    f"{cpu:.3f},{rss:.0f}"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
