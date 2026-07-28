#!/usr/bin/env python3
"""Summarize cancellation latency by requested deadline and backend."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

from analyze import distribution

DEADLINE_PATTERN = re.compile(r"-(\d+)us\.isat$")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    groups: dict[tuple[str, int], list[dict[str, object]]] = defaultdict(list)
    failures: list[dict[str, object]] = []
    for line in arguments.results.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        record = json.loads(line)
        if record.get("record_type", "query") != "query":
            failures.append(record)
            continue
        match = DEADLINE_PATTERN.search(str(record["session"]))
        if match is None:
            raise ValueError(f"cannot extract deadline from {record['session']!r}")
        groups[(str(record["backend"]), int(match.group(1)))].append(record)

    backends: dict[str, object] = {}
    for backend in sorted({key[0] for key in groups}):
        deadlines: dict[str, object] = {}
        for name, deadline_us in sorted(
            (name, deadline) for name, deadline in groups if name == backend
        ):
            records = groups[(name, deadline_us)]
            elapsed = [int(record["elapsed_ns"]) for record in records]
            deadlines[str(deadline_us)] = {
                "statuses": dict(
                    sorted(Counter(str(r["status"]) for r in records).items())
                ),
                "elapsed_ns": distribution(elapsed),
                "max_over_deadline_ratio": max(elapsed) / (deadline_us * 1_000),
            }
        backends[backend] = deadlines

    summary = {
        "schema": 1,
        "records": sum(len(records) for records in groups.values()),
        "failures": failures,
        "backends": backends,
        "valid": not failures
        and all(
            record.get("status") == "unknown"
            for records in groups.values()
            for record in records
        ),
    }
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if summary["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
