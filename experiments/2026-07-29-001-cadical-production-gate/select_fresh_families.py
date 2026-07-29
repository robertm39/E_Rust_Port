#!/usr/bin/env python3
"""Freeze fresh-family SATCheck inputs without observing SAT capture shape."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path

FAMILIES = ("ALG", "GEO", "ITP", "LAT", "NLP", "SCT")
CATEGORIES = {"EPS", "EPU", "FEQ", "FNE", "UEQ"}
SELECTION_SALT = "umlaut-cadical-production-gate-v1"
PER_FAMILY = 6
MIN_BYTES = 500
MAX_BYTES = 250_000


def selection_rank(record: dict[str, object]) -> str:
    material = (
        f"{SELECTION_SALT}\0{record['family']}\0{record['sha256']}"
    ).encode()
    return hashlib.sha256(material).hexdigest()


def select(manifest: Path) -> list[dict[str, object]]:
    grouped: dict[str, list[dict[str, object]]] = defaultdict(list)
    lines = manifest.read_text(encoding="utf-8").splitlines()
    for line in lines[1:]:
        if not line:
            continue
        record = json.loads(line)
        if (
            record["family"] in FAMILIES
            and record["category"] in CATEGORIES
            and MIN_BYTES <= int(record["size_bytes"]) <= MAX_BYTES
        ):
            selected = dict(record)
            selected["casc_holdout_split"] = selected["holdout_split"]
            selected["holdout_split"] = "fresh-family-heldout"
            selected["selection_salt"] = SELECTION_SALT
            selected["selection_rank"] = selection_rank(selected)
            grouped[str(selected["family"])].append(selected)

    output: list[dict[str, object]] = []
    for family in FAMILIES:
        candidates = sorted(
            grouped[family],
            key=lambda record: (
                str(record["selection_rank"]),
                str(record["problem_id"]),
            ),
        )
        if len(candidates) < PER_FAMILY:
            raise ValueError(
                f"family {family} has only {len(candidates)} eligible problems"
            )
        output.extend(candidates[:PER_FAMILY])
    return output


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    records = select(arguments.manifest)
    arguments.output.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "families": list(FAMILIES),
                "problems": len(records),
                "per_family": PER_FAMILY,
                "salt": SELECTION_SALT,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
