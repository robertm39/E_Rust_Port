#!/usr/bin/env python3
"""Audit the final PROVER Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [
    1002,
    1006,
    1007,
    1012,
    1017,
    1020,
    1021,
    1023,
    1027,
    1029,
    1030,
    1034,
    1036,
    1037,
    1038,
    1039,
    1040,
    1041,
    1042,
    1045,
    1047,
    1055,
    1056,
    1059,
    1067,
    1069,
    1070,
    1085,
    1090,
    1095,
    1097,
    1106,
    1112,
    1122,
    1125,
    1129,
    1132,
    1134,
    1135,
    1139,
    1141,
    1142,
    1145,
    1147,
    1148,
    1149,
    1150,
    1151,
    1152,
    1153,
    1154,
    1155,
    1156,
    1157,
    1158,
    1160,
    1161,
    1162,
    1163,
    1164,
    1166,
    1167,
    1169,
    1170,
    1172,
    1173,
    1174,
    1175,
    1178,
    1180,
    1185,
    1188,
    1189,
    1190,
    1192,
    1193,
    1194,
    1195,
    1196,
    1197,
    1198,
    1202,
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
        "standalone_option_output_and_safe_undefined_boundaries_are_pinned": contains(
            repo,
            "src/prover/checkproof.rs",
            "fn short_v_is_not_a_version_option()",
            "fn scheme_setheo_matches_release_failure_and_split_unchecked_paths()",
            "fn output_file_is_created_before_later_input_open_failure()",
        )
        and contains(
            repo,
            "src/prover/classify_problem.rs",
            "fn output_file_is_created_before_later_input_open_failure()",
            "fn formula_axioms_are_clausified_before_standard_clause_preprocessing()",
            "fn real_input_dispatcher_accepts_lop_old_tptp_and_mixed_tstp_records()",
        )
        and contains(
            repo,
            "src/prover/direct_examples.rs",
            "fn output_file_is_created_before_later_input_open_failure()",
        )
        and contains(
            repo,
            "src/prover/e_axfilter.rs",
            "fn process_options_records_formats_and_seed_quirks()",
            "fn seeded_all_methods_fixture_pins_names_routing_and_generated_files()",
            "fn run_applies_verbose_and_output_globals_before_usage_error()",
        )
        and contains(
            repo,
            "src/prover/options.rs",
            "fn rust_option_table_matches_c_long_option_surface()",
            "fn rust_option_table_matches_c_short_option_surface()",
            "fn rust_option_table_matches_c_help_prose()",
        )
        and contains(
            repo,
            "src/prover/version.rs",
            "pub const PVERSION: &str = \"3.3.5\"",
            "pub const VERSION: &str = PVERSION",
            "pub const E_URL: &str = \"http://www.eprover.org\"",
        ),
        "network_server_client_and_ltb_boundaries_are_explicit": contains(
            repo,
            "src/prover/e_client.rs",
            "fn invalid_and_reserved_ports_match_c_surface()",
            "fn client_protocol_sends_c_sequence_and_echoes_until_expected_replies()",
            "fn load_problem_files_concatenates_files_and_stdin_without_separators()",
            "fn output_file_is_created_before_network_connection_attempt()",
        )
        and contains(
            repo,
            "src/prover/e_deduction_server.rs",
            "fn process_options_preserves_c_defaults_and_positional_prover()",
            "fn concurrent_tcp_clients_keep_isolated_axioms_and_parser_dialects()",
            "fn tcp_run_matches_live_c_reference_frame_bytes()",
            "fn run_applies_output_and_verbose_globals_for_stdout_unimplemented_path()",
        )
        and contains(
            repo,
            "src/prover/e_ltb_runner.rs",
            "division.category.training_data /tmp/train",
            "fn usage_error_opens_configured_output_like_c()",
        )
        and contains(
            repo,
            "src/prover/e_server.rs",
            "fn legacy_connection_replies_wait_ready_for_each_message_until_close()",
            "fn legacy_poll_keeps_one_active_connection_and_rejects_second_client()",
            "fn legacy_poll_preserves_c_failed_accept_minus_one_output()",
        ),
        "legacy_dpll_kb_and_normalizer_behavior_is_owned": contains(
            repo,
            "src/prover/edpll.rs",
            "Not completed yet!",
            "fn contradictory_units_preserve_c_trace_only_no_solver_contract()",
            "fn trailing_non_clause_tokens_are_ignored_like_c()",
        )
        and contains(
            repo,
            "src/prover/ekb_create.rs",
            "fn creates_empty_knowledge_base_files()",
            "fn existing_directory_is_rejected_like_base_mkdir_failure()",
        )
        and contains(
            repo,
            "src/prover/ekb_delete.rs",
            "fn deletes_example_and_rewrites_kb_files()",
        )
        and contains(
            repo,
            "src/prover/ekb_ginsert.rs",
            "fn stdin_protocol_generates_default_example_and_rewrites_kb_files()",
            "fn parse_options_disable_local_clause_variables_like_c()",
        )
        and contains(
            repo,
            "src/prover/enormalizer.rs",
            "fn help_and_version_preserve_c_text()",
            "fn normalizes_tstp_formula_targets()",
            "fn thf_formula_targets_parse_typed_let_under_higher_order_problem_type()",
            "fn old_tptp_formula_targets_map_lemma_and_unknown_roles_to_axiom_like_c()",
        ),
        "pcl_tools_preserve_version_streaming_weight_and_numeric_quirks": contains(
            repo,
            "src/prover/epclanalyse.rs",
            "fn short_v_is_not_a_version_option()",
            "fn output_file_is_created_before_later_input_open_failure()",
        )
        and contains(
            repo,
            "src/prover/epclextract.rs",
            "fn forward_comments_are_written_before_extracted_steps()",
            "fn output_file_is_created_before_later_input_open_failure()",
        )
        and contains(
            repo,
            "src/prover/epcllemma.rs",
            "fn short_v_is_not_a_version_option()",
            "config.params.pas_simpl_w = 0.0;",
            "fn large_protocol_uses_c_single_precision_relative_limit()",
            "fn output_file_is_created_before_later_input_open_failure()",
        )
        and contains(
            repo,
            "src/prover/termprops.rs",
            "let Some(nested) = first.argument(1) else",
            "return false;",
        )
        and contains(
            repo,
            "src/prover/tsm_classify.rs",
            "fn output_file_receives_summary_after_concatenated_inputs()",
        ),
        "formula_owner_syntax_include_and_mixed_dialect_paths_are_current": contains(
            repo,
            "src/prover/eprover.rs",
            "fn run_syntax_only_parses_supported_fof_fragment()",
            "fn run_syntax_only_parses_supported_old_tptp_input_formula()",
            "fn run_syntax_only_parses_vendored_old_tptp_formula_sample()",
            "fn run_proof_search_honors_tstp_include_name_selector()",
            "fn run_proof_search_reports_missing_tstp_include_selector()",
            "fn run_print_formulas_preserves_repeated_include_like_c()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-048-formula-owner-mode-matrix/FINDINGS.md",
            "28-case",
            "exact",
        ),
        "thf_application_lambda_and_fool_surfaces_are_regression_pinned": contains(
            repo,
            "src/prover/eprover.rs",
            "fn run_syntax_only_parses_thf_lambda_as_arrow_typed_argument()",
            "fn run_syntax_only_parses_thf_lambda_equality()",
            "fn run_syntax_only_accepts_quantified_thf_application_fragments()",
            "fn run_syntax_only_parses_tstp_logical_head_application_formula()",
            "fn run_syntax_only_parses_parenthesized_tstp_applied_head()",
            "fn run_syntax_only_parses_fof_boolean_ite_formula()",
            "fn run_syntax_only_parses_fof_boolean_let_formula()",
            "fn run_syntax_only_parses_quantified_variable_fool_term_branches()",
        ),
        "auto_sine_prune_cnf_and_app_encode_phase_order_is_exact": contains(
            repo,
            "src/prover/eprover.rs",
            "fn run_auto_mode_selects_generated_preprocessing_and_search_strategies()",
            "fn run_auto_mode_replays_explicit_sine_after_generated_strategy()",
            "fn run_prune_only_exits_before_clause_preprocessing()",
            "fn run_prune_only_applies_threshold_sine_before_initial_docs()",
            "fn run_prune_only_applies_clause_relevance_before_initial_docs()",
            "fn run_app_encode_runs_auto_preprocessing_selection_before_rendering()",
            "fn run_app_encode_applies_threshold_sine_before_rendering()",
            "fn run_cnf_only_auto_detects_tstp_sections_after_initialization()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "29/29",
        )
        and contains(
            repo,
            "experiments/2026-07-18-101-sine-formula-proof-search-closure/FINDINGS.md",
            "passes all 16 C-source",
        ),
        "ordered_proof_objects_results_and_statistics_are_complete": contains(
            repo,
            "src/prover/eprover.rs",
            "fn run_proof_object_zero_does_not_enable_proof_object_output()",
            "fn proof_object_list_display_clauses_prints_parents_before_children()",
            "fn proof_object_list_graph_prints_formula_nodes_and_remapped_edges()",
            "fn run_proof_object_list_prints_saturation_block_for_no_proof_tstp()",
            "fn run_force_deriv_level2_prints_unprocessed_resource_roots()",
            "fn run_proof_statistics_with_record_gcs_prints_success_proof_object_analysis()",
            "fn proof_success_status_follows_formula_conjecture_parents()",
        )
        and contains(
            repo,
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "exact final C order",
            "all 56 Rust `DerivationCode`",
            "entries against C's operation-id/status/theory tables",
        ),
        "option_fallthrough_format_types_and_resource_ownership_are_pinned": contains(
            repo,
            "src/prover/eprover.rs",
            "fn process_options_records_lpo_literal_comparison_fallthrough_like_c()",
            "fn process_options_rejects_invalid_term_ordering_args()",
            "fn run_emits_lpo_recursion_limit_warning_like_c()",
            "fn run_keeps_lpo_recursion_limit_warning_before_later_option_error()",
            "fn run_print_formulas_honors_print_types()",
            "fn run_resources_info_prints_c_shaped_footer()",
            "fn run_proof_search_resources_info_prints_preprocessing_time()",
            "fn resource_limit_warning_text_matches_c_shapes()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-107-resource-limit-ownership/FINDINGS.md",
            "resource",
        ),
        "support_tools_and_full_candidate_have_zero_unexpected_differences": contains(
            repo,
            "experiments/2026-07-18-127-support-tool-matrix-closure/FINDINGS.md",
            "all 216 configured cases",
            "zero unexpected mismatches",
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

    c_files = [
        f"eprover/PROVER/{module}.c"
        for module in [
            "checkproof",
            "classify_problem",
            "direct_examples",
            "e_axfilter",
            "e_client",
            "e_deduction_server",
            "e_ltb_runner",
            "e_server",
            "edpll",
            "ekb_create",
            "ekb_delete",
            "ekb_ginsert",
            "enormalizer",
            "epclanalyse",
            "epclextract",
            "epcllemma",
            "eprover",
            "termprops",
            "tsm_classify",
        ]
    ]
    c_files.extend(
        [
            "eprover/PROVER/e_options.h",
            "eprover/PROVER/e_version.h",
        ]
    )
    rust_modules = [
        "checkproof",
        "classify_problem",
        "direct_examples",
        "e_axfilter",
        "e_client",
        "e_deduction_server",
        "e_ltb_runner",
        "options",
        "e_server",
        "version",
        "edpll",
        "ekb_create",
        "ekb_delete",
        "ekb_ginsert",
        "enormalizer",
        "epclanalyse",
        "epclextract",
        "epcllemma",
        "eprover",
        "termprops",
        "tsm_classify",
    ]
    source_files = c_files + [
        f"src/prover/{module}.rs" for module in rust_modules
    ]
    binary_modules = [
        module
        for module in rust_modules
        if module not in {"options", "version"}
    ]
    source_files.extend(f"src/bin/{module}.rs" for module in binary_modules)
    source_files.extend(
        [
            "docs/rust-port-status.md",
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "experiments/2026-07-16-023-deduction-server-concurrency/FINDINGS.md",
            "experiments/2026-07-16-025-e-server-loop-compatibility/FINDINGS.md",
            "experiments/2026-07-16-027-e-axfilter-comparison-matrix/FINDINGS.md",
            "experiments/2026-07-16-039-epclextract-expanded-comparison/FINDINGS.md",
            "experiments/2026-07-16-040-epclanalyse-platform-boundaries/FINDINGS.md",
            "experiments/2026-07-16-041-checkproof-external-coverage/FINDINGS.md",
            "experiments/2026-07-16-042-epcllemma-expanded-comparison/FINDINGS.md",
            "experiments/2026-07-16-043-classify-parser-merged-boundaries/FINDINGS.md",
            "experiments/2026-07-16-050-enormalizer-wformula-parity/FINDINGS.md",
            "experiments/2026-07-17-044-deduction-server-run-framing/FINDINGS.md",
            "experiments/2026-07-17-048-formula-owner-mode-matrix/FINDINGS.md",
            "experiments/2026-07-18-101-sine-formula-proof-search-closure/FINDINGS.md",
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "experiments/2026-07-18-107-resource-limit-ownership/FINDINGS.md",
            "experiments/2026-07-18-127-support-tool-matrix-closure/FINDINGS.md",
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
    selected_are_prover = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "prover"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 82
        or report["content_hashes_verified"] != 82
        or not selected_are_prover
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("PROVER reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
