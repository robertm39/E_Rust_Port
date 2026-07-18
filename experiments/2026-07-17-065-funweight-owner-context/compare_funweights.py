#!/usr/bin/env python3
"""Audit and compare funweight owner-context integration."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
from pathlib import Path


GENERIC_INITIALIZERS = (
    "fun_weight_wfcb_init",
    "conjecture_symbol_weight_wfcb_init",
    "conjecture_symbol_type_weight_wfcb_init",
    "conjecture_type_based_weight_wfcb_init",
    "relevance_level_weight_wfcb_init",
    "relevance_level_weight_wfcb_init_with_formulas",
    "relevance_level_weight2_wfcb_init",
    "relevance_level_weight2_wfcb_init_with_formulas",
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


def ordered(body: str, markers: tuple[str, ...]) -> bool:
    positions = [body.find(marker) for marker in markers]
    return all(position >= 0 for position in positions) and positions == sorted(positions)


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
    c_source = (root / "eprover/HEURISTICS/che_funweights.c").read_text(
        encoding="utf-8"
    )
    rust_source = (root / "src/heuristics/funweights.rs").read_text(
        encoding="utf-8"
    )
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    c_generic = function_body(c_source, "GenericFunWeightCompute")
    c_offset = function_body(c_source, "SymOffsetWeightCompute")
    rust_generic = function_body(rust_source, "generic_fun_weight_compute_with_bank")
    rust_offset = function_body(rust_source, "sym_offset_weight_compute_with_bank")
    proofcontrol_regression = function_body(
        proofcontrol, "proof_control_installs_funweights_with_active_owner_context"
    )
    generic_init_count = sum(
        "wfcb_alloc_with_bank(" in function_body(rust_source, name)
        and "generic_fun_weight_wfcb_compute_with_bank" in function_body(rust_source, name)
        for name in GENERIC_INITIALIZERS
    )
    sym_init = function_body(rust_source, "sym_offset_weight_wfcb_init")

    checks = {
        "c_generic_init_mark_score_order": ordered(
            c_generic,
            ("local->init_fun(data)", "ClauseCondMarkMaximalTerms", "ClauseFunWeight"),
        ),
        "c_offset_init_mark_score_reset_order": ordered(
            c_offset,
            (
                "local->init_fun(data)",
                "ClauseCondMarkMaximalTerms",
                "ClauseWeight",
                "ClauseAddFunOccs",
                "PDArrayAssignInt",
            ),
        ),
        "rust_generic_init_mark_score_order": ordered(
            rust_generic,
            ("ensure_fun_weights", "cond_mark_maximal_terms_with_bank", "clause.fun_weight"),
        ),
        "rust_offset_init_mark_score_reset_order": ordered(
            rust_offset,
            (
                "ensure_fun_weights",
                "cond_mark_maximal_terms_with_bank",
                "clause.literal_weight",
                "clause.add_fun_occs",
                "f_occur.assign",
            ),
        ),
        "rust_generic_banked_initializer_count": generic_init_count,
        "rust_offset_banked_initializer": (
            "wfcb_alloc_with_bank(" in sym_init
            and "sym_offset_weight_wfcb_compute_with_bank" in sym_init
        ),
        "proof_control_repeat_owner_regression_present": (
            proofcontrol_regression.count(
                "hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut"
            )
            == 2
            and "first_evaluations.eval(index).heuristic().to_bits()"
            in proofcontrol_regression
            and "second_evaluations.eval(index).heuristic().to_bits()"
            in proofcontrol_regression
        ),
    }
    expected = {
        "c_generic_init_mark_score_order": True,
        "c_offset_init_mark_score_reset_order": True,
        "rust_generic_init_mark_score_order": True,
        "rust_offset_init_mark_score_reset_order": True,
        "rust_generic_banked_initializer_count": 8,
        "rust_offset_banked_initializer": True,
        "proof_control_repeat_owner_regression_present": True,
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
        ("generic", "FunWeight(ConstPrio,2,1,3.0,5.0,7.0,f:10,a:20)"),
        ("symbol_offset", "SymOffsetWeight(ConstPrio,2,1,3.0,5.0,7.0,f:10,a:-3)"),
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
