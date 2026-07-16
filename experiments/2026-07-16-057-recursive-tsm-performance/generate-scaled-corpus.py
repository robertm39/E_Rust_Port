#!/usr/bin/env python3
"""Scale the recursive-mixed shape without growing the symbol vocabulary."""

from __future__ import annotations

import argparse
from pathlib import Path


def section(lines: list[str], heading: str, next_heading: str | None) -> list[str]:
    start = lines.index(f"{heading}:") + 1
    if next_heading is None:
        end = len(lines) - 1
    else:
        end = lines.index(f"{next_heading}:") - 1
    return lines[start:end]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--repeats", type=int)
    mode.add_argument("--distinct-terms", type=int)
    args = parser.parse_args()
    scale = args.repeats if args.repeats is not None else args.distinct_terms
    if scale is None or scale < 1:
        parser.error("the selected scale must be positive")

    if args.repeats is not None:
        lines = args.source.read_text(encoding="utf-8").splitlines()
        training = section(lines, "Training", "Test") * args.repeats
        test = section(lines, "Test", None) * args.repeats
    else:
        training = [annotation(index) for index in range(args.distinct_terms)]
        test = [
            annotation(index)
            for index in range(args.distinct_terms, args.distinct_terms * 2)
        ]

    output = ["Training:"]
    output.extend(training)
    output.extend((".", "Test:"))
    output.extend(test)
    output.append(".")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(output) + "\n", encoding="utf-8")


def annotation(index: int) -> str:
    term = encoded_term(index + 1)
    if index % 2 == 0:
        return f"{term} : 1:(1,-1)."
    return f"{term} : 2:(1,1)."


def encoded_term(value: int) -> str:
    term = "a"
    for bit in f"{value:b}":
        if bit == "0":
            term = f"f({term})"
        else:
            term = f"g(b,{term})"
    return term


if __name__ == "__main__":
    main()
