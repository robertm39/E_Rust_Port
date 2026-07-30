#!/usr/bin/env python3
"""Generate the frozen family-separated synthetic FM corpus."""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

from fm_core import SCHEMA, load_corpus


SEED = 20_270_730


def arithmetic(
    coefficients: dict[str, int | str],
    constant: int | str = 0,
    *,
    strict: bool = False,
    sort: str = "Rat",
) -> dict[str, Any]:
    return {
        "kind": "arith",
        "sort": sort,
        "strict": strict,
        "coefficients": {
            variable: str(value) for variable, value in coefficients.items()
        },
        "constant": str(constant),
    }


def proposition(name: str, positive: bool = True) -> dict[str, Any]:
    return {"kind": "prop", "name": name, "positive": positive}


def make_workload(
    partition: str,
    family: str,
    variant: int,
    expected: str,
    clauses: list[list[dict[str, Any]]],
    *,
    supported: bool = True,
    unsupported_reason: str | None = None,
) -> dict[str, Any]:
    identifier = f"synthetic_{partition}_{family}_{variant:02d}"
    result = {
        "id": identifier,
        "source": "synthetic",
        "partition": partition,
        "template_family": f"{partition}_{family}",
        "variant": variant,
        "expected": expected,
        "supported": supported,
        "clauses": [
            {"id": f"{identifier}_c{index:02d}", "literals": literals}
            for index, literals in enumerate(clauses)
        ],
    }
    if unsupported_reason is not None:
        result["unsupported_reason"] = unsupported_reason
    return result


def direct_conflict(partition: str, variant: int) -> dict[str, Any]:
    offset = variant + 1
    return make_workload(
        partition,
        "direct_conflict",
        variant,
        "unsat",
        [
            [arithmetic({"x": variant + 1}, -(offset * (variant + 2)), strict=True)],
            [arithmetic({"z": -(variant + 2)}, offset - 1)],
        ],
    )


def propositional_neutral(partition: str, variant: int) -> dict[str, Any]:
    suffix = f"{partition}_{variant}"
    return make_workload(
        partition,
        "propositional_neutral",
        variant,
        "sat",
        [
            [proposition(f"p_{suffix}"), proposition(f"q_{suffix}")],
            [proposition(f"p_{suffix}", False), proposition(f"q_{suffix}")],
            [proposition(f"q_{suffix}"), proposition(f"r_{suffix}")],
        ],
    )


def unsupported_integer(partition: str, variant: int) -> dict[str, Any]:
    return make_workload(
        partition,
        "unsupported_integer",
        variant,
        "unknown",
        [
            [
                {
                    "kind": "unsupported",
                    "sort": "Int",
                    "expression": f"$to_int(x_{variant}) >= 0",
                }
            ]
        ],
        supported=False,
        unsupported_reason="integer_or_floor_term",
    )


def two_hop_conflict(partition: str, variant: int) -> dict[str, Any]:
    boundary = variant + 2
    sort = "Real" if variant % 2 else "Rat"
    return make_workload(
        partition,
        "two_hop_conflict",
        variant,
        "unsat",
        [
            [arithmetic({"x": 1, "y": -1}, sort=sort)],
            [arithmetic({"u": -1}, boundary, sort=sort)],
            [arithmetic({"v": 1}, -(boundary + 1), strict=True, sort=sort)],
        ],
    )


def boundary_tautology(partition: str, variant: int) -> dict[str, Any]:
    suffix = f"{partition}_{variant}"
    return make_workload(
        partition,
        "boundary_tautology",
        variant,
        "sat",
        [
            [
                arithmetic({"x": variant + 1}, -(variant + 1), strict=True),
                arithmetic({"x": -(variant + 2)}, variant + 2),
            ],
            [proposition(f"guard_{suffix}")],
        ],
    )


def unsupported_nonlinear(partition: str, variant: int) -> dict[str, Any]:
    return make_workload(
        partition,
        "unsupported_nonlinear",
        variant,
        "unknown",
        [
            [
                {
                    "kind": "unsupported",
                    "sort": "Real",
                    "expression": f"x_{variant} * y_{variant} > 0",
                }
            ]
        ],
        supported=False,
        unsupported_reason="nonlinear_product",
    )


