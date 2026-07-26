#!/usr/bin/env python3
"""Audit the explicit Rust port of C's problem-specific WHNF dereference policy."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = (repo / relative).read_text(encoding="utf-8")
    return all(needle in source for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    checks = {
        "c_subst_selects_whnf_for_ho": contains(
            repo,
            "eprover/TERMS/cte_subst.c",
            "problemType == PROBLEM_HO ? WHNF_deref : TermDerefAlways",
            "PLocalStackPushTermArgsReversed(stack, term)",
        ),
        "c_tb_insert_opt_selects_whnf_for_ho_always": contains(
            repo,
            "eprover/TERMS/cte_termbanks.c",
            "problemType == PROBLEM_HO && deref == DEREF_ALWAYS",
            "WHNF_deref(term) : TermDeref(term, &deref)",
            "TBInsertOpt(bank, term->args[i], CONVERT_DEREF(i, limit, deref))",
        ),
        "rust_subst_accepts_explicit_bank_and_problem_type": contains(
            repo,
            "src/terms/subst.rs",
            "pub fn norm_term_with_bank(",
            "bank: &mut TermBank",
            "problem_type: ProblemType",
            "whnf_deref(bank, &candidate)?",
        ),
        "rust_subst_retains_fo_fast_path": contains(
            repo,
            "src/terms/subst.rs",
            "if problem_type != ProblemType::HigherOrder",
            "return Ok(self.norm_term(term, vars))",
        ),
        "rust_subst_has_ho_beta_regression": contains(
            repo,
            "src/terms/subst.rs",
            "fn norm_term_with_bank_weak_head_normalizes_higher_order_roots()",
            "assert!(subst.is_empty())",
            "assert!(discarded.binding().is_none())",
        ),
        "rust_tb_insert_opt_uses_c_policy": contains(
            repo,
            "src/terms/termbanks.rs",
            "problem_type() == ProblemType::HigherOrder && deref == DerefType::Always",
            "whnf_deref(self, term)?",
            "self.insert_opt(arg, Self::convert_lfho_deref(index, limit, deref))?",
        ),
        "rust_tb_insert_opt_retains_unshared_ground_safety": contains(
            repo,
            "src/terms/termbanks.rs",
            "if term.is_shared()",
            "return self.insert(&term, DerefType::Never)",
        ),
        "rust_tb_insert_opt_has_ho_beta_regression": contains(
            repo,
            "src/terms/termbanks.rs",
            "fn optimized_insertion_weak_head_normalizes_higher_order_deref_always()",
            "set_problem_type_for_test(ProblemType::HigherOrder)",
            "assert_eq!(inserted, a)",
        ),
        "eqn_and_eqnlist_expose_bank_aware_normalization": contains(
            repo,
            "src/clauses/eqn.rs",
            "pub fn subst_norm_with_bank(",
        )
        and contains(
            repo,
            "src/clauses/eqnlist.rs",
            "pub fn subst_norm_except_with_bank(",
            "pub fn subst_norm_with_bank(",
        ),
        "clause_and_formula_collection_use_bank_aware_normalization": contains(
            repo,
            "src/clauses/clause.rs",
            "subst_norm_with_bank(&mut subst, fresh_vars, bank, problem_type())",
        )
        and contains(
            repo,
            "src/clauses/clausefunc.rs",
            "literals.subst_norm_with_bank(&mut subst, fresh_vars, bank, problem_type())",
        ),
        "resolution_and_factoring_use_bank_aware_normalization": contains(
            repo,
            "src/clauses/eqnresolution.rs",
            "subst_norm_except_with_bank(",
            "problem_type()",
        )
        and contains(
            repo,
            "src/clauses/factor.rs",
            "subst_norm_except_with_bank(",
            "problem_type()",
        ),
        "paramodulation_uses_bank_aware_normalization": contains(
            repo,
            "src/clauses/paramodulation.rs",
            "norm_term_with_bank(",
            "subst_norm_except_with_bank(",
            "problem_type()",
        ),
        "temporary_bindings_are_backtracked_on_errors": contains(
            repo,
            "src/terms/subst.rs",
            "self.backtrack_to_pos(previous)",
        )
        and contains(
            repo,
            "src/clauses/eqnlist.rs",
            "subst.backtrack_to_pos(result)",
        )
        and contains(
            repo,
            "src/clauses/eqnresolution.rs",
            "let result = (||",
            "subst.backtrack_to_pos(backtrack)",
        )
        and contains(
            repo,
            "src/clauses/factor.rs",
            "let result = (||",
            "subst.backtrack_to_pos(backtrack)",
        )
        and contains(
            repo,
            "src/clauses/paramodulation.rs",
            "subst.backtrack_to_pos(oldstate)",
            "return Err(error)",
        ),
    }

    source_files = [
        "src/terms/subst.rs",
        "src/terms/termbanks.rs",
        "src/clauses/eqn.rs",
        "src/clauses/eqnlist.rs",
        "src/clauses/clause.rs",
        "src/clauses/clausefunc.rs",
        "src/clauses/eqnresolution.rs",
        "src/clauses/factor.rs",
        "src/clauses/paramodulation.rs",
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "checks": checks,
        "passed": sum(checks.values()),
        "schema_version": 1,
        "source_digest": source_digest,
        "source_files": source_files,
        "total": len(checks),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    if not all(checks.values()):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("WHNF dereference policy reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
