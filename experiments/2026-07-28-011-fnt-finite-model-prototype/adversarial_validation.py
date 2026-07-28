#!/usr/bin/env python3
"""Confirm that semantic validation rejects four model corruptions."""

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


def validate(
    problem: Path,
    solution: Path,
    report: Path,
    validator: Path,
    vampire: Path,
) -> tuple[int, str | None]:
    completed = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve().with_name("validate_model.py")),
            str(problem),
            str(solution),
            "--validator",
            str(validator),
            "--vampire",
            str(vampire),
            "--report",
            str(report),
        ],
        check=False,
    )
    verdict = None
    if report.exists():
        verdict = json.loads(report.read_text(encoding="utf-8")).get("verdict")
    return completed.returncode, verdict


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--counter-problem", type=Path, required=True)
    parser.add_argument("--counter-solution", type=Path, required=True)
    parser.add_argument("--constant-problem", type=Path, required=True)
    parser.add_argument("--constant-solution", type=Path, required=True)
    parser.add_argument("--two-problem", type=Path, required=True)
    parser.add_argument("--two-solution", type=Path, required=True)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    counter = args.counter_solution.read_text(encoding="utf-8")
    constant = args.constant_solution.read_text(encoding="utf-8")
    two = args.two_solution.read_text(encoding="utf-8")
    cases = {
        "predicate": (
            args.counter_problem,
            replace_once(
                counter,
                "~q(umlaut_fmb_d0_0)",
                "q(umlaut_fmb_d0_0)",
            ),
        ),
        "constant": (
            args.constant_problem,
            replace_once(
                constant,
                "b = umlaut_fmb_d0_1",
                "b = umlaut_fmb_d0_0",
            ),
        ),
        "domain": (
            args.two_problem,
            replace_once(
                two,
                "X = umlaut_fmb_d0_0 | X = umlaut_fmb_d0_1 | "
                "X = umlaut_fmb_d1_0",
                "X = umlaut_fmb_d0_0 | X = umlaut_fmb_d1_0",
            ),
        ),
        "status": (
            args.counter_problem,
            replace_once(
                counter,
                "% SZS status CounterSatisfiable",
                "% SZS status Theorem",
            ),
        ),
    }

    results: list[dict[str, object]] = []
    for name, (problem, text) in cases.items():
        solution = args.output / f"{name}.s"
        report = args.output / f"{name}.validation.json"
        solution.write_text(text, encoding="utf-8", newline="\n")
        returncode, verdict = validate(
            problem.resolve(),
            solution.resolve(),
            report.resolve(),
            args.validator.resolve(),
            args.vampire.resolve(),
        )
        rejected = returncode != 0 and verdict != "verified"
        results.append(
            {
                "case": name,
                "returncode": returncode,
                "verdict": verdict,
                "rejected": rejected,
            }
        )

    summary = args.output / "summary.json"
    summary.write_text(
        json.dumps(results, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(summary.read_text(encoding="utf-8"), end="")
    return 0 if all(result["rejected"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
