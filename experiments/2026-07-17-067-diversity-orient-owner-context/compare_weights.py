#!/usr/bin/env python3
"""Audit and compare diversity/orient weight owner-context integration."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
from pathlib import Path


def function_body(source: str, name: str) -> str:
    match = re.search(rf"(?:\bfn|\bdouble|\bWFCB_p)\s+{name}\s*\(", source)
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


def banked_initializer(source: str, initializer: str, callback: str) -> bool:
    body = function_body(source, initializer)
    return "wfcb_alloc_with_bank(" in body and callback in body


def callback_forwards(source: str, callback: str, helper: str) -> bool:
    return helper in function_body(source, callback)


def source_audit(root: Path) -> dict[str, object]:
    c_diversity = (
        root / "eprover/HEURISTICS/che_diversityweight.c"
    ).read_text(encoding="utf-8")
    c_orient = (root / "eprover/HEURISTICS/che_orientweight.c").read_text(
        encoding="utf-8"
    )
    rust_diversity = (root / "src/heuristics/diversityweight.rs").read_text(
        encoding="utf-8"
    )
    rust_orient = (root / "src/heuristics/orientweight.rs").read_text(
        encoding="utf-8"
    )
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    c_diversity_compute = function_body(c_diversity, "DiversityWeightCompute")
    c_orient_compute = function_body(c_orient, "ClauseOrientWeightCompute")
    c_lmax_compute = function_body(c_orient, "OrientLMaxWeightCompute")
    rust_diversity_banked = function_body(
        rust_diversity, "diversity_weight_compute_with_bank"
    )
    rust_orient_banked = function_body(
        rust_orient, "clause_orient_weight_compute_with_bank"
    )
    rust_lmax_banked = function_body(
        rust_orient, "orient_lmax_weight_compute_with_bank"
    )
    proofcontrol_regression = function_body(
        proofcontrol,
        "proof_control_installs_diversity_and_orient_weights_with_active_owner_context",
    )

    checks = {
        "c_diversity_mark_weight_diversity_order": ordered(
            c_diversity_compute,
            (
                "ClauseCondMarkMaximalTerms",
                "ClauseWeight",
                "ClauseReturnFCodes",
                "ClauseCollectVariables",
            ),
        ),
        "c_orient_mark_score_order": ordered(
            c_orient_compute, ("ClauseCondMarkMaximalTerms", "ClauseOrientWeight")
        ),
        "c_lmax_mark_score_order": ordered(
            c_lmax_compute, ("ClauseCondMarkMaximalTerms", "EqnMaxWeight")
        ),
        "rust_diversity_mark_score_order": ordered(
            rust_diversity_banked,
            ("cond_mark_maximal_terms_with_bank", "diversity_weight_compute"),
        ),
        "rust_orient_mark_score_order": ordered(
            rust_orient_banked,
            ("cond_mark_maximal_terms_with_bank", "clause_orient_weight_compute"),
        ),
        "rust_lmax_mark_score_order": ordered(
            rust_lmax_banked,
            ("cond_mark_maximal_terms_with_bank", "orient_lmax_weight_compute"),
        ),
        "rust_banked_initializer_count": sum(
            (
                banked_initializer(
                    rust_diversity,
                    "diversity_weight_wfcb_init",
                    "diversity_weight_wfcb_compute_with_bank",
                ),
                banked_initializer(
                    rust_orient,
                    "clause_orient_weight_wfcb_init",
                    "clause_orient_weight_wfcb_compute_with_bank",
                ),
                banked_initializer(
                    rust_orient,
                    "orient_lmax_weight_wfcb_init",
                    "orient_lmax_weight_wfcb_compute_with_bank",
                ),
            )
        ),
        "rust_banked_callback_forward_count": sum(
            (
                callback_forwards(
                    rust_diversity,
                    "diversity_weight_wfcb_compute_with_bank",
                    "diversity_weight_compute_with_bank",
                ),
                callback_forwards(
                    rust_orient,
                    "clause_orient_weight_wfcb_compute_with_bank",
                    "clause_orient_weight_compute_with_bank",
                ),
                callback_forwards(
                    rust_orient,
                    "orient_lmax_weight_wfcb_compute_with_bank",
                    "orient_lmax_weight_compute_with_bank",
                ),
            )
        ),
        "proof_control_repeat_owner_regression_present": (
            proofcontrol_regression.count(
                "hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut"
            )
            == 2
            and "for index in 0..3" in proofcontrol_regression
            and "CP_IS_ORIENTED" in proofcontrol_regression
            and "is_maximal()" in proofcontrol_regression
        ),
    }
    expected = {
        "c_diversity_mark_weight_diversity_order": True,
        "c_orient_mark_score_order": True,
        "c_lmax_mark_score_order": True,
        "rust_diversity_mark_score_order": True,
        "rust_orient_mark_score_order": True,
        "rust_lmax_mark_score_order": True,
        "rust_banked_initializer_count": 3,
        "rust_banked_callback_forward_count": 3,
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
        (
            "diversity",
            "Diversityweight(ConstPrio,2,3,1.0,1.0,1.0,10.0,1.0,20.0,2.0)",
        ),
        ("orient", "Orientweight(ConstPrio,2,1,7.0,5.0,3.0)"),
        ("orient_lmax", "OrientLMaxWeight(ConstPrio,2,1,7.0,5.0,3.0)"),
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
