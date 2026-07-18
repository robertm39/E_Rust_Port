#!/usr/bin/env python3
"""Audit migrated definition, CNF, symbol, and encoding option routes."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROUTES = {
    "define-weight-function": {
        "rust": [
            ("src/prover/eprover.rs", ".weight_function_definitions\n                .push(value);"),
            ("src/prover/eprover.rs", "let wfcb_defs = &config.search.heuristic.weight_function_definitions;"),
            ("src/prover/eprover.rs", "proof_control_init_with_formula_axioms("),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "PStackPushP(wfcb_definitions, arg);"),
            ("eprover/PROVER/eprover.c", "fvi_parms, wfcb_definitions, hcb_definitions);"),
        ],
    },
    "define-heuristic": {
        "rust": [
            ("src/prover/eprover.rs", "heuristic_definitions.push(value);"),
            ("src/prover/eprover.rs", "let mut hcb_defs = config.search.heuristic.heuristic_definitions.clone();"),
            ("src/prover/eprover.rs", "proof_control_init_with_formula_axioms("),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "PStackPushP(hcb_definitions, arg);"),
            ("eprover/PROVER/eprover.c", "fvi_parms, wfcb_definitions, hcb_definitions);"),
        ],
    },
    "free-numbers": {
        "rust": [
            ("src/prover/eprover.rs", "FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT"),
            ("src/prover/eprover.rs", "proof_state_alloc(config.free_symbol_properties)?"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "free_symb_prop|FPIsInteger|FPIsRational|FPIsFloat"),
            ("eprover/PROVER/eprover.c", "ProofStateAlloc(free_symb_prop_local);"),
        ],
    },
    "free-objects": {
        "rust": [
            ("src/prover/eprover.rs", "config.free_symbol_properties |= FP_IS_OBJECT;"),
            ("src/prover/eprover.rs", "proof_state_alloc(config.free_symbol_properties)?"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "free_symb_prop|FPIsObject"),
            ("eprover/PROVER/eprover.c", "ProofStateAlloc(free_symb_prop_local);"),
        ],
    },
    "definitional-cnf": {
        "rust": [
            ("src/prover/eprover.rs", "config.preprocessing.formula_def_limit ="),
            ("src/prover/eprover.rs", ".with_def_limit(params.formula_def_limit)"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->formula_def_limit ="),
            ("eprover/PROVER/eprover.c", "h_parms->formula_def_limit,"),
        ],
    },
    "fool-unroll": {
        "rust": [
            ("src/prover/eprover.rs", "config.preprocessing.fool_unroll ="),
            ("src/prover/eprover.rs", "params.fool_unroll,"),
            ("src/clauses/formulasets.rs", "if options.fool_unroll"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->fool_unroll = CLStateGetBoolArg"),
            ("eprover/PROVER/eprover.c", "h_parms->fool_unroll);"),
        ],
    },
    "miniscope-limit": {
        "rust": [
            ("src/prover/eprover.rs", "config.preprocessing.miniscope_limit ="),
            ("src/prover/eprover.rs", "params.miniscope_limit,"),
            ("src/clauses/formulasets.rs", "drain.options.miniscope_limit"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->miniscope_limit ="),
            ("eprover/PROVER/eprover.c", "h_parms->miniscope_limit,"),
        ],
    },
    "print-types": {
        "rust": [
            ("src/prover/eprover.rs", "config.encoding.print_types = true"),
            ("src/prover/eprover.rs", ".with_print_types(config.encoding.print_types)"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "TermPrintTypes = true;"),
            ("eprover/TERMS/cte_termfunc.c", "if(TermPrintTypes)"),
        ],
    },
    "app-encode": {
        "rust": [
            ("src/prover/eprover.rs", "config.encoding.app_encode = true"),
            ("src/prover/eprover.rs", "let status = run_app_encode(output, runtime_config)?;"),
            ("src/clauses/formulasets.rs", "app_encode_string_with_type_suffixes("),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "app_encode = true;"),
            ("eprover/PROVER/eprover.c", "FormulaSetAppEncode(stdout, proofstate->f_axioms);"),
        ],
    },
    "arg-cong": {
        "rust": [
            ("src/prover/eprover.rs", "higher_order.arg_cong = mode"),
            ("src/heuristics/proofcontrol.rs", "compute_arg_cong(terms, clause"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->arg_cong = AllLits;"),
            ("eprover/CONTROL/cco_ho_inferences.c", "ComputeArgCong(state, control, orig_clause);"),
        ],
    },
    "neg-ext": {
        "rust": [
            ("src/prover/eprover.rs", "higher_order.neg_ext = mode"),
            ("src/heuristics/proofcontrol.rs", "compute_neg_ext(terms, clause"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->neg_ext = AllLits;"),
            ("eprover/CONTROL/cco_ho_inferences.c", "ComputeNegExt(state, control, orig_clause);"),
        ],
    },
    "pos-ext": {
        "rust": [
            ("src/prover/eprover.rs", "higher_order.pos_ext = mode"),
            ("src/heuristics/proofcontrol.rs", "compute_pos_ext(terms, clause"),
        ],
        "c": [
            ("eprover/PROVER/eprover.c", "h_parms->pos_ext = AllLits;"),
            ("eprover/CONTROL/cco_ho_inferences.c", "ComputePosExt(state, control, orig_clause);"),
        ],
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
    route_report: dict[str, dict[str, list[str]]] = {}
    rust_options = (repo / "src/prover/options.rs").read_text(encoding="utf-8")
    c_options = (repo / "eprover/PROVER/e_options.h").read_text(encoding="utf-8")
    for option, languages in ROUTES.items():
        checks[f"{option}:rust-cli"] = f'Some("{option}")' in rust_options
        checks[f"{option}:c-cli"] = f'"{option}"' in c_options
        route_report[option] = {}
        for language, routes in languages.items():
            route_report[option][language] = []
            for path, token in routes:
                source = (repo / path).read_text(encoding="utf-8")
                key = f"{option}:{language}:{path}:{token}"
                checks[key] = token in source
                route_report[option][language].append(f"{path}: {token}")

    c_ho = (repo / "eprover/CONTROL/cco_ho_inferences.c").read_text(encoding="utf-8")
    rust_ho = (repo / "src/heuristics/proofcontrol.rs").read_text(encoding="utf-8")
    checks["c_pos_ext_preserves_neg_ext_gate"] = (
        "if (control->heuristic_parms.neg_ext != NoLits)\n      {\n         ComputePosExt"
        in c_ho
    )
    checks["rust_pos_ext_preserves_neg_ext_gate"] = (
        "if parms.neg_ext != ExtInferenceType::NoLits {\n            generated += compute_pos_ext"
        in rust_ho
    )
    c_driver = (repo / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    rust_driver = (repo / "src/prover/eprover.rs").read_text(encoding="utf-8")
    checks["c_extension_errors_exit_zero"] = all(
        token in c_driver
        for token in (
            'Error("neg-ext excepts either all, max or off", 0);',
            'Error("neg-ext excepts either all or max", 0);',
            'Error("pos-ext excepts either all or max", 0);',
        )
    )
    checks["rust_extension_errors_exit_zero"] = (
        "ErrorCode::NO_ERROR,\n            ext_inference_error_message(option),"
        in rust_driver
    )
    c_term_bank = (repo / "eprover/TERMS/cte_termbanks.c").read_text(encoding="utf-8")
    rust_term = (repo / "src/terms/termfunc.rs").read_text(encoding="utf-8")
    checks["c_distinct_argument_errors_use_current_token"] = (
        c_term_bank.count("AktTokenError(in,") >= 4
        and "Number cannot have argument list" in c_term_bank
        and "Object cannot have argument list" in c_term_bank
    )
    checks["rust_distinct_argument_errors_use_current_token"] = all(
        token in rust_term
        for token in (
            "fn distinct_argument_list_diagnostic(scanner: &Scanner, message: &str)",
            "token_pos_rep(token)",
            "token.literal()",
        )
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
