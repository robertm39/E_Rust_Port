#!/usr/bin/env python3
"""Audit and compare conjecture-term weight owner-context integration."""

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
        "termweight",
        "termweights",
        "ConjectureRelativeTermWeightCompute",
        "conjecture_relative_term_weight_init",
        "conjecture_relative_term_weight_compute_with_bank",
        "conjecture_relative_term_weight_wfcb_compute_with_bank",
        "conjecture_relative_term_weight_compute",
    ),
    (
        "prefixweight",
        "prefixweight",
        "ConjectureTermPrefixWeightCompute",
        "conjecture_term_prefix_weight_init",
        "conjecture_term_prefix_weight_compute_with_bank",
        "conjecture_term_prefix_weight_wfcb_compute_with_bank",
        "conjecture_term_prefix_weight_compute",
    ),
    (
        "tfidfweight",
        "tfidfweight",
        "ConjectureTermTfIdfWeightCompute",
        "conjecture_term_tfidf_weight_init",
        "conjecture_term_tfidf_weight_compute_with_bank",
        "conjecture_term_tfidf_weight_wfcb_compute_with_bank",
        "conjecture_term_tfidf_weight_compute",
    ),
    (
        "levweight",
        "levweight",
        "ConjectureLevDistanceWeightCompute",
        "conjecture_lev_distance_weight_init",
        "conjecture_lev_distance_weight_compute_with_bank",
        "conjecture_lev_distance_weight_wfcb_compute_with_bank",
        "conjecture_lev_distance_weight_compute",
    ),
    (
        "strucweight",
        "strucweight",
        "ConjectureStrucDistanceWeightCompute",
        "conjecture_struc_distance_weight_init",
        "conjecture_struc_distance_weight_compute_with_bank",
        "conjecture_struc_distance_weight_wfcb_compute_with_bank",
        "conjecture_struc_distance_weight_compute",
    ),
    (
        "treeweight",
        "treeweight",
        "ConjectureTreeDistanceWeightCompute",
        "conjecture_tree_distance_weight_init",
        "conjecture_tree_distance_weight_compute_with_bank",
        "conjecture_tree_distance_weight_wfcb_compute_with_bank",
        "conjecture_tree_distance_weight_compute",
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
    proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )
    c_order_count = 0
    rust_initializer_count = 0
    rust_banked_order_count = 0
    rust_callback_count = 0
    for (
        c_module,
        rust_module,
        c_compute,
        rust_init,
        rust_banked,
        rust_callback,
        rust_core,
    ) in FAMILIES:
        c_source = (root / f"eprover/HEURISTICS/che_{c_module}.c").read_text(
            encoding="utf-8"
        )
        rust_source = (root / f"src/heuristics/{rust_module}.rs").read_text(
            encoding="utf-8"
        )
        c_order_count += ordered(
            function_body(c_source, c_compute),
            ("init_fun", "ClauseCondMarkMaximalTerms", "ClauseTermExtWeight"),
        )
        init_body = function_body(rust_source, rust_init)
        rust_initializer_count += (
            "wfcb_alloc_with_bank(" in init_body and rust_callback in init_body
        )
        rust_banked_order_count += ordered(
            function_body(rust_source, rust_banked),
            ("ensure_init", "cond_mark_maximal_terms_with_bank", rust_core),
        )
        rust_callback_count += rust_banked in function_body(rust_source, rust_callback)

    tfidf_source = (root / "src/heuristics/tfidfweight.rs").read_text(
        encoding="utf-8"
    )
    proofcontrol_regression = function_body(
        proofcontrol,
        "proof_control_installs_conjecture_term_weights_with_active_owner_context",
    )
    checks = {
        "c_lazy_mark_score_family_count": c_order_count,
        "rust_banked_initializer_count": rust_initializer_count,
        "rust_banked_init_mark_score_count": rust_banked_order_count,
        "rust_banked_callback_forward_count": rust_callback_count,
        "rust_tfidf_scores_before_document_update": ordered(
            function_body(tfidf_source, "conjecture_term_tfidf_weight_compute"),
            ("clause.term_ext_weight", "tfidf_documents_add_clause_to_state"),
        ),
        "proof_control_repeat_owner_regression_present": (
            proofcontrol_regression.count(
                "hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut"
            )
            == 2
            and "for index in 0..6" in proofcontrol_regression
            and "CP_TYPE_NEG_CONJECTURE" in proofcontrol_regression
        ),
    }
    expected = {
        "c_lazy_mark_score_family_count": 6,
        "rust_banked_initializer_count": 6,
        "rust_banked_init_mark_score_count": 6,
        "rust_banked_callback_forward_count": 6,
        "rust_tfidf_scores_before_document_update": True,
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
    fixture = Path(__file__).resolve().parent / "problem.p"
    cases = (
        (
            "relative_term",
            "ConjectureRelativeTermWeight(ConstPrio,0,0,2.0,10,3,20,1,0,1.0,1.0,1.0)",
        ),
        (
            "prefix",
            "ConjectureTermPrefixWeight(ConstPrio,0,0,0.5,5.0,0,1.0,1.0,1.0)",
        ),
        (
            "tfidf",
            "ConjectureTermTfIdfWeight(ConstPrio,0,0,0,1.0,0,1.0,1.0,1.0)",
        ),
        (
            "levenshtein",
            "ConjectureLevDistanceWeight(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0)",
        ),
        (
            "structural",
            "ConjectureStrucDistanceWeight(ConstPrio,0,0,5.0,10.0,2.0,3.0,0,1.0,1.0,1.0)",
        ),
        (
            "tree",
            "ConjectureTreeDistanceWeight(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0)",
        ),
    )

    comparisons: list[dict[str, object]] = []
    for name, definition in cases:
        common_args = [f"--expert-heuristic=(1*{definition})"]
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
