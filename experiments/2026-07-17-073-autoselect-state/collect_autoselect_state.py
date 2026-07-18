#!/usr/bin/env python3
"""Collect the automatic-ordering search state from an isolated C build."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def wsl_path(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def fnv1a64(payload: bytes) -> str:
    value = 0xCBF29CE484222325
    for byte in payload:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFF_FFFF_FFFF_FFFF
    return f"{value:016X}"


def parse_csv_line(line: str, tag: str, fields: int) -> list[int]:
    values = line.split(",")
    if values[0] != tag or len(values) != fields + 1:
        raise ValueError(f"invalid {tag} line: {line!r}")
    return [int(value) for value in values[1:]]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    gdb_script = Path(__file__).resolve().parent / "collect_reference.gdb"
    completed = subprocess.run(
        [
            "wsl",
            "-d",
            args.distro,
            "--",
            "gdb",
            "--batch",
            "-q",
            args.c_exe,
            "-x",
            wsl_path(gdb_script),
        ],
        capture_output=True,
        check=False,
    )
    stdout = completed.stdout.decode("utf-8", errors="strict")
    stderr = completed.stderr.decode("utf-8", errors="strict")
    if completed.returncode != 0:
        raise SystemExit(f"GDB collector failed:\n{stdout}\n{stderr}")

    tagged = [
        line
        for line in stdout.splitlines()
        if line.startswith(("INIT,", "SEQ,", "FINAL,"))
    ]
    init_lines = [line for line in tagged if line.startswith("INIT,")]
    sequence_lines = [line for line in tagged if line.startswith("SEQ,")]
    final_lines = [line for line in tagged if line.startswith("FINAL,")]
    if len(init_lines) != 1 or len(final_lines) != 1 or not sequence_lines:
        raise SystemExit(f"unexpected tagged GDB output: {tagged!r}")

    init_values = parse_csv_line(init_lines[0], "INIT", 12)
    sequence = [parse_csv_line(line, "SEQ", 5) for line in sequence_lines]
    final = parse_csv_line(final_lines[0], "FINAL", 5)
    payload = ("\n".join(sequence_lines) + "\n").encode("ascii")
    output = {
        "reference_commit": REFERENCE_COMMIT,
        "init": {
            "ordertype": init_values[0],
            "weight_gen": init_values[1],
            "prec_gen": init_values[2],
            "const_weight": init_values[3],
            "conj_only_mod": init_values[4],
            "conj_axiom_mod": init_values[5],
            "axiom_only_mod": init_values[6],
            "lit_cmp": init_values[7],
            "ho_order_kind": init_values[8],
            "db_weight": init_values[9],
            "lambda_weight": init_values[10],
            "force_kbo_var_weight": init_values[11],
        },
        "sequence_count": len(sequence),
        "sequence_sha256": hashlib.sha256(payload).hexdigest().upper(),
        "sequence_fnv1a64": fnv1a64(payload),
        "sequence": sequence,
        "wrapped_final_state": final,
    }
    rendered = json.dumps(output, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if output != expected:
            raise SystemExit("automatic-ordering state differs from snapshot")


if __name__ == "__main__":
    main()
