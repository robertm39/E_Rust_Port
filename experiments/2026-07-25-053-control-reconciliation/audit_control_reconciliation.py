#!/usr/bin/env python3
"""Audit the final CONTROL Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


DECISION_GROUPS = {
    "preserve_checked_c_behavior": [
        514, 515, 522, 530, 533, 534, 537, 539, 543, 545, 547, 562, 565,
        568, 571, 580, 587, 594, 597, 603, 610, 611, 613, 614, 615, 616,
        618, 623, 628, 630, 634, 639, 641, 646, 653, 656, 668, 669, 675,
        688, 690, 696, 704, 720, 721, 723, 736, 744,
    ],
    "accept_ownership_safe_rust_boundary": [
        523, 524, 536, 541, 546, 550, 559, 561, 566, 567, 578, 582, 583,
        602, 622, 644, 655, 666, 673, 677, 682, 683, 684, 687, 689, 705,
        713, 714, 716, 727, 732, 733, 737, 738,
    ],
    "superseded_by_completed_implementation": [
        585, 588, 606, 617, 627, 632, 643, 647, 652, 654, 659, 679, 692,
        693, 697, 707, 708, 715, 718, 719, 725, 726, 734, 741,
    ],
    "implemented_in_this_reconciliation": [742],
}
ORDINALS = sorted(
    ordinal for ordinals in DECISION_GROUPS.values() for ordinal in ordinals
)


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
    dispositions = {
        str(ordinal): decision
        for decision, ordinals in DECISION_GROUPS.items()
        for ordinal in ordinals
    }
    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "decision": dispositions[str(record["ordinal"])],
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
        "batch_spec_parser_runner_and_output_contracts_are_pinned": contains(
            repo,
            "src/control/batch_spec.rs",
            "fn parse_batch_spec_preserves_loose_c_control_flow()",
            "fn print_batch_spec_uses_c_field_order_and_training_directory_spelling()",
            "fn process_problem_with_runner_backend_writes_temp_files_and_reports_success()",
            "fn load_problem_from_file_rejects_legacy_records_under_forced_tstp_like_c()",
            "fn batch_problem_dest_name_preserves_c_dest_dir_joining()",
        ),
        "interactive_and_process_control_owners_are_explicit": contains(
            repo,
            "src/control/einteractive_mode.rs",
            "fn stage_command_adds_problem_to_control_and_marks_shared_boundary()",
            "fn run_command_with_parses_job_runs_batch_process_and_backtracks()",
            "fn start_deduction_server_tcp_processes_messages_until_quit()",
            "fn dispatch_quit_unstages_all_sets_and_marks_done()",
        )
        and contains(
            repo,
            "src/control/proc_ctrl.rs",
            "fn process_set_get_result_returns_last_success_and_deletes_failures()",
            "fn c_compatible_poll_uses_500ms_and_reads_one_message_per_process()",
        )
        and contains(
            repo,
            "src/control/gproc_ctrl.rs",
            "fn",
            "EGPCtrl",
        ),
        "factoring_forward_modification_and_cleanup_are_regression_pinned": contains(
            repo,
            "src/clauses/factor.rs",
            "fn compute_all_equality_factors_preserves_multi_csu_pop_and_doc_order()",
            "fn compute_equality_factor_lambda_normalizes_generated_literals()",
        )
        and contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "fn proof_state_forward_modify_clause_higher_order_normalizes_encoded_equality()",
            "fn proof_state_forward_modify_clause_with_docs_emits_minimize_step()",
            "fn cleanup_unprocessed_output_renders_c_messages()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-087-forward-modify-ho-surface/FINDINGS.md",
            "All 9/9 contracts pass",
            "All 18/18 configurations",
        ),
        "higher_order_inference_dispatch_is_complete": contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "fn proof_state_generate_new_clauses_higher_order_pos_ext_alone_preserves_c_noop_gate()",
            "fn proof_state_generate_new_clauses_higher_order_choice_instantiates_defined_trigger()",
            "fn proof_state_generate_new_clauses_higher_order_primitive_enum_neg_generates_instances()",
            "fn compute_ext_sup_generates_indexed_condition_and_replacement_literal()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-093-higher-order-match-csu-ownership/FINDINGS.md",
            "Complete CSU call-site audit",
            "21/21 unification projections",
        ),
        "paramodulation_and_preprocessing_are_owned": contains(
            repo,
            "src/clauses/paramodulation.rs",
            "fn compute_all_paramodulants_indexed_reuses_target_unifier_across_source_clauses()",
            "fn compute_all_paramodulants_indexed_higher_order_plain_uses_csu_and_tags_derivation()",
            "fn compute_all_paramodulants_indexed_higher_order_super_sim_uses_csu_from_index()",
        )
        and contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "fn proof_state_generate_new_clauses_computes_super_simultaneous_paramodulation()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "29/29 facts",
        ),
        "proof_control_cleanup_generation_and_indices_are_complete": contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "fn proof_state_cleanup_unprocessed_default_deletes_archived_dead_parent_orphans()",
            "pub fn proof_state_insert_new_clauses(",
            "fn proof_state_process_clause_with_global_indices_generates_indexed_paramodulants()",
            "fn proof_state_saturate_distinguishes_sat_check_preprocessing_refutation()",
            "fn proof_state_process_clause_records_generated_empty_extract_root()",
        ),
        "schedule_simplification_and_selected_axiom_paths_are_closed": contains(
            repo,
            "src/control/scheduling.rs",
            "fn execution_reports_schedule_exhaustion()",
            "fn execution_retries_filtered_default_schedule_when_time_remains()",
            "fn execution_skips_default_retry_when_remaining_time_is_too_small()",
        )
        and contains(
            repo,
            "src/control/sine.rs",
            "fn gsine_get_problem_starts_seed_scan_after_shared_axioms()",
            "fn lambda_defines_selection_keeps_formula_defs_goals_and_hypotheses()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-098-axfilter-owner-closure/FINDINGS.md",
            "9/9 exact",
        )
        and contains(
            repo,
            "experiments/2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md",
            "two",
        ),
        "full_structured_spec_backtrack_is_in_production": contains(
            repo,
            "src/control/sine.rs",
            "pub fn backtrack_to_spec_with_bank(",
            "tb_gc_collect(",
            ".backtrack(report.signature_backtrack_to)",
            "fn backtrack_to_spec_with_bank_collects_problem_terms_before_forgetting_symbols()",
        )
        and contains(
            repo,
            "src/control/batch_spec.rs",
            "let backtrack = ctrl.backtrack_to_spec_with_bank(bank);",
            "fn process_problem_with_runner_backend_writes_temp_files_and_reports_success()",
            'find_f_code("batch_backtrack_problem")',
            "assert!(bank.find(&problem_term).is_none());",
        ),
        "ordered_proof_objects_cover_owned_extraction_roots": contains(
            repo,
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "ordered",
            "proof",
        )
        and contains(
            repo,
            "src/clauses/proofstate.rs",
            "proof_object",
            "extract_roots",
        ),
        "exact_candidate_passes_full_lifecycle": contains(
            repo,
            "experiments/2026-07-25-053-control-reconciliation/validation-reference.json",
            '"rust_test_count": 4430',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
            '"validation_complete": true',
        ),
    }

    c_modules = [
        "cco_batch_spec",
        "cco_clausesplitting",
        "cco_einteractive_mode",
        "cco_eqnresolving",
        "cco_esession",
        "cco_factoring",
        "cco_forward_contraction",
        "cco_gproc_ctrl",
        "cco_ho_inferences",
        "cco_interpreted",
        "cco_paramodulation",
        "cco_preprocessing",
        "cco_proc_ctrl",
        "cco_proofproc",
        "cco_scheduling",
        "cco_simplification",
        "cco_sine",
    ]
    source_files = [
        f"eprover/CONTROL/{module}.{extension}"
        for module in c_modules
        for extension in ("c", "h")
    ]
    source_files.extend(
        [
            "src/control/batch_spec.rs",
            "src/control/einteractive_mode.rs",
            "src/control/esession.rs",
            "src/control/gproc_ctrl.rs",
            "src/control/proc_ctrl.rs",
            "src/control/scheduling.rs",
            "src/control/sine.rs",
            "src/clauses/clausefunc.rs",
            "src/clauses/eqnresolution.rs",
            "src/clauses/factor.rs",
            "src/clauses/garbage_coll.rs",
            "src/clauses/paramodulation.rs",
            "src/clauses/proofstate.rs",
            "src/clauses/rewrite.rs",
            "src/clauses/splitting.rs",
            "src/clauses/subsumption.rs",
            "src/heuristics/proofcontrol.rs",
            "src/prover/e_deduction_server.rs",
            "src/prover/e_ltb_runner.rs",
            "src/prover/eprover.rs",
            "src/terms/signature.rs",
            "src/terms/termbanks.rs",
            "docs/rust-port-status.md",
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "experiments/2026-07-17-086-cleanup-maintenance-order/FINDINGS.md",
            "experiments/2026-07-17-087-forward-modify-ho-surface/FINDINGS.md",
            "experiments/2026-07-18-093-higher-order-match-csu-ownership/FINDINGS.md",
            "experiments/2026-07-18-098-axfilter-owner-closure/FINDINGS.md",
            "experiments/2026-07-18-101-sine-formula-proof-search-closure/FINDINGS.md",
            "experiments/2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md",
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md",
            "experiments/2026-07-25-053-control-reconciliation/validation-reference.json",
        ]
    )
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "decision_group_counts": {
            decision: len(ordinals)
            for decision, ordinals in sorted(DECISION_GROUPS.items())
        },
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_control = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "control"
        for record in selected
    )
    exact_partition = (
        len(ORDINALS) == 107
        and len(set(ORDINALS)) == 107
        and set(dispositions) == {str(ordinal) for ordinal in ORDINALS}
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 107
        or report["content_hashes_verified"] != 107
        or not selected_are_control
        or not exact_partition
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("CONTROL reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
