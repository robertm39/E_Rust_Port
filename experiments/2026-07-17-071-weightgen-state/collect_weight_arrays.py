#!/usr/bin/env python3
"""Collect actual OCB function weights from an instrumented C reference."""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
from pathlib import Path


WEIGHT_LINE = re.compile(r"^% Ordering weights: (?P<weights>.*)$", re.MULTILINE)


def wsl_path(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def quote_wsl(argument: str) -> str:
    if any(character in argument for character in "<>|&;()$`"):
        return shlex.quote(argument)
    return argument


def parse_weights(stdout: bytes) -> dict[str, int]:
    text = stdout.decode("utf-8", errors="strict")
    match = WEIGHT_LINE.search(text)
    if match is None:
        raise ValueError(f"instrumented weight line missing from output: {text!r}")
    weights: dict[str, int] = {}
    for assignment in match.group("weights").split():
        name, value = assignment.rsplit(":", maxsplit=1)
        weights[name] = int(value)
    return weights


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    fol_problem = experiment_dir.parent / "2026-07-17-070-classic-kbo-integration" / "problem.lop"
    typed_problem = experiment_dir / "typed_problem.p"
    common = ["--output-level=0", "--term-ordering=KBO6"]
    cases: list[tuple[str, Path, list[str]]] = [
        (
            "late_user_override",
            fol_problem,
            [
                "--lop-in",
                "--order-weight-generation=arity",
                "--order-constant-weight=3",
                "--order-weights=a:9",
            ],
        ),
        (
            "partial_precedence",
            fol_problem,
            ["--lop-in", "--order-weight-generation=precedence", "--precedence=f>g"],
        ),
        (
            "partial_inverse_precedence",
            fol_problem,
            ["--lop-in", "--order-weight-generation=invprecedence", "--precedence=f>g"],
        ),
        (
            "partial_precrank5",
            fol_problem,
            ["--lop-in", "--order-weight-generation=precrank5", "--precedence=f>g"],
        ),
        (
            "inverse_conjecture_frequency_rank",
            fol_problem,
            ["--lop-in", "--order-weight-generation=invconjfreqrank"],
        ),
        (
            "frequency_rank_square",
            fol_problem,
            ["--lop-in", "--order-weight-generation=freqranksquare"],
        ),
        (
            "inverse_modified_frequency_rank",
            fol_problem,
            ["--lop-in", "--order-weight-generation=invmodfreqrank"],
        ),
    ]
    for method in (
        "typefreqrank",
        "typefreqcount",
        "invtypefreqrank",
        "invtypefreqcount",
        "combfreqrank",
        "combfreqcount",
        "invcombfreqrank",
        "invcombfreqcount",
    ):
        cases.append(
            (
                method,
                typed_problem,
                [f"--order-weight-generation={method}"],
            )
        )

    records: list[dict[str, object]] = []
    for name, problem, extra in cases:
        arguments = [*common, *extra, wsl_path(problem)]
        result = subprocess.run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(quote_wsl(argument) for argument in arguments),
            ],
            capture_output=True,
            check=False,
        )
        records.append(
            {
                "case": name,
                "exit_code": result.returncode,
                "weights": parse_weights(result.stdout),
            }
        )

    result = {
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "case_count": len(records),
        "cases": records,
    }
    rendered = json.dumps(result, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if result != expected:
            raise SystemExit("instrumented C weights differ from retained snapshot")


if __name__ == "__main__":
    main()
