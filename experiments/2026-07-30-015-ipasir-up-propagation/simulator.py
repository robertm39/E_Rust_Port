#!/usr/bin/env python3
"""Deterministic IPASIR-UP-style external propagation simulation."""

from __future__ import annotations

import dataclasses
import enum
import hashlib
import itertools
import json
import random
import time
from dataclasses import dataclass
from typing import Any, Sequence

FROZEN_SEED = 0x4950415349525550


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def semantic_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


@dataclass(frozen=True, slots=True)
class Case:
    case_id: str
    pigeons: int
    holes: int
    atom_to_pair: tuple[tuple[int, int], ...]
    clauses: tuple[tuple[int, ...], ...]
    expected: bool
    family: str

    @property
    def variables(self) -> int:
        return len(self.atom_to_pair) - 1

    @property
    def observed(self) -> frozenset[int]:
        return frozenset(range(1, self.variables + 1))

    def groups(self) -> tuple[tuple[str, int, tuple[int, ...]], ...]:
        pigeon_groups = [
            (
                "pigeon",
                pigeon,
                tuple(
                    variable
                    for variable in range(1, self.variables + 1)
                    if self.atom_to_pair[variable][0] == pigeon
                ),
            )
            for pigeon in range(self.pigeons)
        ]
        hole_groups = [
            (
                "hole",
                hole,
                tuple(
                    variable
                    for variable in range(1, self.variables + 1)
                    if self.atom_to_pair[variable][1] == hole
                ),
            )
            for hole in range(self.holes)
        ]
        return tuple(pigeon_groups + hole_groups)

    def theory_clauses(self) -> tuple[tuple[int, int], ...]:
        clauses: set[tuple[int, int]] = set()
        for _, _, variables in self.groups():
            for left, right in itertools.combinations(variables, 2):
                clauses.add(tuple(sorted((-left, -right))))
        return tuple(sorted(clauses))

    def describe(self) -> Any:
        return {
            "case_id": self.case_id,
            "pigeons": self.pigeons,
            "holes": self.holes,
            "atom_to_pair": [list(pair) for pair in self.atom_to_pair],
            "clauses": [list(clause) for clause in self.clauses],
            "expected": self.expected,
            "family": self.family,
        }


def make_case(
    case_id: str,
    pigeons: int,
    holes: int,
    permutation: Sequence[int],
    *,
    family: str,
) -> Case:
    pairs = [
        (pigeon, hole)
        for pigeon in range(pigeons)
        for hole in range(holes)
    ]
    if sorted(permutation) != list(range(1, len(pairs) + 1)):
        raise ValueError("permutation is not a complete atom renaming")
    pair_to_atom = {
        pair: permutation[index] for index, pair in enumerate(pairs)
    }
    atom_to_pair = [(-1, -1)] * (len(pairs) + 1)
    for pair, atom in pair_to_atom.items():
        atom_to_pair[atom] = pair
    clauses = tuple(
        tuple(sorted(pair_to_atom[(pigeon, hole)] for hole in range(holes)))
        for pigeon in range(pigeons)
    )
    return Case(
        case_id,
        pigeons,
        holes,
        tuple(atom_to_pair),
        clauses,
        pigeons <= holes,
        family,
    )


def hand_cases() -> tuple[Case, ...]:
    return (
        make_case("hand-sat-2-2", 2, 2, (1, 2, 3, 4), family="hand_sat"),
        make_case(
            "hand-unsat-3-2",
            3,
            2,
            (1, 2, 3, 4, 5, 6),
            family="hand_unsat",
        ),
        make_case(
            "hand-propagation-3-3",
            3,
            3,
            tuple(range(1, 10)),
            family="hand_propagation",
        ),
    )


