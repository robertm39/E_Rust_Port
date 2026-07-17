#!/usr/bin/env python3
"""Compare stable term-ordering option and proof-search surfaces."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"

WEIGHT_METHODS = (
    "firstmaximal0",
    "arity",
    "aritymax0",
    "modarity",
    "modaritymax0",
    "aritysquared",
    "aritysquaredmax0",
    "invarity",
    "invaritymax0",
    "invaritysquared",
    "invaritysquaredmax0",
    "precedence",
    "invprecedence",
    "precrank5",
    "precrank10",
    "precrank20",
    "freqcount",
    "invfreqcount",
    "freqrank",
    "invfreqrank",
    "invconjfreqrank",
    "freqranksquare",
    "invfreqranksquare",
    "invmodfreqrank",
    "invmodfreqrankmax0",
    "typefreqrank",
    "typefreqcount",
    "invtypefreqrank",
    "invtypefreqcount",
    "combfreqrank",
    "combfreqcount",
    "invcombfreqrank",
    "invcombfreqcount",
    "constant",
)

PRECEDENCE_METHODS = (
    "unary_first",
    "unary_freq",
    "arity",
    "invarity",
    "const_max",
    "const_min",
    "freq",
    "invfreq",
    "invconjfreq",
    "invfreqconjmax",
    "invfreqconjmin",
    "invfreqconstmin",
    "invfreqhack",
    "typefreq",
    "invtypefreq",
    "combfreq",
    "invcombfreq",
    "arrayopt",
    "orient_axioms",
)

HO_WEIGHT_METHODS = frozenset(WEIGHT_METHODS[25:33])
HO_PRECEDENCE_METHODS = frozenset(PRECEDENCE_METHODS[13:17])


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


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


def quote_wsl_shell_metacharacters(argument: str) -> str:
    if any(character in argument for character in "<>|&;()$`"):
        return shlex.quote(argument)
    return argument


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--c-ho-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    problem = experiment_dir / "problem.lop"
    lambda_problem = experiment_dir / "lambda.p"
    path_map = {
        str(problem): wsl_path(problem),
        str(lambda_problem): wsl_path(lambda_problem),
    }

    strategy = ["--print-strategy"]
    proof = ["--output-level=0", "--lop-in"]
    cases: list[tuple[str, list[str]]] = []

    for ordering in ("LPO", "LPOCopy", "LPO4", "LPO4Copy", "KBO", "KBO6"):
        cases.append(
            (f"ordering_{ordering.lower()}", [*proof, f"--term-ordering={ordering}", str(problem)])
        )

    for method in WEIGHT_METHODS:
        cases.append(
            (
                f"weight_{method}",
                [*strategy, "--term-ordering=KBO6", f"--order-weight-generation={method}"],
            )
        )

    for method in PRECEDENCE_METHODS:
        cases.append(
            (
                f"precedence_{method}",
                [*strategy, "--term-ordering=KBO6", f"--order-precedence-generation={method}"],
            )
        )

    cases.extend(
        [
            (
                "combined_ordering_controls",
                [
                    *proof,
                    "--term-ordering=KBO6",
                    "--order-weight-generation=arity",
                    "--order-weights=f:2,g:3",
                    "--order-precedence-generation=invfreq",
                    "--prec-pure-conj=10",
                    "--prec-conj-axiom=6",
                    "--prec-pure-axiom=2",
                    "--prec-skolem=4",
                    "--prec-defpred=2",
                    "--order-constant-weight=3",
                    "--precedence=f>g",
                    "--lpo-recursion-limit=25",
                    "--literal-comparison=TFOEqMin",
                    "--kbo-lam-weight=30",
                    "--kbo-db-weight=12",
                    str(problem),
                ],
            ),
            (
                "optional_ordering_defaults",
                [
                    *strategy,
                    "--prec-pure-conj",
                    "--prec-conj-axiom",
                    "--prec-pure-axiom",
                    "--prec-skolem",
                    "--prec-defpred",
                    "--precedence",
                    "--lpo-recursion-limit",
                ],
            ),
            (
                "restrict_literal_comparisons",
                [*strategy, "--restrict-literal-comparisons"],
            ),
            (
                "lambda_order_weights",
                [
                    "--output-level=0",
                    "--term-ordering=KBO6",
                    "--ho-order-kind=lambda",
                    "--kbo-lam-weight=30",
                    "--kbo-db-weight=12",
                    str(lambda_problem),
                ],
            ),
            ("invalid_ordering_auto", ["--term-ordering=Auto"]),
            ("invalid_ordering_rpo", ["--term-ordering=RPO"]),
            ("invalid_weight_none", ["--order-weight-generation=none"]),
            ("invalid_weight_name", ["--order-weight-generation=missing"]),
            ("invalid_precedence_none", ["--order-precedence-generation=none"]),
            ("invalid_precedence_name", ["--order-precedence-generation=missing"]),
            ("invalid_constant_weight", ["--order-constant-weight=0"]),
            ("invalid_lpo_limit", ["--lpo-recursion-limit=0"]),
            ("invalid_literal_comparison", ["--literal-comparison=missing"]),
            (
                "large_lpo_warning_before_error",
                ["--lpo-recursion-limit=20001", "--literal-comparison=missing"],
            ),
        ]
    )

    def to_c_arg(argument: str) -> str:
        return path_map.get(argument, argument)

    def c_oracle(case_name: str) -> tuple[str, str]:
        if case_name.removeprefix("weight_") in HO_WEIGHT_METHODS:
            return ("ho", args.c_ho_exe)
        if case_name.removeprefix("precedence_") in HO_PRECEDENCE_METHODS:
            return ("ho", args.c_ho_exe)
        if case_name in {
            "lambda_order_weights",
            "invalid_weight_none",
            "invalid_weight_name",
            "invalid_precedence_none",
            "invalid_precedence_name",
        }:
            return ("ho", args.c_ho_exe)
        return ("fol", args.c_exe)

    results: list[dict[str, object]] = []
    for case_name, rust_args in cases:
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c_args = [to_c_arg(argument) for argument in rust_args]
        c_variant, c_exe = c_oracle(case_name)
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                c_exe,
                *(quote_wsl_shell_metacharacters(argument) for argument in c_args),
            ]
        )
        exact_match = rust == c
        result: dict[str, object] = {
            "case": case_name,
            "c_variant": c_variant,
            "exact_match": exact_match,
            "rust": summarize(rust),
            "c": summarize(c),
        }
        if not exact_match:
            result["mismatch"] = {"rust": readable(rust), "c": readable(c)}
        results.append(result)

    rendered = json.dumps(
        {
            "reference_commit": REFERENCE_COMMIT,
            "case_count": len(results),
            "exact_count": sum(bool(result["exact_match"]) for result in results),
            "results": results,
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
