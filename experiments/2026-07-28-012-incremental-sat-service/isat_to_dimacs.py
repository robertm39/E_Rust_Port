#!/usr/bin/env python3
"""Materialize one global, assumption-free ISAT query as DIMACS."""

from __future__ import annotations

import argparse
from pathlib import Path


def convert(session: Path, query_id: str) -> str:
    maximum: int | None = None
    clauses: list[tuple[int, ...]] = []
    found = False
    for line_number, line in enumerate(
        session.read_text(encoding="utf-8").splitlines(), 1
    ):
        if not line or line.startswith("c"):
            continue
        fields = line.split()
        if fields[0] == "p":
            if len(fields) != 3 or fields[1] != "isat":
                raise ValueError(f"{session}:{line_number}: invalid header")
            maximum = int(fields[2])
        elif fields[0] == "a":
            if fields[-1] != "0":
                raise ValueError(f"{session}:{line_number}: unterminated clause")
            clauses.append(tuple(int(field) for field in fields[1:-1]))
        elif fields[0] == "q" and fields[1] == query_id:
            if fields[4:] != ["0"]:
                raise ValueError("proof conversion requires an assumption-free query")
            found = True
            break
    if maximum is None or not found:
        raise ValueError(f"query {query_id!r} not found")
    lines = [f"p cnf {maximum} {len(clauses)}"]
    for clause in clauses:
        literals = " ".join(str(literal) for literal in clause)
        lines.append(f"{literals} 0" if literals else "0")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("session", type=Path)
    parser.add_argument("query")
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    arguments.output.write_text(
        convert(arguments.session, arguments.query), encoding="ascii", newline="\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
