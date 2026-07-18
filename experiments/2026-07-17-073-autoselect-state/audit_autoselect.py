#!/usr/bin/env python3
"""Audit C/Rust automatic-ordering selection and production reachability."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


def function_body(source: str, name: str) -> str:
    match = re.search(rf"(?:\bfn|\bvoid|\bbool|\bdouble|\bOCB_p)\s+{name}\s*\(", source)
    if match is None:
        raise ValueError(f"function not found: {name}")
    opening = source.find("{", match.end())
    if opening < 0:
        raise ValueError(f"function body not found: {name}")
    depth = 0
    for index in range(opening, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[opening + 1 : index]
    raise ValueError(f"unterminated function body: {name}")


def ordered(body: str, markers: tuple[str, ...]) -> bool:
    positions = [body.find(marker) for marker in markers]
    return all(position >= 0 for position in positions) and positions == sorted(positions)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve() if args.root else Path(__file__).resolve().parents[2]

    c_auto_path = root / "eprover/HEURISTICS/che_to_autoselect.c"
    c_auto = c_auto_path.read_text(encoding="utf-8")
    c_prover = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    rust_auto = (root / "src/heuristics/to_autoselect.rs").read_text(encoding="utf-8")
    rust_prover = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")

    c_eval = function_body(c_auto, "OrderEvaluate")
    rust_eval = function_body(rust_auto, "order_evaluate_with_bank")
    c_find = function_body(c_auto, "OrderFindOptimal")
    rust_find = function_body(rust_auto, "order_find_optimal_with_params")
    c_select = function_body(c_auto, "TOSelectOrdering")
    rust_select = function_body(rust_auto, "to_select_ordering")
    rust_init = function_body(rust_auto, "init_oparms")
    rust_snapshot = function_body(
        rust_auto, "instrumented_c_reference_ordering_search_state_matches"
    )

    legacy_generators = [
        "generate_auto_ordering",
        "generate_autocasc_ordering",
        "generate_autodev_ordering",
        *(f"generate_autosched{index}_ordering" for index in range(10)),
    ]
    other_c_sources = "\n".join(
        path.read_text(encoding="utf-8", errors="strict")
        for path in (root / "eprover").rglob("*.c")
        if path != c_auto_path
    )
    no_legacy_calls = all(
        re.search(rf"\b{name}\s*\(", other_c_sources) is None
        for name in legacy_generators
    )

    checks = {
        "evaluation_order_and_penalties": (
            ordered(
                c_eval,
                (
                    "ClauseSetMarkMaximalTerms",
                    "ClauseSetCountMaximalTerms",
                    "ClauseSetCountMaximalLiterals",
                    "ClauseSetCountUnorientableLiterals",
                    "ocb->type == KBO",
                ),
            )
            and ordered(
                rust_eval,
                (
                    "mark_maximal_terms_with_bank",
                    "clause_set_count_maximal_terms",
                    "clause_set_count_maximal_literals",
                    "clause_set_count_unorientable_literals",
                    "ocb.ordering_type == TermOrdering::Kbo",
                ),
            )
        ),
        "optimal_search_seed_iteration_and_strict_replacement": (
            ordered(
                c_find,
                (
                    "local.ordertype",
                    "local.to_weight_gen",
                    "local.to_prec_gen",
                    "local.to_const_weight",
                    "store = local",
                    "best_ocb  = TOCreateOrdering",
                    "while(OrderNextOrdering",
                    "if(tmp_eval < best_eval)",
                ),
            )
            and c_find.count("store = local") == 2
            and ordered(
                c_find.split("if(tmp_eval < best_eval)", maxsplit=1)[1],
                ("best_ocb = tmp_ocb", "best_eval = tmp_eval", "store = local"),
            )
            and ordered(
                rust_find,
                (
                    "local.ordertype",
                    "local.to_weight_gen",
                    "local.to_prec_gen",
                    "local.to_const_weight",
                    "let mut best_params = local.clone()",
                    "let mut best_ocb = to_create_ordering",
                    "while order_next_ordering",
                    "if next_eval < best_eval",
                ),
            )
            and rust_find.count("best_params = local.clone()") == 2
            and ordered(
                rust_find.split("if next_eval < best_eval", maxsplit=1)[1],
                ("best_ocb = next_ocb", "best_eval = next_eval", "best_params = local.clone()"),
            )
        ),
        "selection_normalization_and_rewrite_flag": (
            ordered(
                c_select,
                (
                    "tmp = params->order_params",
                    "tmp.ordertype = KBO",
                    "tmp.to_const_weight = WConstNoSpecialWeight",
                    "result = TOCreateOrdering",
                    "result->rewrite_strong_rhs_inst",
                ),
            )
            and ordered(
                rust_select,
                (
                    "let mut tmp = params.order_params.clone()",
                    "tmp.ordertype = TermOrdering::Kbo",
                    "tmp.to_const_weight = W_CONST_NO_SPECIAL_WEIGHT",
                    "to_create_ordering",
                    "result.rewrite_strong_rhs_inst",
                ),
            )
        ),
        "initialized_auto_fields_match_c": (
            all(
                marker in rust_init
                for marker in (
                    "TermOrdering::Kbo6",
                    "W_CONST_NO_SPECIAL_WEIGHT",
                    "TOWeightGenMethod::SelectMaximal",
                    "TOPrecGenMethod::UnaryFirst",
                    "LiteralCmp::Normal",
                    "HoOrderKind::LfhoOrder",
                    "DEFAULT_DB_WEIGHT",
                    "DEFAULT_LAMBDA_WEIGHT",
                    "force_kbo_var_weight = false",
                )
            )
            and all(
                marker in function_body(c_auto, "init_oparms")
                for marker in (
                    "KBO6",
                    "WConstNoSpecialWeight",
                    "WSelectMaximal",
                    "PUnaryFirst",
                    "LCNormal",
                    "LFHO_ORDER",
                    "DEFAULT_DB_WEIGHT",
                    "DEFAULT_LAMBDA_WEIGHT",
                    "force_kbo_var_weight = false",
                )
            )
        ),
        "all_thirteen_legacy_auto_generators_are_dormant": (
            len(legacy_generators) == 13 and no_legacy_calls
        ),
        "optimize_cli_path_is_upstream_disabled": (
            '/* else if(strcmp(arg, "Optimize")==0) */' in c_prover
            and 'process_options(["eprover", "-t", "Auto"])' in rust_prover
            and 'process_options(["eprover", "-t", "Optimize"])' in rust_prover
            and 'process_options(["eprover", "--term-ordering=RPO"])' in rust_prover
        ),
        "full_reference_sequence_regression_present": (
            "assert_eq!(index + 1, 1_972)" in rust_snapshot
            and "0x8C88_4832_231F_E663" in rust_snapshot
            and "order_next_ordering(&mut ordering, &mask)" in rust_snapshot
        ),
    }
    expected = {name: True for name in checks}
    output = {
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "checks": checks,
        "expected": expected,
        "passed": checks == expected,
    }
    rendered = json.dumps(output, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if not output["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
