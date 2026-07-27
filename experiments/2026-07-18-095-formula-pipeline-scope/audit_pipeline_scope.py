#!/usr/bin/env python3
"""Audit migrated formula-pipeline umbrella claims against C and Rust source."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


CHECKS_BY_FILE = {
    "eprover/PROVER/eprover.c": {
        "c_driver_formula_set_cnf": "FormulaSetCNF2(proofstate->f_axioms",
        "c_driver_proof_control_init": "ProofControlInit(proofstate, proofcontrol",
        "c_driver_app_encode": "FormulaSetAppEncode(stdout, proofstate->f_axioms)",
    },
    "eprover/CLAUSES/ccl_formulafunc.c": {
        "c_cnf_named_to_db": "TFormulaSetNamedToDBLambdas(set, archive, terms)",
        "c_cnf_lift_ites": "TFormulaSetLiftItes(set, archive, terms)",
        "c_cnf_lift_lets": "TFormulaSetLiftLets(set, archive, terms)",
        "c_cnf_unfold_defs": "TFormulaSetUnfoldDefSymbols(set, archive, terms, unfold_only_forms)",
        "c_cnf_lambda_normalize": "TFormulaSetLambdaNormalize(set, archive, terms)",
        "c_cnf_fool_unroll": "WFormulaSetUnrollFOOL(set, archive, terms)",
        "c_cnf_simplify": "FormulaSetSimplify(set, terms, true)",
        "c_cnf_introduce_defs": "TFormulaSetIntroduceDefs(set, archive, terms, def_limit)",
        "c_cnf_archive_original": "FormulaSetInsert(archive, handle)",
        "c_cnf_wrapped_conversion": "WFormulaCNF2(handle, clauseset, terms, fresh_vars",
        "c_cnf_lift_clause_lambdas": "ClauseSetLiftLambdas(clauseset, archive, terms, fresh_vars, fool_unroll)",
    },
    "eprover/CLAUSES/ccl_formulasets.c": {
        "c_app_encode_preload": "PreloadTypes(handle->terms, handle->tformula)",
    },
    "eprover/CLAUSES/ccl_global_indices.h": {
        "c_ext_into_owner": "ExtIndex_p        ext_sup_into_index",
        "c_ext_from_owner": "ExtIndex_p        ext_sup_from_index",
    },
    "eprover/CLAUSES/ccl_proofstate.h": {
        "c_proof_state_global_indices": "GlobalIndices gindices",
    },
    "eprover/CONTROL/cco_ho_inferences.c": {
        "c_dispatch_arg_cong": "ComputeArgCong(state, control, orig_clause)",
        "c_dispatch_neg_ext": "ComputeNegExt(state, control, orig_clause)",
        "c_dispatch_pos_ext": "ComputePosExt(state, control, orig_clause)",
        "c_dispatch_inverse": "InferInjectiveDefinition(state, control, orig_clause)",
        "c_dispatch_ext_sup": "ComputeExtSup(state, control, renamed_cl, orig_clause)",
        "c_dispatch_ext_eq_res": "ComputeExtEqRes(state, control, orig_clause)",
        "c_dispatch_ext_eq_fact": "ComputeExtEqFact(state, control, orig_clause)",
        "c_dispatch_leibniz": "EliminateLeibnizEquality(state->tmp_store, orig_clause",
        "c_dispatch_primitive_enum": "PrimitiveEnumeration(state->tmp_store, orig_clause",
        "c_dispatch_choice": "InstantiateChoiceClauses(state->tmp_store, state->archive, state->choice_opcodes",
    },
    "src/prover/eprover.rs": {
        "rust_driver_formula_set_cnf_docs": "formulas.cnf2_into_with_docs_and_gc_context(",
        "rust_driver_formula_set_cnf_silent": ".cnf2_into_with_gc_context(archive, clauses, bank, &fresh_vars, options, &gc_context)",
        "rust_driver_proof_control_init": "proof_control_init_with_formula_axioms(",
        "rust_driver_global_indices_init": "proof_state_init_global_indices(&mut state, &control, problem_type())",
        "rust_driver_app_encode_owner": "formula_set.app_encode_string_with_type_suffixes(bank, problem_type, true, print_types)",
        "rust_driver_free_symbol_owner": "proof_state_alloc(config.free_symbol_properties)",
        "rust_driver_print_types": "with_print_types(config.encoding.print_types)",
        "rust_thf_represented_owner_route": '"thf" => true',
    },
    "src/clauses/formulasets.rs": {
        "rust_cnf_named_to_db": "self.named_to_db_lambdas_with_docs(",
        "rust_cnf_lift_ites": "self.lift_ites(bank, options.problem_type)",
        "rust_cnf_lift_lets": "self.lift_lets(bank, options.problem_type)",
        "rust_cnf_unfold_defs": "self.unfold_def_symbols_with_docs(",
        "rust_cnf_lambda_normalize": "self.lambda_normalize_forall_with_docs(",
        "rust_cnf_fool_unroll": "self.unroll_fool(bank)",
        "rust_cnf_simplify": "self.simplify_with_garbage_collection_and_docs_context(",
        "rust_cnf_introduce_defs": "self.introduce_defs_with_docs(",
        "rust_cnf_wrapped_conversion": "drain_formula_set_to_cnf_with_docs(",
        "rust_cnf_lift_clause_lambdas": "apply_post_cnf_clause_lambda_lifting(",
        "rust_cnf_gc": "collect_formula_set_cnf_garbage(",
        "rust_app_encode_preload": "tformula_preload_types(bank, formula.formula())",
    },
    "src/clauses/proofstate.rs": {
        "rust_proof_state_global_indices": "global_indices: GlobalIndices",
        "rust_proof_state_watchlist_indices": "watchlist_indices: GlobalIndices",
    },
    "src/clauses/global_indices.rs": {
        "rust_ext_into_owner": "ext_sup_into_index: Option<ExtIndex>",
        "rust_ext_from_owner": "ext_sup_from_index: Option<ExtIndex>",
        "rust_ext_into_allocate": "self.ext_sup_into_index = Some(ExtIndex::new())",
        "rust_ext_from_allocate": "self.ext_sup_from_index = Some(ExtIndex::new())",
    },
    "src/heuristics/proofcontrol.rs": {
        "rust_weight_formula_context": "WeightParseContext::new_with_formulas_and_signature(",
        "rust_weight_defs_install": "install_option_weight_functions(control, definition, context)",
        "rust_heuristic_defs_install": "install_option_heuristics(control, definition, context)",
        "rust_dispatch_arg_cong": "compute_arg_cong(terms, clause, generation.tmp_store, parms.arg_cong)",
        "rust_dispatch_neg_ext": "compute_neg_ext(terms, clause, generation.tmp_store, parms.neg_ext)",
        "rust_dispatch_pos_ext": "compute_pos_ext(terms, clause, generation.tmp_store, parms.pos_ext)",
        "rust_dispatch_inverse": "compute_inverse_recognition(terms, clause, generation.tmp_store)",
        "rust_dispatch_ext_sup": "compute_ext_sup(",
        "rust_dispatch_ext_eq_res": "compute_ext_eq_res(",
        "rust_dispatch_ext_eq_fact": "compute_ext_eq_fact(",
        "rust_dispatch_leibniz": "compute_leibniz_elimination(",
        "rust_dispatch_primitive_enum": "compute_primitive_enumeration(",
        "rust_dispatch_choice": "instantiate_choice_clauses(",
        "rust_generation_dispatch_call": "let _ = compute_ho_inferences(",
        "rust_state_index_consumer": "indices.has_ext_into_index() || !indices.has_ext_from_index()",
    },
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
    routes: dict[str, list[str]] = {}
    for path, expected in CHECKS_BY_FILE.items():
        source = (repo / path).read_text(encoding="utf-8")
        routes[path] = []
        for name, token in expected.items():
            checks[name] = token in source
            routes[path].append(token)

    report = {"checks": checks, "routes": routes}
    report["sha256"] = digest(report)
    if args.output:
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("formula-pipeline scope audit differs from the retained reference")
            return 1
    failed = [name for name, passed in checks.items() if not passed]
    if failed:
        print("failed checks:")
        for name in failed:
            print(f"- {name}")
        return 1
    print(f"validated {len(checks)} C/Rust formula-pipeline ownership checks")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
