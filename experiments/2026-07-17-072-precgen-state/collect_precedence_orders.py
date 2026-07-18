#!/usr/bin/env python3
"""Collect generated user-symbol precedence orders from instrumented C."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


PRECEDENCE_LINE = re.compile(
    r"^% Ordering precedence: (?P<order>.*)$", re.MULTILINE
)


def wsl_path(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def parse_order(stdout: bytes) -> list[str]:
    text = stdout.decode("utf-8", errors="strict")
    match = PRECEDENCE_LINE.search(text)
    if match is None:
        raise ValueError(f"instrumented precedence line missing: {text!r}")
    return [symbol.strip() for symbol in match.group("order").split(">")]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    ordinary = experiment_dir / "problem.p"
    array_problem = experiment_dir / "array_problem.lop"
    typed = experiment_dir.parent / "2026-07-17-071-weightgen-state" / "typed_problem.p"
    common = ["--output-level=0", "--term-ordering=KBO6"]
    cases: list[tuple[str, Path, list[str]]] = []
    for method in (
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
    ):
        cases.append((method, ordinary, [f"--order-precedence-generation={method}"]))
    for method in ("typefreq", "invtypefreq", "combfreq", "invcombfreq"):
        cases.append((method, typed, [f"--order-precedence-generation={method}"]))
    cases.append(
        (
            "arrayopt",
            array_problem,
            ["--lop-in", "--order-precedence-generation=arrayopt"],
        )
    )

    records: list[dict[str, object]] = []
    for name, problem, extra in cases:
        result = subprocess.run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *common,
                *extra,
                wsl_path(problem),
            ],
            capture_output=True,
            check=False,
        )
        high_to_low = parse_order(result.stdout)
        records.append(
            {
                "case": name,
                "exit_code": result.returncode,
                "high_to_low": high_to_low,
                "low_to_high": list(reversed(high_to_low)),
            }
        )

    output = {
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "case_count": len(records),
        "cases": records,
    }
    rendered = json.dumps(output, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if output != expected:
            raise SystemExit("instrumented C precedence differs from snapshot")


if __name__ == "__main__":
    main()
