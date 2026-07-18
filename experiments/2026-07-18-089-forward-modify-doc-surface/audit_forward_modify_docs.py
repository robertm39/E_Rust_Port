#!/usr/bin/env python3
"""Audit documenting and deliberately silent ForwardModifyClause mutations."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def function_body(text: str, signature: str) -> str:
    start = text.index(signature)
    brace = text.index("{", start)
    depth = 0
    for index in range(brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[brace : index + 1]
    raise ValueError(f"unterminated function: {signature}")


def ordered(text: str, fragments: list[str]) -> bool:
    cursor = 0
    for fragment in fragments:
        cursor = text.find(fragment, cursor)
        if cursor < 0:
            return False
        cursor += len(fragment)
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    root = args.repo.resolve()
    c_forward = (root / "eprover/CONTROL/cco_forward_contraction.c").read_text(
        encoding="utf-8"
    )
    c_rewrite = (root / "eprover/CLAUSES/ccl_rewrite.c").read_text(encoding="utf-8")
    c_clause = (root / "eprover/CLAUSES/ccl_clausefunc.c").read_text(encoding="utf-8")
    c_condense = (root / "eprover/CLAUSES/ccl_condensation.c").read_text(
        encoding="utf-8"
    )
    c_subsumption = (root / "eprover/CLAUSES/ccl_subsumption.c").read_text(
        encoding="utf-8"
    )
    c_ho = (root / "eprover/CONTROL/cco_ho_inferences.c").read_text(encoding="utf-8")
    rust = (root / "src/heuristics/proofcontrol.rs").read_text(encoding="utf-8")

    normalize_body = function_body(c_ho, "bool NormalizeEquations(Clause_p cl)")
    prune_body = function_body(c_ho, "void ClausePruneArgs(Clause_p cl)")
    local_rw_body = function_body(c_rewrite, "bool ClauseLocalRW(OCB_p ocb, Clause_p clause)")

    contracts = {
        "c_forward_modify_calls_normalform": (
            "ClauseComputeLINormalform(control->ocb" in c_forward
        ),
        "c_normalform_emits_rewrite_docs": (
            "DocClauseRewriteDefault(pos, l_old)" in c_rewrite
            and "DocClauseRewriteDefault(pos, r_old)" in c_rewrite
        ),
        "c_forward_modify_emits_minimize_docs": ordered(
            c_forward,
            [
                "removed_lits = ClauseRemoveSuperfluousLiterals(clause);",
                "DocClauseModificationDefault(clause, inf_minimize, NULL);",
            ],
        ),
        "c_ac_resolution_emits_docs": ordered(
            c_clause,
            [
                "int ClauseRemoveACResolved(Clause_p clause)",
                "inf_ac_resolution",
                "ClausePushACResDerivation(clause, sig);",
            ],
        ),
        "c_condensation_emits_docs": ordered(
            c_condense,
            [
                "bool Condense(Clause_p clause)",
                "DocClauseModificationDefault(clause, inf_condense, NULL);",
                "ClausePushDerivation(clause, DCCondense, NULL, NULL);",
            ],
        ),
        "c_positive_sr_emits_docs": ordered(
            c_subsumption,
            [
                "bool ClausePositiveSimplifyReflect(",
                "DocClauseModificationDefault(clause, inf_simplify_reflect,",
            ],
        ),
        "c_negative_sr_emits_docs": ordered(
            c_subsumption,
            [
                "bool ClauseNegativeSimplifyReflect(",
                "DocClauseModificationDefault(clause, inf_simplify_reflect,",
            ],
        ),
        "c_ho_normalization_is_doc_silent": "DocClause" not in normalize_body,
        "c_ho_pruning_is_doc_silent": (
            "DCPruneArg" in prune_body and "DocClause" not in prune_body
        ),
        "c_local_rewrite_is_doc_silent": (
            "DCLocalRewrite" in local_rw_body and "DocClause" not in local_rw_body
        ),
        "rust_normalform_dispatches_docs": (
            "Some((output, session)) => clause_compute_li_normalform_plain_with_docs("
            in rust
        ),
        "rust_minimize_dispatches_docs": ordered(
            rust,
            [
                "let removed_lits = clause_remove_superfluous_literals(clause, terms);",
                "ClauseModificationInference::Minimize",
            ],
        ),
        "rust_ac_dispatches_docs": (
            "clause_remove_ac_resolved_with_docs_and_axioms(" in rust
        ),
        "rust_condense_dispatches_docs": (
            "Some((output, session)) => condense_with_docs(output, session, clause, terms)"
            in rust
        ),
        "rust_both_sr_directions_dispatch_docs": (
            "clause_positive_simplify_reflect_with_strong_and_docs_and_bank(" in rust
            and "clause_negative_simplify_reflect_with_docs_and_bank(" in rust
        ),
        "rust_all_direct_owners_use_optional_docs": (
            rust.count("proof_state_forward_modify_clause_maybe_docs(") == 3
        ),
        "rust_documenting_event_regressions_present": all(
            name in rust
            for name in (
                "proof_state_forward_modify_clause_with_docs_emits_rewrite_steps_at_level_four",
                "proof_state_forward_modify_clause_with_docs_records_ac_resolution",
                "proof_state_forward_modify_clause_with_docs_emits_minimize_step",
                "proof_state_forward_modify_clause_with_docs_emits_condense_step",
                "proof_state_forward_modify_clause_with_docs_emits_simplify_reflect_step",
            )
        ),
        "rust_silent_mutation_regressions_present": (
            "proof_state_forward_modify_clause_honors_local_rewrite_option" in rust
            and "proof_state_forward_modify_clause_higher_order_prunes_constant_argument" in rust
            and rust.count("assert!(rendered.is_empty());") >= 3
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
            print("audit report does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_passed"]:
        print("one or more ForwardModifyClause documentation contracts failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
