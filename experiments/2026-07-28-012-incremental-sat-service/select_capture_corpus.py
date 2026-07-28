#!/usr/bin/env python3
"""Select a deterministic partitioned CASC-30 SATCheck capture corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

CATEGORIES = ("EPS", "EPU", "FEQ", "FNE", "UEQ")
SPLITS = ("train", "validation", "test")
SALT = "umlaut-incremental-sat-capture-v1"


def rank(record: dict[str, object]) -> str:
    identity = f"{SALT}\0{record['problem_id']}\0{record['sha256']}"
    return hashlib.sha256(identity.encode("utf-8")).hexdigest()


def select(
    records: list[dict[str, object]],
    per_category: int,
    splits: tuple[str, ...] = SPLITS,
    offset: int = 0,
    categories: tuple[str, ...] = CATEGORIES,
) -> list[dict[str, object]]:
    selected: list[dict[str, object]] = []
    for split in splits:
        for category in categories:
            candidates = [
                record
                for record in records
                if record.get("record_type") == "problem"
                and record.get("holdout_split") == split
                and record.get("category") == category
            ]
            candidates.sort(key=lambda record: (rank(record), str(record["problem_id"])))
            if len(candidates) < offset + per_category:
                raise ValueError(
                    f"{split}/{category} has {len(candidates)} candidates, "
                    f"needs {offset + per_category}"
                )
            for record in candidates[offset : offset + per_category]:
                output = dict(record)
                output["selection_rank"] = rank(record)
                output["selection_salt"] = SALT
                selected.append(output)
    return selected


def load_manifest(path: Path) -> list[dict[str, object]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--per-category", type=int, default=2)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--split", action="append", choices=SPLITS)
    parser.add_argument("--category", action="append", choices=CATEGORIES)
    arguments = parser.parse_args()
    if arguments.per_category < 1:
        parser.error("--per-category must be positive")
    if arguments.offset < 0:
        parser.error("--offset must be nonnegative")
    splits = tuple(arguments.split) if arguments.split else SPLITS
    categories = tuple(arguments.category) if arguments.category else CATEGORIES
    selected = select(
        load_manifest(arguments.manifest),
        arguments.per_category,
        splits,
        arguments.offset,
        categories,
    )
    arguments.output.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in selected),
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "selected": len(selected),
                "categories": list(categories),
                "splits": list(splits),
                "per_category": arguments.per_category,
                "offset": arguments.offset,
                "salt": SALT,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
