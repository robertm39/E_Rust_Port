#!/usr/bin/env python3
"""Audit the final detailed CLAUSES Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


DECISION_GROUPS = {
    "preserve_checked_c_behavior": [
        135, 139, 144, 146, 152, 163, 166, 173, 180, 186, 188, 193,
        209, 211, 218, 220, 221, 232, 233, 235, 236, 237, 239, 240,
        241, 245, 294, 302, 305, 322, 327, 335, 347, 351, 352, 355,
        373, 376, 379, 388, 390, 398, 400, 410, 416, 422, 423, 426,
        427, 428, 434, 443, 450, 455, 459, 460, 462, 466, 467, 473,
        477, 479, 480, 484, 485, 487, 492,
    ],
    "accept_ownership_safe_rust_boundary": [
        108, 113, 114, 116, 117, 118, 129, 130, 131, 133, 137, 138,
        140, 141, 142, 143, 147, 153, 170, 174, 176, 185, 189, 204,
        219, 227, 228, 247, 248, 259, 281, 282, 292, 303, 321, 338,
        348, 349, 364, 367, 368, 369, 380, 399, 404, 405, 413, 418,
        424, 425, 430, 431, 432, 436, 438, 444, 452, 468, 488, 490,
        495,
    ],
    "superseded_by_completed_implementation": [
        110, 112, 151, 155, 157, 162, 213, 215, 216, 230, 249, 250,
        251, 252, 253, 255, 256, 257, 258, 264, 265, 267, 268, 269,
        270, 271, 272, 273, 274, 275, 276, 277, 278, 318, 326, 334,
        346, 360, 361, 362, 437, 440, 465, 469, 470, 471, 472, 474,
        475, 478, 489, 491, 493, 497,
    ],
    "accept_non_drop_in_internal_or_vendored_surface": [
        119, 121, 125, 300, 311, 312, 336, 389, 415, 446,
    ],
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
        "generation_qualified_clause_owners_cover_bce_and_indices": contains(
            repo,
            "src/clauses/bce.rs",
            "type SymbolMap = BTreeMap<FunCode, Vec<ClauseDerivationRef>>;",
            "fn bce_distinguishes_same_id_clause_generations()",
        )
        and contains(
            repo,
            "src/clauses/clausesets.rs",
            "fn demod_index_candidates_resolve_duplicate_visible_ids_exactly()",
            "pub fn find_by_derivation_ref",
        )
        and contains(
            repo,
            "experiments/2026-07-17-079-bce-handles/FINDINGS.md",
            "ClauseDerivationRef",
            "same-visible-ID",
        )
        and contains(
            repo,
            "experiments/2026-07-17-030-clausesets-exact-demod-occurrences/FINDINGS.md",
            "exact `ClauseDerivationRef`",
            "safe equivalent",
        ),
        "formula_owner_and_complete_cnf_pipeline_are_represented": contains(
            repo,
            "src/clauses/formulasets.rs",
            "fn formula_set_cnf2_runs_supported_ho_preprocessing_before_archive_drain()",
            "fn formula_set_cnf2_drains_inputs_and_archives_originals_then_cnf_copies()",
            "fn formula_set_lift_lambdas_uses_c_pdtree_general_first_order()",
            "fn formula_set_app_encode_preloads_declarations_and_skips_true_formula()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-037-formula-owner-convergence/FINDINGS.md",
            "`ProofState::f_axioms`/`FormulaSet`",
            "identical C/Rust exit codes and statuses.",
            "43/43",
        ),
        "ordered_mixed_proof_objects_and_derivation_metadata_are_complete": contains(
            repo,
            "src/clauses/proofstate.rs",
            "pub fn c_ordered_nodes(&self)",
            "fn proof_object_graph_c_order_preserves_mixed_multi_root_sibling_interleaving()",
        )
        and contains(
            repo,
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "all 56 Rust `DerivationCode`",
            "mixed multi-root/sibling ordering regression: passed",
        ),
        "higher_order_paramodulation_covers_all_six_release_orderings": contains(
            repo,
            "src/clauses/paramodulation.rs",
            "fn compute_clause_clause_paramodulants_higher_order_kbo_matches_release_surface()",
            "fn compute_clause_clause_paramodulants_higher_order_lpo4_copy_matches_release_surface()",
            "fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_lpo()",
        )
        and contains(
            repo,
            "experiments/2026-07-15-003-lfho-paramod-direct-mgu/FINDINGS.md",
            "all six concrete C",
            "for 18 native",
        ),
        "pdtree_occurrence_deletion_and_eta_paths_are_exactly_owned": contains(
            repo,
            "src/clauses/pdtrees.rs",
            "pub fn delete_term_occurrence(",
            "fn delete_term_decrements_shared_prefix_counts_and_prunes_dead_suffix()",
            "fn delete_code_removes_one_duplicate_occurrence_at_a_time()",
        )
        and contains(
            repo,
            "experiments/2026-07-14-011-pdt-eta-normalization/FINDINGS.md",
            "deletes changed keys reliably",
        )
        and contains(
            repo,
            "experiments/2026-07-17-030-clausesets-exact-demod-occurrences/FINDINGS.md",
            "exact clause pointer and equation side",
        ),
        "predicate_elimination_and_sat_owners_are_generation_safe": contains(
            repo,
            "src/clauses/pred_elim.rs",
            "fn singular_elimination_distinguishes_same_id_clause_generations()",
            "pub fn eliminate_predicates_singular_with_picosat(",
        )
        and contains(
            repo,
            "src/clauses/satinterface.rs",
            "fn picosat_core_helper_uses_fresh_solver_state_for_each_export()",
            "fn picosat_satcheck_helper_resets_after_non_unsat_result()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-078-predicate-elimination-handles/FINDINGS.md",
            "All three executions are exact",
            "ClauseDerivationRef",
        ),
        "picosat_deployment_boundary_is_explicit_and_tested": contains(
            repo,
            "src/prover/eprover.rs",
            "fn configured_picosat_library()",
            "fn bundled_picosat_library_prefers_adjacent_library()",
            "fn proof_control_from_config_reports_missing_runtime_picosat_library()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-043-contrib-picosat-reconciliation/FINDINGS.md",
            "runtime-loaded wrapper",
            "not required for E drop-in",
        ),
        "watchlist_relevance_and_proof_state_paths_have_owned_regressions": contains(
            repo,
            "src/clauses/proofstate.rs",
            "pub fn init_watchlist(",
            "pub fn proof_object_graph_for_mixed_roots",
        )
        and contains(
            repo,
            "src/clauses/relevance.rs",
            "fn relevance_data_preserves_c_observed_split_and_same_bucket_order()",
            "fn relevance_data_expands_across_formula_and_clause_symbols()",
        ),
        "rewrite_splitting_and_subsumption_quirks_are_regression_pinned": contains(
            repo,
            "src/clauses/rewrite.rs",
            "fn restricted_max_side_ignores_limited_renaming_rewrites()",
            "fn higher_order_root_rewrite_beta_normalizes_instantiated_rhs()",
        )
        and contains(
            repo,
            "src/clauses/splitting.rs",
            "fn clause_split_archives_fresh_formula_definitions_without_reuse_associations()",
        )
        and contains(
            repo,
            "src/clauses/subsumption.rs",
            "fn clause_subsumption_retries_swapped_literal_after_recursive_failure()",
            "fn clause_subsumption_picked_scratch_resets_and_supports_reentry()",
        ),
        "formula_parser_app_encode_and_unfold_boundaries_are_pinned": contains(
            repo,
            "src/clauses/clausefunc.rs",
            "fn tformula_app_encode_renders_literals_and_left_or_chain_like_c()",
            "fn tformula_app_encode_renders_fool_formula_and_term_positions()",
        )
        and contains(
            repo,
            "src/clauses/formulasets.rs",
            "fn formula_set_unfold_def_symbols_rewrites_and_archives_definitions()",
            "fn formula_set_unfold_def_symbols_duplicate_head_uses_later_definition()",
        ),
        "clausal_preprocessing_owner_transition_is_closed": contains(
            repo,
            "experiments/2026-07-18-099-clausal-preprocessing-owner-closure/FINDINGS.md",
            "19/19 checks",
            "Rust follows the same owner transition",
        )
        and contains(
            repo,
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "29/29",
        ),
        "exact_candidate_passes_full_lifecycle": contains(
            repo,
            "experiments/2026-07-25-054-clauses-reconciliation/validation-reference.json",
            '"rust_test_count": 4430',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
            '"validation_complete": true',
        ),
    }

    c_modules = sorted(
        {Path(record["source_file"]).stem for record in selected}
    )
    source_files = [
        f"eprover/CLAUSES/{module}.{extension}"
        for module in c_modules
        for extension in ("c", "h")
    ]
    source_files.extend(
        sorted(
            path.relative_to(repo).as_posix()
            for path in (repo / "src/clauses").glob("*.rs")
        )
    )
    source_files.extend(
        sorted({record["source_file"] for record in selected})
    )
    source_files.extend(
        [
            "src/heuristics/proofcontrol.rs",
            "src/prover/eprover.rs",
            "docs/rust-port-status.md",
            "experiments/2026-07-14-011-pdt-eta-normalization/FINDINGS.md",
            "experiments/2026-07-15-003-lfho-paramod-direct-mgu/FINDINGS.md",
            "experiments/2026-07-16-021-ordered-proof-object-extraction/FINDINGS.md",
            "experiments/2026-07-17-030-clausesets-exact-demod-occurrences/FINDINGS.md",
            "experiments/2026-07-17-078-predicate-elimination-handles/FINDINGS.md",
            "experiments/2026-07-17-079-bce-handles/FINDINGS.md",
            "experiments/2026-07-18-099-clausal-preprocessing-owner-closure/FINDINGS.md",
            "experiments/2026-07-18-103-preprocessing-umbrella-closure/FINDINGS.md",
            "experiments/2026-07-25-037-formula-owner-convergence/FINDINGS.md",
            "experiments/2026-07-25-043-contrib-picosat-reconciliation/FINDINGS.md",
            "experiments/2026-07-25-054-clauses-reconciliation/validation-reference.json",
        ]
    )
    source_files = sorted(set(source_files))
    missing_sources = [
        relative for relative in source_files if not (repo / relative).is_file()
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest() if not missing_sources else None
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
        "missing_sources": missing_sources,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_clauses = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "clauses"
        for record in selected
    )
    exact_partition = (
        len(ORDINALS) == 192
        and len(set(ORDINALS)) == 192
        and set(dispositions) == {str(ordinal) for ordinal in ORDINALS}
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 192
        or report["content_hashes_verified"] != 192
        or not selected_are_clauses
        or not exact_partition
        or missing_sources
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("CLAUSES reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
