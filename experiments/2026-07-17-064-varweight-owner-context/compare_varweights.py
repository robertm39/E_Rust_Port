#!/usr/bin/env python3
"""Audit and compare variable-weight owner-context integration."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
from pathlib import Path


FAMILIES = (
    (
        "TPTPTypeWeightCompute",
        "tptp_type_weight_wfcb_init",
        "tptp_type_weight_wfcb_compute_with_bank",
        "tptp_type_weight_compute_with_bank",
    ),
    (
        "SigWeightCompute",
        "sig_weight_wfcb_init",
        "sig_weight_wfcb_compute_with_bank",
        "sig_weight_compute_with_bank",
    ),
    (
        "ProofWeightCompute",
        "proof_weight_wfcb_init",
        "proof_weight_wfcb_compute_with_bank",
        "proof_weight_compute_with_bank",
    ),
    (
        "DepthWeightCompute",
        "depth_weight_wfcb_init",
        "depth_weight_wfcb_compute_with_bank",
        "depth_weight_compute_with_bank",
    ),
    (
        "WeightLessDepthCompute",
        "weight_less_depth_wfcb_init",
        "weight_less_depth_wfcb_compute_with_bank",
        "weight_less_depth_compute_with_bank",
    ),
    (
        "NLWeightCompute",
        "nl_weight_wfcb_init",
        "nl_weight_wfcb_compute_with_bank",
        "nl_weight_compute_with_bank",
    ),
    (
        "PNRefinedWeightCompute",
        "pn_refined_weight_wfcb_init",
        "pn_refined_weight_wfcb_compute_with_bank",
        "pn_refined_weight_compute_with_bank",
    ),
    (
        "SymTypeWeightCompute",
        "sym_type_weight_wfcb_init",
        "sym_type_weight_wfcb_compute_with_bank",
        "sym_type_weight_compute_with_bank",
    ),
)


def function_body(source: str, name: str) -> str:
    match = re.search(
        rf"(?:\bfn|\bdouble|\bWFCB_p)\s+{name}\s*\(", source
    )
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


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
        "stderr": completed.stderr.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
    }


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = str(result["stdout"])
    stderr = str(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stdout_sha256": digest(stdout),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "stderr_sha256": digest(stderr),
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def source_audit(root: Path) -> dict[str, object]:
    c_source = (root / "eprover/HEURISTICS/che_varweights.c").read_text(
        encoding="utf-8"
    )
    rust_source = (root / "src/heuristics/varweights.rs").read_text(
        encoding="utf-8"
    )
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    c_marked = 0
    rust_banked_initializers = 0
    rust_banked_callbacks = 0
    rust_marked_computes = 0
    for c_compute, rust_init, rust_callback, rust_compute in FAMILIES:
        c_marked += "ClauseCondMarkMaximalTerms" in function_body(c_source, c_compute)
        init_body = function_body(rust_source, rust_init)
        rust_banked_initializers += (
            "wfcb_alloc_with_bank(" in init_body and rust_callback in init_body
        )
        rust_banked_callbacks += rust_compute in function_body(rust_source, rust_callback)
        rust_marked_computes += (
            "cond_mark_maximal_terms_with_bank" in function_body(rust_source, rust_compute)
        )

    checks = {
        "c_mark_then_score_family_count": c_marked,
        "rust_banked_initializer_count": rust_banked_initializers,
        "rust_banked_callback_forward_count": rust_banked_callbacks,
        "rust_banked_mark_then_score_count": rust_marked_computes,
        "proof_control_owner_regression_present": (
            "proof_control_installs_varweight_with_active_owner_context" in proofcontrol
            and "hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut clause)"
            in proofcontrol
        ),
    }
    expected = {
        "c_mark_then_score_family_count": 8,
        "rust_banked_initializer_count": 8,
        "rust_banked_callback_forward_count": 8,
        "rust_banked_mark_then_score_count": 8,
        "proof_control_owner_regression_present": True,
    }
    return {"checks": checks, "expected": expected, "passed": checks == expected}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = (
        args.root.resolve()
        if args.root is not None
        else Path(__file__).resolve().parents[2]
    )
    fixture = Path(__file__).resolve().parent / "problem.lop"
    cases = (
        (
            "tptp_type",
            "TPTPTypeweight(ConstPrio,2,1,1.0,1.0,1.0,7.0,5.0)",
        ),
        ("signature", "Sigweight(ConstPrio,2,1,1.0,1.0,1.0,3.0)"),
        ("proof", "Proofweight(ConstPrio,2,1,1.0,1.0,1.0,8.0,6.0)"),
        ("depth", "Depthweight(ConstPrio,2,1,3.0,1.0,7.0,11.0)"),
        ("weight_less_depth", "WLessDWeight(ConstPrio,2,1,3.0,1.0,7.0,0.5)"),
        ("nonlinear", "NLweight(ConstPrio,2,7,1,1.0,1.0,1.0)"),
        (
            "positive_negative_refined",
            "PNRefinedweight(ConstPrio,2,1,13,17,1.0,1.0,1.0)",
        ),
        ("symbol_type", "SymbolTypeweight(ConstPrio,2,1,3,11,1.0,1.0,1.0)"),
    )

    comparisons: list[dict[str, object]] = []
    for name, definition in cases:
        common_args = ["--lop-in", f"--expert-heuristic=(1*{definition})"]
        rust = run([str(args.rust_exe.resolve()), *common_args, str(fixture)])
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(shlex.quote(arg) for arg in common_args),
                shlex.quote(wsl_path(fixture)),
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
            comparison["mismatch"] = {"rust": rust, "c": c}
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
