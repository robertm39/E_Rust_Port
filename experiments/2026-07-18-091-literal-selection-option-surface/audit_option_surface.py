#!/usr/bin/env python3
"""Audit literal-selection and inference-option ownership contracts."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    repo = args.repo.resolve()
    literal_selection = (repo / "src/heuristics/litselection.rs").read_text(encoding="utf-8")
    hcb = (repo / "src/heuristics/hcb.rs").read_text(encoding="utf-8")
    proof_control = (repo / "src/heuristics/proofcontrol.rs").read_text(encoding="utf-8")
    prover = (repo / "src/prover/eprover.rs").read_text(encoding="utf-8")
    c_selection = (repo / "eprover/HEURISTICS/che_litselection.c").read_text(
        encoding="utf-8"
    )

    contracts = {
        "c_standard_min_infpos_uses_term_standard_weight": (
            "void SelectMinInfpos(OCB_p ocb, Clause_p clause)" in c_selection
            and "currw = TermStandardWeight(handle->lterm);" in c_selection
        ),
        "rust_standard_min_infpos_uses_default_weights": (
            "use crate::terms::termtypes::{DEFAULT_FWEIGHT, DEFAULT_VWEIGHT};"
            in literal_selection
            and literal_selection.count("DEFAULT_VWEIGHT,\n        DEFAULT_FWEIGHT,") == 2
        ),
        "rust_min2_infpos_retains_two_one_weights": (
            literal_selection.count("MinInfposPositivePolicy::Never,\n                false,\n                2,\n                1,")
            == 1
            and literal_selection.count(
                "MinInfposPositivePolicy::BeforeSelection,\n                false,\n                2,\n                1,"
            )
            == 1
        ),
        "selector_table_count_regression": (
            "fn literal_selection_name_table_preserves_c_order_and_append_shape()"
            in literal_selection
            and "assert_eq!(names.len(), 144);" in literal_selection
        ),
        "all_mutable_bank_selectors_regression": (
            "fn all_advertised_literal_selection_names_dispatch_with_mutable_bank()"
            in literal_selection
        ),
        "standard_weight_direct_regression": (
            "fn min_infpos_uses_standard_function_weight_two()" in literal_selection
        ),
        "standard_weight_mutable_bank_regression": (
            "fn mutable_bank_min_infpos_uses_standard_function_weight_two()"
            in literal_selection
        ),
        "config_bridge_preserves_selection_strategy": (
            "selection_strategy: literal_selection.strategy.clone()," in prover
        ),
        "config_bridge_preserves_equality_factoring": (
            "enable_eq_factoring: inference.enable_eq_factoring," in prover
        ),
        "config_bridge_preserves_paramodulation_mode": (
            "pm_type: hcb_paramodulation_type(inference.paramodulation)," in prover
        ),
        "config_bridge_preserves_raw_split_classes": (
            "split_clauses: hcb_split_class_type(splitting.classes)?," in prover
            and "handle.split_clauses = SplitClassType::from_c_value(value);" in hcb
        ),
        "config_bridge_preserves_disequality_controls": (
            "diseq_decomposition: splitting.diseq_decomposition," in prover
            and "diseq_decomp_maxarity: splitting.diseq_decomp_maxarity," in prover
        ),
        "proof_generation_consumes_equality_factoring": (
            "if enable_eq_factoring {" in proof_control
        ),
        "proof_generation_consumes_paramodulation_mode": (
            "control.heuristic_parms().pm_type," in proof_control
        ),
        "proof_generation_consumes_disequality_controls": (
            "compute_dis_eq_decompositions(" in proof_control
            and "diseq_decomposition,\n            diseq_decomp_maxarity," in proof_control
        ),
        "proof_processing_consumes_split_controls": (
            "controlled_split_class_matches(&clause, control.heuristic_parms().split_clauses)"
            in proof_control
        ),
        "incompleteness_reaches_proof_state": (
            "if relevancy_pruned != 0 || config.search.completeness.incomplete {" in prover
            and "state.set_state_is_complete(false);" in prover
        ),
        "cli_literal_selection_regression": (
            "fn process_options_records_literal_selection_state_like_c()" in prover
        ),
        "cli_heuristic_completeness_regression": (
            "fn process_options_records_heuristic_limits_and_completeness_like_c()" in prover
        ),
        "cli_inference_splitting_regression": (
            "fn process_options_records_inference_and_splitting_state_like_c()" in prover
        ),
        "proof_control_bridge_regression": (
            "fn proof_control_from_config_installs_configured_parameters()" in prover
        ),
        "assumed_incompleteness_status_regression": (
            "fn run_proof_search_reports_assumed_incompleteness_as_gave_up()" in prover
        ),
    }
    report = {
        "schema_version": 1,
        "contracts": contracts,
        "passed": sum(contracts.values()),
        "total": len(contracts),
        "all_passed": all(contracts.values()),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("audit does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_passed"]:
        failed = [name for name, passed in contracts.items() if not passed]
        print(f"literal-selection option contracts failed: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
