#!/usr/bin/env python3
from __future__ import annotations

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def linear_slope(points: list[tuple[int, float]]) -> float:
    mean_x = statistics.mean(point[0] for point in points)
    mean_y = statistics.mean(point[1] for point in points)
    numerator = sum((x - mean_x) * (y - mean_y) for x, y in points)
    denominator = sum((x - mean_x) ** 2 for x, _ in points)
    return numerator / denominator


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: analyze.py SCALING_METRICS.csv", file=sys.stderr)
        return 2

    samples: dict[tuple[str, int, str, str], list[tuple[float, float, int]]] = defaultdict(list)
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        for row in csv.reader(handle):
            if len(row) != 10:
                raise ValueError(f"unexpected metric row: {row}")
            shape, count, implementation, phase, _run, exit_code, wall, user, system, rss = row
            if exit_code != "0":
                raise ValueError(
                    f"{shape} {count} {implementation} {phase} exited with {exit_code}"
                )
            sample = (float(wall), float(user) + float(system), int(rss))
            if any(value < 0 for value in sample):
                raise ValueError(f"negative metric: {row}")
            samples[shape, int(count), implementation, phase].append(sample)

    medians: dict[tuple[str, int, str, str], tuple[float, float, float]] = {}
    for key, values in samples.items():
        if len(values) != 3:
            raise ValueError(f"expected three samples for {key}, found {len(values)}")
        medians[key] = tuple(statistics.median(column) for column in zip(*values))

    counts = sorted({key[1] for key in medians})
    for shape in ("repeated", "unique"):
        print(shape)
        for count in counts:
            fields = [f"  {count:5}"]
            for implementation in ("c", "rust"):
                for phase in ("syntax", "cnf"):
                    wall, _cpu, rss = medians[shape, count, implementation, phase]
                    fields.append(
                        f"{implementation}-{phase}: wall={wall:.3f}s rss={rss:.0f}KiB"
                    )
            print(" | ".join(fields))
        for implementation in ("c", "rust"):
            for phase in ("syntax", "cnf"):
                points = [
                    (count, medians[shape, count, implementation, phase][2])
                    for count in counts
                ]
                print(
                    f"  {implementation}-{phase} RSS slope: "
                    f"{linear_slope(points):.3f} KiB/owner"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
