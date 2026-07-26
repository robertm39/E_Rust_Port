#!/usr/bin/env python3
"""Inventory migrated post-compatibility Beads without changing issue state."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
LEGACY_HEADING = "## Legacy Work Item"
MANUAL_END = "<!-- END MANUAL REVIEW: c_source_docs -->"
REVIEW_SIGNALS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "remaining",
        re.compile(
            r"\b(remain(?:s|ed|ing)?|pending|deferred|incomplete|"
            r"unimplemented|missing)\b",
            re.IGNORECASE,
        ),
    ),
    (
        "not-yet",
        re.compile(
            r"\b(?:not|does not|doesn't)\s+(?:yet\s+)?"
            r"(?:port|implement|represent|integrate|wire|support|own|"
            r"store|model|expose)",
            re.IGNORECASE,
        ),
    ),
    (
        "unfinished",
        re.compile(
            r"\b(?:still\s+(?:need|use|rely|lack)|needs?\s+[^.]{0,120}"
            r"(?:before|to\s+(?:port|implement|integrate|wire|support|own))|"
            r"before\s+[^.]{0,120}\b(?:complete|ported|implemented|"
            r"integrated|wired))",
            re.IGNORECASE,
        ),
    ),
    (
        "transitional",
        re.compile(
            r"\b(?:temporary|bridge|surrogate|subset|partial|narrower?|"
            r"limited|fallback)\b",
            re.IGNORECASE,
        ),
    ),
    (
        "provisional-rust",
        re.compile(
            r"\bRust(?:'s)?\b[^.]{0,180}\b(?:only|currently)\b|"
            r"\b(?:initial|first)\b[^.]{0,100}\b"
            r"(?:slice|bridge|subset|implementation)\b",
            re.IGNORECASE,
        ),
    ),
    (
        "future-port",
        re.compile(
            r"\b(?:future|later)\b[^.]{0,180}\b"
            r"(?:port|implement|integrat|own|cover|support)",
            re.IGNORECASE,
        ),
    ),
)
DECISION_SIGNAL = re.compile(
    r"\bRust(?:'s)?\b[^.]{0,240}\b"
    r"(?:preserv|mirror|match|retain|reproduc|keep|intentional|deliberate|"
    r"port|implement|represent|wire|expose|use|own|provide|replace|avoid|"
    r"model|map|make|correct|reject|handle|record|stage|route|render|"
    r"support|cover)|"
    r"\b(?:preserve|keep|retain|match)\b[^.]{0,160}\bcompatib",
    re.IGNORECASE,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--parent",
        action="append",
        required=True,
        help="Beads parent id to inventory; repeat for multiple parents",
    )
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def load_children(parent: str) -> list[dict[str, Any]]:
    completed = subprocess.run(
        ["bd", "list", "--parent", parent, "--all", "--json", "--limit", "0"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    result = json.loads(completed.stdout)
    if not isinstance(result, list):
        raise RuntimeError(f"{parent}: expected a JSON list")
    return result


def legacy_text(issue: dict[str, Any]) -> str:
    description = issue["description"]
    if LEGACY_HEADING not in description:
        return description.strip()
    return description.split(LEGACY_HEADING, maxsplit=1)[1].strip()


def content_hash(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def source_facts(issue: dict[str, Any], text: str) -> dict[str, Any]:
    metadata = issue.get("metadata", {})
    source_name = metadata.get("source_file")
    if not source_name:
        return {
            "source_exists": False,
            "current_source_sha256": None,
            "source_sha_matches_migration": False,
            "legacy_text_in_current_source": False,
        }

    source = REPO_ROOT / source_name
    if not source.is_file():
        return {
            "source_exists": False,
            "current_source_sha256": None,
            "source_sha_matches_migration": False,
            "legacy_text_in_current_source": False,
        }

    raw = source.read_bytes()
    current_text = raw.decode("utf-8", errors="replace")
    comparable = text.removesuffix(MANUAL_END).rstrip()
    return {
        "source_exists": True,
        "current_source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_sha_matches_migration": (
            hashlib.sha256(raw).hexdigest() == metadata.get("source_sha256")
        ),
        "legacy_text_in_current_source": comparable in current_text,
    }


def issue_record(parent: str, issue: dict[str, Any]) -> dict[str, Any]:
    text = legacy_text(issue)
    expected_hash = issue.get("metadata", {}).get("content_sha256")
    actual_hash = content_hash(text)
    standard_description = LEGACY_HEADING in issue["description"]
    signals = [name for name, pattern in REVIEW_SIGNALS if pattern.search(text)]
    if signals:
        review_class = "manual-review"
    elif DECISION_SIGNAL.search(text):
        review_class = "preservation-decision"
    else:
        review_class = "manual-review-no-decision"
    return {
        "parent": parent,
        "id": issue["id"],
        "status": issue["status"],
        "ordinal": issue.get("metadata", {}).get("source_ordinal"),
        "source_file": issue.get("metadata", {}).get("source_file"),
        "content_sha256": expected_hash,
        "computed_content_sha256": actual_hash,
        "content_sha_matches": (
            actual_hash == expected_hash
            if expected_hash and standard_description
            else None
        ),
        "standard_migration_description": standard_description,
        "review_class": review_class,
        "review_signals": signals,
        "legacy_text": text,
        **source_facts(issue, text),
    }


def validate_parent(parent: str, records: list[dict[str, Any]]) -> None:
    ids = [record["id"] for record in records]
    if len(ids) != len(set(ids)):
        raise RuntimeError(f"{parent}: duplicate issue ids")

    ordinals = [record["ordinal"] for record in records if record["ordinal"] is not None]
    if ordinals and sorted(ordinals) != list(range(1, len(ordinals) + 1)):
        raise RuntimeError(f"{parent}: source ordinals are not contiguous")

    bad_hashes = [
        record["id"]
        for record in records
        if record["content_sha_matches"] is False
    ]
    if bad_hashes:
        raise RuntimeError(
            f"{parent}: content hash mismatch for {', '.join(bad_hashes[:10])}"
        )


def main() -> int:
    args = parse_args()
    records: list[dict[str, Any]] = []
    parent_counts: dict[str, Counter[str]] = {}

    for parent in args.parent:
        parent_records = [
            issue_record(parent, issue) for issue in load_children(parent)
        ]
        validate_parent(parent, parent_records)
        records.extend(parent_records)
        parent_counts[parent] = Counter(
            record["review_class"] for record in parent_records
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "parents": args.parent,
        "counts": {
            parent: dict(sorted(counts.items()))
            for parent, counts in parent_counts.items()
        },
        "records": records,
    }
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    print(f"OK: audited {len(records)} migrated issues")
    for parent in args.parent:
        counts = parent_counts[parent]
        current = [record for record in records if record["parent"] == parent]
        print(
            f"{parent}: total={len(current)} "
            f"preservation-decision={counts['preservation-decision']} "
            f"manual-review={counts['manual-review']} "
            f"manual-review-no-decision={counts['manual-review-no-decision']} "
            f"open={sum(record['status'] == 'open' for record in current)} "
            f"source-present={sum(record['legacy_text_in_current_source'] for record in current)}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
