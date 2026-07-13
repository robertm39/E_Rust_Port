#!/usr/bin/env python3
"""Compare C TSTP and Rust LOP selected-clause traces."""

from __future__ import annotations

import re
import sys
from pathlib import Path


C_CLAUSE = re.compile(r"^%cnf\([^,]+,\s*[^,]+,\s*\((.*)\)\)\.$")
VARIABLE = re.compile(r"X\d+")


def split_top_level(text: str, separator: str) -> list[str]:
    fields: list[str] = []
    depth = 0
    start = 0
    for index, character in enumerate(text):
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
        elif character == separator and depth == 0:
            fields.append(text[start:index])
            start = index + 1
    fields.append(text[start:])
    return fields


def normalize_literal(literal: str, negate: bool = False) -> str:
    literal = literal.strip().replace(" ", "")
    negative = literal.startswith("~")
    if negative:
        literal = literal[1:]
    if "!=" in literal:
        negative = not negative
        literal = literal.replace("!=", "=", 1)
    if negate:
        negative = not negative
    return f"~{literal}" if negative else literal


def canonical_variables(clause: list[str]) -> tuple[str, ...]:
    names: dict[str, str] = {}

    def replace(match: re.Match[str]) -> str:
        source = match.group(0)
        return names.setdefault(source, f"X{len(names) + 1}")

    return tuple(VARIABLE.sub(replace, literal) for literal in clause)


def normalize_c_clause(body: str) -> tuple[str, ...]:
    return canonical_variables(
        [normalize_literal(literal) for literal in split_top_level(body, "|")]
    )


def normalize_lop_clause(line: str) -> tuple[str, ...] | None:
    if line.startswith("%?-"):
        body = line.removeprefix("%?-").strip().removesuffix(".")
        return canonical_variables(
            [normalize_literal(literal, negate=True) for literal in split_top_level(body, ",")]
        )
    if not line.startswith("%") or line.startswith("%%") or not line.endswith("."):
        return None

    clause = line[1:].removesuffix(".")
    if "<-" not in clause:
        return None
    head, body = clause.split("<-", maxsplit=1)
    literals = [
        normalize_literal(literal)
        for literal in split_top_level(head, ";")
        if literal.strip()
    ]
    literals.extend(
        normalize_literal(literal, negate=True)
        for literal in split_top_level(body, ",")
        if literal.strip()
    )
    return canonical_variables(literals)


def selected_clauses(path: Path) -> list[tuple[str, ...]]:
    clauses: list[tuple[str, ...]] = []
    after_presaturation = False
    for raw_line in path.read_text(encoding="utf-8-sig").splitlines():
        line = raw_line.strip()
        if line == "% Presaturation interreduction done":
            after_presaturation = True
            continue
        if not after_presaturation:
            continue

        c_match = C_CLAUSE.fullmatch(line)
        normalized = (
            normalize_c_clause(c_match.group(1))
            if c_match is not None
            else normalize_lop_clause(line)
        )
        if normalized is not None:
            clauses.append(normalized)
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
            print(f"Rust: {' | '.join(rust_clause)}")
            print(f"C:    {' | '.join(c_clause)}")
            print("context:")
            for context_index in range(max(0, index - 4), min(len(rust), len(c), index + 2)):
                marker = ">" if context_index == index - 1 else " "
                print(
                    f"{marker} {context_index + 1} Rust: "
                    f"{' | '.join(rust[context_index])}"
                )
                print(
                    f"{marker} {context_index + 1} C:    "
                    f"{' | '.join(c[context_index])}"
                )
            return 1

    print(f"common selected prefix: {min(len(rust), len(c))}")
    print(f"Rust selected clauses:  {len(rust)}")
    print(f"C selected clauses:     {len(c)}")
    return int(len(rust) != len(c))


if __name__ == "__main__":
    raise SystemExit(main())
