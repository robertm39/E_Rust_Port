#!/usr/bin/env python3
"""Select the frozen per-family CASC TFA source sample."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
DEFAULT_MANIFEST = REPO_ROOT / "benchmarks/casc_2025_manifest.jsonl"
ARITHMETIC_TOKEN = re.compile(
    rb"\$(?:less|lesseq|greater|greatereq|sum|difference|product|quotient)"
)
MAX_PER_FAMILY = 5


class SelectionError(RuntimeError):
    """The manifest or source corpus violates the frozen selection rule."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("utf-8")


def select(
    repo_root: Path,
    manifest_path: Path,
    partition: str,
) -> dict[str, Any]:
    rows = []
    with manifest_path.open(encoding="utf-8") as handle:
        for line in handle:
            record = json.loads(line)
            if (
                record.get("record_type") == "problem"
                and record.get("division") == "TFA"
                and record.get("holdout_split") == partition
            ):
                rows.append(record)
    by_family: dict[str, list[dict[str, Any]]] = collections.defaultdict(list)
    for record in rows:
        path = repo_root / record["path"]
        try:
            source = path.read_bytes()
        except OSError as error:
            raise SelectionError(f"cannot read {path}: {error}") from error
        actual_sha256 = sha256_bytes(source)
        if actual_sha256 != record["sha256"]:
            raise SelectionError(
                f"source hash mismatch for {record['problem_id']}: "
                f"{actual_sha256} != {record['sha256']}"
            )
        if ARITHMETIC_TOKEN.search(source):
            by_family[record["family"]].append(record)

    selected = []
    for family in sorted(by_family):
        ranked = sorted(
            by_family[family],
            key=lambda record: (record["size_bytes"], record["problem_id"]),
        )
        for record in ranked[:MAX_PER_FAMILY]:
            selected.append(
                {
                    "problem_id": record["problem_id"],
                    "family": record["family"],
                    "partition": partition,
                    "category": record["category"],
                    "expected_class": record["expected_class"],
                    "path": record["path"],
                    "source_sha256": record["sha256"],
                    "size_bytes": record["size_bytes"],
                    "includes": record["includes"],
                }
            )
    return {
        "schema": "umlaut-real-ground-source-selection-v1",
        "partition": partition,
        "selection_rule": {
            "division": "TFA",
            "arithmetic_token_required": True,
            "maximum_per_family": MAX_PER_FAMILY,
            "order": ["size_bytes", "problem_id"],
        },
        "manifest_path": manifest_path.relative_to(repo_root).as_posix(),
        "manifest_sha256": sha256_file(manifest_path),
        "source_count": len(selected),
        "families": {
            family: sum(record["family"] == family for record in selected)
            for family in sorted(by_family)
        },
        "sources": selected,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument(
        "--partition",
        choices=("train", "validation", "test"),
        default="train",
    )
    parser.add_argument(
        "--allow-heldout",
        action="store_true",
        help="required before opening validation or test source bytes",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.partition != "train" and not arguments.allow_heldout:
        raise SelectionError(
            "held-out source selection requires the explicit --allow-heldout gate"
        )
    repo_root = arguments.repo_root.resolve()
    manifest = (
        arguments.manifest.resolve()
        if arguments.manifest is not None
        else repo_root / "benchmarks/casc_2025_manifest.jsonl"
    )
    result = select(repo_root, manifest, arguments.partition)
    payload = canonical_bytes(result)
    if arguments.output is None:
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    if arguments.check:
        if arguments.output.read_bytes() != payload:
            raise SelectionError(f"{arguments.output} is not the frozen selection")
        return 0
    arguments.output.write_bytes(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
