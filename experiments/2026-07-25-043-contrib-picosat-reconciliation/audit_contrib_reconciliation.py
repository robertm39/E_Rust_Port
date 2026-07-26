#!/usr/bin/env python3
"""Audit the final vendored PicoSAT CONTRIB decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [502, 503, 504, 510, 511]


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
        "vendored_utility_shell_and_forwarding_boundaries_are_real": contains(
            repo,
            "eprover/CONTRIB/picosat-965/app.c",
            '#define BUNZIP2 "bzcat %s"',
            'popen (cmd, "r")',
            "picosat_main (int argc, char **argv)",
        )
        and contains(
            repo,
            "eprover/CONTRIB/picosat-965/main.c",
            "int picosat_main (int, char **)",
            "return picosat_main (argc, argv)",
        ),
        "grouped_cnf_is_a_standalone_parser": contains(
            repo,
            "eprover/CONTRIB/picosat-965/picogcnf.c",
            'die ("usage: picogcnf <gcnf-file>")',
            'die ("invalid header")',
            "picosat_reset (ps)",
        ),
        "rust_wrapper_exposes_only_e_used_reentrant_api": contains(
            repo,
            "src/clauses/picosat.rs",
            "Runtime PicoSAT loading is the allowed external DLL/shared-library boundary",
            "struct PicoSatApi",
            "pub struct PicoSat",
            "pub fn add_clause(",
            "pub fn solve(",
            "pub fn core_indices(",
        ),
        "runtime_and_bundled_discovery_policy_is_explicit": contains(
            repo,
            "src/prover/eprover.rs",
            'const PICOSAT_LIBRARY_ENV: &str = "E_RUST_PORT_PICOSAT_LIBRARY"',
            "runtime_picosat_library_from_env().or_else(runtime_picosat_library_from_bundle)",
            "picosat_library_candidates_for_executable",
        )
        and contains(
            repo,
            "DOCS.md",
            "## Runtime PicoSAT Selection",
            "falls back to the internal solver",
        ),
        "solver_lifecycle_and_failure_paths_are_tested": contains(
            repo,
            "src/clauses/picosat.rs",
            "fn solver_wrapper_exercises_picosat_abi_lifecycle_with_fake_api()",
            "fn missing_library_reports_load_error()",
        )
        and contains(
            repo,
            "src/clauses/satinterface.rs",
            "fn picosat_core_helper_uses_fresh_solver_state_for_each_export()",
            "fn picosat_satcheck_helper_resets_after_non_unsat_result()",
            "fn picosat_core_helper_resets_after_core_extraction_error()",
        )
        and contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "fn proof_control_keeps_internal_backend_after_missing_picosat_install()",
        ),
        "latest_full_validation_covers_internal_fallback": contains(
            repo,
            "experiments/2026-07-25-041-detailed-terms-reconciliation/validation-reference.json",
            '"rust_test_count": 4427',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }
    source_files = [
        "eprover/CONTRIB/picosat-965/app.c",
        "eprover/CONTRIB/picosat-965/main.c",
        "eprover/CONTRIB/picosat-965/picogcnf.c",
        "eprover/CONTRIB/picosat-965/picosat.h",
        "src/clauses/picosat.rs",
        "src/clauses/satinterface.rs",
        "src/heuristics/proofcontrol.rs",
        "src/prover/eprover.rs",
        "DOCS.md",
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
    selected_are_contrib = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem") == "contrib"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 5
        or report["content_hashes_verified"] != 5
        or not selected_are_contrib
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("CONTRIB reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
