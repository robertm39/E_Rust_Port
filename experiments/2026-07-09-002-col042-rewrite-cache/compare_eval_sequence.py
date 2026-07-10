#!/usr/bin/env python3
"""Compare C and Rust OutputLevel-6 evaluation-allocation sequences."""

from __future__ import annotations

import re
import sys
from pathlib import Path


EVAL_CLAUSE = re.compile(
    r"^cnf\([^,]+,\s*[^,]+,\s*\((.*)\),\s*[^,]+,\['eval'\]\)\.$"
)
VARIABLE = re.compile(r"X\d+")


def canonical_variables(clause: str) -> str:
    names: dict[str, str] = {}

    def replace(match: re.Match[str]) -> str:
        source = match.group(0)
        return names.setdefault(source, f"X{len(names) + 1}")

    return VARIABLE.sub(replace, clause)


def evaluated_clauses(path: Path) -> list[str]:
    clauses: list[str] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        match = EVAL_CLAUSE.fullmatch(raw_line.strip())
        if match is not None:
            clauses.append(canonical_variables(match.group(1).replace(" ", "")))
    return clauses


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare_eval_sequence.py RUST_TRACE C_TRACE", file=sys.stderr)
        return 2

    rust = evaluated_clauses(Path(sys.argv[1]))
    c = evaluated_clauses(Path(sys.argv[2]))
    mismatch = next(
        (index for index, pair in enumerate(zip(rust, c)) if pair[0] != pair[1]),
        None,
    )
    if mismatch is None:
        print(f"common evaluation prefix: {min(len(rust), len(c))}")
    else:
        print(f"first mismatch: evaluation {mismatch + 1}")
        for index in range(max(0, mismatch - 2), min(mismatch + 3, len(rust), len(c))):
            marker = ">" if index == mismatch else " "
            print(f"{marker} {index + 1}: Rust {rust[index]}")
            print(f"{marker} {index + 1}: C    {c[index]}")

    print(f"Rust evaluations: {len(rust)}")
    print(f"C evaluations:    {len(c)}")
    return int(mismatch is not None or len(rust) != len(c))


if __name__ == "__main__":
    raise SystemExit(main())