def mixed_interaction(partition: str, variant: int) -> dict[str, Any]:
    suffix = f"{partition}_{variant}"
    scale = variant + 1
    return make_workload(
        partition,
        "mixed_interaction",
        variant,
        "unsat",
        [
            [
                proposition(f"p_{suffix}"),
                arithmetic({"x": scale}, -(2 * scale), strict=True),
            ],
            [
                proposition(f"q_{suffix}"),
                arithmetic({"z": -1}, 1),
            ],
            [proposition(f"p_{suffix}", False)],
            [proposition(f"q_{suffix}", False)],
        ],
    )


def guarded_arithmetic_neutral(partition: str, variant: int) -> dict[str, Any]:
    suffix = f"{partition}_{variant}"
    return make_workload(
        partition,
        "guarded_arithmetic_neutral",
        variant,
        "sat",
        [
            [
                proposition(f"always_{suffix}"),
                arithmetic({"x": variant + 1}, -(variant + 3), strict=True),
            ],
            [proposition(f"always_{suffix}")],
            [
                proposition(f"spare_{suffix}"),
                arithmetic({"y": -1}, variant),
            ],
        ],
    )


def unsupported_division(partition: str, variant: int) -> dict[str, Any]:
    return make_workload(
        partition,
        "unsupported_division",
        variant,
        "unknown",
        [
            [
                {
                    "kind": "unsupported",
                    "sort": "Rat",
                    "expression": f"x_{variant} / y_{variant} >= 1",
                }
            ]
        ],
        supported=False,
        unsupported_reason="division_by_nonconstant",
    )


def stress_case(
    partition: str,
    variant: int,
    expected: str,
) -> dict[str, Any]:
    suffix = f"{partition}_{variant}"
    if expected == "unsat":
        clauses: list[list[dict[str, Any]]] = [
            [
                proposition(f"context_{suffix}_{index}"),
                arithmetic(
                    {"x": 1 if index % 2 == 0 else -1},
                    -(index + 1) if index % 2 == 0 else index,
                    strict=index % 3 == 0,
                ),
            ]
            for index in range(8)
        ]
        clauses.extend(
            [proposition(f"context_{suffix}_{index}", False)]
            for index in range(8)
        )
        return make_workload(
            partition,
            "growth_stress_unsat",
            variant,
            expected,
            clauses,
        )
    if expected == "sat":
        common = proposition(f"common_{suffix}")
        clauses = [
            [
                common,
                proposition(f"branch_{suffix}_{index}", index % 2 == 0),
                arithmetic({"x": index + 1}, -(index + 2), strict=True),
            ]
            for index in range(12)
        ]
        clauses.append([common])
        return make_workload(
            partition,
            "growth_stress_sat",
            variant,
            expected,
            clauses,
        )
    return make_workload(
        partition,
        "growth_stress_unknown",
        variant,
        expected,
        [
            [
                {
                    "kind": "unsupported",
                    "sort": "Real",
                    "expression": " + ".join(
                        f"f_{suffix}_{index}(x)" for index in range(40)
                    ),
                }
            ]
        ],
        supported=False,
        unsupported_reason="uninterpreted_arithmetic_monomial",
    )


def generate() -> dict[str, Any]:
    random.Random(SEED)  # Seed is frozen even though templates are deterministic.
    workloads: list[dict[str, Any]] = []
    for variant in range(4):
        workloads.extend(
            [
                direct_conflict("train", variant),
                propositional_neutral("train", variant),
                unsupported_integer("train", variant),
                two_hop_conflict("validation", variant),
                boundary_tautology("validation", variant),
                unsupported_nonlinear("validation", variant),
                mixed_interaction("test", variant),
                guarded_arithmetic_neutral("test", variant),
                unsupported_division("test", variant),
            ]
        )
    for partition in ("train", "validation", "test"):
        for variant, expected in enumerate(("unsat", "sat", "unknown")):
            workloads.append(stress_case(partition, variant, expected))
    workloads.sort(key=lambda item: item["id"])
    corpus = {
        "schema": SCHEMA,
        "seed": SEED,
        "generator": "generate_corpus.py",
        "workloads": workloads,
    }
    load_corpus(corpus)
    return corpus


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("synthetic_corpus.json"),
    )
    arguments = parser.parse_args()
    corpus = generate()
    arguments.output.write_text(
        json.dumps(corpus, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    counts: dict[tuple[str, str], int] = {}
    for workload in corpus["workloads"]:
        key = (workload["partition"], workload["expected"])
        counts[key] = counts.get(key, 0) + 1
    print(
        json.dumps(
            {
                "output": str(arguments.output),
                "workloads": len(corpus["workloads"]),
                "counts": {
                    f"{partition}:{expected}": count
                    for (partition, expected), count in sorted(counts.items())
                },
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
