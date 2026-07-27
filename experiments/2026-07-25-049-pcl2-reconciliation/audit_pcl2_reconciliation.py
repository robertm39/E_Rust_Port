#!/usr/bin/env python3
"""Audit the final PCL2 Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [
    926,
    927,
    932,
    935,
    937,
    938,
    942,
    944,
    945,
    947,
    948,
    957,
    958,
    963,
    964,
    965,
    966,
    967,
    973,
    976,
    979,
    981,
    986,
    987,
]


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

    checks = {
        "analysis_dangling_parent_boundaries_are_explicit": contains(
            repo,
            "eprover/PCL2/pcl_analysis.c",
            "step = PCLProtFindStep(prot,PCLExprArg(expr,0));",
            "res = PCLStepProofDistance(prot, step);",
            "PCLExprUpdateGRefs(prot, PCLExprArg(expr,i), proofstep);",
        )
        and contains(
            repo,
            "src/pcl2/analysis.rs",
            "fn dangling_proof_distance_parent_is_a_diagnostic_not_c_null_dereference()",
            "fn dangling_generation_parent_remains_a_silent_c_shaped_no_op()",
        )
        and contains(
            repo,
            "experiments/2026-07-16-062-pcl-analysis-edges/FINDINGS.md",
            "The C failure is a null",
            "proof-distance lookup reports the exact syntax diagnostic",
        ),
        "expression_positions_preserve_pcl_only_output": contains(
            repo,
            "eprover/PCL2/pcl_expressions.c",
            "PCL2PosPrint(out, PCLExprArgPos(expr,0));",
            "void PCLExprPrintTSTP(FILE* out, PCLExpr_p expr, bool mini)",
        )
        and contains(
            repo,
            "src/pcl2/expressions.rs",
            "fn stored_positions_print_in_pcl_and_are_omitted_from_tstp_like_c()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-019-pcl-expression-ownership/FINDINGS.md",
            "manually stored positions are concatenated in PCL output but omitted from",
            "contiguous argument/position pairs as typed structs",
        ),
        "identifier_storage_replaces_the_sentinel_without_syntax_drift": contains(
            repo,
            "eprover/PCL2/pcl_idents.c",
            "AcceptInpTok(in, Fullstop);",
            "NO_PCL_ID_ELEMENT",
        )
        and contains(
            repo,
            "src/pcl2/idents.rs",
            "pub const NO_PCL_ID_ELEMENT: i64 = -1;",
            "elements: Vec<i64>",
            "fn zero_is_a_live_component_not_the_end_sentinel()",
            "fn long_identifier_uses_structural_length_without_stored_sentinel()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-018-pcl-identifier-storage/FINDINGS.md",
            "fullstop-separated decimal components",
            "purely a storage sentinel",
        ),
        "lemma_weights_and_reference_failures_are_deterministic": source(
            repo, "eprover/PCL2/pcl_lemmas.c"
        ).count("(*handle)[PCLOp")
        == 15
        and contains(
            repo,
            "eprover/PCL2/pcl_lemmas.c",
            "InferenceWeight_p handle = InferenceWeightCellAlloc();",
            "handle->pure_quote_refs++;",
        )
        and contains(
            repo,
            "src/pcl2/lemmas.rs",
            "fn inference_weight_defaults_zero_every_c_uninitialized_opcode_slot()",
            "fn dangling_reference_counter_parents_remain_non_fatal()",
        )
        and contains(
            repo,
            "experiments/2026-07-16-063-pcl-lemma-edges/FINDINGS.md",
            "whole typed array to zero before applying every explicit C assignment",
            "terminates with signal 11",
        ),
        "miniclauses_retain_term_identity_and_drop_metadata_like_c": contains(
            repo,
            "eprover/PCL2/pcl_miniclauses.c",
            "void MiniClauseAddTerms(MiniClause_p clause, Clause_p term_clause)",
            "handle = ClauseAlloc(list);",
            "clause = MiniClauseToClause(compact, bank);",
            "ClausePrint(out, clause, full_terms);",
        )
        and "MiniClauseAddTerms" not in source(
            repo, "eprover/PCL2/pcl_miniclauses.h"
        )
        and contains(
            repo,
            "src/pcl2/miniclauses.rs",
            "fn from_clause_keeps_literal_signs_and_shared_term_handles()",
            "fn to_clause_round_trips_through_clause_allocation_shape()",
            "fn print_format_string_with_options_uses_caller_equation_options()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-017-pcl-miniclause-ownership/FINDINGS.md",
            "Both implementations snapshot only literal signs and term pairs",
            "unused, non-header C `MiniClauseAddTerms` helper",
        ),
        "ministep_zero_extra_and_shell_contracts_are_exact": contains(
            repo,
            "eprover/PCL2/pcl_ministeps.c",
            "assert(junk && junk->id && junk->just);",
            "CheckInpTok(in, SQString);",
            "if(SupportShellPCL && TestInpTok(in, Colon))",
        )
        and contains(
            repo,
            "src/pcl2/ministeps.rs",
            "fn mini_extra_field_accepts_only_single_quoted_strings()",
            "fn standalone_zero_id_preserves_parse_behavior_and_drops_safely()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-015-pcl-ministep-ownership/FINDINGS.md",
            "optional extra field accepts only a single-quoted string",
            "standalone id zero parses and prints",
            "call-scoped shell option",
        ),
        "proofcheck_preserves_the_legacy_checker_surface": contains(
            repo,
            "eprover/PCL2/pcl_proofcheck.c",
            'ppipe=popen(command, "r");',
            "while((l=fgets(line, 180, ppipe)))",
            "return CheckNotImplemented;",
            'COMCHAR" Check not implemented, assuming true!\\n\\n"',
            '"-------- PROOF --------"',
            '"Proof found."',
        )
        and contains(
            repo,
            "src/pcl2/proofcheck.rs",
            "fn collect_preconditions_copies_unique_clausal_parent_clauses()",
            "fn neg_skolemize_clause_adds_one_flipped_hypothesis_unit_per_literal()",
            "fn otter_and_dfg_render_c_truth_literal_hacks()",
            "fn prover_success_marker_scans_c_fgets_chunks()",
            "fn eprover_success_marker_accepts_real_e_output()",
        )
        and contains(
            repo,
            "experiments/2026-07-16-065-pcl-proofcheck-edges/FINDINGS.md",
            "copies only clausal parents",
            "polarity-flipped hypothesis unit for each target literal",
            "Otter, and SPASS",
        )
        and contains(
            repo,
            "experiments/2026-07-17-001-pcl-proofcheck-real-e-marker/FINDINGS.md",
            "preserves C's fixed 180-byte `fgets` buffer shape",
            "Otter and SPASS retain their existing markers",
        ),
        "protocol_membership_and_fof_strip_quirks_are_preserved": contains(
            repo,
            "eprover/PCL2/pcl_protocol.h",
            "((prot)->number++),",
            "PTreeObjStore",
        )
        and contains(
            repo,
            "eprover/PCL2/pcl_protocol.c",
            "step->just->op = PCLOpInitial;",
        )
        and contains(
            repo,
            "src/pcl2/protocol.rs",
            "fn duplicate_ids_are_rejected_during_protocol_parse()",
            "fn strip_fof_deletes_formula_steps_and_initializes_clause_dependents()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-013-pcl-protocol-ownership/FINDINGS.md",
            "pre-error count cannot be observed",
            "keeps `step_count()` equal to stored membership",
            "FOF stripping retains C's unusual split exactly",
        ),
        "step_parsing_printing_and_shell_ownership_are_exact": contains(
            repo,
            "eprover/PCL2/pcl_steps.c",
            'else if(TestInpId(in, "que"))',
            'CheckInpId(in, "conj|neg|lemma");',
            "bool SupportShellPCL = false;",
            "ClausePrint(out, step->logic.clause, true);",
            "fputc(')',out);",
        )
        and contains(
            repo,
            "src/pcl2/steps.rs",
            "fn parse_external_type_error_surface_omits_accepted_question_token_like_c()",
            "fn full_step_extra_accepts_names_and_positive_integers()",
            '"input_formula(pclid7_2,negated_conjecture,(p(a)|q(a)))"',
            "pub fn print_example_format_string(",
        )
        and contains(
            repo,
            "experiments/2026-07-17-012-pcl-step-ownership/FINDINGS.md",
            "`PclStepParseOptions::support_shell_pcl` replaces the global",
            "omitted `que` diagnostic",
            "missing formula TPTP period",
        ),
        "full_pcl2_and_port_compatibility_evidence_is_current": contains(
            repo,
            "docs/rust-port-status.md",
            "Initial PCL2 `pcl_expressions` support",
            "Initial PCL2 `pcl_protocol` support",
            "Initial PCL2 `pcl_steps` support",
        )
        and contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
            '"rust_test_count": 4429',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }

    source_files = [
        "eprover/PCL2/pcl_analysis.c",
        "eprover/PCL2/pcl_analysis.h",
        "eprover/PCL2/pcl_expressions.c",
        "eprover/PCL2/pcl_expressions.h",
        "eprover/PCL2/pcl_idents.c",
        "eprover/PCL2/pcl_idents.h",
        "eprover/PCL2/pcl_lemmas.c",
        "eprover/PCL2/pcl_lemmas.h",
        "eprover/PCL2/pcl_miniclauses.c",
        "eprover/PCL2/pcl_miniclauses.h",
        "eprover/PCL2/pcl_ministeps.c",
        "eprover/PCL2/pcl_ministeps.h",
        "eprover/PCL2/pcl_proofcheck.c",
        "eprover/PCL2/pcl_proofcheck.h",
        "eprover/PCL2/pcl_protocol.c",
        "eprover/PCL2/pcl_protocol.h",
        "eprover/PCL2/pcl_steps.c",
        "eprover/PCL2/pcl_steps.h",
        "src/pcl2/analysis.rs",
        "src/pcl2/expressions.rs",
        "src/pcl2/idents.rs",
        "src/pcl2/lemmas.rs",
        "src/pcl2/miniclauses.rs",
        "src/pcl2/ministeps.rs",
        "src/pcl2/proofcheck.rs",
        "src/pcl2/protocol.rs",
        "src/pcl2/steps.rs",
        "docs/rust-port-status.md",
        "experiments/2026-07-16-062-pcl-analysis-edges/FINDINGS.md",
        "experiments/2026-07-16-063-pcl-lemma-edges/FINDINGS.md",
        "experiments/2026-07-16-065-pcl-proofcheck-edges/FINDINGS.md",
        "experiments/2026-07-17-001-pcl-proofcheck-real-e-marker/FINDINGS.md",
        "experiments/2026-07-17-012-pcl-step-ownership/FINDINGS.md",
        "experiments/2026-07-17-013-pcl-protocol-ownership/FINDINGS.md",
        "experiments/2026-07-17-015-pcl-ministep-ownership/FINDINGS.md",
        "experiments/2026-07-17-017-pcl-miniclause-ownership/FINDINGS.md",
        "experiments/2026-07-17-018-pcl-identifier-storage/FINDINGS.md",
        "experiments/2026-07-17-019-pcl-expression-ownership/FINDINGS.md",
        "experiments/2026-07-25-046-external-reconciliation/"
        "validation-reference.json",
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
    selected_are_pcl2 = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "pcl2"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 24
        or report["content_hashes_verified"] != 24
        or not selected_are_pcl2
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("PCL2 reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
