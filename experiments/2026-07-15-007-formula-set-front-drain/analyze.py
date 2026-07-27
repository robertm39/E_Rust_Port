#!/usr/bin/env python3
from __future__ import annotations

import csv
import statistics
import sys
from pathlib import Path


def read_metrics(path: Path) -> dict[str, float]:
    wall: list[float] = []
    cpu: list[float] = []
    rss: list[int] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.reader(handle):
            if len(row) != 5:
                raise ValueError(f"unexpected metric row in {path}: {row}")
            wall.append(float(row[1]))
            cpu.append(float(row[2]) + float(row[3]))
            rss.append(int(row[4]))
    if len(wall) != 5:
        raise ValueError(f"expected five runs in {path}, found {len(wall)}")
    return {
        "wall_median": statistics.median(wall),
        "cpu_median": statistics.median(cpu),
        "rss_median_kb": float(statistics.median(rss)),
    }


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: analyze.py LABEL=CSV [LABEL=CSV ...]", file=sys.stderr)
        return 2
    metrics: dict[str, dict[str, float]] = {}
    for argument in sys.argv[1:]:
        label, separator, raw_path = argument.partition("=")
        if not separator or not label or not raw_path:
            raise ValueError(f"expected LABEL=CSV, got {argument}")
        metrics[label] = read_metrics(Path(raw_path))
    for label, values in metrics.items():
        print(
            f"{label}: wall={values['wall_median']:.3f}s "
            f"cpu={values['cpu_median']:.3f}s "
            f"rss={values['rss_median_kb']:.0f}KiB"
        )
    if "baseline" in metrics and "deque" in metrics:
        print(
            "deque/baseline wall ratio: "
            f"{metrics['deque']['wall_median'] / metrics['baseline']['wall_median']:.3f}x"
        )
        print(
            "deque/baseline cpu ratio: "
            f"{metrics['deque']['cpu_median'] / metrics['baseline']['cpu_median']:.3f}x"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
