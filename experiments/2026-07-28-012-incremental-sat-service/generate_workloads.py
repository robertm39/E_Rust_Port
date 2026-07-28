#!/usr/bin/env python3
"""Generate deterministic semantic and structured incremental SAT sessions."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

SEED = 20_260_728


@dataclass(frozen=True)
class Query:
    identifier: str
    assumptions: tuple[int, ...] = ()
    native_limit: int = -1
    deadline_us: int = 0


@dataclass(frozen=True)
class Step:
    clause: tuple[int, ...] | None = None
    query: Query | None = None


def write_session(path: Path, max_variable: int, steps: Iterable[Step]) -> None:
    lines = [f"p isat {max_variable}"]
    for step in steps:
        if step.clause is not None:
            literals = " ".join(str(literal) for literal in step.clause)
            lines.append(f"a {literals} 0" if literals else "a 0")
        elif step.query is not None:
            query = step.query
            assumptions = " ".join(str(literal) for literal in query.assumptions)
            suffix = f" {assumptions}" if assumptions else ""
            lines.append(
                f"q {query.identifier} {query.native_limit} "
                f"{query.deadline_us}{suffix} 0"
            )
        else:
            raise ValueError("step has neither a clause nor a query")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def clause_step(clause: Iterable[int]) -> Step:
    return Step(clause=tuple(clause))


def query_step(
    identifier: str,
    assumptions: Iterable[int] = (),
    native_limit: int = -1,
    deadline_us: int = 0,
) -> Step:
    return Step(
        query=Query(
            identifier,
            tuple(assumptions),
            native_limit=native_limit,
            deadline_us=deadline_us,
        )
    )


def pigeonhole(pigeons: int, holes: int) -> tuple[int, list[tuple[int, ...]]]:
    def variable(pigeon: int, hole: int) -> int:
        return pigeon * holes + hole + 1

    clauses: list[tuple[int, ...]] = []
    for pigeon in range(pigeons):
        clauses.append(tuple(variable(pigeon, hole) for hole in range(holes)))
        for first in range(holes):
            for second in range(first + 1, holes):
                clauses.append(
                    (-variable(pigeon, first), -variable(pigeon, second))
                )
    for hole in range(holes):
        for first in range(pigeons):
            for second in range(first + 1, pigeons):
                clauses.append((-variable(first, hole), -variable(second, hole)))
    return pigeons * holes, clauses


def xor_equivalence(left: int, right: int, output: int) -> list[tuple[int, ...]]:
    # output <-> (left XOR right)
    return [
        (-left, -right, -output),
        (left, right, -output),
        (left, -right, output),
        (-left, right, output),
    ]


def parity_chain(bits: int) -> tuple[int, list[tuple[int, ...]]]:
    if bits < 2:
        raise ValueError("parity chain needs at least two input bits")
    clauses: list[tuple[int, ...]] = []
    previous = bits + 1
    clauses.extend(xor_equivalence(1, 2, previous))
    next_variable = previous + 1
    for bit in range(3, bits + 1):
        clauses.extend(xor_equivalence(previous, bit, next_variable))
        previous = next_variable
        next_variable += 1
    return previous, clauses


def random_clause(rng: random.Random, variables: int) -> tuple[int, ...]:
    width = rng.randint(1, min(4, variables))
    selected = rng.sample(range(1, variables + 1), width)
    return tuple(variable if rng.getrandbits(1) else -variable for variable in selected)


def emit_fixtures(root: Path) -> list[dict[str, object]]:
    fixtures = root / "semantic"
    fixtures.mkdir(parents=True, exist_ok=True)
    manifest: list[dict[str, object]] = []

    cases: list[tuple[str, int, list[Step]]] = [
        ("empty_sat", 0, [query_step("q0")]),
        (
            "global_unsat",
            1,
            [clause_step([1]), clause_step([-1]), query_step("q0")],
        ),
        (
            "assumption_core",
            3,
            [
                clause_step([1, 2]),
                clause_step([-1, 2]),
                clause_step([3]),
                query_step("base"),
                query_step("failed", [-2, -3]),
                query_step("sat_again", [2]),
            ],
        ),
        (
            "incremental_transition",
            3,
            [
                clause_step([1, 2]),
                query_step("sat_prefix", [-1]),
                clause_step([-2, 3]),
                query_step("sat_middle", [-1]),
                clause_step([-3]),
                query_step("unsat_final", [-1]),
            ],
        ),
        (
            "empty_clause",
            2,
            [clause_step([]), query_step("q0")],
        ),
    ]
    for name, maximum, steps in cases:
        path = fixtures / f"{name}.isat"
        write_session(path, maximum, steps)
        manifest.append({"name": name, "kind": "semantic", "path": str(path)})

    rng = random.Random(SEED)
    for index in range(80):
        variables = rng.randint(1, 8)
        clause_count = rng.randint(1, 4 * variables + 4)
        clauses = [random_clause(rng, variables) for _ in range(clause_count)]
        steps = [clause_step(clause) for clause in clauses]
        for query_index in range(4):
            assumption_count = rng.randint(0, min(variables, 4))
            assumption_vars = rng.sample(range(1, variables + 1), assumption_count)
            assumptions = [
                variable if rng.getrandbits(1) else -variable
                for variable in assumption_vars
            ]
            steps.append(query_step(f"q{query_index}", assumptions))
        name = f"seeded_{index:03d}"
        path = fixtures / f"{name}.isat"
        write_session(path, variables, steps)
        manifest.append(
            {
                "name": name,
                "kind": "semantic",
                "seed": SEED,
                "path": str(path),
            }
        )
    return manifest


def emit_structured(root: Path) -> list[dict[str, object]]:
    structured = root / "structured"
    structured.mkdir(parents=True, exist_ok=True)
    manifest: list[dict[str, object]] = []

    for pigeons in range(4, 10):
        maximum, clauses = pigeonhole(pigeons, pigeons - 1)
        steps = [clause_step(clause) for clause in clauses]
        steps.append(query_step("unlimited"))
        steps.append(query_step("limit0", native_limit=0))
        steps.append(query_step("deadline1ms", deadline_us=1_000))
        name = f"pigeonhole_{pigeons}_{pigeons - 1}"
        path = structured / f"{name}.isat"
        write_session(path, maximum, steps)
        manifest.append({"name": name, "kind": "structured", "path": str(path)})

    for bits in (8, 16, 32, 64, 128):
        maximum, clauses = parity_chain(bits)
        steps = [clause_step(clause) for clause in clauses]
        steps.extend(
            [
                query_step("even", [maximum]),
                query_step("odd", [-maximum]),
                query_step("repeat_even", [maximum]),
            ]
        )
        name = f"parity_{bits}"
        path = structured / f"{name}.isat"
        write_session(path, maximum, steps)
        manifest.append({"name": name, "kind": "structured", "path": str(path)})

    return manifest


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)

    manifest = emit_fixtures(arguments.output) + emit_structured(arguments.output)
    for record in manifest:
        path = Path(str(record["path"]))
        record["path"] = str(path.relative_to(arguments.output))
        record["sha256"] = sha256(path)
    manifest.sort(key=lambda record: str(record["name"]))
    contract = {
        "schema": 1,
        "seed": SEED,
        "generator_sha256": sha256(Path(__file__)),
        "sessions": manifest,
    }
    (arguments.output / "manifest.json").write_text(
        json.dumps(contract, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "sessions": len(manifest),
                "semantic": sum(item["kind"] == "semantic" for item in manifest),
                "structured": sum(
                    item["kind"] == "structured" for item in manifest
                ),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
