#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


C_QUERY = re.compile(
    r"query=(\d+)(?: raw_query=\d+)? ident=(\S+) matcher_calls=(\d+)(?: result=\S+)? clause=(.*?) vector=([\d,-]+)"
)
RUST_QUERY = re.compile(
    r"TRACE_FVI_QUERY query=(\d+) ident=(\S+) matcher_calls=(\d+)(?: result=\S+)? clause=(.*?) vector=\[([\d, ]+)\]"
)


@dataclass(frozen=True)
class Query:
    ordinal: int
    ident: str
    matcher_calls: int
    clause: str
    vector: tuple[int, ...]


def read_queries(path: Path, pattern: re.Pattern[str]) -> list[Query]:
    result = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = pattern.search(line)
        if match is None:
            continue
        vector = tuple(int(value) for value in match.group(5).split(",") if value.strip())
        result.append(
            Query(
                ordinal=int(match.group(1)),
                ident=match.group(2),
                matcher_calls=int(match.group(3)),
                clause=" ".join(match.group(4).split()),
                vector=vector,
            )
        )
    return result


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-fvi-queries.py C_TRACE RUST_TRACE", file=sys.stderr)
        return 2

    c_queries = read_queries(Path(sys.argv[1]), C_QUERY)
    rust_queries = read_queries(Path(sys.argv[2]), RUST_QUERY)
    c_cursor = 0
    aligned = 0
    for rust in rust_queries:
        while c_cursor < len(c_queries):
            candidate = c_queries[c_cursor]
            c_cursor += 1
            if (candidate.ident, candidate.clause) == (rust.ident, rust.clause):
                break
        else:
            print(
                f"first unaligned Rust query: {rust.ordinal} ident={rust.ident} clause={rust.clause}"
            )
            print(f"aligned prefix: {aligned}")
            return 1

        aligned += 1
        if candidate.vector != rust.vector:
            print(f"first vector mismatch: C query {candidate.ordinal}, Rust query {rust.ordinal}")
            print(f"C:    {candidate.vector}")
            print(f"Rust: {rust.vector}")
            return 1
        if candidate.matcher_calls != rust.matcher_calls:
            print(f"first lookup mismatch: C query {candidate.ordinal}, Rust query {rust.ordinal}")
            print(f"C:    matcher_calls={candidate.matcher_calls}")
            print(f"Rust: matcher_calls={rust.matcher_calls}")
            return 1

    print(f"aligned Rust query prefix: {aligned}")
    print(f"C queries consumed: {c_cursor}/{len(c_queries)}")
    print(f"Rust queries: {len(rust_queries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
