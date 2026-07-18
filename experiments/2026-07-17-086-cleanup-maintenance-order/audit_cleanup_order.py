#!/usr/bin/env python3
"""Audit the C/Rust cleanup gate, storage, liveness, and saturation order."""

from __future__ import annotations

import argparse
import json
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

    c_path = root / "eprover/CONTROL/cco_proofproc.c"
    c_text = c_path.read_text(encoding="utf-8")
    c_cleanup = function_slice(
        c_text,
        "static Clause_p cleanup_unprocessed_clauses(",
        "return unsatisfiable;\n}",
        c_path,
    )

    rust_path = root / "src/heuristics/proofcontrol.rs"
    rust_text = rust_path.read_text(encoding="utf-8")
    rust_cleanup = function_slice(
        rust_text,
        "fn proof_state_cleanup_unprocessed_clauses_impl(",
        "Ok(outcome)\n}",
        rust_path,
    )
    rust_default = function_slice(
        rust_text,
        "pub fn proof_state_cleanup_unprocessed_clauses(\n",
        "    )\n}",
        rust_path,
    )
    rust_saturate = function_slice(
        rust_text,
        "fn proof_state_saturate_impl<",
        "fn write_cleanup_unprocessed_output(",
        rust_path,
    )

    contracts = {
        "c_gate_order": ordered(
            c_cleanup,
            "ClauseSetDeleteOrphans(state->unprocessed);",
            "ForwardContractSet(state, control,",
            "current_storage  = ProofStateStorage(state);",
            "HCBClauseSetDeleteBadClauses(control->hcb,",
            "TBGCCollect(state->terms);",
        ),
        "c_storage_after_forward_reweight": ordered(
            c_cleanup,
            "ClauseSetReweight(control->hcb,  state->unprocessed);",
            "current_storage  = ProofStateStorage(state);",
        ),
        "rust_gate_order": ordered(
            rust_cleanup,
            "let deleted = delete_orphans(state);",
            "proof_state_forward_contract_set(",
            "if current_storage(state) > control.heuristic_parms().delete_bad_limit",
            "let orphan_count = delete_orphans(state);",
            "state.collect_term_garbage();",
        ),
        "rust_storage_is_late_callback": rust_cleanup.count("current_storage(state)") == 1,
        "rust_orphan_hook_runs_at_both_gates": rust_cleanup.count("delete_orphans(state)") == 2,
        "rust_default_uses_live_storage_estimator": "proof_state_storage_estimate,"
        in rust_default,
        "rust_default_refreshes_liveness_in_gate": ordered(
            rust_default,
            "let parent_liveness = ParentLivenessSnapshot::from_state(state);",
            "clause_set_delete_orphans_with(state.unprocessed_mut(), |parent|",
        ),
        "rust_saturation_cleanup_after_processing": ordered(
            rust_saturate,
            "proof_state_process_clause_impl(",
            "let cleanup = proof_state_cleanup_unprocessed_clauses(state, control)?;",
            "write_cleanup_unprocessed",
        ),
        "rust_post_orphan_storage_regression": (
            "fn cleanup_default_measures_storage_after_orphan_deletion()" in rust_text
            and "assert!(!outcome.delete_bad_triggered);" in rust_text
        ),
        "rust_c_shaped_forward_output": (
            '"{DEFAULT_COMCHAR_RAW} Special forward-contraction deletes {} clauses(remaining: {}) "'
            in rust_text
            and '"{DEFAULT_COMCHAR_RAW} Reweighting unprocessed clauses..."' in rust_text
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
        print("cleanup maintenance audit failed", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("cleanup maintenance audit reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