def generated_cases(
    seed: int = FROZEN_SEED, count: int = 100
) -> tuple[Case, ...]:
    if count < 2 or count % 2:
        raise ValueError("case count must be a positive even number")
    rng = random.Random(seed)
    cases: list[Case] = []
    for index in range(count // 2):
        for pigeons, holes, family in (
            (4, 4, "sat_4_4"),
            (4, 3, "unsat_4_3"),
        ):
            permutation = list(range(1, pigeons * holes + 1))
            rng.shuffle(permutation)
            cases.append(
                make_case(
                    f"generated-{family}-{index:03d}",
                    pigeons,
                    holes,
                    permutation,
                    family=family,
                )
            )
    return tuple(cases)


def exhaustive_oracle(case: Case) -> bool:
    return any(
        len(set(placement)) == case.pigeons
        for placement in itertools.product(range(case.holes), repeat=case.pigeons)
    )


class Treatment(str, enum.Enum):
    LAZY = "lazy"
    CONFLICT = "conflict"
    PROPAGATE = "propagate"
    ENCODED = "encoded"


@dataclass(frozen=True, slots=True)
class TrailEntry:
    literal: int
    level: int
    source: str

    def describe(self) -> Any:
        return [self.literal, self.level, self.source]


@dataclass(frozen=True, slots=True)
class LearnEvent:
    kind: str
    clause: tuple[int, ...]
    propagated: int | None
    assignment: tuple[tuple[int, bool], ...]
    trail: tuple[TrailEntry, ...]

    def describe(self) -> Any:
        return {
            "kind": self.kind,
            "clause": list(self.clause),
            "propagated": self.propagated,
            "assignment": [[var, value] for var, value in self.assignment],
            "trail": [entry.describe() for entry in self.trail],
        }


@dataclass(slots=True)
class Metrics:
    steps: int = 0
    decisions: int = 0
    assignments: int = 0
    cnf_propagations: int = 0
    theory_callbacks: int = 0
    theory_propagations: int = 0
    theory_conflicts: int = 0
    learned_clauses: int = 0
    duplicate_clauses: int = 0
    root_backtracks: int = 0
    restarts: int = 0
    maximum_depth: int = 0
    elapsed_seconds: float = 0.0

    def semantic(self) -> Any:
        result = dataclasses.asdict(self)
        result.pop("elapsed_seconds")
        return result


@dataclass(frozen=True, slots=True)
class Outcome:
    treatment: Treatment
    decision: bool | None
    reason: str
    model: tuple[int, ...] | None
    metrics: Metrics
    learned: tuple[tuple[int, ...], ...]
    events: tuple[Any, ...]
    semantic_sha256: str


def literal_value(literal: int, assignment: dict[int, bool]) -> bool | None:
    value = assignment.get(abs(literal))
    if value is None:
        return None
    return value if literal > 0 else not value


def first_theory_conflict(
    case: Case, assignment: dict[int, bool]
) -> tuple[int, int] | None:
    for _, _, variables in case.groups():
        true_variables = [
            variable for variable in variables if assignment.get(variable) is True
        ]
        if len(true_variables) > 1:
            return true_variables[0], true_variables[1]
    return None


def first_theory_propagation(
    case: Case, assignment: dict[int, bool]
) -> tuple[int, int] | None:
    for _, _, variables in case.groups():
        true_variables = [
            variable for variable in variables if assignment.get(variable) is True
        ]
        if len(true_variables) != 1:
            continue
        for variable in variables:
            if variable not in assignment:
                return true_variables[0], variable
    return None


def validate_reason(case: Case, event: LearnEvent) -> bool:
    if len(event.clause) != 2 or any(literal >= 0 for literal in event.clause):
        return False
    left, right = (abs(literal) for literal in event.clause)
    if left == right or left not in case.observed or right not in case.observed:
        return False
    left_pair = case.atom_to_pair[left]
    right_pair = case.atom_to_pair[right]
    if left_pair[0] != right_pair[0] and left_pair[1] != right_pair[1]:
        return False
    assignment = dict(event.assignment)
    values = [literal_value(literal, assignment) for literal in event.clause]
    if event.kind == "conflict":
        return event.propagated is None and values == [False, False]
    if event.kind != "propagation" or event.propagated not in event.clause:
        return False
    propagated_index = event.clause.index(event.propagated)
    other_index = 1 - propagated_index
    return values[propagated_index] is None and values[other_index] is False


def validate_root_backtrack(
    event: LearnEvent,
    target: int,
    post_trail: Sequence[TrailEntry],
) -> bool:
    if target != 0 or post_trail:
        return False
    reconstructed: dict[int, bool] = {}
    previous_level = 0
    for entry in event.trail:
        if entry.level < previous_level:
            return False
        previous_level = entry.level
        variable = abs(entry.literal)
        value = entry.literal > 0
        if variable in reconstructed and reconstructed[variable] != value:
            return False
        reconstructed[variable] = value
    return tuple(sorted(reconstructed.items())) == event.assignment


def validate_model(case: Case, model: Sequence[int]) -> bool:
    assignment = {abs(literal): literal > 0 for literal in model}
    if len(assignment) != case.variables:
        return False
    if any(not any(literal_value(lit, assignment) for lit in clause) for clause in case.clauses):
        return False
    return first_theory_conflict(case, assignment) is None


class Simulator:
    def __init__(self, case: Case, treatment: Treatment, max_steps: int) -> None:
        self.case = case
        self.treatment = treatment
        self.max_steps = max_steps
        self.metrics = Metrics()
        self.learned: list[tuple[int, ...]] = []
        self.learned_set: set[tuple[int, ...]] = set()
        self.events: list[Any] = []
        self.base_clauses = list(case.clauses)
        if treatment is Treatment.ENCODED:
            self.base_clauses.extend(case.theory_clauses())

    def tick(self) -> None:
        self.metrics.steps += 1
        if self.metrics.steps > self.max_steps:
            raise RuntimeError("step limit exceeded")

    @staticmethod
    def assign(
        assignment: dict[int, bool],
        trail: list[TrailEntry],
        literal: int,
        level: int,
        source: str,
    ) -> bool:
        variable = abs(literal)
        value = literal > 0
        previous = assignment.get(variable)
        if previous is not None:
            return previous == value
        assignment[variable] = value
        trail.append(TrailEntry(literal, level, source))
        return True

    def theory_event(
        self,
        assignment: dict[int, bool],
        trail: list[TrailEntry],
        *,
        complete: bool,
    ) -> LearnEvent | None:
        if self.treatment is Treatment.ENCODED:
            return None
        if self.treatment is Treatment.LAZY and not complete:
            return None
        self.metrics.theory_callbacks += 1
        conflict = first_theory_conflict(self.case, assignment)
        if conflict is not None:
            self.metrics.theory_conflicts += 1
            clause = tuple(sorted((-conflict[0], -conflict[1])))
            return LearnEvent(
                "conflict",
                clause,
                None,
                tuple(sorted(assignment.items())),
                tuple(trail),
            )
        if self.treatment is Treatment.PROPAGATE and not complete:
            propagation = first_theory_propagation(self.case, assignment)
            if propagation is not None:
                antecedent, variable = propagation
                propagated = -variable
                self.metrics.theory_propagations += 1
                return LearnEvent(
                    "propagation",
                    tuple(sorted((-antecedent, propagated))),
                    propagated,
                    tuple(sorted(assignment.items())),
                    tuple(trail),
                )
        return None

    def dfs(
        self,
        assignment: dict[int, bool],
        trail: list[TrailEntry],
        level: int,
    ) -> tuple[str, Any]:
        self.tick()
        clauses = (*self.base_clauses, *self.learned)
        changed = True
        while changed:
            changed = False
            for clause in clauses:
                values = [literal_value(literal, assignment) for literal in clause]
                if any(value is True for value in values):
                    continue
                unassigned = [
                    literal
                    for literal, value in zip(clause, values, strict=True)
                    if value is None
                ]
                if not unassigned:
                    return "branch_unsat", None
                if len(unassigned) == 1:
                    if not self.assign(
                        assignment, trail, unassigned[0], level, "cnf"
                    ):
                        return "branch_unsat", None
                    self.metrics.assignments += 1
                    self.metrics.cnf_propagations += 1
                    changed = True

        complete = len(assignment) == self.case.variables
        event = self.theory_event(assignment, trail, complete=complete)
        if event is not None:
            return "learn", event
        if complete:
            model = tuple(
                variable if assignment[variable] else -variable
                for variable in range(1, self.case.variables + 1)
            )
            return "sat", model

        variable = next(
            variable
            for variable in range(1, self.case.variables + 1)
            if variable not in assignment
        )
        self.metrics.decisions += 1
        self.metrics.maximum_depth = max(self.metrics.maximum_depth, level + 1)
        for value in (False, True):
            child_assignment = dict(assignment)
            child_trail = list(trail)
            literal = variable if value else -variable
            self.assign(child_assignment, child_trail, literal, level + 1, "decision")
            self.metrics.assignments += 1
            result, payload = self.dfs(child_assignment, child_trail, level + 1)
            if result in {"sat", "learn"}:
                return result, payload
        return "branch_unsat", None

    def run(self) -> Outcome:
        started = time.perf_counter()
        try:
            while True:
                result, payload = self.dfs({}, [], 0)
                if result == "sat":
                    model = payload
                    if not validate_model(self.case, model):
                        raise AssertionError("returned model failed independent validation")
                    decision = True
                    reason = "complete model"
                    break
                if result == "branch_unsat":
                    model = None
                    decision = False
                    reason = "Boolean search exhausted"
                    break
                event: LearnEvent = payload
                if not validate_reason(self.case, event):
                    raise AssertionError("external reason failed independent replay")
                if not validate_root_backtrack(event, 0, ()):
                    raise AssertionError("root backtrack failed independent replay")
                clause = tuple(sorted(event.clause))
                if clause in self.learned_set or clause in self.base_clauses:
                    self.metrics.duplicate_clauses += 1
                    raise AssertionError("external callback repeated an existing clause")
                self.learned_set.add(clause)
                self.learned.append(clause)
                self.metrics.learned_clauses += 1
                self.metrics.root_backtracks += 1
                self.metrics.restarts += 1
                self.events.append(
                    {
                        "callback": event.describe(),
                        "backtrack": {
                            "from_level": max(
                                (entry.level for entry in event.trail), default=0
                            ),
                            "to_level": 0,
                            "pre_trail": [
                                entry.describe() for entry in event.trail
                            ],
                            "post_trail": [],
                        },
                    }
                )
        except RuntimeError as error:
            model = None
            decision = None
            reason = str(error)
        self.metrics.elapsed_seconds = time.perf_counter() - started
        semantic = {
            "treatment": self.treatment.value,
            "decision": decision,
            "reason": reason,
            "model": model,
            "metrics": self.metrics.semantic(),
            "learned": self.learned,
            "events": self.events,
        }
        return Outcome(
            self.treatment,
            decision,
            reason,
            model,
            self.metrics,
            tuple(self.learned),
            tuple(self.events),
            semantic_hash(semantic),
        )


def run_case(
    case: Case, treatment: Treatment | str, max_steps: int = 1_000_000
) -> Outcome:
    selected = treatment if isinstance(treatment, Treatment) else Treatment(treatment)
    return Simulator(case, selected, max_steps).run()
