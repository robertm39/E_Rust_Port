#!/usr/bin/env python3
"""Frozen hand-authored and seeded corpora for Experiment 014."""

from __future__ import annotations

import random
from dataclasses import dataclass
from fractions import Fraction
from typing import Any, Sequence

import cd_viras

base = cd_viras.base

FROZEN_SEED = 0x43445649524153
FROZEN_GENERATED_CASES = 300


@dataclass(frozen=True, slots=True)
class Case:
    case_id: str
    family: str
    literals: tuple[base.Literal, ...]
    expected_supported: bool
    expected_decision: bool | None

    def describe(self) -> Any:
        return {
            "case_id": self.case_id,
            "family": self.family,
            "literals": [literal.describe() for literal in self.literals],
            "expected_supported": self.expected_supported,
            "expected_decision": self.expected_decision,
        }


def equality(left: base.Term, right: base.Term) -> base.Literal:
    return base.Literal(base.subtract(left, right), base.Relation.EQ)


def ge(left: base.Term, right: base.Term) -> base.Literal:
    return base.Literal(base.subtract(left, right), base.Relation.GE)


def gt(left: base.Term, right: base.Term) -> base.Literal:
    return base.Literal(base.subtract(left, right), base.Relation.GT)


def hand_cases() -> tuple[Case, ...]:
    x = base.variable("x")
    y = base.variable("y")
    a = base.variable("a")
    b = base.variable("b")
    c = base.variable("c")
    z = base.variable("z")
    return (
        Case(
            "hand-ground-true",
            "ground",
            (ge(base.constant(1), base.constant(0)),),
            True,
            True,
        ),
        Case(
            "hand-ground-false",
            "ground",
            (ge(base.constant(-1), base.constant(0)),),
            True,
            False,
        ),
        Case(
            "hand-first-variable-unsat",
            "first_variable_exhaustion",
            (equality(x, base.constant(0)), equality(x, base.constant(1))),
            True,
            False,
        ),
        Case(
            "hand-early-sat",
            "early_sat",
            (
                equality(x, y),
                equality(x, base.constant(2)),
                equality(y, base.constant(2)),
            ),
            True,
            True,
        ),
        Case(
            "hand-cross-variable-candidate",
            "context_lifting",
            (
                equality(x, base.add(y, base.constant(1))),
                equality(y, base.constant(2)),
                ge(x, base.constant(0)),
            ),
            True,
            True,
        ),
        Case(
            "hand-irrelevant-prefix-conflict",
            "focused_conflict",
            (
                equality(a, b),
                equality(a, c),
                equality(b, base.constant(0)),
                equality(c, base.constant(0)),
                equality(z, base.constant(0)),
                equality(z, base.constant(1)),
            ),
            True,
            False,
        ),
        Case(
            "boundary-epsilon",
            "unsupported_virtual_shape",
            (gt(x, base.constant(0)),),
            False,
            None,
        ),
        Case(
            "boundary-periodic-grid",
            "unsupported_virtual_shape",
            (
                equality(base.floor_term(x), base.constant(0)),
            ),
            False,
            None,
        ),
    )


def _graph_literals(
    rng: random.Random,
    values: Sequence[Fraction],
) -> list[base.Literal]:
    variables = [base.variable(f"x{index}") for index in range(len(values))]
    literals: list[base.Literal] = []
    final = len(values) - 1
    for index in range(final):
        later = list(range(index + 1, len(values)))
        rng.shuffle(later)
        selected = sorted(later[: min(3, len(later))])
        if index + 1 not in selected:
            selected[0] = index + 1
            selected.sort()
        for target in selected:
            offset = values[index] - values[target]
            literals.append(
                equality(
                    variables[index],
                    base.add(variables[target], base.constant(offset)),
                )
            )
        lower = values[index] - Fraction(rng.randint(1, 4))
        upper = values[index] + Fraction(rng.randint(1, 4))
        literals.append(ge(variables[index], base.constant(lower)))
        literals.append(ge(base.constant(upper), variables[index]))
    literals.append(equality(variables[final], base.constant(values[final])))
    literals.append(
        ge(
            variables[final],
            base.constant(values[final] - Fraction(rng.randint(1, 4))),
        )
    )
    return literals


def generated_cases(
    *,
    seed: int = FROZEN_SEED,
    count: int = FROZEN_GENERATED_CASES,
) -> tuple[Case, ...]:
    if count < 3 or count % 3:
        raise ValueError("generated case count must be a positive multiple of three")
    rng = random.Random(seed)
    per_family = count // 3
    cases: list[Case] = []
    families = ("sat_graph", "unsat_tail", "unsat_sparse")
    for family in families:
        for family_index in range(per_family):
            variable_count = rng.randint(3, 7)
            values = [
                Fraction(rng.randint(-5, 5), rng.choice((1, 1, 1, 2)))
                for _ in range(variable_count)
            ]
            literals = _graph_literals(rng, values)
            variables = [
                base.variable(f"x{index}") for index in range(variable_count)
            ]
            if family == "unsat_tail":
                literals.append(
                    equality(
                        variables[-1],
                        base.constant(values[-1] + Fraction(1)),
                    )
                )
                expected = False
            elif family == "unsat_sparse":
                literals.append(
                    equality(
                        variables[0],
                        base.add(
                            variables[-1],
                            base.constant(values[0] - values[-1] + Fraction(1)),
                        ),
                    )
                )
                expected = False
            else:
                expected = True
            cases.append(
                Case(
                    f"generated-{family}-{family_index:03d}",
                    family,
                    tuple(literals),
                    True,
                    expected,
                )
            )
    return tuple(cases)
