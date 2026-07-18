#!/usr/bin/env python3
"""Audit higher-order ForwardModifyClause staging and ordering admission."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


def function_slice(text: str, start: str, end: str, source: Path) -> str:
    begin = text.find(start)
    if begin < 0:
        raise SystemExit(f"{source}: missing function start {start!r}")
    finish = text.find(end, begin)
    if finish < 0:
        raise SystemExit(f"{source}: missing function end {end!r}")
    return text[begin : finish + len(end)]


def ordered(text: str, *needles: str) -> bool:
    position = -1
    for needle in needles:
        position = text.find(needle, position + 1)
        if position < 0:
            return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    root = args.repo.resolve()

    c_path = root / "eprover/CONTROL/cco_forward_contraction.c"
    c_text = c_path.read_text(encoding="utf-8")
    c_forward = function_slice(
        c_text,
        "bool ForwardModifyClause(",
        "return false;\n}",
        c_path,
    )

    rust_path = root / "src/heuristics/proofcontrol.rs"
    rust_text = rust_path.read_text(encoding="utf-8")
    rust_forward = function_slice(
        rust_text,
        "fn proof_state_forward_modify_clause_impl<",
        "Ok(trivial)\n}",
        rust_path,
    )
    rust_gate = function_slice(
        rust_text,
        "fn forward_modify_check_higher_order_ordering(",
        "    ))\n}",
        rust_path,
    )

    orderings = {"Kbo", "Kbo6", "Lpo", "LpoCopy", "Lpo4", "Lpo4Copy"}
    admitted_orderings = set(re.findall(r"TermOrdering::([A-Za-z0-9]+)", rust_gate))
    contracts = {
        "c_has_four_ho_normalization_hooks": c_forward.count("NormalizeEquations(clause);") == 4,
        "c_hook_and_prune_order": ordered(
            c_forward,
            "NormalizeEquations(clause);",
            "ClauseComputeLINormalform(",
            "NormalizeEquations(clause);",
            "ClauseRemoveSuperfluousLiterals(clause);",
            "ClauseLocalRW(control->ocb, clause)",
            "NormalizeEquations(clause);",
            "ClauseOrientLiterals(control->ocb, clause);",
            "ClauseIsTrivial(clause)",
            "ClausePruneArgs(clause);",
            "NormalizeEquations(clause);",
            "ClausePositiveSimplifyReflect(",
            "ClauseNegativeSimplifyReflect(",
        ),
        "rust_has_four_ho_normalization_hooks": rust_forward.count(
            "forward_modify_normalize_if_higher_order(higher_order, clause, terms);"
        )
        == 4,
        "rust_hook_and_prune_order": ordered(
            rust_forward,
            "forward_modify_normalize_if_higher_order(higher_order, clause, terms);",
            "clause_compute_li_normalform_plain",
            "forward_modify_normalize_if_higher_order(higher_order, clause, terms);",
            "clause_remove_superfluous_literals(clause, terms);",
            "if local_rw && clause_local_rw(ocb, terms, clause)?",
            "forward_modify_normalize_if_higher_order(higher_order, clause, terms);",
            "clause.orient_literals_with_bank(ocb, terms)?;",
            "if clause.is_trivial(terms)",
            "let _ = clause_prune_args(clause, terms)?;",
            "forward_modify_normalize_if_higher_order(higher_order, clause, terms);",
            "forward_modify_positive_simplify_reflect(",
            "forward_modify_negative_simplify_reflect(",
        ),
        "rust_admits_all_six_release_orderings": orderings <= admitted_orderings,
        "rust_uses_owner_bank_for_orientation": rust_forward.count(
            "clause.orient_literals_with_bank(ocb, terms)?;"
        )
        == 2,
        "rust_normalizes_real_encoded_equality": (
            "fn proof_state_forward_modify_clause_higher_order_normalizes_encoded_equality()"
            in rust_text
        ),
        "rust_prunes_real_constant_argument": (
            "fn proof_state_forward_modify_clause_higher_order_prunes_constant_argument()"
            in rust_text
            and "derivation_contains_operation(&clause, DC_PRUNE_ARG)" in rust_text
        ),
        "rust_covers_lfho_and_lambda_surfaces": all(
            test in rust_text
            for test in (
                "fn proof_state_forward_modify_clause_higher_order_lpo_surface_matches_release()",
                "fn proof_state_forward_modify_clause_higher_order_lpo4_ignores_kbo_ho_order_kind()",
                "fn proof_state_forward_modify_clause_higher_order_lfho_applied_var_ordering_runs()",
                "fn proof_state_forward_modify_clause_higher_order_lambda_order_surface_runs()",
            )
        ),
    }
    report = {
        "schema_version": 1,
        "contract_count": len(contracts),
        "pass_count": sum(contracts.values()),
        "contracts": contracts,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output is not None:
        args.output.write_text(encoded, encoding="utf-8", newline="\n")
    else:
        print(encoded, end="")

    if not all(contracts.values()):
        print("ForwardModifyClause audit failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("ForwardModifyClause audit reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
