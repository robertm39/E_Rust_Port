#!/usr/bin/env python3
"""Audit formula relevance-pruning ownership and accounting."""

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
    c_relevance = (root / "eprover/CLAUSES/ccl_relevance.c").read_text(
        encoding="utf-8"
    )
    rust_driver = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_relevance = (root / "src/clauses/relevance.rs").read_text(encoding="utf-8")
    rust_proof_search = rust_driver.split("fn run_proof_search", 1)[1].split(
        "fn hard_time_limit_expired_in_saturation", 1
    )[0]
    rust_apply_relevance = rust_driver.split("fn apply_relevance_pruning", 1)[1].split(
        "fn apply_clause_set_preprocessing", 1
    )[0]
    owner_reference = load_json(
        root
        / "experiments/2026-07-17-058-formula-relevance-ownership/results-summary.json"
    )
    order_reference = load_json(
        root / "experiments/2026-07-17-057-relevance-pruning-order/results-summary.json"
    )

    checks = {
        "c_sine_then_relevance_then_formula_cnf": (
            c_driver.index("ProofStateSinE(")
            < c_driver.index("ProofStateRelevancyProcess(")
            < c_driver.index("FormulaSetCNF2(")
        ),
        "c_combines_sine_and_relevance_removals": (
            "relevancy_pruned += ProofStateSinE" in c_driver
            and "relevancy_pruned += ProofStateRelevancyProcess" in c_driver
        ),
        "c_moves_clause_and_formula_owners": (
            "move_clauses(set, new_ax);" in c_relevance
            and "move_formulas(set, new_fax);" in c_relevance
        ),
        "c_replaces_both_proof_state_owners": (
            "state->axioms   = new_ax;" in c_relevance
            and "state->f_axioms = new_fax;" in c_relevance
        ),
        "c_reports_combined_owner_delta": (
            "old_axno = ProofStateAxNo(state);" in c_relevance
            and "new_axno = ProofStateAxNo(state);" in c_relevance
            and "return old_axno-new_axno;" in c_relevance
        ),
        "rust_sine_then_relevance_then_formula_cnf": (
            rust_proof_search.index("apply_proof_state_sine(")
            < rust_proof_search.index("apply_relevance_pruning(")
            < rust_proof_search.index("clausify_formula_axioms_with_docs(")
        ),
        "rust_combines_sine_and_relevance_removals": (
            "sine_pruned + apply_relevance_pruning" in rust_proof_search
        ),
        "rust_prunes_clause_and_formula_owners_together": all(
            marker in rust_apply_relevance
            for marker in (
                "state.axioms(),",
                "state.f_axioms(),",
                "*state.axioms_mut() = pruned;",
                "*state.f_axioms_mut() = pruned_formulas;",
                "removed",
            )
        ),
        "rust_threads_combined_count_to_statistics": (
            "relevancy_pruned," in rust_proof_search
            and '"{DEFAULT_COMCHAR_RAW} Removed by relevancy pruning/SinE' in rust_driver
        ),
        "permanent_relevance_data_formula_regression": (
            "relevance_pruning_keeps_formula_levels_and_reports_removed_count"
            in rust_relevance
        ),
        "permanent_proof_state_formula_regression": (
            "proof_state_relevance_pruning_filters_represented_formula_axioms"
            in rust_driver
        ),
        "permanent_proof_search_statistics_regression": (
            "run_proof_search_statistics_count_formula_relevance_pruning"
            in rust_driver
        ),
        "retained_owner_reference_matches": (
            owner_reference.get("case_count") == 2
            and owner_reference.get("matching_cases") == 2
        ),
        "retained_order_reference_matches": (
            order_reference.get("case_count") == 3
            and order_reference.get("matching_cases") == 3
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
        print(f"relevance owner audit failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {report['passed']}/{report['total']} relevance owner checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
