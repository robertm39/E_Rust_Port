#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


CALL = re.compile(
    r"call=(\d+) ident=(\S+) current_eval=(\d+) select_count=(-?\d+)"
)


def read_calls(path: Path) -> list[tuple[str, int, int]]:
    calls = []
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) or b"\x00" in raw else "utf-8"
    for line in raw.decode(encoding, errors="replace").splitlines():
        match = CALL.search(line)
        if match is not None:
            calls.append((match.group(2), int(match.group(3)), int(match.group(4))))
    return calls


def main() -> int:
    if len(sys.argv) not in (3, 4) or (len(sys.argv) == 4 and sys.argv[3] != "--all"):
        print("usage: compare-hcb.py C_TRACE RUST_TRACE [--all]", file=sys.stderr)
        return 2

    report_all = len(sys.argv) == 4
    c_calls = read_calls(Path(sys.argv[1]))
    rust_calls = read_calls(Path(sys.argv[2]))
    mismatches = 0
    for index, (c_call, rust_call) in enumerate(zip(c_calls, rust_calls), start=1):
        if c_call != rust_call:
            mismatches += 1
            print(f"mismatch {mismatches}: call {index}")
            print(f"C:    ident={c_call[0]} current_eval={c_call[1]} select_count={c_call[2]}")
            print(
                f"Rust: ident={rust_call[0]} current_eval={rust_call[1]} "
                f"select_count={rust_call[2]}"
            )
            if not report_all:
                return 1

    print(f"common HCB-call prefix: {min(len(c_calls), len(rust_calls))}")
    print(f"C calls: {len(c_calls)}")
    print(f"Rust calls: {len(rust_calls)}")
    print(f"mismatches in common calls: {mismatches}")
    return int(mismatches != 0 or len(c_calls) != len(rust_calls))


if __name__ == "__main__":
    raise SystemExit(main())
