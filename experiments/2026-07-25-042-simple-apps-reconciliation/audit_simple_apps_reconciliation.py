#!/usr/bin/env python3
"""Audit the final SIMPLE_APPS post-compatibility decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [1213, 1215, 1216]


def load_backlog_audit(repo: Path):
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
    expected_ids = {f"E_Rust_Port-j76.4.{ordinal}" for ordinal in ORDINALS}
    selected = sorted(
        (record for record in records if record["id"] in expected_ids),
        key=lambda record: record["ordinal"],
    )
    issues_by_id = {issue["id"]: issue for issue in issues}

    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "id": record["id"],
            "legacy_text": record["legacy_text"],
            "ordinal": record["ordinal"],
            "source_file": record["source_file"],
        }
        for record in selected
    ]
    decision_digest = hashlib.sha256(
        json.dumps(
            stable_records, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()
    checks = {
        "c_help_retains_spacer_runs": contains(
            repo,
            "eprover/SIMPLE_APPS/term2dag.c",
            "term2dag \"VERSION\"",
            "Usage: term2dag [options] [files]",
            "Read a set of terms and print a DAG representing it.",
        ),
        "rust_help_names_and_pins_all_spacer_runs": contains(
            repo,
            "src/simple_apps/term2dag.rs",
            "const HELP_SPACE_46",
            "const HELP_SPACE_28",
            "const HELP_SPACE_13",
            "const HELP_SPACE_54",
            "{HELP_SPACE_46}Usage: term2dag [options] [files]",
        ),
        "c_tree_sort_is_replaced_by_direct_entry_sort": contains(
            repo,
            "eprover/TERMS/cte_termbanks.c",
            "void TBPrintBankInOrder(FILE* out, TB_p bank)",
            "NumTreeStore(&tree, cell->entry_no,dummy, dummy)",
            "NumTreeFree(tree)",
        )
        and contains(
            repo,
            "src/terms/termbanks.rs",
            "terms.sort_by_key(Term::entry_no)",
            "self.write_dag_term(output, &term)?",
        ),
        "mixed_signature_stream_is_confined_to_executable_boundary": contains(
            repo,
            "src/terms/signature.rs",
            "pub fn print_with_c_stdout_side_channel(",
            "writes the `(no type)` marker and every",
            "Use this only at executable",
        )
        and contains(
            repo,
            "src/simple_apps/term2dag.rs",
            "signature.print_with_c_stdout_side_channel(file, stdout)",
            "Self::Stdout => signature.print(stdout)",
        ),
        "permanent_matrix_covers_help_data_and_failure": contains(
            repo,
            "tools/linode-runner/linux_compat.py",
            "\"term2dag\": (",
            "\"stdin-basic\"",
            "\"shared-typed-boundary\"",
            "\"missing-input\"",
        )
        and contains(
            repo,
            "experiments/2026-07-25-041-detailed-terms-reconciliation/validation-reference.json",
            '"tool_matrix_case_count": 216',
            '"tool_unexpected_difference_count": 0',
        ),
    }
    source_files = [
        "eprover/SIMPLE_APPS/term2dag.c",
        "eprover/TERMS/cte_termbanks.c",
        "eprover/TERMS/cte_signature.c",
        "src/simple_apps/term2dag.rs",
        "src/terms/termbanks.rs",
        "src/terms/signature.rs",
        "tools/linode-runner/linux_compat.py",
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_simple_apps = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "simple_apps"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 3
        or report["content_hashes_verified"] != 3
        or not selected_are_simple_apps
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("SIMPLE_APPS reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
