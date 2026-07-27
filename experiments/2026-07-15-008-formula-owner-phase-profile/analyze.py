#!/usr/bin/env python3
from __future__ import annotations

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

PhaseMedians = dict[tuple[str, str], tuple[float, float, float]]


def read_metrics(path: Path) -> PhaseMedians:
    samples: dict[tuple[str, str], list[tuple[float, float, int]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.reader(handle):
            if len(row) != 8:
                raise ValueError(f"unexpected metric row in {path}: {row}")
            implementation, mode, _run, exit_code, wall, user, system, rss = row
            if exit_code != "0":
                raise ValueError(f"{implementation} {mode} exited with {exit_code}")
            sample = (float(wall), float(user) + float(system), int(rss))
            if any(value < 0 for value in sample):
                raise ValueError(f"negative metric in {path}: {row}")
            samples[implementation, mode].append(sample)

    medians: PhaseMedians = {}
    for implementation in ("c", "rust"):
        for mode in ("syntax", "cnf", "auto"):
            values = samples[implementation, mode]
            if len(values) != 5:
                raise ValueError(
                    f"expected five {implementation} {mode} samples in {path}, "
                    f"found {len(values)}"
                )
            medians[implementation, mode] = tuple(
                statistics.median(column) for column in zip(*values)
            )
    return medians


def print_metrics(label: str, medians: PhaseMedians) -> None:
    print(label)
    for implementation in ("c", "rust"):
        for mode in ("syntax", "cnf", "auto"):
            median = medians[implementation, mode]
            print(
                f"  {implementation:4} {mode:6} "
                f"wall={median[0]:.3f}s cpu={median[1]:.3f}s rss={median[2]:.0f}KiB"
            )
    for mode in ("syntax", "cnf", "auto"):
        c_wall = medians["c", mode][0]
        rust_wall = medians["rust", mode][0]
        print(f"  rust/c {mode:6} wall ratio: {rust_wall / c_wall:.3f}x")
    for implementation in ("c", "rust"):
        syntax_wall = medians[implementation, "syntax"][0]
        cnf_wall = medians[implementation, "cnf"][0]
        auto_wall = medians[implementation, "auto"][0]
        print(
            f"  {implementation:4} incremental wall: "
            f"cnf-syntax={cnf_wall - syntax_wall:.3f}s "
            f"auto-cnf={auto_wall - cnf_wall:.3f}s"
        )


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: analyze.py LABEL=PHASE_METRICS.csv [...]", file=sys.stderr)
        return 2

    results: dict[str, PhaseMedians] = {}
    for argument in sys.argv[1:]:
        label, separator, raw_path = argument.partition("=")
        if not separator or not label or not raw_path:
            raise ValueError(f"expected LABEL=CSV, got {argument}")
        results[label] = read_metrics(Path(raw_path))
        print_metrics(label, results[label])

    if "baseline" in results and "shared_buffer" in results:
        for mode in ("syntax", "cnf", "auto"):
            baseline_wall, baseline_cpu, _ = results["baseline"]["rust", mode]
            current_wall, current_cpu, _ = results["shared_buffer"]["rust", mode]
            print(
                f"shared_buffer/baseline rust {mode:6}: "
                f"wall={current_wall / baseline_wall:.3f}x "
                f"cpu={current_cpu / baseline_cpu:.3f}x"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
