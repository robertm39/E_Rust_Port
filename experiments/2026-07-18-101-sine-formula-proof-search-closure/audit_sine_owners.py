#!/usr/bin/env python3
"""Audit destructive SInE formula ownership, phase order, and accounting."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_driver = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    c_sine = (root / "eprover/CONTROL/cco_sine.c").read_text(encoding="utf-8")
    rust_driver = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_proof_search = rust_driver.split("fn run_proof_search", 1)[1].split(
        "fn hard_time_limit_expired_in_saturation", 1
    )[0]
    rust_threshold = rust_driver.split("fn apply_threshold_sine_filter", 1)[1].split(
        "fn apply_gsine_clause_filter", 1
    )[0]
    rust_gsine = rust_driver.split("fn apply_gsine_clause_filter", 1)[1].split(
        "fn apply_lambda_defines_filter", 1
    )[0]
    rust_replace = rust_driver.split(
        "fn replace_axiom_owners_with_selected_ids", 1
    )[1].split("fn selected_ids_in_last_occurrence_order", 1)[0]
    reference = json.loads(args.reference.read_text(encoding="utf-8"))
    cases = {case["name"]: case for case in reference["cases"]}

    checks = {
        "c_documents_proof_state_sine_as_destructive": (
            "This is\n//   destructive. Returns number of axioms deleted." in c_sine
        ),
        "c_counts_clause_and_formula_owners_before_selection": (
            "ClauseSetCardinality(state->axioms)+\n      FormulaSetCardinality(state->f_axioms)"
            in c_sine
        ),
        "c_selects_from_both_live_owner_sets": (
            "StructFOFSpecAddProblem(spec, state->axioms, state->f_axioms" in c_sine
        ),
        "c_replaces_and_moves_both_owner_sets": all(
            marker in c_sine
            for marker in (
                "state->axioms   = ClauseSetAlloc();",
                "state->f_axioms = FormulaSetAlloc();",
                "PStackClausesMove(clauses, state->axioms);",
                "PStackFormulasMove(formulas, state->f_axioms);",
            )
        ),
        "c_returns_combined_deleted_owner_count": "return axno_orig-axno;" in c_sine,
        "c_sine_then_relevance_then_formula_cnf": (
            c_driver.index("ProofStateSinE(")
            < c_driver.index("ProofStateRelevancyProcess(")
            < c_driver.index("FormulaSetCNF2(")
        ),
        "c_combines_sine_and_relevance_statistics": (
            "relevancy_pruned += ProofStateSinE" in c_driver
            and "relevancy_pruned += ProofStateRelevancyProcess" in c_driver
        ),
        "rust_sine_then_relevance_then_formula_cnf": (
            rust_proof_search.index("apply_proof_state_sine(")
            < rust_proof_search.index("apply_relevance_pruning(")
            < rust_proof_search.index("clausify_formula_axioms_with_docs(")
        ),
        "rust_combines_sine_and_relevance_statistics": (
            "sine_pruned + apply_relevance_pruning" in rust_proof_search
        ),
        "rust_threshold_counts_and_clears_both_owner_sets": all(
            marker in rust_threshold
            for marker in (
                "state.axiom_count()",
                "state.axioms_mut().clear()",
                "state.f_axioms_mut().clear()",
                "original_axioms - selected_axioms",
            )
        ),
        "rust_gsine_selects_both_owner_sets": all(
            marker in rust_gsine
            for marker in (
                "clause_sets.push(state.axioms())",
                "formula_sets.push(state.f_axioms())",
                "selected_clause_ids",
                "selected_formula_ids",
            )
        ),
        "rust_replaces_both_owner_sets_and_returns_delta": all(
            marker in rust_replace
            for marker in (
                "take_selected_clause_ids",
                "take_selected_formula_entry_ids",
                "*state.axioms_mut() = selected_clauses;",
                "*state.f_axioms_mut() = selected_formulas;",
                "original_axioms - selected_count",
            )
        ),
        "permanent_threshold_proof_search_regression": (
            "run_proof_search_statistics_count_formula_threshold_sine_pruning"
            in rust_driver
        ),
        "permanent_gsine_proof_search_regression": (
            "run_proof_search_statistics_count_formula_gsine_pruning" in rust_driver
        ),
        "fresh_threshold_reference_exact": (
            cases["threshold"]["all_exact"]
            and cases["threshold"]["c"]["statistics"]["Parsed axioms"] == 2
            and cases["threshold"]["c"]["statistics"]
            ["Removed by relevancy pruning/SinE"]
            == 2
            and cases["threshold"]["c"]["statistics"]["Initial clauses"] == 0
            and cases["threshold"]["c"]["surviving_formula_owners"] == []
        ),
        "fresh_gsine_reference_exact": (
            cases["gsine"]["all_exact"]
            and cases["gsine"]["c"]["statistics"]["Parsed axioms"] == 4
            and cases["gsine"]["c"]["statistics"]
            ["Removed by relevancy pruning/SinE"]
            == 1
            and cases["gsine"]["c"]["statistics"]["Initial clauses"] == 3
            and cases["gsine"]["c"]["surviving_formula_owners"]
            == ["goal", "link", "far"]
            and cases["gsine"]["c"]["formula_docs_before_initialization"]
        ),
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
        print(f"SInE owner audit failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {report['passed']}/{report['total']} SInE owner checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
