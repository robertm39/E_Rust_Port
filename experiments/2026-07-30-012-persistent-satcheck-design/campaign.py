#!/usr/bin/env python3
"""Run the frozen randomized persistent-SAT lifecycle campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import random
from pathlib import Path
from typing import Any, Sequence

from model import (
    PersistentSatModel,
    atom,
    fresh_satisfiable,
    negative,
    positive,
)


ATOMS = tuple(atom(name) for name in ("a", "b", "c", "d"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def run_campaign(*, seed_count: int, steps_per_trace: int) -> dict[str, Any]:
    metrics = {
        "traces": seed_count,
        "steps_per_trace": steps_per_trace,
        "transitions": 0,
        "oracle_checks": 0,
        "rebuilds": 0,
        "incremental_transitions": 0,
        "sat_snapshots": 0,
        "unsat_snapshots": 0,
        "source_core_checks": 0,
        "maximum_permanent_clauses": 0,
        "maximum_retired_clauses": 0,
    }
    for seed in range(seed_count):
        randomizer = random.Random(seed)
        session = PersistentSatModel(
            variable_cap=18,
            minimum_permanent_limit=8,
            permanent_factor=3,
            minimum_retired_limit=4,
            retired_factor=2,
        )
        snapshot: dict[int, tuple] = {}
        context = "g0"
        for step in range(steps_per_trace):
            action = randomizer.randrange(8)
            source = randomizer.randrange(1, 9)
            if action <= 4:
                literals = []
                for _ in range(randomizer.randrange(4)):
                    key = randomizer.choice(ATOMS)
                    literal = (
                        positive(key)
                        if randomizer.getrandbits(1)
                        else negative(key)
                    )
                    literals.append(literal)
                    if randomizer.randrange(5) == 0:
                        literals.append(literal)
                snapshot[source] = tuple(literals)
            elif action == 5:
                snapshot.pop(source, None)
            elif action == 6:
                context = f"g{step}"
            else:
                items = list(snapshot.items())
                randomizer.shuffle(items)
                snapshot = dict(items)

            transition = session.reconcile(snapshot, context=context)
            result = session.solve()
            expected = fresh_satisfiable(snapshot)
            if result.satisfiable != expected:
                raise AssertionError(
                    f"seed {seed} step {step}: persistent/fresh outcome differs"
                )
            if not result.satisfiable:
                core = {
                    source_id: session.active_by_source[source_id].literals
                    for source_id in result.core_sources
                }
                if not result.core_sources or fresh_satisfiable(core):
                    raise AssertionError(
                        f"seed {seed} step {step}: mapped source core is not UNSAT"
                    )
                metrics["source_core_checks"] += 1
            session.assert_invariants()

            metrics["transitions"] += 1
            metrics["oracle_checks"] += 1
            metrics["rebuilds" if transition.rebuilt else "incremental_transitions"] += 1
            metrics[
                "sat_snapshots" if result.satisfiable else "unsat_snapshots"
            ] += 1
            metrics["maximum_permanent_clauses"] = max(
                metrics["maximum_permanent_clauses"],
                session.permanent_clause_count,
            )
            metrics["maximum_retired_clauses"] = max(
                metrics["maximum_retired_clauses"],
                session.retired_clause_count,
            )

    return {
        "schema_version": 1,
        "kind": "umlaut-persistent-satcheck-model-campaign",
        "status": "pass",
        "metrics": metrics,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seed-count", type=int, default=100)
    parser.add_argument("--steps-per-trace", type=int, default=60)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    result = run_campaign(
        seed_count=arguments.seed_count,
        steps_per_trace=arguments.steps_per_trace,
    )
    root = Path(__file__).resolve().parent
    result["environment"] = {
        "platform": platform.platform(),
        "python": platform.python_version(),
    }
    result["inputs"] = {
        "model_sha256": sha256_file(root / "model.py"),
        "preregistration_sha256": sha256_file(root / "PREREGISTRATION.md"),
        "test_sha256": sha256_file(root / "test_model.py"),
    }
    canonical = json.dumps(result, sort_keys=True, separators=(",", ":")).encode()
    result["result_id"] = hashlib.sha256(canonical).hexdigest()
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
