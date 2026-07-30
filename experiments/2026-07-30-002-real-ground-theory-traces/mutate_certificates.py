#!/usr/bin/env python3
"""Require the Rust replay checker to reject six evidence mutations."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from fractions import Fraction
from pathlib import Path
from typing import Callable


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def decision_range(
    lines: list[str],
    status: str,
) -> tuple[int, int]:
    for start, line in enumerate(lines):
        if line.startswith("DECISION\t") and line.endswith(f"\t{status}"):
            for end in range(start + 1, len(lines)):
                if lines[end] == "END_DECISION":
                    return start, end
            break
    raise ValueError(f"no complete {status} decision")


def empty_core(lines: list[str]) -> None:
    start, end = decision_range(lines, "unsat")
    for index in range(start, end):
        if lines[index].startswith("CORE\t"):
            lines[index] = "CORE\t"
            return
    raise ValueError("no unsat core")


def unknown_core_label(lines: list[str]) -> None:
    start, end = decision_range(lines, "unsat")
    for index in range(start, end):
        if lines[index].startswith("CORE\t"):
            lines[index] += ",not_a_certificate_label"
            return
    raise ValueError("no unsat core")


def remove_negative_cycle(lines: list[str]) -> None:
    start, end = decision_range(lines, "unsat")
    changed = 0
    for index in range(start, end):
        if lines[index].startswith("CONSTRAINT\t"):
            fields = lines[index].split("\t")
            fields[4:6] = ["1", "1"]
            lines[index] = "\t".join(fields)
            changed += 1
    if not changed:
        raise ValueError("no unsat constraints")


def missing_model_variable(lines: list[str]) -> None:
    start, end = decision_range(lines, "sat")
    for index in range(start, end):
        if lines[index].startswith("MODEL\t"):
            del lines[index]
            return
    raise ValueError("no sat model")


def tighten_below_model(lines: list[str]) -> None:
    start, end = decision_range(lines, "sat")
    model: dict[str, Fraction] = {"zero": Fraction(0)}
    constraint_index = None
    constraint_fields: list[str] | None = None
    for index in range(start, end):
        fields = lines[index].split("\t")
        if fields[0] == "CONSTRAINT" and constraint_index is None:
            constraint_index = index
            constraint_fields = fields
        elif fields[0] == "MODEL":
            model[fields[1]] = Fraction(int(fields[2]), int(fields[3]))
    if constraint_index is None or constraint_fields is None:
        raise ValueError("no sat constraint")
    lhs, rhs = constraint_fields[2:4]
    if lhs not in model or rhs not in model:
        raise ValueError("model does not cover first constraint")
    violating_bound = model[lhs] - model[rhs] - 1
    constraint_fields[4:6] = [
        str(violating_bound.numerator),
        str(violating_bound.denominator),
    ]
    lines[constraint_index] = "\t".join(constraint_fields)


def flip_status(lines: list[str]) -> None:
    start, _ = decision_range(lines, "unsat")
    lines[start] = lines[start][: -len("unsat")] + "sat"


MUTATIONS: dict[str, Callable[[list[str]], None]] = {
    "empty_core": empty_core,
    "unknown_core_label": unknown_core_label,
    "remove_negative_cycle": remove_negative_cycle,
    "missing_model_variable": missing_model_variable,
    "tighten_below_model": tighten_below_model,
    "flip_status": flip_status,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checker", required=True, type=Path)
    parser.add_argument("--certificate", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    original = arguments.certificate.read_text(encoding="utf-8").splitlines()
    arguments.output_dir.mkdir(parents=True, exist_ok=True)
    outcomes = {}
    for name, mutation in MUTATIONS.items():
        lines = original.copy()
        mutation(lines)
        mutant = arguments.output_dir / f"{name}.txt"
        mutant.write_text("\n".join(lines) + "\n", encoding="utf-8")
        completed = subprocess.run(
            [str(arguments.checker), str(mutant)],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
        )
        outcomes[name] = {
            "mutant_sha256": sha256(mutant),
            "rejected": completed.returncode != 0,
            "returncode": completed.returncode,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        }
    report = {
        "schema": "umlaut-real-ground-mutation-report-v1",
        "certificate_sha256": sha256(arguments.certificate),
        "mutations": outcomes,
        "passed": all(outcome["rejected"] for outcome in outcomes.values()),
    }
    arguments.report.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
