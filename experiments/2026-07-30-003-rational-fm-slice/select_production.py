#!/usr/bin/env python3
"""Select preregistered CASC-30 TFA rational/real source problems."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import tarfile
from pathlib import Path
from typing import Any


SORT_PATTERN = re.compile(r"\$(?:rat|real)\b")
LINEAR_TOKEN_PATTERN = re.compile(
    r"\$(?:sum|difference|uminus|product|quotient|less|lesseq|greater|greatereq)\b"
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_records(manifest: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line in manifest.read_text(encoding="utf-8").splitlines():
        value = json.loads(line)
        if value.get("record_type") == "problem":
            records.append(value)
    return records


def select(manifest: Path, repository: Path) -> dict[str, Any]:
    candidates: dict[tuple[str, str], list[dict[str, Any]]] = {}
    exclusions: dict[str, int] = {}
    for record in load_records(manifest):
        if record["division"] != "TFA":
            continue
        path = repository / record["path"]
        raw = path.read_bytes()
        if sha256_bytes(raw) != record["sha256"]:
            raise ValueError(f"source digest differs for {record['problem_id']}")
        text = raw.decode("utf-8")
        if SORT_PATTERN.search(text) is None:
            exclusions["no_rat_or_real_declaration"] = (
                exclusions.get("no_rat_or_real_declaration", 0) + 1
            )
            continue
        if LINEAR_TOKEN_PATTERN.search(text) is None:
            exclusions["no_linear_arithmetic_token"] = (
                exclusions.get("no_linear_arithmetic_token", 0) + 1
            )
            continue
        key = (record["holdout_split"], record["family"])
        candidates.setdefault(key, []).append(record)

    selected: list[dict[str, Any]] = []
    for (partition, family), records in sorted(candidates.items()):
        ordered = sorted(
            records,
            key=lambda item: (item["size_bytes"], item["problem_id"]),
        )
        for rank, record in enumerate(ordered[:5], start=1):
            selected.append(
                {
                    "problem_id": record["problem_id"],
                    "partition": partition,
                    "family": family,
                    "rank_within_family": rank,
                    "path": record["path"],
                    "sha256": record["sha256"],
                    "size_bytes": record["size_bytes"],
                    "expected_class": record["expected_class"],
                }
            )
    selected.sort(
        key=lambda item: (
            item["partition"],
            item["family"],
            item["rank_within_family"],
        )
    )
    return {
        "schema": "umlaut-rational-fm-production-selection-v1",
        "manifest": str(manifest.relative_to(repository)),
        "selection_rule": (
            "TFA; direct source contains $rat/$real and a linear arithmetic "
            "token; up to five per partition/family by size_bytes, problem_id"
        ),
        "selected": selected,
        "exclusions": dict(sorted(exclusions.items())),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("benchmarks/casc_2025_manifest.jsonl"),
    )
    parser.add_argument("--repository", type=Path, default=Path("."))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    arguments = parser.parse_args()
    result = select(arguments.manifest, arguments.repository)
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    if arguments.archive is not None:
        arguments.archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(arguments.archive, "w:gz") as archive:
            for item in result["selected"]:
                archive.add(
                    arguments.repository / item["path"],
                    arcname=item["path"].replace("\\", "/"),
                    recursive=False,
                )
    counts: dict[str, int] = {}
    families: set[tuple[str, str]] = set()
    for item in result["selected"]:
        counts[item["partition"]] = counts.get(item["partition"], 0) + 1
        families.add((item["partition"], item["family"]))
    print(
        json.dumps(
            {
                "selected": len(result["selected"]),
                "counts": dict(sorted(counts.items())),
                "partition_families": len(families),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
