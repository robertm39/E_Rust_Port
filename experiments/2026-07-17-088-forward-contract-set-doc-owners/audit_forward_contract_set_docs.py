#!/usr/bin/env python3
"""Audit proof-documentation ownership for set-level forward contraction."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


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
    c_main = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    rust_control = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )
    rust_main = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")

    contracts = {
        "c_forward_contract_set_reuses_documenting_keep": ordered(
            c_forward,
            ["Clause_p ForwardContractSet(", "forward_contract_keep(state, control, handle"],
        ),
        "c_filter_routes_through_forward_contract_set": ordered(
            c_forward,
            ["Clause_p ProofStateFilterUnprocessed(", "ForwardContractSet(state, control"],
        ),
        "c_executable_owns_post_saturation_filter": ordered(
            c_main,
            ["if(filter_sat)", "ProofStateFilterUnprocessed(proofstate"],
        ),
        "rust_set_exposes_documenting_wrapper": ordered(
            rust_control,
            [
                "pub fn proof_state_forward_contract_set_with_docs(",
                "Some((output, session))",
            ],
        ),
        "rust_set_dispatches_to_documenting_keep": ordered(
            rust_control,
            [
                "fn proof_state_forward_contract_set_impl",
                "Some((output, session)) => proof_state_forward_contract_keep_with_docs(",
            ],
        ),
        "rust_filter_exposes_documenting_wrapper": ordered(
            rust_control,
            [
                "pub fn proof_state_filter_unprocessed_with_docs(",
                "proof_state_filter_unprocessed_impl(state, control, desc, Some((output, session)))",
            ],
        ),
        "rust_filter_threads_docs_into_set_contraction": ordered(
            rust_control,
            [
                "fn proof_state_filter_contract_step",
                "proof_state_forward_contract_set_with_docs(",
            ],
        ),
        "rust_executable_starts_successor_session": ordered(
            rust_main,
            [
                "fn filter_saturated_unprocessed",
                "if config.output_level >= 2",
                "clause_proof_doc_session(config, start_doc_ident)",
                "proof_state_filter_unprocessed_with_docs(",
            ],
        ),
        "rust_executable_returns_advanced_identifier": ordered(
            rust_main,
            [
                "let next_doc_ident = session.id_source.current_ident().saturating_add(1);",
                "Ok((empty, next_doc_ident))",
            ],
        ),
        "rust_owner_updates_identifier_before_side_outputs": ordered(
            rust_main,
            [
                "filter_saturated_unprocessed(output, config, next_doc_ident",
                "next_doc_ident = filtered_next_doc_ident;",
                "write_proof_search_side_outputs(output, config, &mut state, &outcome, next_doc_ident)",
            ],
        ),
        "set_level_modification_regression_present": (
            "fn proof_state_forward_contract_set_with_docs_emits_modification_step()"
            in rust_control
        ),
        "executable_session_continuity_regression_present": (
            "fn run_config_filter_saturated_continues_proof_doc_session()" in rust_main
            and 'minimize.starts_with("cnf(c_0_5,")' in rust_main
            and 'survivor.starts_with("cnf(c_0_7,")' in rust_main
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
        print("one or more forward-contract documentation contracts failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
