#!/usr/bin/env python3
"""Audit the completed migrated E Rust port namespace."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any


ROOT_ID = "E_Rust_Port-j76"
PARENT_COUNTS = {
    "E_Rust_Port-j76.1": 47,
    "E_Rust_Port-j76.2": 140,
    "E_Rust_Port-j76.3": 649,
    "E_Rust_Port-j76.4": 1327,
    "E_Rust_Port-j76.5": 5,
}
ROOT_CHILDREN = {
    "E_Rust_Port-j76.1",
    "E_Rust_Port-j76.2",
    "E_Rust_Port-j76.3",
    "E_Rust_Port-j76.4",
    "E_Rust_Port-j76.5",
    "E_Rust_Port-j76.6",
    "E_Rust_Port-j76.7",
}


def run_bd(repo: Path, *args: str) -> list[dict[str, Any]]:
    completed = subprocess.run(
        ["bd", *args, "--json", "--limit", "0"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    result = json.loads(completed.stdout)
    if not isinstance(result, list):
        raise RuntimeError("expected a JSON list from bd")
    return result


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    text = source(repo, relative)
    return all(needle in text for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    all_issues = run_bd(repo, "list", "--all")
    namespace = sorted(
        (
            issue
            for issue in all_issues
            if issue["id"] == ROOT_ID or issue["id"].startswith(f"{ROOT_ID}.")
        ),
        key=lambda issue: issue["id"],
    )
    namespace_statuses = Counter(issue["status"] for issue in namespace)
    stable_namespace = [
        {
            "close_reason": issue.get("close_reason"),
            "id": issue["id"],
            "status": issue["status"],
        }
        for issue in namespace
    ]
    namespace_digest = hashlib.sha256(
        json.dumps(
            stable_namespace, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()

    root_records = run_bd(repo, "list", "--parent", ROOT_ID, "--all")
    direct_root_ids = {issue["id"] for issue in root_records}
    parent_counts: dict[str, int] = {}
    parent_nonclosed: dict[str, int] = {}
    for parent in PARENT_COUNTS:
        children = run_bd(repo, "list", "--parent", parent, "--all")
        parent_counts[parent] = len(children)
        parent_nonclosed[parent] = sum(
            issue["status"] != "closed" for issue in children
        )

    root = next(issue for issue in namespace if issue["id"] == ROOT_ID)
    checks = {
        "compatibility_milestone_is_closed": any(
            issue["id"] == "E_Rust_Port-j76.5"
            and issue["status"] == "closed"
            and "1.0801753448x C" in issue.get("close_reason", "")
            for issue in namespace
        ),
        "detailed_change_later_epic_is_closed": any(
            issue["id"] == "E_Rust_Port-j76.4"
            and issue["status"] == "closed"
            and "Experiment 354" in issue.get("close_reason", "")
            for issue in namespace
        ),
        "final_full_lifecycle_is_zero_unexpected": contains(
            repo,
            "experiments/2026-07-25-053-control-reconciliation/validation-reference.json",
            '"rust_test_count": 4430',
            '"main_case_count": 50',
            '"main_unexpected_difference_count": 0',
            '"tool_case_count": 216',
            '"tool_unexpected_difference_count": 0',
            '"validation_complete": true',
        ),
        "clauses_reconciliation_is_exact": contains(
            repo,
            "experiments/2026-07-25-054-clauses-reconciliation/audit-reference.json",
            '"content_hashes_verified": 192',
            '"decision_count": 192',
            '"exact_candidate_passes_full_lifecycle": true',
        ),
        "status_ledger_records_compatibility_and_final_reconciliation": contains(
            repo,
            "docs/rust-port-status.md",
            "Compact term arguments close native main-prover performance parity",
            "The final detailed CONTROL review is reconciled.",
            "The final detailed CLAUSES review is reconciled.",
            "The migrated port namespace is fully closed.",
        ),
    }

    source_files = [
        "Cargo.lock",
        "Cargo.toml",
        "docs/rust-code-standards.md",
        "docs/rust-port-status.md",
        "experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md",
        "experiments/2026-07-25-037-formula-owner-convergence/FINDINGS.md",
        "experiments/2026-07-25-052-prover-reconciliation/FINDINGS.md",
        "experiments/2026-07-25-053-control-reconciliation/FINDINGS.md",
        "experiments/2026-07-25-053-control-reconciliation/validation-reference.json",
        "experiments/2026-07-25-054-clauses-reconciliation/FINDINGS.md",
        "experiments/2026-07-25-054-clauses-reconciliation/audit-reference.json",
    ]
    missing_sources = [
        relative for relative in source_files if not (repo / relative).is_file()
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest() if not missing_sources else None
    report = {
        "all_namespace_records_closed": (
            namespace_statuses == Counter({"closed": 2176})
        ),
        "checks": checks,
        "descendant_count": len(namespace) - 1,
        "direct_root_child_count": len(direct_root_ids),
        "namespace_count": len(namespace),
        "namespace_digest": namespace_digest,
        "namespace_status_counts": dict(sorted(namespace_statuses.items())),
        "parent_child_counts": parent_counts,
        "parent_nonclosed_counts": parent_nonclosed,
        "root_close_reason": root.get("close_reason"),
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    valid = (
        len(namespace) == 2176
        and len(namespace) - 1 == 2175
        and namespace_statuses == Counter({"closed": 2176})
        and direct_root_ids == ROOT_CHILDREN
        and parent_counts == PARENT_COUNTS
        and all(count == 0 for count in parent_nonclosed.values())
        and root["status"] == "closed"
        and "All 2,175 descendant records" in root.get("close_reason", "")
        and all(checks.values())
        and not missing_sources
    )
    if not valid:
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("port-completion reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
