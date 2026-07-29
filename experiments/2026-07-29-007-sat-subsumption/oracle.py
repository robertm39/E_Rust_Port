#!/usr/bin/env python3
"""Independent semantic oracle for SAT subsumption constraints."""

from __future__ import annotations

import argparse
import itertools
import json
import random
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


Term = tuple
Literal = tuple[bool, Term, Term]
Clause = tuple[Literal, ...]


@dataclass(frozen=True)
class Choice:
    source: int
    target: int
    negative: bool
    bindings: tuple[tuple[int, Term], ...]


def variable(index: int) -> Term:
    return ("v", index)


def function(symbol: int, *arguments: Term) -> Term:
    return ("f", symbol, tuple(arguments))


def match_term(
    pattern: Term, target: Term, bindings: dict[int, Term]
) -> bool:
    if pattern[0] == "v":
        index = pattern[1]
        previous = bindings.get(index)
        if previous is None:
            bindings[index] = target
            return True
        return previous == target
    return (
        target[0] == "f"
        and pattern[1] == target[1]
        and len(pattern[2]) == len(target[2])
        and all(
            match_term(left, right, bindings)
            for left, right in zip(pattern[2], target[2], strict=True)
        )
    )


def literal_matches(pattern: Literal, target: Literal) -> list[Choice]:
    negative = pattern[0] != target[0]
    matches: list[Choice] = []
    for reversed_target in (False, True):
        target_left, target_right = (
            (target[2], target[1])
            if reversed_target
            else (target[1], target[2])
        )
        bindings: dict[int, Term] = {}
        if match_term(pattern[1], target_left, bindings) and match_term(
            pattern[2], target_right, bindings
        ):
            choice = Choice(
                source=-1,
                target=-1,
                negative=negative,
                bindings=tuple(sorted(bindings.items())),
            )
            if choice not in matches:
                matches.append(choice)
    return matches


def choices(side: Clause, main: Clause) -> list[Choice]:
    result: list[Choice] = []
    for source, pattern in enumerate(side):
        for target, candidate in enumerate(main):
            for partial in literal_matches(pattern, candidate):
                result.append(
                    Choice(
                        source=source,
                        target=target,
                        negative=partial.negative,
                        bindings=partial.bindings,
                    )
                )
    return result


def compatible(left: Choice, right: Choice) -> bool:
    left_map = dict(left.bindings)
    right_map = dict(right.bindings)
    return all(
        variable not in right_map or right_map[variable] == target
        for variable, target in left_map.items()
    )


def brute_subsumes(side: Clause, main: Clause) -> bool:
    candidates = choices(side, main)

    def visit(
        source: int, selected: tuple[Choice, ...], used: frozenset[int]
    ) -> bool:
        if source == len(side):
            return True
        for choice in candidates:
            if (
                choice.source == source
                and not choice.negative
                and choice.target not in used
                and all(compatible(choice, previous) for previous in selected)
                and visit(
                    source + 1,
                    (*selected, choice),
                    used | {choice.target},
                )
            ):
                return True
        return False

    return visit(0, (), frozenset())


def brute_resolution(side: Clause, main: Clause) -> bool:
    candidates = choices(side, main)

    def visit(source: int, selected: tuple[Choice, ...]) -> bool:
        if source == len(side):
            negative_targets = {
                choice.target for choice in selected if choice.negative
            }
            if len(negative_targets) != 1:
                return False
            resolution_target = next(iter(negative_targets))
            return not any(
                not choice.negative and choice.target == resolution_target
                for choice in selected
            )
        for choice in candidates:
            if (
                choice.source == source
                and all(compatible(choice, previous) for previous in selected)
                and visit(source + 1, (*selected, choice))
            ):
                return True
        return False

    return visit(0, ())


def add_at_most_one(
    clauses: list[list[int]], variables: Sequence[int]
) -> None:
    for left, right in itertools.combinations(variables, 2):
        clauses.append([-left, -right])


def add_compatibility(
    clauses: list[list[int]], indexed: Sequence[tuple[int, Choice]]
) -> None:
    for (left_var, left), (right_var, right) in itertools.combinations(
        indexed, 2
    ):
        if not compatible(left, right):
            clauses.append([-left_var, -right_var])


def ordinary_cnf(side: Clause, main: Clause) -> list[list[int]]:
    indexed = list(enumerate(choices(side, main), start=1))
    clauses: list[list[int]] = []
    for source in range(len(side)):
        group = [
            variable_id
            for variable_id, choice in indexed
            if choice.source == source and not choice.negative
        ]
        clauses.append(group)
        add_at_most_one(clauses, group)
    for target in range(len(main)):
        add_at_most_one(
            clauses,
            [
                variable_id
                for variable_id, choice in indexed
                if choice.target == target and not choice.negative
            ],
        )
    add_compatibility(clauses, indexed)
    return clauses


def resolution_cnf(side: Clause, main: Clause) -> list[list[int]]:
    indexed = list(enumerate(choices(side, main), start=1))
    clauses: list[list[int]] = []
    for source in range(len(side)):
        group = [
            variable_id
            for variable_id, choice in indexed
            if choice.source == source
        ]
        clauses.append(group)
        add_at_most_one(clauses, group)
    clauses.append(
        [
            variable_id
            for variable_id, choice in indexed
            if choice.negative
        ]
    )
    for (left_var, left), (right_var, right) in itertools.combinations(
        indexed, 2
    ):
        if left.negative and right.negative and left.target != right.target:
            clauses.append([-left_var, -right_var])
        if (
            left.target == right.target
            and left.negative != right.negative
        ):
            clauses.append([-left_var, -right_var])
    add_compatibility(clauses, indexed)
    return clauses


