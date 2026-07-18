#!/usr/bin/env python3
"""Audit C and Rust equality-definition unfolding production owners."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def between(source: str, start: str, end: str) -> str:
    return source.split(start, 1)[1].split(end, 1)[0]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_unfold = (root / "eprover/CLAUSES/ccl_unfold_defs.c").read_text(encoding="utf-8")
    c_classifier = (root / "eprover/PROVER/classify_problem.c").read_text(
        encoding="utf-8"
    )
    c_preprocessing = (root / "eprover/CONTROL/cco_preprocessing.c").read_text(
        encoding="utf-8"
    )
    rust_unfold = (root / "src/clauses/unfold_defs.rs").read_text(encoding="utf-8")
    rust_classifier = (root / "src/prover/classify_problem.rs").read_text(
        encoding="utf-8"
    )
    rust_prover = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")

    c_preprocess_body = between(
        c_unfold,
        "long ClauseSetPreprocess(",
        "// Function: ClauseSetUnfoldEqDefNormalize()",
    )
    c_classifier_standard = between(
        c_classifier,
        "FormulaSetPreprocConjectures(",
        "if(!specsig_classify)",
    )
    rust_classifier_owner = between(
        rust_classifier,
        "fn preprocess_real_input_clauses(",
        "fn clausify_real_input_formula_axioms(",
    )
    rust_proof_owner = between(
        rust_prover,
        "fn apply_clause_set_preprocessing_with_docs",
        "fn apply_choice_axiom_recognition",
    )

    checks = {
        "c_preprocess_body_has_no_eqdef_normalization": (
            "ClauseSetUnfoldEqDefNormalize(" not in c_preprocess_body
        ),
        "c_classifier_calls_only_clause_preprocess": (
            "ClauseSetPreprocess(" in c_classifier_standard
            and "ClauseSetUnfoldEqDefNormalize(" not in c_classifier_standard
        ),
        "c_proof_owner_normalizes_after_preprocess_gate": all(
            marker in c_preprocessing
            for marker in (
                "if(!h_parms->no_preproc)",
                "preproc_removed += ClauseSetUnfoldEqDefNormalize(",
            )
        ),
        "rust_classifier_calls_only_clause_preprocess": (
            "clause_set_preprocess(" in rust_classifier_owner
            and "clause_set_unfold_eq_def_normalize(" not in rust_classifier_owner
        ),
        "rust_classifier_clausifies_formula_owners_first": (
            rust_classifier_owner.index("clausify_real_input_formula_axioms(")
            < rust_classifier_owner.index("clause_set_preprocess(")
        ),
        "rust_proof_owner_retains_eqdef_normalization": all(
            marker in rust_proof_owner
            for marker in (
                "clause_set_unfold_eq_def_normalize_with_docs(",
                "clause_set_unfold_eq_def_normalize(",
            )
        ),
        "rust_public_unfold_surface_is_present": all(
            marker in rust_unfold
            for marker in (
                "pub fn clause_unfold_eq_def(",
                "pub fn clause_set_unfold_eq_def(",
                "pub fn clause_set_unfold_all_eq_defs(",
                "pub fn clause_set_unfold_eq_def_normalize(",
            )
        ),
        "permanent_cnf_classifier_regression": (
            "standard_real_input_preprocessing_keeps_eq_definitions_at_c_caller_boundary"
            in rust_classifier
        ),
        "permanent_formula_classifier_regression": (
            "standard_formula_input_preprocessing_keeps_cnf_eq_definitions"
            in rust_classifier
        ),
        "permanent_formula_proof_search_regression": (
            "run_proof_search_unfolds_formula_origin_eq_definition_before_saturation"
            in rust_prover
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
        print("equality-definition owner audit failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
