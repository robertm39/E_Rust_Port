#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


C_CALL = re.compile(r"call=(\d+) ident=(\S+) clause=(.*)")
RUST_CALL = re.compile(r"TRACE_HCB_STRUCT call=(\d+) ident=(\S+) clause=(.*)")


def read_calls(path: Path, pattern: re.Pattern[str]) -> list[tuple[str, str]]:
    calls = []
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) or b"\x00" in raw else "utf-8"
    for line in raw.decode(encoding, errors="replace").splitlines():
        match = pattern.search(line)
        if match is not None:
            calls.append((match.group(2), " ".join(match.group(3).split())))
    return calls


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: compare-hcb-structures.py C_TRACE RUST_TRACE", file=sys.stderr)
        return 2

    c_calls = read_calls(Path(sys.argv[1]), C_CALL)
    rust_calls = read_calls(Path(sys.argv[2]), RUST_CALL)
    for call, (c_entry, rust_entry) in enumerate(zip(c_calls, rust_calls), start=1):
        if c_entry[1] != rust_entry[1]:
            print(f"first structural mismatch: call {call}")
            print(f"C:    ident={c_entry[0]} clause={c_entry[1]}")
            print(f"Rust: ident={rust_entry[0]} clause={rust_entry[1]}")
            return 1

    print(f"common structural HCB-call prefix: {min(len(c_calls), len(rust_calls))}")
    print(f"C calls: {len(c_calls)}")
    print(f"Rust calls: {len(rust_calls)}")
    return int(len(c_calls) != len(rust_calls))


if __name__ == "__main__":
    raise SystemExit(main())
