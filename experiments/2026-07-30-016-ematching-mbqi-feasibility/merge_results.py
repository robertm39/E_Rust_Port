#!/usr/bin/env python3
"""Merge independently written treatment shards into the frozen matrix."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path


EXPECTED_RUNS = 156


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", type=Path, required=True)
    arguments = parser.parse_args()
    output_root = arguments.output_root.resolve()
    paths = [
        output_root / "results.jsonl",
        output_root / "results-clausify-shard.jsonl",
        output_root / "results-ematch-shard.jsonl",
        output_root / "results-mbqi-shard.jsonl",
    ]
    by_id: dict[str, dict[str, object]] = {}
    for path in paths:
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line:
                continue
            record = json.loads(line)
            run_id = record["run_id"]
            previous = by_id.get(run_id)
            if previous is not None and previous != record:
                raise ValueError(f"conflicting duplicate run: {run_id}")
            by_id[run_id] = record
    if len(by_id) != EXPECTED_RUNS:
        raise ValueError(f"expected {EXPECTED_RUNS} runs, found {len(by_id)}")
    temporary = output_root / "results.merged.jsonl"
    temporary.write_text(
        "".join(
            json.dumps(by_id[run_id], sort_keys=True) + "\n"
            for run_id in sorted(by_id)
        ),
        encoding="utf-8",
        newline="\n",
    )
    os.replace(temporary, output_root / "results.jsonl")
    print(json.dumps({"runs": len(by_id)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
