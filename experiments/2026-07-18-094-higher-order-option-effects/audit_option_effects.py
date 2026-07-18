#!/usr/bin/env python3
"""Audit every migrated higher-order option from CLI spelling to consumer."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROUTES = {
    "ext-sup-max-depth": [
        ("src/prover/eprover.rs", "ext_rules_max_depth"),
        ("src/heuristics/proofcontrol.rs", "parms.ext_rules_max_depth"),
    ],
    "inverse-recognition": [
        ("src/prover/eprover.rs", "inverse_recognition"),
        ("src/heuristics/proofcontrol.rs", "parms.inverse_recognition"),
    ],
    "replace-inj-defs": [
        ("src/prover/eprover.rs", "replace_inj_defs"),
        ("src/prover/eprover.rs", "replace_injectivity_defs: heuristic_params.replace_inj_defs"),
    ],
    "bce": [
        ("src/prover/eprover.rs", "apply_blocked_clause_elimination("),
        ("src/clauses/bce.rs", "eliminate_blocked_clauses"),
    ],
    "pred-elim": [
        ("src/prover/eprover.rs", "apply_predicate_elimination("),
        ("src/clauses/pred_elim.rs", "eliminate_predicates_singular"),
    ],
    "cnf-lambda-to-forall": [
        ("src/prover/eprover.rs", ".with_lambda_to_forall(params.lambda_to_forall)"),
        ("src/clauses/formulasets.rs", "options.higher_order.lambda_to_forall"),
    ],
    "eta-normalize": [
        ("src/prover/eprover.rs", "apply_eta_normalization_state"),
        ("src/terms/lambda.rs", "get_eta_normalizer()(bank, &beta_normal)"),
    ],
    "ho-order-kind": [
        ("src/prover/eprover.rs", "params.order_params.ho_order_kind"),
        ("src/orderings/cto_kbolin.rs", "ocb.ho_order_kind"),
    ],
    "eliminate-leibniz-eq": [
        ("src/prover/eprover.rs", "elim_leibniz_max_depth"),
        ("src/heuristics/proofcontrol.rs", "compute_leibniz_elimination("),
    ],
    "unroll-formulas-only": [
        ("src/prover/eprover.rs", ".with_unfold_only_forms(params.unroll_only_formulas)"),
        ("src/clauses/formulasets.rs", "options.higher_order.unfold_only_forms"),
    ],
    "prim-enum-mode": [
        ("src/prover/eprover.rs", "prim_enum_mode"),
        ("src/heuristics/proofcontrol.rs", "parms.prim_enum_mode"),
    ],
    "prim-enum-max-depth": [
        ("src/prover/eprover.rs", "prim_enum_max_depth"),
        ("src/heuristics/proofcontrol.rs", "parms.prim_enum_max_depth"),
    ],
    "inst-choice-max-depth": [
        ("src/prover/eprover.rs", "apply_choice_axiom_recognition"),
        ("src/heuristics/proofcontrol.rs", "parms.inst_choice_max_depth"),
    ],
    "local-rw": [
        ("src/prover/eprover.rs", "local_rw: ho_search.local_rw"),
        ("src/heuristics/proofcontrol.rs", "clause_local_rw("),
    ],
    "prune-args": [
        ("src/prover/eprover.rs", "prune_args: ho_search.prune_args"),
        ("src/heuristics/proofcontrol.rs", "clause_prune_args("),
    ],
    "func-proj-limit": [
        ("src/prover/eprover.rs", "func_proj_limit"),
        ("src/terms/ho_bindings.rs", "params.func_proj_limit"),
    ],
    "unif-mode": [
        ("src/prover/eprover.rs", "unif_mode"),
        ("src/terms/ho_csu.rs", "params.unif_mode"),
    ],
    "pattern-oracle": [
        ("src/prover/eprover.rs", "pattern_oracle"),
        ("src/terms/ho_csu.rs", "params.pattern_oracle"),
    ],
    "fixpoint-oracle": [
        ("src/prover/eprover.rs", "fixpoint_oracle"),
        ("src/terms/ho_csu.rs", "params.fixpoint_oracle"),
    ],
    "max-unifiers": [
        ("src/prover/eprover.rs", "max_unifiers"),
        ("src/terms/ho_csu.rs", "params.max_unifiers"),
    ],
    "max-unif-steps": [
        ("src/prover/eprover.rs", "max_unif_steps"),
        ("src/terms/ho_csu.rs", "params.max_unif_steps"),
    ],
    "preinstantiate-induction": [
        ("src/prover/eprover.rs", "apply_induction_preinstantiation"),
        ("src/heuristics/proofcontrol.rs", "pub fn preinstantiate_induction("),
    ],
}


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    checks: dict[str, bool] = {}
    route_report: dict[str, list[str]] = {}
    option_source = (repo / "src/prover/options.rs").read_text(encoding="utf-8")
    for option, routes in ROUTES.items():
        checks[f"{option}:cli"] = f'Some("{option}")' in option_source
        route_report[option] = []
        for path, token in routes:
            source = (repo / path).read_text(encoding="utf-8")
            key = f"{option}:{path}:{token}"
            checks[key] = token in source
            route_report[option].append(f"{path}: {token}")

    c_preprocessing = (repo / "eprover/CONTROL/cco_preprocessing.c").read_text(
        encoding="utf-8"
    )
    rust_driver = (repo / "src/prover/eprover.rs").read_text(encoding="utf-8")
    checks["c_bce_fo_gate"] = "if(problemType == PROBLEM_FO && h_parms->bce)" in c_preprocessing
    checks["c_pred_elim_fo_gate"] = (
        "if(problemType == PROBLEM_FO && h_parms->pred_elim)" in c_preprocessing
    )
    checks["rust_bce_fo_gate"] = (
        "if !enabled || problem_type() != ProblemType::FirstOrder" in rust_driver
    )
    checks["rust_pred_elim_fo_gate"] = (
        "if !config.enabled || problem_type() != ProblemType::FirstOrder" in rust_driver
    )
    checks["print_strategy_bce_prefix"] = (
        'b"% BCE start: 0\\n% BCE eliminated: 0.\\n"' in rust_driver
    )
    checks["print_strategy_pred_elim_prefix"] = (
        'b"% PE start: 0\\n% PE eliminated: 0\\n"' in rust_driver
    )

    report = {"checks": checks, "option_routes": route_report}
    report["sha256"] = digest(report)
    if args.output:
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("option-effect audit differs from the retained reference")
            return 1
    failed = [name for name, passed in checks.items() if not passed]
    if failed:
        print("failed checks:")
        for name in failed:
            print(f"- {name}")
        return 1
    print(f"validated {len(ROUTES)} option routes and {len(checks)} total checks")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
