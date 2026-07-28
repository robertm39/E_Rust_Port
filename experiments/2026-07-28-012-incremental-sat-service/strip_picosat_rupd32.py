#!/usr/bin/env python3
"""Validate and remove PicoSAT's RUPD32 metadata line for DRAT checking."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

HEADER = re.compile(rb"%RUPD32 ([0-9]+) ([0-9]+) *")


def dimacs_shape(path: Path) -> tuple[int, int]:
    for line in path.read_text(encoding="ascii").splitlines():
        if line.startswith("p cnf "):
            fields = line.split()
            if len(fields) != 4:
                break
            return int(fields[2]), int(fields[3])
    raise ValueError(f"{path}: missing DIMACS header")


def strip_trace(trace: bytes, expected: tuple[int, int]) -> bytes:
    header, separator, body = trace.partition(b"\n")
    if not separator:
        raise ValueError("RUPD32 trace has no metadata line")
    match = HEADER.fullmatch(header)
    if match is None:
        raise ValueError("invalid RUPD32 metadata line")
    actual = (int(match.group(1)), int(match.group(2)))
    if actual != expected:
        raise ValueError(f"RUPD32 shape {actual} does not match DIMACS {expected}")
    return body


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("dimacs", type=Path)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    arguments.output.write_bytes(
        strip_trace(arguments.trace.read_bytes(), dimacs_shape(arguments.dimacs))
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
