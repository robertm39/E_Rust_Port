#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


PHASE = re.compile(
    r"phase=(\S+)(?: formula=(\d+))? in_count=(\d+) insertions=(\d+) "
    r"recovered=(\d+) live=(\d+)"
)


@dataclass(frozen=True)
class Counters:
    in_count: int
    insertions: int
    recovered: int
    live: int

    def structural(self) -> tuple[int, int, int]:
        return self.in_count, self.recovered, self.live


def read_phases(path: Path) -> tuple[dict[str, Counters], dict[int, Counters]]:
    named = {}
    formulas = {}
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) or b"\x00" in raw else "utf-8"
    for line in raw.decode(encoding, errors="replace").splitlines():
        match = PHASE.search(line)
        if match is None:
            continue
        counters = Counters(*(int(match.group(index)) for index in range(3, 7)))
        if match.group(2) is None:
            named[match.group(1)] = counters
        elif match.group(1) == "formula-entry":
            formulas[int(match.group(2))] = counters
    return named, formulas


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-cnf-phases.py C_TRACE RUST_TRACE", file=sys.stderr)
        return 2

    c_named, c_formulas = read_phases(Path(sys.argv[1]))
    rust_named, rust_formulas = read_phases(Path(sys.argv[2]))
    pairs = [
        ("cnf-entry", "cnf-entry"),
        ("simplify-entry", "simplify-entry"),
        ("defs-entry", "defs-entry"),
        ("clausal-preproc-entry", "cnf-exit"),
    ]
    insertion_delta = None
    for c_name, rust_name in pairs:
        if c_name not in c_named or rust_name not in rust_named:
            print(f"missing phase pair: C {c_name}, Rust {rust_name}", file=sys.stderr)
            return 2
        c_counters = c_named[c_name]
        rust_counters = rust_named[rust_name]
        if c_counters.structural() != rust_counters.structural():
            print(f"structural mismatch: C {c_name}={c_counters}, Rust {rust_name}={rust_counters}")
            return 1
        current_delta = rust_counters.insertions - c_counters.insertions
        insertion_delta = current_delta if insertion_delta is None else insertion_delta
        if current_delta != insertion_delta:
            print(f"attempted-insertion delta changed at {c_name}: {current_delta}")
            return 1

    if c_formulas.keys() != rust_formulas.keys():
        print(f"formula ordinals differ: C={len(c_formulas)}, Rust={len(rust_formulas)}")
        return 1
    for ordinal in c_formulas:
        c_counters = c_formulas[ordinal]
        rust_counters = rust_formulas[ordinal]
        if c_counters.structural() != rust_counters.structural():
            print(f"formula {ordinal} structural mismatch: C={c_counters}, Rust={rust_counters}")
            return 1
        if rust_counters.insertions - c_counters.insertions != insertion_delta:
            print(f"formula {ordinal} attempted-insertion delta changed")
            return 1

    print(f"matching structural phase counters: {len(pairs)}")
    print(f"matching formula-entry counters: {len(c_formulas)}")
    print(f"constant Rust/C attempted-insertion delta: {insertion_delta}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
