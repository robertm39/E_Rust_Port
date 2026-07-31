#!/usr/bin/env python3
"""Focused unit tests for the quantifier-instantiation worker."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]


def load(name: str, path: Path):
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


WORKER = load("quantifier_worker_test", HERE / "quantifier_worker.py")
INSTGEN = WORKER.load_instgen(REPO_ROOT)


class TriggerInferenceTests(unittest.TestCase):
    def parse_hand(self, name: str):
        path = HERE / "hand" / name
        return INSTGEN.parse_problem(path.read_text(encoding="utf-8"))

    def test_prefers_first_atom_covering_all_variables(self) -> None:
        problem = self.parse_hand("unary-chain-unsat.p")
        step = problem.clauses[1]
        trigger = WORKER.infer_trigger(step)
        self.assertEqual(
            [atom.canonical() for atom in trigger], ["p0(X)"]
        )

    def test_infers_multipattern_when_no_atom_covers_all_variables(
        self,
    ) -> None:
        problem = self.parse_hand("multipattern-unsat.p")
        join = problem.clauses[2]
        trigger = WORKER.infer_trigger(join)
        self.assertEqual(
            [atom.canonical() for atom in trigger], ["p(X)", "q(Y)"]
        )

    def test_matching_joins_bindings_deterministically(self) -> None:
        problem = self.parse_hand("multipattern-unsat.p")
        join = problem.clauses[2]
        trigger = WORKER.infer_trigger(join)
        atoms = [
            INSTGEN.Atom("p", ("a",)),
            INSTGEN.Atom("p", ("b",)),
            INSTGEN.Atom("q", ("a",)),
            INSTGEN.Atom("q", ("b",)),
        ]
        substitutions = list(
            WORKER.matching_substitutions(
                trigger, join.variables, atoms
            )
        )
        self.assertEqual(
            substitutions,
            [
                {"X": "a", "Y": "a"},
                {"X": "a", "Y": "b"},
                {"X": "b", "Y": "a"},
                {"X": "b", "Y": "b"},
            ],
        )

    def test_conflicting_repeated_variable_does_not_match(self) -> None:
        pattern = INSTGEN.Atom("edge", ("X", "X"))
        ground = INSTGEN.Atom("edge", ("a", "b"))
        self.assertIsNone(
            WORKER.match_atom(
                pattern, ground, frozenset({"X"}), {}
            )
        )

    def test_semantic_hash_ignores_resource_measurements(self) -> None:
        problem = self.parse_hand("ground-unsat.p")
        payload = WORKER.semantic_payload(
            method="clausify",
            problem=problem,
            status="unsat",
            reason="ground_abstraction_unsat",
            instances=[],
            method_data={"enumeration_complete": True},
        )
        first = WORKER.stable_json_sha256(payload)
        second = WORKER.stable_json_sha256(dict(payload))
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
