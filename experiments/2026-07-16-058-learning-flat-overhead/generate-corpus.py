#!/usr/bin/env python3
"""Generate distinct annotated terms over a fixed four-symbol vocabulary."""

from __future__ import annotations

import argparse
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--terms-per-section", type=int, default=10_000)
    args = parser.parse_args()
    if args.terms_per_section < 1:
        parser.error("--terms-per-section must be positive")

    lines = ["Training:"]
    lines.extend(annotation(index) for index in range(args.terms_per_section))
    lines.extend((".", "Test:"))
    lines.extend(
        annotation(index)
        for index in range(args.terms_per_section, args.terms_per_section * 2)
    )
    lines.append(".")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")


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
