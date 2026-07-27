#!/usr/bin/env python3
"""Audit the final EXTERNAL Change Later decisions and CSSCPA benchmark."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [745, 748, 749, 754, 755, 759]


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

    c_process = source(repo, "eprover/EXTERNAL/cex_csscpa.c")
    rust_process = source(repo, "src/external/csscpa.rs")
    benchmark = json.loads(
        source(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "benchmark-reference.json",
        )
    )
    benchmark_cases = {
        case["command_count"]: case for case in benchmark["cases"]
    }
    largest = benchmark_cases.get(500_000, {})
    largest_implementations = largest.get("implementations", {})
    checks = {
        "persistent_signature_synchronized_tautology_bank_matches_c": contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.h",
            "TB_p              terms;",
            "TB_p              tmp_terms;",
        )
        and contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.c",
            "handle->terms     = TBAlloc(handle->sig);",
            "handle->tmp_terms = TBAlloc(handle->sig);",
            "ClauseIsTautology(state->tmp_terms,clause)",
        )
        and contains(
            repo,
            "src/external/csscpa.rs",
            "tmp_terms: TermBank,",
            "let tmp_terms = TermBank::new(terms.signature().clone())?;",
            "*self.tmp_terms.signature_mut() = main_signature.clone();",
            "clause_is_tautology(&mut self.tmp_terms, clause)",
            "fn tautology_checks_reuse_a_signature_synchronized_scratch_bank()",
        )
        and contains(
            repo,
            "src/clauses/proofstate.rs",
            "pub fn tmp_terms_mut(&mut self) -> &mut TermBank",
            "*self.tmp_terms.signature_mut() = main_signature.clone();",
        ),
        "requested_status_is_confined_to_loop_state_reporting": contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.h",
            "requested,",
            "ClauseStatusType;",
        )
        and contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.c",
            "print_csscpa_state(state,requested, NULL);",
        )
        and "clause_status = requested" not in c_process
        and contains(
            repo,
            "src/external/csscpa.rs",
            "Self::Requested => 4,",
            "self.state_line_for_source(CsscpaClauseStatus::Requested, 0)",
            "assert_eq!(CsscpaClauseStatus::Requested.c_value(), 4);",
            "starts_with(\"% CSSCPAState: requested  by 0, 0, 0, 0\")",
        )
        and "status = CsscpaClauseStatus::Requested" not in rust_process,
        "loop_output_level_keeps_the_c_zero_one_mutation_rule": contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.c",
            "if(AktToken(in)->numval == 1 || AktToken(in)->numval == 0)",
            "OutputLevel = AktToken(in)->numval == 1;",
            "AcceptInpTok(in,PosInt);",
        )
        and contains(
            repo,
            "src/external/csscpa.rs",
            "scanner.check_tok(TokenType::POS_INT)?;",
            "0 => 0,",
            "1 => 1,",
            "_ => current_output_level,",
            "fn loop_output_level_accepts_only_zero_or_one_as_state_changes()",
        ),
        "negative_cli_output_level_keeps_split_truthy_and_threshold_gates": contains(
            repo,
            "eprover/EXTERNAL/CSSCPA_filter.c",
            "OutputLevel = CLStateGetIntArg(handle, arg);",
            "if(OutputLevel>1)",
        )
        and contains(
            repo,
            "eprover/EXTERNAL/cex_csscpa.c",
            "if(OutputLevel)",
            "OUTPRINT(1, COMCHAR\" Unit contradiction found!\\n\");",
        )
        and contains(
            repo,
            "src/external/csscpa_filter.rs",
            "if level > 1 {",
            "config.output_level = level;",
            "fn negative_output_level_keeps_c_truthy_trace_but_not_outprint_line()",
            "assert!(!output.contains(\"% Unit contradiction found!\"));",
        )
        and contains(
            repo,
            "src/external/csscpa.rs",
            "output_level != 0",
            "required_level <= output_level",
        ),
        "legacy_input_clause_bridge_is_narrow_and_tested_at_both_boundaries": contains(
            repo,
            "eprover/EXTERNAL/CSSCPA_filter.c",
            "ScannerSetFormat(in, TSTPFormat);",
        )
        and contains(
            repo,
            "src/external/csscpa.rs",
            "saved_format == IoFormat::Tstp && scanner.test_id(\"input_clause\")",
            "scanner.set_format(IoFormat::Tptp);",
            "scanner.set_format(saved_format);",
            "fn loop_accepts_old_tptp_input_clause_under_tstp_filter_mode()",
        )
        and contains(
            repo,
            "src/external/csscpa_filter.rs",
            "scanner.set_format(IoFormat::Tstp);",
            "fn filter_accepts_old_tptp_input_clause_under_tstp_mode()",
        ),
        "eager_scanner_has_one_shared_input_allocation_and_a_large_benchmark": contains(
            repo,
            "src/inout/streams.rs",
            "data: Arc<Vec<u8>>,",
            "data: Arc::new(data),",
            "fn file_content_stream_reuses_the_input_vector_allocation()",
            "assert_eq!(stream.data.as_ptr(), original);",
            "fn cloned_streams_share_input_bytes_and_advance_independently()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "benchmark_large_csscpa.py",
            "TAUTOLOGY_COMMAND =",
            "candidate_over_baseline_wall_ratio",
            "candidate_minus_baseline_rss_kib",
        )
        and benchmark.get("repetitions") == 3
        and largest.get("fixture_bytes") == 21_000_000
        and largest.get("fixture_sha256")
        == "c77a5dab2dfe70ee4ed4abdd2139c3456e5352bb888050293b54530661a52413"
        and largest.get("candidate_over_baseline_wall_ratio", 1.0) < 0.18
        and largest.get("rust_over_c_wall_ratio", 2.0) < 1.34
        and largest.get("candidate_minus_baseline_rss_kib") == -15_876
        and largest_implementations.get("rust", {}).get("median_max_rss_kib")
        == 27_900,
        "exact_csscpa_and_full_port_compatibility_evidence_is_retained": contains(
            repo,
            "experiments/2026-07-16-022-csscpa-filter-coverage/FINDINGS.md",
            "72-clause-command stateful corpus",
            "four tautologies",
            "28 focused CSSCPA library tests passed",
        )
        and contains(
            repo,
            "experiments/2026-07-18-127-support-tool-matrix-closure/"
            "reference.json",
            '"case_count": 216',
            '"CSSCPA_filter": 6',
            '"mismatch_count": 0',
        )
        and contains(
            repo,
            "experiments/2026-07-25-041-detailed-terms-reconciliation/"
            "validation-reference.json",
            '"main_matrix_case_count": 50',
            '"main_unexpected_difference_count": 0',
            '"tool_matrix_case_count": 216',
            '"tool_unexpected_difference_count": 0',
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "CSSCPALoop",
            "only-0-or-1 mutation rule",
            "negative values preserved as truthy-but-below-level-1 output",
            "compatibility bridge for historical old `input_clause(...)` clauses",
        ),
    }
    source_files = [
        "eprover/EXTERNAL/cex_csscpa.c",
        "eprover/EXTERNAL/cex_csscpa.h",
        "eprover/EXTERNAL/CSSCPA_filter.c",
        "src/external/csscpa.rs",
        "src/external/csscpa_filter.rs",
        "src/inout/streams.rs",
        "src/clauses/proofstate.rs",
        "docs/rust-port-status.md",
        "experiments/2026-07-16-022-csscpa-filter-coverage/FINDINGS.md",
        "experiments/2026-07-18-127-support-tool-matrix-closure/reference.json",
        "experiments/2026-07-25-046-external-reconciliation/"
        "benchmark_large_csscpa.py",
        "experiments/2026-07-25-046-external-reconciliation/"
        "benchmark-reference.json",
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
    selected_are_external = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "external"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 6
        or report["content_hashes_verified"] != 6
        or not selected_are_external
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("EXTERNAL reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
