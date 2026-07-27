#!/usr/bin/env python3
"""Audit detailed Change Later records with an affirmative Rust decision."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from collections import Counter
from pathlib import Path
from types import ModuleType
from typing import Any


def load_backlog_audit(repo: Path) -> ModuleType:
    path = (
        repo
        / "experiments/2026-07-25-029-post-compat-backlog-audit/audit_backlog.py"
    )
    spec = importlib.util.spec_from_file_location("post_compat_backlog_audit", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the post-compatibility audit module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def subsystem(issue: dict[str, Any]) -> str:
    value = issue.get("metadata", {}).get("subsystem")
    return value if isinstance(value, str) and value else "(none)"


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = (repo / relative).read_text(encoding="utf-8")
    return all(needle in source for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    audit = load_backlog_audit(repo)
    issues = audit.load_children("E_Rust_Port-j76.4")
    records = [
        audit.issue_record("E_Rust_Port-j76.4", issue) for issue in issues
    ]
    audit.validate_parent("E_Rust_Port-j76.4", records)
    issues_by_id = {issue["id"]: issue for issue in issues}
    decisions = [
        record
        for record in records
        if record["review_class"] == "preservation-decision"
    ]
    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "id": record["id"],
            "legacy_text": record["legacy_text"],
            "ordinal": record["ordinal"],
            "source_file": record["source_file"],
        }
        for record in decisions
    ]
    stable_digest = hashlib.sha256(
        json.dumps(
            stable_records, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()
    subsystem_counts = Counter(
        subsystem(issues_by_id[record["id"]]) for record in decisions
    )
    evidence_checks = {
        "compatibility_matrix_has_zero_unexpected_differences": contains(
            repo,
            "experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md",
            "all 50 main cases",
            "all 216 support-tool cases",
            "zero unexpected differences",
        ),
        "performance_is_within_compatibility_threshold": contains(
            repo,
            "experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md",
            "`1.0801753448x` C",
        ),
        "latest_full_validation_has_4425_tests": contains(
            repo,
            "experiments/2026-07-25-036-strategy-io-timing/FINDINGS.md",
            "4,425 total",
            "strict all-target/all-feature pedantic Clippy",
        ),
    }
    report = {
        "decision_count": len(decisions),
        "decision_digest": stable_digest,
        "evidence_checks": evidence_checks,
        "exact_text_still_in_current_docs": sum(
            record["legacy_text_in_current_source"] for record in decisions
        ),
        "nonstandard_descriptions": sum(
            record["content_sha_matches"] is None for record in decisions
        ),
        "schema_version": 1,
        "source_file_count": len(
            {record["source_file"] for record in decisions}
        ),
        "standard_content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in decisions
        ),
        "subsystem_counts": dict(sorted(subsystem_counts.items())),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    if (
        len(decisions) != 586
        or not all(evidence_checks.values())
        or report["standard_content_hashes_verified"] != 583
        or report["nonstandard_descriptions"] != 3
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("detailed preservation-decision reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
