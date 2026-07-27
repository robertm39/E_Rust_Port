#!/usr/bin/env python3
"""Audit formula-to-clause preprocessing ownership and permanent coverage."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_driver = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    c_preprocessing = (root / "eprover/CONTROL/cco_preprocessing.c").read_text(
        encoding="utf-8"
    )
    c_bce = (root / "eprover/CLAUSES/ccl_bce.h").read_text(encoding="utf-8")
    c_pred_elim = (root / "eprover/CLAUSES/ccl_pred_elim.h").read_text(
        encoding="utf-8"
    )
    c_goal_defs = (
        root / "eprover/CLAUSES/ccl_gd_transformation.h"
    ).read_text(encoding="utf-8")
    c_unfold_defs = (root / "eprover/CLAUSES/ccl_unfold_defs.h").read_text(
        encoding="utf-8"
    )
    rust_driver = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_proof_search = rust_driver.split("fn run_proof_search", 1)[1].split(
        "fn hard_time_limit_expired_in_saturation", 1
    )[0]

    eqdef_reference = load_json(
        root
        / "experiments/2026-07-17-076-classifier-eqdef-boundary/reference.json"
    )
    goal_reference = load_json(
        root / "experiments/2026-07-17-077-goal-definition-order/reference.json"
    )
    predicate_reference = load_json(
        root
        / "experiments/2026-07-17-078-predicate-elimination-handles/reference.json"
    )
    bce_reference = load_json(
        root / "experiments/2026-07-17-079-bce-handles/reference.json"
    )
    option_reference = load_json(
        root
        / "experiments/2026-07-18-094-higher-order-option-effects/comparison-reference.json"
    )

    c_headers = "\n".join((c_bce, c_pred_elim, c_goal_defs, c_unfold_defs))
    c_order = (
        c_preprocessing.index("ClauseSetPreprocess(")
        < c_preprocessing.index("ClauseSetUnfoldEqDefNormalize(")
        < c_preprocessing.index("EliminateBlockedClauses(")
        < c_preprocessing.index("PredicateElimination(")
        < c_preprocessing.index("ClauseSetGDTransform(")
    )
    rust_order = (
        rust_proof_search.index("clausify_formula_axioms_with_docs(")
        < rust_proof_search.index("apply_clause_set_preprocessing_with_docs(")
        < rust_proof_search.index("apply_blocked_clause_elimination(")
        < rust_proof_search.index("apply_predicate_elimination(")
        < rust_proof_search.index("apply_goal_definition_transformation(")
        < rust_proof_search.index("write_initial_clause_docs(")
        < rust_proof_search.index("run_presaturation_interreduction(")
    )

    checks = {
        "c_formula_cnf_precedes_clause_preprocessing": (
            c_driver.index("FormulaSetCNF2(")
            < c_driver.index("ProofStateClausalPreproc(")
        ),
        "c_clause_preprocessing_order": c_order,
        "c_bce_first_order_gate": (
            "if(problemType == PROBLEM_FO && h_parms->bce)" in c_preprocessing
        ),
        "c_predicate_elimination_first_order_gate": (
            "if(problemType == PROBLEM_FO && h_parms->pred_elim)"
            in c_preprocessing
        ),
        "c_exposes_no_parallel_formula_set_pass": not any(
            marker in c_headers
            for marker in (
                "FormulaSetBCE",
                "FormulaSetPredicateElimination",
                "FormulaSetGDTransform",
                "FormulaSetUnfoldEqDef",
            )
        ),
        "rust_formula_cnf_and_clause_preprocessing_order": rust_order,
        "rust_bce_first_order_gate": (
            "if !enabled || problem_type() != ProblemType::FirstOrder"
            in rust_driver
        ),
        "rust_predicate_elimination_first_order_gate": (
            "if !config.enabled || problem_type() != ProblemType::FirstOrder"
            in rust_driver
        ),
        "permanent_bce_formula_owner_regression": (
            "run_proof_search_applies_bce_to_fof_formula_origin_clauses"
            in rust_driver
        ),
        "permanent_predicate_formula_owner_regression": (
            "run_proof_search_applies_pred_elim_to_fof_formula_origin_clauses"
            in rust_driver
        ),
        "permanent_goal_formula_owner_regression": (
            "run_proof_search_applies_goal_defs_to_fof_formula_origin_conjecture"
            in rust_driver
        ),
        "permanent_eqdef_formula_owner_regression": (
            "run_proof_search_unfolds_formula_origin_eq_definition_before_saturation"
            in rust_driver
        ),
        "permanent_presaturation_formula_owner_regression": (
            "run_proof_search_presaturation_handles_fof_formula_origin_clause"
            in rust_driver
        ),
        "permanent_thf_first_order_pass_gate_regression": (
            "run_proof_search_skips_fo_only_preprocessing_after_thf_cnf"
            in rust_driver
        ),
        "eqdef_reference_exact": eqdef_reference.get("all_exact") is True,
        "goal_definition_reference_exact": goal_reference.get("all_exact") is True,
        "predicate_elimination_reference_exact": (
            predicate_reference.get("all_exact") is True
        ),
        "bce_reference_exact": bce_reference.get("all_exact") is True,
        "higher_order_gate_reference_exact": (
            option_reference.get("thf_fo_preprocessing_gate_exact") is True
            and option_reference.get("fo_preprocessing_effects_observed") is True
            and option_reference.get("mismatches") == []
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
        print(f"preprocessing owner audit failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {report['passed']}/{report['total']} preprocessing owner checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
