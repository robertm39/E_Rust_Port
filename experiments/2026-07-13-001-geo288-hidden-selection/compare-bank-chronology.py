#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


C_CALL = re.compile(r"^window selection=(\d+) ident=(\S+) in_count=(\d+)$")
RUST_CALL = re.compile(r"^TRACE fc call=(\d+) ident=(\S+) in_count=(\d+)$")


def read_calls(path: Path, pattern: re.Pattern[str]) -> list[tuple[int, str, int]]:
    calls = []
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) or b"\x00" in raw else "utf-8"
    for line in raw.decode(encoding, errors="replace").splitlines():
        match = pattern.fullmatch(line.strip())
        if match is not None:
            calls.append((int(match.group(1)), match.group(2), int(match.group(3))))
    return calls


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-bank-chronology.py C_TRACE RUST_TRACE", file=sys.stderr)
        return 2

    c_calls = read_calls(Path(sys.argv[1]), C_CALL)
    rust_calls = read_calls(Path(sys.argv[2]), RUST_CALL)
    if not c_calls or not rust_calls:
        print("both traces must contain at least one matching call", file=sys.stderr)
        return 2

    c_base = c_calls[0][2]
    rust_base = rust_calls[0][2]
    for c_call, rust_call in zip(c_calls, rust_calls):
        c_delta = c_call[2] - c_base
        rust_delta = rust_call[2] - rust_base
        if c_call[:2] != rust_call[:2] or c_delta != rust_delta:
            print(f"first mismatch: call {min(c_call[0], rust_call[0])}")
            print(f"C:    call={c_call[0]} ident={c_call[1]} delta={c_delta}")
            print(f"Rust: call={rust_call[0]} ident={rust_call[1]} delta={rust_delta}")
            return 1

    print(f"common normalized prefix: {min(len(c_calls), len(rust_calls))}")
    print(f"C calls: {len(c_calls)}")
    print(f"Rust calls: {len(rust_calls)}")
    print(f"initial absolute offset: {rust_base - c_base}")
    return int(len(c_calls) != len(rust_calls))


if __name__ == "__main__":
    raise SystemExit(main())
