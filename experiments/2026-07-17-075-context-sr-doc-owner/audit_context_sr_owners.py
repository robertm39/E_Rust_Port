#!/usr/bin/env python3
"""Audit C and Rust contextual simplify-reflect production owners."""

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
    c_forward = (root / "eprover/CONTROL/cco_forward_contraction.c").read_text(
        encoding="utf-8"
    )
    c_simplification = (root / "eprover/CONTROL/cco_simplification.c").read_text(
        encoding="utf-8"
    )
    rust_control = (root / "src/heuristics/proofcontrol.rs").read_text(encoding="utf-8")
    rust_executable = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")

    checks = {
        "c_forward_owner_calls_modifier": (
            "ClauseContextualSimplifyReflect(state->processed_non_units" in c_forward
        ),
        "c_backward_owner_uses_discovery_then_move": all(
            marker in c_simplification
            for marker in (
                "ClauseSetFindContextSRClauses(from, simplifier, stack);",
                "ClauseMoveSimplified(gindices, handle, into, archive, lambda_demod);",
            )
        ),
        "rust_process_owner_selects_documented_forward_wrapper": all(
            marker in rust_control
            for marker in (
                "if let Some((output, session, _output_level)) = doc_context.as_mut()",
                "proof_state_forward_contract_clause_with_docs(",
                "proof_state_forward_contract_clause(state, control, clause, options)?",
            )
        ),
        "rust_forward_wrapper_threads_context_docs": (
            "clause_contextual_simplify_reflect_with_docs_and_bank(" in rust_control
        ),
        "rust_backward_owner_quotes_before_requeue": all(
            marker in rust_control
            for marker in (
                "proof_state_eliminate_context_sr_clauses(",
                "Some(\"simplifiable\")",
                "proof_state_move_simplified_clause_to_tmp(state, clause)?",
            )
        ),
        "rust_executable_owns_documented_saturation": all(
            marker in rust_executable
            for marker in (
                "if config.output_level >= 2",
                "proof_state_saturate_with_global_and_watchlist_indices_and_docs(",
            )
        ),
        "permanent_process_owner_regression": (
            "proof_state_process_clause_with_docs_emits_forward_context_sr_modification"
            in rust_control
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
        print("contextual simplify-reflect owner audit failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
