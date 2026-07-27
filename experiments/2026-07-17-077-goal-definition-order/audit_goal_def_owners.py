#!/usr/bin/env python3
"""Audit C and Rust goal-definition transformation owners."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_gd = (root / "eprover/CLAUSES/ccl_gd_transformation.c").read_text(
        encoding="utf-8"
    )
    c_preprocessing = (root / "eprover/CONTROL/cco_preprocessing.c").read_text(
        encoding="utf-8"
    )
    c_terms = (root / "eprover/TERMS/cte_termfunc.c").read_text(encoding="utf-8")
    rust_gd = (root / "src/clauses/gd_transformation.rs").read_text(encoding="utf-8")
    rust_prover = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_terms = (root / "src/terms/termfunc.rs").read_text(encoding="utf-8")
    rust_types = (root / "src/terms/termtypes.rs").read_text(encoding="utf-8")
    rust_proof_search = rust_prover.split("fn run_proof_search", 1)[1].split(
        "match apply_auto_mode_search_selection", 1
    )[0]

    checks = {
        "c_goal_terms_use_pointer_tree": all(
            marker in c_gd
            for marker in (
                "PTree_p goal_terms = NULL",
                "PTreeTraverseInit(goal_terms)",
                "ClauseCollectGroundTerms(handle, &goal_terms",
            )
        ),
        "c_recursive_subterms_are_defined_child_first": (
            "res += gd_term_rek_define(terms, term->args[i], defs, clauses);"
            in c_gd
            and "res += gd_term_define(terms, term, defs, clauses);" in c_gd
        ),
        "c_proof_owner_runs_after_bce_and_predicate_elimination": (
            c_preprocessing.index("EliminateBlockedClauses(")
            < c_preprocessing.index("PredicateElimination(")
            < c_preprocessing.index("ClauseSetGDTransform(")
        ),
        "rust_goal_terms_use_live_pointer_identity": all(
            marker in rust_gd or marker in rust_terms or marker in rust_types
            for marker in (
                "let mut goal_terms = BTreeMap::new();",
                "insert(term_identity_id(&current), current.clone())",
                "Rc::as_ptr(&term.0).cast::<()>() as usize",
            )
        ),
        "rust_recursive_subterms_are_defined_child_first": (
            "result += gd_term_rek_define(bank, &arg, defs, clauses)?;" in rust_gd
            and "result += gd_term_define(bank, term, defs, clauses)?;" in rust_gd
        ),
        "rust_formula_cnf_precedes_goal_definition_owner": (
            rust_proof_search.index("clausify_formula_axioms_with_docs(")
            < rust_proof_search.index("apply_goal_definition_transformation(")
        ),
        "rust_goal_owner_runs_after_bce_and_predicate_elimination": (
            rust_proof_search.index("apply_blocked_clause_elimination(")
            < rust_proof_search.index("apply_predicate_elimination(")
            < rust_proof_search.index("apply_goal_definition_transformation(")
        ),
        "rust_goal_owner_precedes_initial_documentation": (
            rust_proof_search.index("apply_goal_definition_transformation(")
            < rust_proof_search.index("write_initial_clause_docs(")
        ),
        "permanent_low_level_sign_regression": (
            "gd_transform_respects_literal_sign_selection" in rust_gd
        ),
        "permanent_low_level_subterm_regression": (
            "gd_transform_can_define_subterms_before_parent_terms" in rust_gd
        ),
        "permanent_live_pointer_order_regression": (
            "gd_transform_assigns_definitions_in_live_term_identity_order" in rust_gd
        ),
        "permanent_formula_executable_regression": (
            "run_proof_search_applies_goal_defs_to_fof_formula_origin_conjecture"
            in rust_prover
        ),
        "permanent_prune_boundary_regression": (
            "run_prune_only_exits_before_goal_defs_on_formula_conjectures" in rust_prover
        ),
        "c_ground_collector_filters_predicates_and_constants": all(
            marker in c_terms
            for marker in (
                "!TermIsConst(term)",
                "!TermCellQueryProp(term, TPPredPos)",
                "PTreeStore(result, term)",
            )
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
        print("goal-definition owner audit failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
