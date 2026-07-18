#!/usr/bin/env python3
"""Audit classic KBO integration and compare FOL/HO executable paths."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
from pathlib import Path


def function_body(source: str, name: str) -> str:
    pattern = re.compile(rf"(?:\bfn|\bbool|\bCompareResult)\s+{name}\s*\(")
    search_from = 0
    while True:
        match = pattern.search(source, search_from)
        if match is None:
            raise ValueError(f"function definition not found: {name}")
        opening = source.find("{", match.end())
        semicolon = source.find(";", match.end())
        if opening >= 0 and (semicolon < 0 or opening < semicolon):
            break
        search_from = match.end()
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
    c_kbo = (root / "eprover/ORDERINGS/cto_kbo.c").read_text(encoding="utf-8")
    c_orderings = (root / "eprover/ORDERINGS/cto_orderings.c").read_text(
        encoding="utf-8"
    )
    c_autoselect = (
        root / "eprover/HEURISTICS/che_to_autoselect.c"
    ).read_text(encoding="utf-8")
    rust_kbo = (root / "src/orderings/cto_kbo.rs").read_text(encoding="utf-8")
    rust_orderings = (root / "src/orderings/cto_orderings.rs").read_text(
        encoding="utf-8"
    )
    rust_autoselect = (
        root / "src/heuristics/to_autoselect.rs"
    ).read_text(encoding="utf-8")
    rust_proofcontrol = (root / "src/heuristics/proofcontrol.rs").read_text(
        encoding="utf-8"
    )

    c_compare = function_body(c_kbo, "KBOCompare")
    c_greater = function_body(c_kbo, "kbogtrnew")
    rust_compare = function_body(rust_kbo, "kbo_compare")
    rust_greater = function_body(rust_kbo, "kbo_greater_new")
    rust_dispatch = function_body(rust_orderings, "to_compare")
    rust_bank_dispatch = function_body(rust_orderings, "to_compare_with_bank")
    deref_regression = function_body(
        rust_kbo, "deref_once_reaches_bound_terms_before_classic_comparison"
    )
    proofcontrol_regression = function_body(
        rust_proofcontrol,
        "proof_control_init_preserves_explicit_classic_kbo_for_higher_order_problem",
    )

    checks = {
        "c_compare_deref_weight_then_variable_condition": (
            ordered(c_compare, ("TermDeref", "gettermweight", "KBOVarCompare"))
            and "assert(problemType != PROBLEM_HO)" in c_compare
        ),
        "rust_compare_deref_weight_then_variable_condition": ordered(
            rust_compare, ("term_deref", "get_term_weight", "kbo_var_compare")
        ),
        "c_greater_delays_variable_condition": ordered(
            c_greater, ("TermDeref", "gettermweight", "KBOVarGreater")
        ),
        "rust_greater_delays_variable_condition": ordered(
            rust_greater, ("term_deref", "get_term_weight", "kbo_var_greater")
        ),
        "classic_kbo_dispatch_and_bank_policy": (
            "TermOrdering::Kbo => kbo_compare" in rust_dispatch
            and "TermOrdering::Kbo6 => kbo6_compare_with_bank" in rust_bank_dispatch
            and "TermOrdering::Lpo4 => lpo4_compare_with_bank" in rust_bank_dispatch
            and "_ => Ok(to_compare" in rust_bank_dispatch
        ),
        "rpo_unimplemented_matches_c": (
            c_orderings.count('RPO not yet implemented!') == 2
            and 'RPO not yet implemented!' in c_autoselect
            and "TermOrdering::Rpo => panic!" in rust_dispatch
            and 'TermOrdering::Rpo => panic!("RPO not yet implemented!")'
            in rust_autoselect
        ),
        "deref_once_regression_present": (
            deref_regression.count("DerefType::Once") >= 3
            and "CompareResult::Greater" in deref_regression
            and "CompareResult::Lesser" in deref_regression
            and "kbo_greater(" in deref_regression
        ),
        "proof_control_preserves_explicit_classic_kbo": (
            "params.order_params.ordertype = TermOrdering::Kbo"
            in proofcontrol_regression
            and "assert_eq!(ocb.ordering_type, TermOrdering::Kbo)"
            in proofcontrol_regression
            and "true," in proofcontrol_regression
        ),
    }
    expected = {
        "c_compare_deref_weight_then_variable_condition": True,
        "rust_compare_deref_weight_then_variable_condition": True,
        "c_greater_delays_variable_condition": True,
        "rust_greater_delays_variable_condition": True,
        "classic_kbo_dispatch_and_bank_policy": True,
        "rpo_unimplemented_matches_c": True,
        "deref_once_regression_present": True,
        "proof_control_preserves_explicit_classic_kbo": True,
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
            "fol_classic_kbo",
            args.c_exe,
            [
                "--output-level=0",
                "--lop-in",
                "--term-ordering=KBO",
                str(experiment_dir / "problem.lop"),
            ],
        ),
        (
            "thf_classic_kbo_release_surface",
            args.c_ho_exe,
            [
                "--output-level=0",
                "--term-ordering=KBO",
                str(experiment_dir / "higher_order.p"),
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
