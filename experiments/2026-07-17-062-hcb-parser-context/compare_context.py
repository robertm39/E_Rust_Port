#!/usr/bin/env python3
"""Compare HCB parser-context paths and audit their Rust owner handoff."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
from pathlib import Path


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
    c_header = (root / "eprover/HEURISTICS/che_hcbadmin.h").read_text(
        encoding="utf-8"
    )
    hcbadmin = (root / "src/heuristics/hcbadmin.rs").read_text(encoding="utf-8")
    wfcbadmin = (root / "src/heuristics/wfcbadmin.rs").read_text(encoding="utf-8")
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )
    executable = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")

    checks = {
        "c_hcb_entry_points_with_ocb_state": sum(
            marker in c_header
            for marker in (
                "HCB_p      HeuristicParse(Scanner_p in, WFCBAdmin_p wfcbs, OCB_p ocb,",
                "wfcbs, OCB_p ocb, ProofState_p state);",
                "WFCBAdmin_p wfcbs, OCB_p ocb,",
            )
        ),
        "rust_context_axiom_field_count": wfcbadmin.count(
            "axioms: Option<&'a ClauseSet>"
        ),
        "rust_context_formula_field_count": wfcbadmin.count(
            "formula_axioms: Option<&'a FormulaSet>"
        ),
        "rust_context_signature_field_count": wfcbadmin.count(
            "signature: Option<&'a Signature>"
        ),
        "hcb_inline_context_forward_count": hcbadmin.count(
            "weight_fun_def_parse_with_context(scanner, context)?"
        ),
        "proof_control_full_context_count": proofcontrol.count(
            "WeightParseContext::new_with_formulas_and_signature("
        ),
        "proof_control_hcb_context_parse_count": proofcontrol.count(
            ".heuristic_def_list_parse_with_context("
        ),
        "executable_proof_control_init_count": executable.count(
            "proof_control_init_with_formula_axioms("
        ),
        "banked_hcb_eval_accepts_live_ocb": (
            "ocb: &mut OrderControlBlock" in (root / "src/heuristics/hcb.rs").read_text(encoding="utf-8")
        ),
        "context_handoff_regression_present": (
            "heuristic_parse_threads_proof_state_context_to_inline_weight_defs"
            in hcbadmin
        ),
    }
    expected = {
        "c_hcb_entry_points_with_ocb_state": 3,
        "rust_context_axiom_field_count": 1,
        "rust_context_formula_field_count": 1,
        "rust_context_signature_field_count": 1,
        "hcb_inline_context_forward_count": 1,
        "proof_control_full_context_count": 2,
        "proof_control_hcb_context_parse_count": 2,
        "executable_proof_control_init_count": 1,
        "banked_hcb_eval_accepts_live_ocb": True,
        "context_handoff_regression_present": True,
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
    fixture = Path(__file__).resolve().parent / "context_unsat.lop"
    cases = [
        (
            "proof_state_backed_inline",
            ["--lop-in", "--expert-heuristic=(1*StaggeredWeight(ConstPrio,1.0))"],
        ),
        (
            "ordering_dependent_inline",
            [
                "--lop-in",
                "--expert-heuristic=(1*Orientweight(ConstPrio,2,1,7.0,5.0,3.0))",
            ],
        ),
        (
            "axiom_derived_inline",
            [
                "--lop-in",
                "--expert-heuristic=(1*ConjectureSymbolWeight(ConstPrio,10,99,1,88,1,1.0,1.0,1.0))",
            ],
        ),
        (
            "named_context_backed_hcb",
            [
                "--lop-in",
                "--define-heuristic=ContextSearch=(1*StaggeredWeight(ConstPrio,1.0))",
                "--expert-heuristic=ContextSearch",
            ],
        ),
    ]

    comparisons: list[dict[str, object]] = []
    for name, common_args in cases:
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
