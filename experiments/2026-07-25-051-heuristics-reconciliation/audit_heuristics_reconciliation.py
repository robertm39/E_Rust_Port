#!/usr/bin/env python3
"""Audit the final HEURISTICS Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [
    761,
    762,
    765,
    770,
    773,
    774,
    775,
    777,
    778,
    783,
    784,
    786,
    791,
    792,
    801,
    802,
    805,
    808,
    809,
    810,
    811,
    812,
    813,
    814,
    815,
    816,
    820,
    821,
    822,
    825,
    829,
    831,
    832,
    833,
    834,
    838,
    839,
    840,
    841,
    842,
    843,
    844,
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


def excludes(repo: Path, relative: str, *needles: str) -> bool:
    text = source(repo, relative)
    return all(needle not in text for needle in needles)


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

    scorer_tests = {
        "src/heuristics/dagweight.rs":
            "fn refined_dag_weight_parse_banked_callback_marks_clause_like_c()",
        "src/heuristics/diversityweight.rs":
            "fn diversity_weight_parse_banked_callback_marks_clause_like_c()",
        "src/heuristics/funweights.rs":
            "fn fun_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/levweight.rs":
            "fn conjecture_lev_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/orientweight.rs":
            "fn orient_weight_parse_banked_callbacks_mark_clause_like_c()",
        "src/heuristics/prefixweight.rs":
            "fn conjecture_prefix_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/refinedweight.rs":
            "fn refined_weight_parse_banked_callbacks_mark_clause_like_c()",
        "src/heuristics/strucweight.rs":
            "fn conjecture_struc_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/termweights.rs":
            "fn relative_term_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/tfidfweight.rs":
            "fn conjecture_tfidf_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/treeweight.rs":
            "fn conjecture_tree_weight_parse_uses_banked_wfcb_callback()",
        "src/heuristics/varweights.rs":
            "fn type_signature_and_proof_weight_parsers_use_banked_wfcb_callbacks()",
        "src/heuristics/wfcb.rs":
            "fn clause_add_evaluation_with_bank_uses_banked_callback()",
    }
    checks = {
        "axfilter_parser_defaults_and_live_owner_are_reconciled": contains(
            repo,
            "src/heuristics/axfilter.rs",
            "fn default_ax_filter_matches_c_allocation_defaults()",
            "fn ax_filter_print_strings_match_c_shapes()",
            "fn ax_filter_set_preserves_stack_order_and_name_lookup()",
            "fn ax_filter_parser_preserves_unimplemented_generality_measure_diagnostic()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-023-f-generality-reconciliation/FINDINGS.md",
            "reject every selection measure except term and formula counts",
        )
        and contains(
            repo,
            "experiments/2026-07-18-098-axfilter-owner-closure/FINDINGS.md",
            "PStack<&Clause>",
            "allocation-unique `WrappedFormula::entry_id`",
            "9/9",
        ),
        "rendering_scalars_noop_and_priority_quirk_are_explicit": contains(
            repo,
            "src/heuristics/clausefeatures.rs",
            "fn clause_prop_info_stats_string_matches_c_stat_block_format()",
            "fn clause_prop_info_string_prefixes_pcl_text_and_appends_stats()",
            "fn clause_prop_info_print_string_uses_default_pcl_clause_rendering()",
        )
        and contains(
            repo,
            "src/heuristics/fifo.rs",
            "fn fifo_eval_increments_before_returning_like_c()",
            "fn fifo_wfcb_init_wraps_stateful_counter()",
        )
        and contains(
            repo,
            "src/heuristics/lifo.rs",
            "fn lifo_eval_decrements_before_returning_like_c()",
            "fn lifo_wfcb_init_wraps_stateful_counter()",
        )
        and contains(
            repo,
            "eprover/HEURISTICS/che_normsubst.h",
            "NormSubstFree",
        )
        and excludes(
            repo,
            "eprover/HEURISTICS/che_normsubst.c",
            "NormSubstFree",
        )
        and contains(
            repo,
            "src/heuristics/normsubst.rs",
            "pub fn norm_subst_free(_junk: NormSubstCell) {}",
        )
        and contains(
            repo,
            "src/heuristics/prio_funs.rs",
            "fn higher_order_priority_quirks_preserve_current_c_results()",
            "prio_fun_prefer_ho_steps(&bank, &clause), PRIO_NORMAL",
        ),
        "banked_scorer_lifecycle_covers_every_marking_family": all(
            contains(repo, relative, needle)
            for relative, needle in scorer_tests.items()
        )
        and contains(
            repo,
            "experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md",
            "remaining immutable functions are deliberate low-level/test adapters",
            "eight banked proof-control calls",
        ),
        "hcb_selection_uses_stable_liveness_and_banked_dispatch": contains(
            repo,
            "src/heuristics/hcb.rs",
            "fn standard_clause_select_extracts_best_and_discards_orphans()",
            "fn hcb_clause_evaluate_with_bank_uses_banked_wfcb_dispatch()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-063-orphan-liveness-identity/FINDINGS.md",
            "generation",
            "liveness",
        ),
        "literal_selection_surface_and_edge_cases_are_exact": contains(
            repo,
            "src/heuristics/litselection.rs",
            "fn orientable_selectors_fall_back_to_weight_when_none_orient()",
            "fn min_optimal_type_filtered_no_selection_preserves_direct_call_state()",
            "fn diversification_selectors_preserve_c_counter_and_into_priority()",
            "fn new_complex_falls_back_to_largest_non_type_x_type_literal()",
            "fn min_infpos_uses_standard_function_weight_two()",
            "fn min_infpos_no_type_pred_variants_filter_and_allow_no_selection()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-091-literal-selection-option-surface/FINDINGS.md",
            "144/144 names in exact C table order",
            "144/144 exact normalized execution summaries",
            "nine executable",
        ),
        "autoschedule_static_tables_and_parser_quirks_are_exact": contains(
            repo,
            "src/heuristics/new_autoschedule.rs",
            "fn generated_static_tables_exactly_match_schedule_vars()",
            "fn schedule_string_distance_matches_c_positional_difference()",
            "fn generated_schedule_tables_resolve_preprocessing_search_and_default()",
            "fn generated_schedule_partial_match_reports_selected_class()",
            "fn placeholder_schedule_insertion_preserves_c_mutation_shape()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-006-static-autoschedule-tables/FINDINGS.md",
            "all 419 predefined strategies",
            "all 1,618",
            "both class maps",
        )
        and contains(
            repo,
            "experiments/2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md",
            "19/19",
        ),
        "proof_control_initialization_sat_and_ho_bank_surface_are_owned": contains(
            repo,
            "src/heuristics/proofcontrol.rs",
            "sat_solver_backend: SatSolverBackend::Internal",
            "pub fn install_picosat_solver",
            "fn proof_control_reset_sat_solver_reinitializes_trace_state()",
            "fn proof_control_keeps_internal_backend_after_missing_picosat_install()",
            "fn proof_control_init_installs_default_definitions_and_active_hcb()",
            "fn proof_state_forward_modify_clause_higher_order_normalizes_encoded_equality()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-043-contrib-picosat-reconciliation/FINDINGS.md",
            "explicit runtime-loaded wrapper plus tested internal fallback",
            "eight reentrant symbols E",
        )
        and contains(
            repo,
            "experiments/2026-07-17-087-forward-modify-ho-surface/FINDINGS.md",
            "higher-order",
        ),
        "autoselect_safe_initialization_preserves_observable_search": contains(
            repo,
            "src/heuristics/to_autoselect.rs",
            "fn auto_ordering_params_match_initialized_c_auto_sched_variants()",
            "fn casc_and_dev_auto_orderings_use_initialized_kbo6_defaults()",
            "fn predefined_only_precedence_dependent_weights_use_parsed_matrix()",
            "fn order_find_optimal_treats_optimize_as_wildcard_order_type()",
            "fn instrumented_c_reference_ordering_search_state_matches()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-073-autoselect-state/FINDINGS.md",
            "exactly 1,972 candidate states",
            "same 1,972 indexed states",
        ),
        "current_full_port_validation_is_green": contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
            '"rust_test_count": 4429',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }

    c_modules = [
        "axfilter",
        "clausefeatures",
        "dagweight",
        "diversityweight",
        "fifo",
        "funweights",
        "hcb",
        "levweight",
        "lifo",
        "litselection",
        "new_autoschedule",
        "normsubst",
        "orientweight",
        "prefixweight",
        "prio_funs",
        "proofcontrol",
        "refinedweight",
        "strucweight",
        "termweight",
        "tfidfweight",
        "to_autoselect",
        "treeweight",
        "varweights",
        "wfcb",
    ]
    source_files = [
        relative
        for module in c_modules
        for relative in (
            f"eprover/HEURISTICS/che_{module}.c",
            f"eprover/HEURISTICS/che_{module}.h",
        )
    ]
    rust_modules = [
        "axfilter",
        "clausefeatures",
        "dagweight",
        "diversityweight",
        "fifo",
        "funweights",
        "hcb",
        "levweight",
        "lifo",
        "litselection",
        "new_autoschedule",
        "normsubst",
        "orientweight",
        "prefixweight",
        "prio_funs",
        "proofcontrol",
        "refinedweight",
        "strucweight",
        "termweights",
        "tfidfweight",
        "to_autoselect",
        "treeweight",
        "varweights",
        "wfcb",
    ]
    source_files.extend(f"src/heuristics/{module}.rs" for module in rust_modules)
    source_files.extend(
        [
            "src/heuristics/schedule_vars_parser.rs",
            "src/prover/eprover.rs",
            "docs/rust-port-status.md",
            "experiments/2026-07-17-023-f-generality-reconciliation/FINDINGS.md",
            "experiments/2026-07-16-026-e-axfilter-formula-filters/FINDINGS.md",
            "experiments/2026-07-18-098-axfilter-owner-closure/FINDINGS.md",
            "experiments/2026-07-17-085-clause-feature-output-callers/FINDINGS.md",
            "experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md",
            "experiments/2026-07-17-063-orphan-liveness-identity/FINDINGS.md",
            "experiments/2026-07-18-091-literal-selection-option-surface/FINDINGS.md",
            "experiments/2026-07-25-006-static-autoschedule-tables/FINDINGS.md",
            "experiments/2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md",
            "experiments/2026-07-17-073-autoselect-state/FINDINGS.md",
            "experiments/2026-07-17-052-inference-satcheck-option-matrix/FINDINGS.md",
            "experiments/2026-07-25-043-contrib-picosat-reconciliation/FINDINGS.md",
            "experiments/2026-07-17-087-forward-modify-ho-surface/FINDINGS.md",
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
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
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_heuristics = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "heuristics"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 42
        or report["content_hashes_verified"] != 42
        or not selected_are_heuristics
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("HEURISTICS reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
