#!/usr/bin/env python3
"""Independent adapters and seeded generators for the VIRAS experiment."""

from __future__ import annotations

import random
import sys
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path

EXPERIMENT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_DIR.parents[1]
VALIDATION_DIR = REPOSITORY_ROOT / "tools" / "validation"
sys.path.insert(0, str(EXPERIMENT_DIR))
sys.path.insert(0, str(VALIDATION_DIR))

import arithmetic_qe_oracle as oracle  # noqa: E402
import prototype  # noqa: E402


@dataclass(frozen=True, slots=True)
class GeneratedCase:
    case_id: str
    literals: tuple[prototype.Literal, ...]
    oracle_formula: oracle.Formula


def to_oracle_expr(term: prototype.Term) -> oracle.Expr:
    if term.op is prototype.TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return oracle.Expr.constant(term.value)
    if term.op is prototype.TermOp.VAR:
        assert isinstance(term.value, str)
        return oracle.Expr.variable(term.value)
    if term.op is prototype.TermOp.ADD:
        return oracle.Expr.add(*(to_oracle_expr(argument) for argument in term.args))
    if term.op is prototype.TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return oracle.Expr.scale(term.value, to_oracle_expr(term.args[0]))
    return oracle.Expr.floor(to_oracle_expr(term.args[0]))


def to_oracle_formula(
    literals: tuple[prototype.Literal, ...] | list[prototype.Literal],
) -> oracle.Formula:
    relation_map = {
        prototype.Relation.EQ: oracle.Relation.EQ,
        prototype.Relation.NE: oracle.Relation.NE,
        prototype.Relation.GT: oracle.Relation.GT,
        prototype.Relation.GE: oracle.Relation.GE,
    }
    return oracle.Formula.and_(
        *(
            oracle.Formula.atom(
                oracle.Atom(
                    to_oracle_expr(literal.term),
                    relation_map[literal.relation],
                    oracle.Expr.constant(0),
                )
            )
            for literal in literals
        )
    )


_COEFFICIENTS = (
    Fraction(-3),
    Fraction(-2),
    Fraction(-3, 2),
    Fraction(-1),
    Fraction(-2, 3),
    Fraction(-1, 2),
    Fraction(1, 2),
    Fraction(2, 3),
    Fraction(1),
    Fraction(3, 2),
    Fraction(2),
    Fraction(3),
)

_CONSTANTS = tuple(Fraction(value, denominator) for value in range(-4, 5) for denominator in (1, 2, 3))


def generated_term(
    rng: random.Random,
    *,
    variable_name: str = "x",
    depth: int = 3,
) -> prototype.Term:
    x = prototype.variable(variable_name)
    if depth <= 0:
        return x if rng.random() < 0.55 else prototype.constant(rng.choice(_CONSTANTS))
    choice = rng.random()
    if choice < 0.20:
        return x
    if choice < 0.34:
        return prototype.constant(rng.choice(_CONSTANTS))
    if choice < 0.58:
        return prototype.add(
            generated_term(rng, variable_name=variable_name, depth=depth - 1),
            generated_term(rng, variable_name=variable_name, depth=depth - 1),
        )
    if choice < 0.78:
        return prototype.scale(
            rng.choice(_COEFFICIENTS),
            generated_term(rng, variable_name=variable_name, depth=depth - 1),
        )
    return prototype.floor_term(
        generated_term(rng, variable_name=variable_name, depth=depth - 1)
    )


def generate_cases(seed: int, count: int) -> tuple[GeneratedCase, ...]:
    rng = random.Random(seed)
    x = prototype.variable("x")
    lower_bound = prototype.Literal(
        prototype.add(x, prototype.constant(8)),
        prototype.Relation.GE,
    )
    upper_bound = prototype.Literal(
        prototype.add(prototype.constant(8), prototype.negate(x)),
        prototype.Relation.GE,
    )
    cases: list[GeneratedCase] = []
    for index in range(count):
        random_literals = [
            prototype.Literal(
                generated_term(rng, depth=3 if index % 5 == 0 else 2),
                rng.choice(tuple(prototype.Relation)),
            )
            for _ in range(rng.randint(1, 4))
        ]
        # Explicit bounds make the independent bounded oracle a complete oracle
        # for the otherwise unbounded existential query.
        literals = tuple([lower_bound, upper_bound, *random_literals])
        cases.append(
            GeneratedCase(
                case_id=f"generated-{index:04d}",
                literals=literals,
                oracle_formula=to_oracle_formula(literals),
            )
        )
    return tuple(cases)


def exact_oracle_decision(case: GeneratedCase) -> bool:
    query = oracle.BoundedQuery.create(
        "x",
        -8,
        8,
        case.oracle_formula,
        name=case.case_id,
    )
    outcome = oracle.decide_exact(query, max_cells=100_000)
    if outcome.decision is oracle.Decision.UNKNOWN:
        raise RuntimeError(f"{case.case_id}: exact oracle returned unknown: {outcome.reason}")
    if outcome.decision is oracle.Decision.ERROR:
        raise RuntimeError(f"{case.case_id}: exact oracle returned error: {outcome.reason}")
    return outcome.decision is oracle.Decision.SAT
