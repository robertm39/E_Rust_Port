#!/usr/bin/env python3
"""Compare C and Rust OutputLevel-1 selected-clause traces."""

from __future__ import annotations

import re
import sys
from pathlib import Path


C_CLAUSE = re.compile(r"^%cnf\([^,]+,\s*[^,]+,\s*\((.*)\)\)\.$")
VARIABLE = re.compile(r"X\d+")


def canonical_variables(clause: str) -> str:
    names: dict[str, str] = {}

    def replace(match: re.Match[str]) -> str:
        source = match.group(0)
        return names.setdefault(source, f"X{len(names) + 1}")

    return VARIABLE.sub(replace, clause)


def selected_clauses(path: Path) -> list[str]:
    clauses: list[str] = []
    after_presaturation = False
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if line == "% Presaturation interreduction done":
            after_presaturation = True
            continue
        if not after_presaturation:
            continue

        c_match = C_CLAUSE.fullmatch(line)
        if c_match is not None:
            clause = c_match.group(1)
            if "!=" in clause:
                clause = clause.replace("!=", "=", 1) + " [negative]"
        elif line.startswith("%?-"):
            clause = line.removeprefix("%?- ").removesuffix(".") + " [negative]"
        elif line.startswith("%") and not line.startswith("%%") and line.endswith(" <- ."):
            clause = line[1:].removesuffix(" <- .")
        else:
            continue
        clauses.append(canonical_variables(clause.replace(" ", "")))
    return clauses


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare_selected.py RUST_TRACE C_TRACE", file=sys.stderr)
        return 2

    rust = selected_clauses(Path(sys.argv[1]))
    c = selected_clauses(Path(sys.argv[2]))
    for index, (rust_clause, c_clause) in enumerate(zip(rust, c), start=1):
        if rust_clause != c_clause:
            print(f"first mismatch: selected clause {index}")
            print(f"Rust: {rust_clause}")
            print(f"C:    {c_clause}")
            return 1

    print(f"common selected prefix: {min(len(rust), len(c))}")
    print(f"Rust selected clauses:  {len(rust)}")
    print(f"C selected clauses:     {len(c)}")
    if len(rust) != len(c):
        extra_side = "Rust" if len(rust) > len(c) else "C"
        extra = rust[len(c) :] if len(rust) > len(c) else c[len(rust) :]
        print(f"{extra_side} extra clauses:")
        for clause in extra:
            print(clause)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
