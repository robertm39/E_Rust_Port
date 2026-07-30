#!/usr/bin/env python3
"""Require the Rust replay checker to reject four certificate mutations."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Callable


def mutate_core_member(lines: list[str]) -> None:
    for index, line in enumerate(lines):
        if line.startswith("CORE\t"):
            labels = line.split("\t", 1)[1].split(",")
            if len(labels) >= 2:
                lines[index] = "CORE\t" + ",".join(labels[1:])
                return
    raise ValueError("no removable core member")


def mutate_core_bound(lines: list[str]) -> None:
    in_unsat = False
    for index, line in enumerate(lines):
        if line.startswith("DECISION\t"):
            in_unsat = line.endswith("\tunsat")
        elif in_unsat and line.startswith("CONSTRAINT\t"):
            fields = line.split("\t")
            fields[4] = "1000000"
            fields[5] = "1"
            lines[index] = "\t".join(fields)
            return
    raise ValueError("no unsat constraint")


def mutate_model(lines: list[str]) -> None:
    in_sat = False
    changed = 0
    for index, line in enumerate(lines):
        if line.startswith("DECISION\t"):
            in_sat = line.endswith("\tsat")
            changed = 0
        elif in_sat and line.startswith("MODEL\t") and changed < 2:
            fields = line.split("\t")
            fields[2] = "1000000" if changed == 0 else "-1000000"
            fields[3] = "1"
            lines[index] = "\t".join(fields)
            changed += 1
            if changed == 2:
                return
    raise ValueError("no sat model with two variables")


def mutate_status(lines: list[str]) -> None:
    for index, line in enumerate(lines):
        if line.startswith("DECISION\t") and line.endswith("\tunsat"):
            lines[index] = line[: -len("unsat")] + "sat"
            return
    raise ValueError("no unsat decision")


MUTATIONS: dict[str, Callable[[list[str]], None]] = {
    "remove_core_member": mutate_core_member,
    "alter_core_bound": mutate_core_bound,
    "corrupt_model": mutate_model,
    "flip_status": mutate_status,
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checker", type=Path, required=True)
    parser.add_argument("--certificate", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    original = args.certificate.read_text(encoding="utf-8").splitlines()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    outcomes = {}
    for name, mutation in MUTATIONS.items():
        lines = original.copy()
        mutation(lines)
        mutant = args.output_dir / f"{name}.txt"
        mutant.write_text("\n".join(lines) + "\n", encoding="utf-8")
        completed = subprocess.run(
            [str(args.checker), str(mutant)],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=30,
        )
        outcomes[name] = {
            "rejected": completed.returncode != 0,
            "returncode": completed.returncode,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        }
    report = {
        "mutations": outcomes,
        "passed": all(outcome["rejected"] for outcome in outcomes.values()),
    }
    args.report.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
