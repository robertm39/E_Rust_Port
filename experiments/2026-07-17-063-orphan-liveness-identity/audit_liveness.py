#!/usr/bin/env python3
"""Audit stable parent identity across HCB orphan-selection owners."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = (
        args.root.resolve()
        if args.root is not None
        else Path(__file__).resolve().parents[2]
    )
    c_clausefunc = (root / "eprover/CLAUSES/ccl_clausefunc.c").read_text(
        encoding="utf-8"
    )
    derivation = (root / "src/clauses/derivation.rs").read_text(encoding="utf-8")
    hcb = (root / "src/heuristics/hcb.rs").read_text(encoding="utf-8")
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    checks = {
        "c_orphan_check_uses_raw_parent_pointer": (
            "Clause_p parent;" in c_clausefunc
            and "ClauseQueryProp(parent,CPIsDead)" in c_clausefunc
        ),
        "stable_ref_has_generation": "generation: u64" in derivation,
        "stable_ref_equality_uses_generation": (
            "match (self.generation, other.generation)" in derivation
        ),
        "stable_ref_hash_uses_generation": "self.generation.hash(state);" in derivation,
        "clause_conversion_captures_generation": (
            "clause.derivation_generation()," in derivation
        ),
        "cleanup_snapshot_uses_stable_ref_set": (
            "live: ClauseRefSet" in proofcontrol
            and "self.live.insert(ClauseDerivationRef::from(clause));" in proofcontrol
        ),
        "selection_compares_exact_stable_ref": (
            "ClauseDerivationRef::from(clause) == parent" in proofcontrol
        ),
        "selection_uses_processed_id_index": (
            "set.find_indexed_by_id(parent.ident())" in proofcontrol
        ),
        "selection_excludes_waiting_child_owners": all(
            owner not in proofcontrol.split("fn selection_parent_is_dead", maxsplit=1)[1]
            .split("/// Applies", maxsplit=1)[0]
            for owner in ("state.unprocessed()", "state.tmp_store()", "state.eval_store()")
        ),
        "selection_detaches_unprocessed_owner": (
            "std::mem::take(state.unprocessed_mut())" in proofcontrol
        ),
        "hcb_standard_accepts_orphan_predicate": (
            "mut is_orphaned: impl FnMut(&Clause) -> bool" in hcb
        ),
        "same_id_snapshot_regression_present": (
            "compact_parent_liveness_distinguishes_same_id_generations" in proofcontrol
        ),
        "same_id_selection_regression_present": (
            "assert_ne!(dead_ref, live_alias_ref);" in proofcontrol
        ),
        "end_to_end_orphan_selection_regression_present": (
            "proof_state_process_clause_skips_orphaned_best_unprocessed_clause"
            in proofcontrol
        ),
    }
    expected = {
        "c_orphan_check_uses_raw_parent_pointer": True,
        "stable_ref_has_generation": True,
        "stable_ref_equality_uses_generation": True,
        "stable_ref_hash_uses_generation": True,
        "clause_conversion_captures_generation": True,
        "cleanup_snapshot_uses_stable_ref_set": True,
        "selection_compares_exact_stable_ref": True,
        "selection_uses_processed_id_index": True,
        "selection_excludes_waiting_child_owners": True,
        "selection_detaches_unprocessed_owner": True,
        "hcb_standard_accepts_orphan_predicate": True,
        "same_id_snapshot_regression_present": True,
        "same_id_selection_regression_present": True,
        "end_to_end_orphan_selection_regression_present": True,
    }
    result = {"checks": checks, "expected": expected, "passed": checks == expected}
    rendered = json.dumps(result, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if not result["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
