#!/usr/bin/env python3
"""Build the deterministic CASC-J11 FNN/FNQ experiment manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


SALT = "umlaut-fnt-prototype-family-v1"


def family_splits(families: set[str]) -> dict[str, str]:
    ordered = sorted(
        families,
        key=lambda family: (
            hashlib.sha256(f"{SALT}:{family}".encode()).digest(),
            family,
        ),
    )
    validation_count = max(1, round(len(ordered) * 0.15))
    test_count = max(1, round(len(ordered) * 0.15))
    train_count = len(ordered) - validation_count - test_count
    return {
        family: (
            "train"
            if index < train_count
            else "validation"
            if index < train_count + validation_count
            else "test"
        )
        for index, family in enumerate(ordered)
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus_root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    pending: list[tuple[str, str, Path]] = []
    for category in ("FNN", "FNQ"):
        directory = args.corpus_root / category / f"{category}ProblemFiles"
        for problem in sorted(directory.glob("*.p")):
            problem_id = problem.name[:-2]
            match = re.match(r"([A-Z]{3})", problem_id)
            family = match.group(1) if match else problem_id
            pending.append((category, family, problem))

    splits = family_splits({family for _, family, _ in pending})
    records: list[dict[str, object]] = []
    for category, family, problem in pending:
        problem_id = problem.name[:-2]
        data = problem.read_bytes()
        records.append(
            {
                "schema_version": 1,
                "problem_id": problem_id,
                "category": category,
                "family": family,
                "split": splits[family],
                "path": problem.relative_to(args.corpus_root).as_posix(),
                "bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
                "source": "CASC-J11 Problems.tgz",
                "source_url": "https://tptp.org/CASC/J11/Problems.tgz",
            }
        )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
        newline="\n",
    )
    summary: dict[str, object] = {
        "problems": len(records),
        "families": len({record["family"] for record in records}),
        "categories": {
            category: sum(record["category"] == category for record in records)
            for category in ("FNN", "FNQ")
        },
        "splits": {
            split: sum(record["split"] == split for record in records)
            for split in ("train", "validation", "test")
        },
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
