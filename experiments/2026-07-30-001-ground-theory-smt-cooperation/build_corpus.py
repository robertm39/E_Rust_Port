#!/usr/bin/env python3
"""Build the deterministic ground-theory branch corpus."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


EXPERIMENT_ROOT = Path(__file__).resolve().parent
DEFAULT_OUTPUT = EXPERIMENT_ROOT / "corpus.json"
PARTITIONS = (("train", 4), ("validation", 8), ("test", 16))
SORTS = ("Int", "Real")


def difference(
    label: str,
    lhs: str,
    rhs: str,
    bound: int | str,
) -> dict[str, str]:
    return {
        "kind": "difference",
        "label": label,
        "lhs": lhs,
        "rhs": rhs,
        "bound": str(bound),
    }


def general(
    label: str,
    terms: dict[str, int],
    bound: int | str,
) -> dict[str, Any]:
    return {
        "kind": "general_linear",
        "label": label,
        "terms": terms,
        "bound": str(bound),
    }


def cycle_base(size: int) -> list[dict[str, str]]:
    return [
        difference(f"base_{index}", f"x{index}", f"x{index + 1}", 0)
        for index in range(size - 1)
    ]


def supported_workloads(partition: str, size: int, sort: str) -> list[dict[str, Any]]:
    prefix = f"{partition}-{sort.lower()}"
    variables = [f"x{index}" for index in range(size)]
    close_branches = [
        {
            "id": f"close_{index}",
            "expected": "unsat",
            "constraints": [
                difference(
                    f"close_{index}_edge",
                    variables[-1],
                    variables[0],
                    -(index + 1),
                )
            ],
        }
        for index in range(4)
    ]

    mixed_branches = []
    for index in range(6):
        is_unsat = index % 2 == 0
        mixed_branches.append(
            {
                "id": f"mixed_{index}",
                "expected": "unsat" if is_unsat else "sat",
                "constraints": [
                    difference(
                        f"mixed_{index}_edge",
                        variables[-1],
                        variables[0],
                        -1 if is_unsat else index + 1,
                    )
                ],
            }
        )

    sat_branches = [
        {
            "id": f"sat_{index}",
            "expected": "sat",
            "constraints": [
                difference(
                    f"sat_{index}_edge",
                    variables[-1],
                    variables[0],
                    index,
                )
            ],
        }
        for index in range(6)
    ]

    return [
        {
            "id": f"{prefix}-closed",
            "partition": partition,
            "cohort": "theory_heavy_closed",
            "sort": sort,
            "eligible": True,
            "expected_closed": True,
            "variables": variables,
            "base": cycle_base(size),
            "branches": close_branches,
        },
        {
            "id": f"{prefix}-mixed",
            "partition": partition,
            "cohort": "theory_heavy_mixed",
            "sort": sort,
            "eligible": True,
            "expected_closed": False,
            "variables": variables,
            "base": cycle_base(size),
            "branches": mixed_branches,
        },
        {
            "id": f"{prefix}-sat",
            "partition": partition,
            "cohort": "theory_heavy_sat",
            "sort": sort,
            "eligible": True,
            "expected_closed": False,
            "variables": variables,
            "base": cycle_base(size),
            "branches": sat_branches,
        },
    ]


def neutral_workload(partition: str, sort: str) -> dict[str, Any]:
    return {
        "id": f"{partition}-{sort.lower()}-neutral",
        "partition": partition,
        "cohort": "neutral",
        "sort": sort,
        "eligible": False,
        "expected_closed": False,
        "variables": [],
        "base": [],
        "branches": [
            {
                "id": "neutral_0",
                "expected": "unknown",
                "constraints": [],
            }
        ],
    }


def unsupported_workload(partition: str, sort: str) -> dict[str, Any]:
    integral = sort == "Int"
    half = "2" if integral else "3/2"
    return {
        "id": f"{partition}-{sort.lower()}-unsupported",
        "partition": partition,
        "cohort": "unsupported_general_linear",
        "sort": sort,
        "eligible": False,
        "expected_closed": False,
        "variables": ["x0", "x1"],
        "base": [general("unsupported_base", {"x0": 2, "x1": 3}, half)],
        "branches": [
            {
                "id": "unsupported_sat",
                "expected": "sat",
                "constraints": [
                    general("unsupported_sat_lower", {"x0": -2, "x1": -3}, 0)
                ],
            },
            {
                "id": "unsupported_unsat",
                "expected": "unsat",
                "constraints": [
                    general(
                        "unsupported_unsat_lower",
                        {"x0": -2, "x1": -3},
                        -3 if integral else -2,
                    )
                ],
            },
        ],
    }


def build_corpus() -> dict[str, Any]:
    workloads: list[dict[str, Any]] = []
    for partition, size in PARTITIONS:
        for sort in SORTS:
            workloads.extend(supported_workloads(partition, size, sort))
            workloads.append(neutral_workload(partition, sort))
            workloads.append(unsupported_workload(partition, sort))
    return {
        "schema": "umlaut-ground-theory-corpus-v1",
        "date_frozen": "2026-07-30",
        "description": (
            "Typed ground difference-logic branches, unsupported controls, "
            "and neutral bypass workloads."
        ),
        "workloads": workloads,
    }


def canonical_bytes(corpus: dict[str, Any]) -> bytes:
    return (json.dumps(corpus, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    expected = canonical_bytes(build_corpus())
    if args.check:
        if not args.output.is_file():
            raise SystemExit(f"missing frozen corpus: {args.output}")
        actual = args.output.read_bytes()
        if actual != expected:
            raise SystemExit(f"frozen corpus differs from generator: {args.output}")
        print(f"corpus is deterministic: {args.output}")
        return 0

    args.output.write_bytes(expected)
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