def simplify(
    clauses: Sequence[Sequence[int]], assignment: dict[int, bool]
) -> list[list[int]] | None:
    reduced: list[list[int]] = []
    for clause in clauses:
        pending: list[int] = []
        satisfied = False
        for literal in clause:
            value = assignment.get(abs(literal))
            if value is None:
                pending.append(literal)
            elif value == (literal > 0):
                satisfied = True
                break
        if satisfied:
            continue
        if not pending:
            return None
        reduced.append(pending)
    return reduced


def dpll(
    clauses: Sequence[Sequence[int]],
    assignment: dict[int, bool] | None = None,
) -> bool:
    current = {} if assignment is None else dict(assignment)
    reduced = simplify(clauses, current)
    if reduced is None:
        return False
    if not reduced:
        return True
    while True:
        unit = next((clause[0] for clause in reduced if len(clause) == 1), None)
        if unit is None:
            break
        current[abs(unit)] = unit > 0
        reduced = simplify(reduced, current)
        if reduced is None:
            return False
        if not reduced:
            return True
    decision = abs(reduced[0][0])
    for value in (True, False):
        branch = dict(current)
        branch[decision] = value
        if dpll(reduced, branch):
            return True
    return False


def random_ground_term(rng: random.Random, depth: int = 0) -> Term:
    if depth >= 2 or rng.random() < 0.55:
        return function(rng.randrange(4))
    arity = rng.choice((1, 2))
    return function(
        10 + rng.randrange(4),
        *(random_ground_term(rng, depth + 1) for _ in range(arity)),
    )


def random_pattern_term(
    rng: random.Random, variables: Sequence[Term], depth: int = 0
) -> Term:
    if rng.random() < 0.30:
        return rng.choice(variables)
    if depth >= 2 or rng.random() < 0.50:
        return function(rng.randrange(4))
    arity = rng.choice((1, 2))
    return function(
        10 + rng.randrange(4),
        *(
            random_pattern_term(rng, variables, depth + 1)
            for _ in range(arity)
        ),
    )


def instantiate(term: Term, substitution: dict[int, Term]) -> Term:
    if term[0] == "v":
        return substitution[term[1]]
    return function(
        term[1],
        *(instantiate(argument, substitution) for argument in term[2]),
    )


def generated_pair(rng: random.Random, index: int) -> tuple[Clause, Clause]:
    variables = tuple(variable(position) for position in range(3))
    side_count = rng.randint(1, 4)
    side = tuple(
        (
            bool(rng.getrandbits(1)),
            random_pattern_term(rng, variables),
            random_pattern_term(rng, variables),
        )
        for _ in range(side_count)
    )
    substitution = {
        variable_index: random_ground_term(rng)
        for variable_index in range(3)
    }
    mode = index % 4
    main: list[Literal] = []
    resolution_target = rng.randrange(side_count)
    for source, literal in enumerate(side):
        sign = literal[0]
        if mode == 1 and source == resolution_target:
            sign = not sign
        if mode == 2 and source % 2:
            substitution[source % 3] = random_ground_term(rng)
        main.append(
            (
                sign,
                instantiate(literal[1], substitution),
                instantiate(literal[2], substitution),
            )
        )
    for _ in range(rng.randint(0, 2)):
        main.append(
            (
                bool(rng.getrandbits(1)),
                random_ground_term(rng),
                random_ground_term(rng),
            )
        )
    if mode == 3:
        rng.shuffle(main)
        if main:
            main[rng.randrange(len(main))] = (
                bool(rng.getrandbits(1)),
                random_ground_term(rng),
                random_ground_term(rng),
            )
    return side, tuple(main)


def validate_case(
    side: Clause, main: Clause, *, corrupt_expected: bool = False
) -> tuple[bool, bool]:
    expected_subsumption = brute_subsumes(side, main)
    expected_resolution = brute_resolution(side, main)
    if corrupt_expected:
        expected_subsumption = not expected_subsumption
    actual_subsumption = dpll(ordinary_cnf(side, main))
    actual_resolution = dpll(resolution_cnf(side, main))
    if (
        actual_subsumption != expected_subsumption
        or actual_resolution != expected_resolution
    ):
        raise AssertionError(
            json.dumps(
                {
                    "side": side,
                    "main": main,
                    "expected_subsumption": expected_subsumption,
                    "actual_subsumption": actual_subsumption,
                    "expected_resolution": expected_resolution,
                    "actual_resolution": actual_resolution,
                },
                default=list,
                sort_keys=True,
            )
        )
    return actual_subsumption, actual_resolution


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=0x5A75_5B5)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--corrupt-expected", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.cases < 1:
        raise ValueError("--cases must be positive")
    rng = random.Random(arguments.seed)
    subsumption_true = 0
    resolution_true = 0
    for index in range(arguments.cases):
        side, main = generated_pair(rng, index)
        subsumption, resolution = validate_case(
            side,
            main,
            corrupt_expected=arguments.corrupt_expected and index == 0,
        )
        subsumption_true += int(subsumption)
        resolution_true += int(resolution)
    report = {
        "schema_version": 1,
        "cases": arguments.cases,
        "seed": arguments.seed,
        "subsumption_true": subsumption_true,
        "resolution_true": resolution_true,
        "status": "passed",
    }
    rendered = json.dumps(report, sort_keys=True, separators=(",", ":"))
    if arguments.output:
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
