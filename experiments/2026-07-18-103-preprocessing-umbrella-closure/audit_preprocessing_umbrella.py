#!/usr/bin/env python3
"""Audit the migrated preprocessing umbrella against its dedicated owners."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


OWNER_BEADS = (
    "E_Rust_Port-j76.2.37",  # SInE formula owners
    "E_Rust_Port-j76.2.38",  # formula relevance
    "E_Rust_Port-j76.2.39",  # clause passes and presaturation
    "E_Rust_Port-j76.2.43",  # higher-order option effects and induction
    "E_Rust_Port-j76.2.46",  # option-to-parameter bridge
    "E_Rust_Port-j76.2.47",  # ProofControl ownership
    "E_Rust_Port-j76.2.57",  # AC scanning/activation
    "E_Rust_Port-j76.2.61",  # preprocessing/equality unfolding/watchlists
    "E_Rust_Port-j76.2.105",  # defined-choice recognition
)


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_bead_statuses(path: Path) -> dict[str, str]:
    statuses = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        issue = json.loads(line)
        if issue.get("_type") == "issue":
            statuses[str(issue["id"])] = str(issue["status"])
    return statuses


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--induction-reference", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_driver = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    c_preprocessing = (root / "eprover/CONTROL/cco_preprocessing.c").read_text(
        encoding="utf-8"
    )
    rust_driver = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_search = rust_driver.split("fn run_proof_search", maxsplit=1)[1].split(
        "fn hard_time_limit_expired_in_saturation", maxsplit=1
    )[0]
    rust_prefix = rust_driver.split(
        "fn apply_clause_set_preprocessing_prefix", maxsplit=1
    )[1].split("fn apply_choice_axiom_recognition", maxsplit=1)[0]
    statuses = load_bead_statuses(root / ".beads/issues.jsonl")

    references = {
        "eqdef": load_json(
            root / "experiments/2026-07-17-076-classifier-eqdef-boundary/reference.json"
        ).get("all_exact")
        is True,
        "goal_defs": load_json(
            root / "experiments/2026-07-17-077-goal-definition-order/reference.json"
        ).get("all_exact")
        is True,
        "predicate_elimination": load_json(
            root
            / "experiments/2026-07-17-078-predicate-elimination-handles/reference.json"
        ).get("all_exact")
        is True,
        "bce": load_json(
            root / "experiments/2026-07-17-079-bce-handles/reference.json"
        ).get("all_exact")
        is True,
        "ac": load_json(
            root / "experiments/2026-07-17-080-axiomscan-ac-ownership/reference.json"
        ).get("all_exact")
        is True,
        "ho_option_effects": (
            load_json(
                root
                / "experiments/2026-07-18-094-higher-order-option-effects/comparison-reference.json"
            ).get("mismatches")
            == []
        ),
        "clausal_owner_audit": load_json(
            root
            / "experiments/2026-07-18-099-clausal-preprocessing-owner-closure/owner-audit.json"
        ).get("all_passed")
        is True,
        "relevance": load_json(
            root
            / "experiments/2026-07-18-100-formula-relevance-proof-search-closure/reference.json"
        ).get("all_exact")
        is True,
        "sine": load_json(
            root
            / "experiments/2026-07-18-101-sine-formula-proof-search-closure/reference.json"
        ).get("all_exact")
        is True,
    }
    induction = load_json(args.induction_reference)

    c_phase_markers = (
        "ClauseSetArchiveCopy(",
        "ClauseSetPreprocess(",
        "ClauseSetUnfoldEqDefNormalize(",
        "ClauseSetRecognizeChoice(",
        "PreinstantiateInduction(",
        "EliminateBlockedClauses(",
        "PredicateElimination(",
        "ClauseSetGDTransform(",
    )
    rust_phase_markers = (
        "apply_proof_state_sine(",
        "apply_relevance_pruning(",
        "clausify_formula_axioms_with_docs(",
        "apply_clause_set_preprocessing_with_docs(",
        "apply_choice_axiom_recognition(",
        "apply_induction_preinstantiation(",
        "apply_blocked_clause_elimination(",
        "apply_predicate_elimination(",
        "apply_goal_definition_transformation(",
        "write_initial_clause_docs(",
        "proof_control_init_with_formula_axioms(",
        "run_presaturation_interreduction(",
    )
    c_positions = [c_preprocessing.index(marker) for marker in c_phase_markers]
    rust_positions = [rust_search.index(marker) for marker in rust_phase_markers]

    materialized_fields = (
        "no_preproc: preprocessing.no_preprocessing",
        "eqdef_maxclauses: preprocessing.eqdef_maxclauses",
        "eqdef_incrlimit: preprocessing.eqdef_incrlimit",
        "add_goal_defs_pos: preprocessing.goal_definitions.positive",
        "add_goal_defs_neg: preprocessing.goal_definitions.negative",
        "add_goal_defs_subterms: preprocessing.goal_definitions.subterms",
        "bce: preprocessing.bce.enabled",
        "pred_elim: pred_elim.enabled",
        "ac_handling: hcb_ac_handling(preprocessing.ac_handling)",
        "presat_interreduction: preprocessing.presat_interreduction",
        "preinstantiate_induction: ho_preprocessing.preinstantiate_induction",
    )
    permanent_tests = (
        "process_options_records_preprocessing_state_like_c",
        "heuristic_parms_from_config_maps_cli_search_state",
        "apply_clause_set_preprocessing_archives_original_axioms_like_c",
        "run_proof_search_statistics_count_formula_relevance_pruning",
        "run_proof_search_statistics_count_formula_threshold_sine_pruning",
        "run_proof_search_statistics_count_formula_gsine_pruning",
        "run_cnf_only_preinstantiates_induction_like_c",
        "run_proof_search_applies_bce_to_fof_formula_origin_clauses",
        "run_proof_search_skips_fo_only_preprocessing_after_thf_cnf",
        "run_proof_search_applies_pred_elim_to_fof_formula_origin_clauses",
        "run_proof_search_applies_goal_defs_to_fof_formula_origin_conjecture",
        "run_proof_search_unfolds_formula_origin_eq_definition_before_saturation",
        "run_proof_search_presaturation_handles_fof_formula_origin_clause",
        "run_output_level_two_unfolds_inline_watchlist_before_docs",
        "run_output_level_two_unfolds_file_watchlist_before_docs",
        "run_proof_search_prints_initial_ac_scan_status",
        "run_proof_search_reports_associativity_without_ac_activation",
        "run_proof_search_reports_combined_ac_status",
    )

    checks = {
        **{
            f"owner_bead_closed:{issue_id}": statuses.get(issue_id) == "closed"
            for issue_id in OWNER_BEADS
        },
        "c_sine_relevance_cnf_order": (
            c_driver.index("ProofStateSinE(")
            < c_driver.index("ProofStateRelevancyProcess(")
            < c_driver.index("FormulaSetCNF2(")
            < c_driver.index("ProofStateClausalPreproc(")
        ),
        "c_clausal_preprocessing_order": c_positions == sorted(c_positions),
        "c_no_preprocessing_still_unfolds": (
            c_preprocessing.index("if(!h_parms->no_preproc)")
            < c_preprocessing.index("ClauseSetPreprocess(")
            < c_preprocessing.index("ClauseSetUnfoldEqDefNormalize(")
        ),
        "rust_full_preprocessing_order": rust_positions == sorted(rust_positions),
        "rust_archives_before_optional_clause_preprocess": (
            rust_prefix.index("clause_set_archive_copy(")
            < rust_prefix.index("if !no_preprocessing")
            < rust_prefix.index("clause_set_preprocess(")
        ),
        "rust_unfolds_after_optional_prefix": (
            rust_search.index("apply_clause_set_preprocessing_with_docs(")
            < rust_search.index("apply_choice_axiom_recognition(")
            and "clause_set_unfold_eq_def_normalize" in rust_driver
        ),
        "rust_no_eq_unfolding_uses_c_sentinel": (
            "config.preprocessing.eqdef_incrlimit = i64::MIN" in rust_driver
        ),
        "rust_materializes_all_umbrella_fields": all(
            marker in rust_driver for marker in materialized_fields
        ),
        "rust_permanent_regressions_present": all(
            marker in rust_driver for marker in permanent_tests
        ),
        **{f"retained_reference_exact:{name}": exact for name, exact in references.items()},
        "fresh_induction_reference_exact": induction.get("all_exact") is True,
        "fresh_induction_effect_observed": induction.get("effect_observed") is True,
    }
    report = {
        "schema_version": 1,
        "checks": checks,
        "passed": sum(checks.values()),
        "total": len(checks),
        "all_passed": all(checks.values()),
    }
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    if not report["all_passed"]:
        failed = [name for name, passed in checks.items() if not passed]
        print(f"preprocessing umbrella audit failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {report['passed']}/{report['total']} umbrella owner checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
