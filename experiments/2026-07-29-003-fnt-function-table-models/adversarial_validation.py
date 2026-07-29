#!/usr/bin/env python3
"""Verify rejection of six single-change typed-model corruptions."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def replace_once(text: str, old: str, new: str) -> str:
    if text.count(old) != 1:
        raise ValueError(f"expected exactly one occurrence of {old!r}")
    return text.replace(old, new, 1)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unary-problem", type=Path, required=True)
    parser.add_argument("--unary-solution", type=Path, required=True)
    parser.add_argument("--nested-problem", type=Path, required=True)
    parser.add_argument("--nested-solution", type=Path, required=True)
    parser.add_argument("--native-problem", type=Path, required=True)
    parser.add_argument("--native-solution", type=Path, required=True)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    unary = args.unary_solution.read_text(encoding="utf-8")
    nested = args.nested_solution.read_text(encoding="utf-8")
    native = args.native_solution.read_text(encoding="utf-8")
    cases = {
        "function": (
            args.unary_problem,
            replace_once(
                unary,
                "f(umlaut_fmb_d_s__i_0) = umlaut_fmb_d_s__i_1",
                "f(umlaut_fmb_d_s__i_0) = umlaut_fmb_d_s__i_0",
            ),
        ),
        "predicate": (
            args.native_problem,
            replace_once(
                native,
                "likes(umlaut_fmb_d_person_0,umlaut_fmb_d_color_0)",
                "~(likes(umlaut_fmb_d_person_0,umlaut_fmb_d_color_0))",
            ),
        ),
        "constant": (
            args.nested_problem,
            replace_once(
                nested,
                "a = umlaut_fmb_d_s__i_1",
                "a = umlaut_fmb_d_s__i_0",
            ),
        ),
        "native_domain": (
            args.native_problem,
            replace_once(
                native,
                "finite_domain_person",
                "corrupt_domain_person",
            ),
        ),
        "status": (
            args.unary_problem,
            replace_once(
                unary,
                "% SZS status Satisfiable",
                "% SZS status Theorem",
            ),
        ),
        "type": (
            args.native_problem,
            replace_once(
                native,
                "umlaut_fmb_d_person_0:person",
                "umlaut_fmb_d_person_0:color",
            ),
        ),
    }

    validator_script = Path(__file__).resolve().with_name("validate_model.py")
    results: list[dict[str, object]] = []
    for name, (problem, artifact) in cases.items():
        solution = args.output / f"{name}.s"
        report = args.output / f"{name}.validation.json"
        solution.write_text(artifact, encoding="utf-8", newline="\n")
        completed = subprocess.run(
            [
                sys.executable,
                str(validator_script),
                str(problem.resolve()),
                str(solution.resolve()),
                "--validator",
                str(args.validator.resolve()),
                "--vampire",
                str(args.vampire.resolve()),
                "--report",
                str(report.resolve()),
            ],
            check=False,
        )
        verdict = (
            json.loads(report.read_text(encoding="utf-8")).get("verdict")
            if report.exists()
            else None
        )
        results.append(
            {
                "case": name,
                "returncode": completed.returncode,
                "verdict": verdict,
                "rejected": completed.returncode != 0 and verdict != "verified",
            }
        )

    summary = args.output / "summary.json"
    summary.write_text(
        json.dumps(results, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(summary.read_text(encoding="utf-8"), end="")
    return 0 if all(record["rejected"] for record in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
