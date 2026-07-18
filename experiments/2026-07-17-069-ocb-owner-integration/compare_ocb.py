#!/usr/bin/env python3
"""Audit OCB higher-order state and proof-control ownership integration."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
from pathlib import Path


def function_body(source: str, name: str) -> str:
    match = re.search(rf"(?:\bfn|\bvoid|\bOCB_p)\s+{name}\s*\(", source)
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


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = bytes(result["stdout"])
    stderr = bytes(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "stdout_bytes": len(stdout),
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_bytes": len(stderr),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
    }


def readable(result: dict[str, object]) -> dict[str, object]:
    return {
        "exit_code": result["exit_code"],
        "stdout": bytes(result["stdout"]).decode("utf-8", errors="backslashreplace"),
        "stderr": bytes(result["stderr"]).decode("utf-8", errors="backslashreplace"),
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def quote_wsl(argument: str) -> str:
    if any(character in argument for character in "<>|&;()$`"):
        return shlex.quote(argument)
    return argument


def source_audit(root: Path) -> dict[str, object]:
    c_ocb = (root / "eprover/ORDERINGS/cto_ocb.c").read_text(encoding="utf-8")
    c_kbolin = (root / "eprover/ORDERINGS/cto_kbolin.c").read_text(
        encoding="utf-8"
    )
    c_proofproc = (root / "eprover/CONTROL/cco_proofproc.c").read_text(
        encoding="utf-8"
    )
    rust_ocb = (root / "src/orderings/ocb.rs").read_text(encoding="utf-8")
    rust_kbolin = (root / "src/orderings/cto_kbolin.rs").read_text(
        encoding="utf-8"
    )
    rust_proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    c_inc = function_body(c_kbolin, "inc_vb_ho")
    c_dec = function_body(c_kbolin, "dec_vb_ho")
    c_reset = function_body(c_ocb, "OCBResetHOVarMap")
    rust_inc = function_body(rust_ocb, "inc_ho_var_balance")
    rust_dec = function_body(rust_ocb, "dec_ho_var_balance")
    rust_reset = function_body(rust_ocb, "reset_ho_var_map")
    rust_kbo_reset = function_body(rust_kbolin, "kbo6_reset")
    rust_pc_init = function_body(rust_proofcontrol, "proof_control_init")
    rust_pc_formula_init = function_body(
        rust_proofcontrol, "proof_control_init_with_formula_axioms"
    )
    c_pc_init = function_body(c_proofproc, "ProofControlInit")
    ocb_regression = function_body(
        rust_ocb,
        "higher_order_variable_map_uses_term_identity_and_c_reset_boundary",
    )
    proofcontrol_regression = function_body(
        rust_proofcontrol, "proof_control_init_owns_higher_order_lambda_ocb_like_c"
    )

    checks = {
        "c_identity_map_balance_transitions": (
            "PObjMapGetRef(&ocb->ho_vb, var, PCmpFun, NULL)" in c_inc
            and ordered(c_inc, ("pos_bal", "neg_bal", "**bal_ref += 1", "wb"))
            and "PObjMapGetRef(&ocb->ho_vb, var, PCmpFun, NULL)" in c_dec
            and ordered(c_dec, ("neg_bal", "pos_bal", "**bal_ref -= 1", "wb"))
        ),
        "rust_identity_map_balance_transitions": (
            "term_identity_id(term)" in rust_inc
            and ordered(rust_inc, ("pos_bal", "neg_bal", "*balance += 1", "wb"))
            and "term_identity_id(term)" in rust_dec
            and ordered(rust_dec, ("neg_bal", "pos_bal", "*balance -= 1", "wb"))
        ),
        "map_only_reset_matches_c": (
            ordered(c_reset, ("PObjMapFreeWDeleter", "ocb->ho_vb = NULL"))
            and "self.ho_vb.clear()" in rust_reset
            and "pos_bal" not in rust_reset
            and "neg_bal" not in rust_reset
            and "wb" not in rust_reset
        ),
        "kbo6_lambda_reset_uses_ocb_map": (
            "HoOrderKind::LambdaOrder" in rust_kbo_reset
            and "ocb.reset_ho_var_map()" in rust_kbo_reset
        ),
        "c_proof_control_single_ordering_owner_call": (
            c_proofproc.count("TOSelectOrdering(") == 1
            and ordered(
                c_pc_init,
                (
                    "control->ocb = TOSelectOrdering",
                    "CreateScanner",
                    "WeightFunDefListParse",
                    "HeuristicDefListParse",
                ),
            )
        ),
        "rust_proof_control_owner_bridges": (
            rust_proofcontrol.count("let ocb = to_select_ordering(") == 2
            and ordered(
                rust_pc_init,
                ("let ocb = to_select_ordering", "control.ocb = Some(ocb)", "WeightParseContext"),
            )
            and ordered(
                rust_pc_formula_init,
                ("let ocb = to_select_ordering", "control.ocb = Some(ocb)", "WeightParseContext"),
            )
        ),
        "ocb_identity_reset_regression_present": (
            "term_identity_id(&first)" in ocb_regression
            and "assert_eq!(ocb.ho_vb.len(), 2)" in ocb_regression
            and "ocb.reset_ho_var_map()" in ocb_regression
        ),
        "proof_control_lambda_owner_regression_present": (
            "HoOrderKind::LambdaOrder" in proofcontrol_regression
            and "assert_eq!(ocb.sig_size, expected_sig_size)" in proofcontrol_regression
            and "assert_eq!(ocb.lam_weight, 30)" in proofcontrol_regression
            and "assert_eq!(ocb.db_weight, 12)" in proofcontrol_regression
        ),
    }
    expected = {
        "c_identity_map_balance_transitions": True,
        "rust_identity_map_balance_transitions": True,
        "map_only_reset_matches_c": True,
        "kbo6_lambda_reset_uses_ocb_map": True,
        "c_proof_control_single_ordering_owner_call": True,
        "rust_proof_control_owner_bridges": True,
        "ocb_identity_reset_regression_present": True,
        "proof_control_lambda_owner_regression_present": True,
    }
    return {"checks": checks, "expected": expected, "passed": checks == expected}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--c-ho-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = (
        args.root.resolve()
        if args.root is not None
        else Path(__file__).resolve().parents[2]
    )
    experiment_dir = Path(__file__).resolve().parent
    cases = (
        (
            "fol_kbo6",
            args.c_exe,
            [
                "--output-level=0",
                "--lop-in",
                "--term-ordering=KBO6",
                str(experiment_dir / "problem.lop"),
            ],
        ),
        (
            "thf_lambda_kbo6",
            args.c_ho_exe,
            [
                "--output-level=0",
                "--term-ordering=KBO6",
                "--ho-order-kind=lambda",
                "--kbo-lam-weight=30",
                "--kbo-db-weight=12",
                str(experiment_dir / "lambda.p"),
            ],
        ),
    )

    comparisons: list[dict[str, object]] = []
    for name, c_exe, rust_args in cases:
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c_args = [
            wsl_path(Path(argument)) if argument in rust_args[-1:] else argument
            for argument in rust_args
        ]
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                c_exe,
                *(quote_wsl(argument) for argument in c_args),
            ]
        )
        exact_match = rust == c
        comparison: dict[str, object] = {
            "case": name,
            "exact_match": exact_match,
            "rust": summarize(rust),
            "c": summarize(c),
        }
        if not exact_match:
            comparison["mismatch"] = {"rust": readable(rust), "c": readable(c)}
        comparisons.append(comparison)

    audit = source_audit(root)
    result = {
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "source_audit": audit,
        "case_count": len(comparisons),
        "exact_count": sum(bool(case["exact_match"]) for case in comparisons),
        "comparisons": comparisons,
    }
    rendered = json.dumps(result, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if not audit["passed"] or result["exact_count"] != result["case_count"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
