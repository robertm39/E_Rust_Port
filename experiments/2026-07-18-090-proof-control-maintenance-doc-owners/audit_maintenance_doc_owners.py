#!/usr/bin/env python3
"""Audit proof-session ownership at saturation maintenance gates."""

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
    rust = (repo / "src/heuristics/proofcontrol.rs").read_text(encoding="utf-8")
    c_source = (repo / "eprover/CONTROL/cco_proofproc.c").read_text(encoding="utf-8")

    rust_contracts = {
        "documenting_set_reweight_wrapper": (
            "pub fn proof_state_forward_contract_set_reweight_with_docs(" in rust
        ),
        "cleanup_impl_accepts_session": (
            "mut doc_context: Option<(&mut W, &mut ProofDocSession)>" in rust
        ),
        "cleanup_dispatches_documenting_contraction": (
            "Some((output, session)) => proof_state_forward_contract_set_with_docs(" in rust
        ),
        "cleanup_exposes_documenting_wrapper": (
            "pub fn proof_state_cleanup_unprocessed_clauses_with_docs(" in rust
        ),
        "saturate_dispatches_documenting_cleanup": (
            "proof_state_cleanup_unprocessed_clauses_with_docs(" in rust
            and "write_cleanup_unprocessed_fmt_output" in rust
        ),
        "sat_gate_receives_session": (
            "fn proof_state_saturate_sat_check_gate<W: fmt::Write>(" in rust
            and "doc_context: &mut Option<(&mut W, &mut ProofDocSession, i64)>" in rust
        ),
        "sat_gate_forwards_session": (
            "proof_state_sat_check(state, control, doc_context)?" in rust
        ),
        "sat_normalization_dispatches_documenting_reweight": (
            "proof_state_forward_contract_set_reweight_with_docs(" in rust
            and "let contraction_result = match doc_context.as_mut()" in rust
        ),
        "plain_cleanup_fallback_retained": (
            "proof_state_cleanup_unprocessed_clauses(state, control)?" in rust
        ),
        "plain_sat_reweight_fallback_retained": (
            "None => proof_state_forward_contract_set_reweight(" in rust
        ),
    }
    c_contracts = {
        "c_cleanup_uses_set_contraction": (
            "ForwardContractSet(state, control," in c_source
        ),
        "c_cleanup_reports_after_contraction": (
            c_source.index("ForwardContractSet(state, control,")
            < c_source.index('COMCHAR" Special forward-contraction deletes')
        ),
        "c_satcheck_uses_set_reweight": (
            "empty = ForwardContractSetReweight(state, control, state->unprocessed," in c_source
        ),
    }
    tests = {
        "set_reweight_docs_regression": (
            "fn proof_state_forward_contract_set_reweight_with_docs_keeps_modified_survivor_evaluated()"
            in rust
        ),
        "cleanup_docs_regression": (
            "fn proof_state_cleanup_unprocessed_with_docs_records_preprocessing_refutation()"
            in rust
        ),
        "saturate_cleanup_docs_regression": (
            "fn proof_state_saturate_with_docs_records_cleanup_contraction_before_status()"
            in rust
        ),
        "saturate_satcheck_docs_regression": (
            "fn proof_state_saturate_with_docs_records_sat_check_normalization_refutation()"
            in rust
        ),
    }
    contracts = {**rust_contracts, **c_contracts, **tests}
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
        print(f"maintenance proof-owner contracts failed: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
