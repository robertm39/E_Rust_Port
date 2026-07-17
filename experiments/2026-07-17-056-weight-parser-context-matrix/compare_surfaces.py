#!/usr/bin/env python3
"""Compare C/Rust executable weight-function definition parsing."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"

SPECS = (
    "Clauseweight(ConstPrio,2,1,3.0)",
    "ClauseLMaxWeight(ConstPrio,2,1,3.0)",
    "ClauseCMaxWeight(ConstPrio,2,1,3.0)",
    "Uniqweight(ConstPrio)",
    "Defaultweight(ConstPrio)",
    "DAGweight(ConstPrio,2,1,3.0,1,true,false,false,true,false,false,false)",
    "RDAGweight(ConstPrio,10,3,1,5.0,2.0,7.0,4.0)",
    "RDAGweight2(ConstPrio,10,3,1,4.0,2.0)",
    "RDAGweight3(ConstPrio,2,1,13,17,1,3.0,5.0,7.0,11.0)",
    "Refinedweight(ConstPrio,2,1,7.0,5.0,3.0)",
    "Refinedweight2(ConstPrio,2,1,7.0,5.0,3.0)",
    "Diversityweight(ConstPrio,2,3,1.0,1.0,1.0,10.0,1.0,20.0,2.0)",
    "PNRefinedweight(ConstPrio,2,1,13,17,1.0,1.0,1.0)",
    "TPTPTypeweight(ConstPrio,2,1,1.0,1.0,1.0,7.0,5.0)",
    "Sigweight(ConstPrio,2,1,1.0,1.0,1.0,3.0)",
    "NLweight(ConstPrio,2,7,1,1.0,1.0,1.0)",
    "RandomWeight(ConstPrio,0,10.0,2.0)",
    "SymbolTypeweight(ConstPrio,2,1,3,11,1.0,1.0,1.0)",
    "Depthweight(ConstPrio,2,1,3.0,1.0,7.0,11.0)",
    "WLessDWeight(ConstPrio,2,1,3.0,1.0,7.0,0.5)",
    "Proofweight(ConstPrio,2,1,1.0,1.0,1.0,8.0,6.0)",
    "Orientweight(ConstPrio,2,1,7.0,5.0,3.0)",
    "OrientLMaxWeight(ConstPrio,2,1,7.0,5.0,3.0)",
    "Simweight(ConstPrio,100.0,3.0,5.0,7.0)",
    "FIFOWeight(ConstPrio)",
    "LIFOWeight(ConstPrio)",
    "StaggeredWeight(ConstPrio,1.0)",
    "ClauseWeightAge(ConstPrio,2,1,1.0,4.0)",
    "TSMWeight(ConstPrio,2,3,0.5,rec,kb,1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0)",
    "TSMRWeight(ConstPrio,2,3,4.0,5.0,6.0,0.5,rec,kb,1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0)",
    "ConjectureSymbolWeight(ConstPrio,10,99,1,88,1,1.0,1.0,1.0)",
    "ConjectureGeneralSymbolWeight(ConstPrio,10,3,99,1,2,88,1,1.0,1.0,1.0)",
    "ConjectureRelativeSymbolWeight(ConstPrio,0.5,10,4,99,1,1.0,1.0,1.0)",
    "ConjectureRelativeTypeSymbolWeight(ConstPrio,0.5,10,4,99,1,1.0,1.0,1.0)",
    "ConjectureTypeBasedWeight(ConstPrio,1,1.0,1.0,1.0)",
    "RelevanceLevelWeight(ConstPrio,0.0,1.0,0.0,10,2,3,5,7,1.0,1.0,1.0)",
    "RelevanceLevelWeight2(ConstPrio,0.0,1.0,0.0,10,2,3,5,7,1.0,1.0,1.0)",
    "FunWeight(ConstPrio,2,1,1.0,1.0,1.0)",
    "SymOffsetWeight(ConstPrio,2,1,1.0,1.0,1.0)",
    "ConjectureRelativeTermWeight(ConstPrio,0,0,2.0,10,3,20,1,0,1.0,1.0,1.0)",
    "ConjectureTermTfIdfWeight(ConstPrio,0,0,0,1.0,0,1.0,1.0,1.0)",
    "ConjectureLevDistanceWeight(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0)",
    "ConjectureTreeDistanceWeight(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0)",
    "ConjectureTermPrefixWeight(ConstPrio,0,0,0.5,5.0,0,1.0,1.0,1.0)",
    "ConjectureStrucDistanceWeight(ConstPrio,0,0,5.0,10.0,2.0,3.0,0,1.0,1.0,1.0)",
    "GDWeight(ConstPrio,2,1,1.0,0.0,5)",
)


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
    if any(character.isspace() for character in argument) or any(
        character in argument for character in "<>|&;()$`"
    ):
        return shlex.quote(argument)
    return argument


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    problem = Path(__file__).resolve().parent / "problem.lop"
    rust_problem = str(problem)
    c_problem = wsl_path(problem)
    base = ["--output-level=0", "--lop-in"]
    cases: list[tuple[str, str]] = []
    for spec in SPECS:
        name = spec.partition("(")[0]
        cases.append((name, f"audit={spec}"))
    cases.append(("anonymous_definition", "FIFOWeight(ConstPrio)"))

    results: list[dict[str, object]] = []
    for case_name, definition in cases:
        rust_args = [*base, f"--define-weight-function={definition}", rust_problem]
        c_args = [*base, f"--define-weight-function={definition}", c_problem]
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(quote_wsl_shell_metacharacters(argument) for argument in c_args),
            ]
        )
        exact_match = rust == c
        result: dict[str, object] = {
            "case": case_name,
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
