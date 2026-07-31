#!/usr/bin/env python3
"""Executable model of a deletion-aware persistent SATCheck session."""

from __future__ import annotations

from dataclasses import dataclass
from itertools import product
from typing import Iterable, Mapping, TypeAlias


SourceId: TypeAlias = int


@dataclass(frozen=True, order=True)
class AtomKey:
    """Complete structural identity for one logical SAT atom."""

    symbol: str
    sort: str = "$i"
    arguments: tuple["AtomKey", ...] = ()


@dataclass(frozen=True, order=True)
class Literal:
    atom: AtomKey
    positive: bool = True


CanonicalClause: TypeAlias = tuple[Literal, ...]
Snapshot: TypeAlias = Mapping[SourceId, Iterable[Literal]]


@dataclass(frozen=True, order=True)
class ClauseKey:
    source: SourceId
    literals: CanonicalClause


@dataclass(frozen=True)
class ClauseRecord:
    key: ClauseKey
    selector: int
    guarded_clause: tuple[int, ...]


@dataclass(frozen=True)
class Transition:
    epoch: int
    rebuilt: bool
    retained: int
    reactivated: int
    added: int
    retired: int


@dataclass(frozen=True)
class SolveResult:
    epoch: int
    satisfiable: bool
    active_sources: tuple[SourceId, ...]
    core_sources: tuple[SourceId, ...]


@dataclass(frozen=True)
class FailurePlan:
    fail_reset: bool = False
    fail_add_after: int | None = None


class ModelError(RuntimeError):
    """Base error for invalid or failed model transitions."""


class SessionPoisoned(ModelError):
    """Raised when a partial backend mutation prohibits solving."""


class InjectedBackendFailure(ModelError):
    """Raised by a frozen failure-injection point."""


class CapacityError(ModelError):
    """Raised when one active snapshot cannot fit the variable namespace."""


def atom(symbol: str, *arguments: AtomKey, sort: str = "$i") -> AtomKey:
    return AtomKey(symbol=symbol, sort=sort, arguments=tuple(arguments))


def positive(key: AtomKey) -> Literal:
    return Literal(key, True)


def negative(key: AtomKey) -> Literal:
    return Literal(key, False)


def canonicalize_clause(literals: Iterable[Literal]) -> CanonicalClause | None:
    """Return a sorted set-like clause, or ``None`` for a tautology."""

    polarity_by_atom: dict[AtomKey, bool] = {}
    for literal in literals:
        previous = polarity_by_atom.get(literal.atom)
        if previous is not None and previous != literal.positive:
            return None
        polarity_by_atom[literal.atom] = literal.positive
    return tuple(
        Literal(key, polarity_by_atom[key]) for key in sorted(polarity_by_atom)
    )


def canonical_snapshot(snapshot: Snapshot) -> dict[SourceId, ClauseKey]:
    result: dict[SourceId, ClauseKey] = {}
    for source, literals in snapshot.items():
        clause = canonicalize_clause(literals)
        if clause is not None:
            result[source] = ClauseKey(source, clause)
    return result


def fresh_satisfiable(snapshot: Snapshot) -> bool:
    """Independent truth-table oracle for the current logical snapshot."""

    clauses = [
        clause
        for literals in snapshot.values()
        if (clause := canonicalize_clause(literals)) is not None
    ]
    atoms = sorted({literal.atom for clause in clauses for literal in clause})
    for values in product((False, True), repeat=len(atoms)):
        assignment = dict(zip(atoms, values, strict=True))
        if all(
            any(assignment[literal.atom] == literal.positive for literal in clause)
            for clause in clauses
        ):
            return True
    return False


class PersistentSatModel:
    """Persistent guarded-clause database with fail-closed epoch rebuilds."""

    def __init__(
        self,
        *,
        variable_cap: int = 1_000_000,
        minimum_permanent_limit: int = 8,
        permanent_factor: int = 3,
        minimum_retired_limit: int = 4,
        retired_factor: int = 2,
    ) -> None:
        if variable_cap < 1:
            raise ValueError("variable_cap must be positive")
        self.variable_cap = variable_cap
        self.minimum_permanent_limit = minimum_permanent_limit
        self.permanent_factor = permanent_factor
        self.minimum_retired_limit = minimum_retired_limit
        self.retired_factor = retired_factor
        self.epoch = 0
        self.context: str | None = None
        self.poisoned = False
        self.next_variable = 1
        self.atom_variables: dict[AtomKey, int] = {}
        self.records: dict[ClauseKey, ClauseRecord] = {}
        self.active_by_source: dict[SourceId, ClauseKey] = {}
        self.backend_clauses: list[tuple[int, ...]] = []

    @property
    def permanent_clause_count(self) -> int:
        return len(self.backend_clauses)

    @property
    def active_clause_count(self) -> int:
        return len(self.active_by_source)

    @property
    def retired_clause_count(self) -> int:
        return len(self.records) - len(set(self.active_by_source.values()))

    def permanent_limit(self, active: int | None = None) -> int:
        count = self.active_clause_count if active is None else active
        return max(self.minimum_permanent_limit, self.permanent_factor * max(count, 1))

    def retired_limit(self, active: int | None = None) -> int:
        count = self.active_clause_count if active is None else active
        return max(self.minimum_retired_limit, self.retired_factor * count)

    def reconcile(
        self,
        snapshot: Snapshot,
        *,
        context: str,
        failure: FailurePlan = FailurePlan(),
    ) -> Transition:
        target = canonical_snapshot(snapshot)
        target_keys = set(target.values())
        retained = sum(
            self.active_by_source.get(source) == key
            for source, key in target.items()
        )
        reactivated = sum(
            key in self.records and self.active_by_source.get(source) != key
            for source, key in target.items()
        )
        added_keys = sorted(target_keys.difference(self.records))
        retired = len(set(self.active_by_source.values()).difference(target_keys))

        projected_permanent = len(self.records) + len(added_keys)
        projected_retired = projected_permanent - len(target_keys)
        missing_atoms = {
            literal.atom
            for key in added_keys
            for literal in key.literals
            if literal.atom not in self.atom_variables
        }
        variables_needed = len(missing_atoms) + len(added_keys)
        variable_pressure = (
            self.next_variable + variables_needed - 1 > self.variable_cap
        )
        rebuild = (
            self.context != context
            or self.poisoned
            or (not target and bool(self.records))
            or projected_permanent > self.permanent_limit(len(target_keys))
            or projected_retired > self.retired_limit(len(target_keys))
            or variable_pressure
        )

        if rebuild:
            self._rebuild(target, context=context, failure=failure)
            transition = Transition(
                epoch=self.epoch,
                rebuilt=True,
                retained=retained,
                reactivated=0,
                added=len(target_keys),
                retired=retired,
            )
        else:
            self._incremental_apply(target, added_keys, failure=failure)
            self.context = context
            transition = Transition(
                epoch=self.epoch,
                rebuilt=False,
                retained=retained,
                reactivated=reactivated,
                added=len(added_keys),
                retired=retired,
            )
        self.assert_invariants()
        return transition

    def reset(
        self,
        *,
        context: str | None = None,
        failure: FailurePlan = FailurePlan(),
    ) -> None:
        self._reset_backend(failure)
        self.epoch += 1
        self.context = context
        self.poisoned = False
        self.next_variable = 1
        self.atom_variables.clear()
        self.records.clear()
        self.active_by_source.clear()

    def solve(self) -> SolveResult:
        if self.poisoned:
            raise SessionPoisoned("persistent SAT session requires a complete rebuild")
        active_sources = tuple(sorted(self.active_by_source))
        selectors = {
            self.records[self.active_by_source[source]].selector
            for source in active_sources
        }
        satisfiable = self._guarded_satisfiable(selectors)
        core_sources: tuple[SourceId, ...] = ()
        if not satisfiable:
            core = list(active_sources)
            index = 0
            while index < len(core):
                trial = core[:index] + core[index + 1 :]
                trial_selectors = {
                    self.records[self.active_by_source[source]].selector
                    for source in trial
                }
                if not self._guarded_satisfiable(trial_selectors):
                    core = trial
                else:
                    index += 1
            core_sources = tuple(core)
            core_snapshot = {
                source: self.active_by_source[source].literals
                for source in core_sources
            }
            if fresh_satisfiable(core_snapshot):
                self.poisoned = True
                raise SessionPoisoned("failed-selector source mapping is not UNSAT")
        return SolveResult(
            epoch=self.epoch,
            satisfiable=satisfiable,
            active_sources=active_sources,
            core_sources=core_sources,
        )

    def assert_invariants(self) -> None:
        if self.poisoned:
            return
        atom_variables = set(self.atom_variables.values())
        selector_variables = {record.selector for record in self.records.values()}
        if atom_variables.intersection(selector_variables):
            raise AssertionError("atom and selector namespaces overlap")
        if len(selector_variables) != len(self.records):
            raise AssertionError("selector variables are not unique")
        if len(self.backend_clauses) != len(self.records):
            raise AssertionError("backend and clause-record cardinalities differ")
        if set(self.active_by_source.values()).difference(self.records):
            raise AssertionError("active clause has no permanent record")
        if any(
            source != key.source for source, key in self.active_by_source.items()
        ):
            raise AssertionError("active source and clause key disagree")
        if any(
            record.guarded_clause[-1] != -record.selector
            for record in self.records.values()
        ):
            raise AssertionError("guarded clause does not end in its selector")
        if self.next_variable - 1 > self.variable_cap:
            raise AssertionError("allocated variable exceeds configured cap")
        if self.permanent_clause_count > self.permanent_limit():
            raise AssertionError("permanent database exceeds growth bound")
        if self.retired_clause_count > self.retired_limit():
            raise AssertionError("retired database exceeds growth bound")

    def selector_for_source(self, source: SourceId) -> int:
        key = self.active_by_source[source]
        return self.records[key].selector

    def atom_variable(self, key: AtomKey) -> int:
        return self.atom_variables[key]

    def _incremental_apply(
        self,
        target: dict[SourceId, ClauseKey],
        added_keys: list[ClauseKey],
        *,
        failure: FailurePlan,
    ) -> None:
        atom_variables = self.atom_variables.copy()
        records = self.records.copy()
        next_variable = self.next_variable
        pending: list[ClauseRecord] = []

        for key in added_keys:
            for literal in key.literals:
                if literal.atom not in atom_variables:
                    atom_variables[literal.atom] = next_variable
                    next_variable += 1
            selector = next_variable
            next_variable += 1
            guarded = tuple(
                atom_variables[literal.atom]
                if literal.positive
                else -atom_variables[literal.atom]
                for literal in key.literals
            ) + (-selector,)
            pending.append(ClauseRecord(key, selector, guarded))

        try:
            for added, record in enumerate(pending):
                if failure.fail_add_after == added:
                    raise InjectedBackendFailure("injected partial clause-add failure")
                self.backend_clauses.append(record.guarded_clause)
                records[record.key] = record
        except InjectedBackendFailure:
            self.poisoned = True
            raise

        self.atom_variables = atom_variables
        self.records = records
        self.next_variable = next_variable
        self.active_by_source = target

    def _rebuild(
        self,
        target: dict[SourceId, ClauseKey],
        *,
        context: str,
        failure: FailurePlan,
    ) -> None:
        atoms = sorted(
            {
                literal.atom
                for key in target.values()
                for literal in key.literals
            }
        )
        required = len(atoms) + len(target)
        if required > self.variable_cap:
            self.poisoned = True
            raise CapacityError(
                f"active snapshot needs {required} variables, cap is {self.variable_cap}"
            )

        self._reset_backend(failure)
        atom_variables = {
            key: index for index, key in enumerate(atoms, start=1)
        }
        next_variable = len(atom_variables) + 1
        records: dict[ClauseKey, ClauseRecord] = {}
        pending: list[ClauseRecord] = []
        for source in sorted(target):
            key = target[source]
            selector = next_variable
            next_variable += 1
            guarded = tuple(
                atom_variables[literal.atom]
                if literal.positive
                else -atom_variables[literal.atom]
                for literal in key.literals
            ) + (-selector,)
            pending.append(ClauseRecord(key, selector, guarded))

        try:
            for added, record in enumerate(pending):
                if failure.fail_add_after == added:
                    raise InjectedBackendFailure("injected rebuild clause-add failure")
                self.backend_clauses.append(record.guarded_clause)
                records[record.key] = record
        except InjectedBackendFailure:
            self.poisoned = True
            raise

        self.epoch += 1
        self.context = context
        self.poisoned = False
        self.atom_variables = atom_variables
        self.records = records
        self.active_by_source = target
        self.next_variable = next_variable

    def _reset_backend(self, failure: FailurePlan) -> None:
        if failure.fail_reset:
            self.poisoned = True
            raise InjectedBackendFailure("injected backend reset failure")
        self.backend_clauses.clear()

    def _guarded_satisfiable(self, active_selectors: set[int]) -> bool:
        active_clauses = [
            guarded[:-1]
            for guarded in self.backend_clauses
            if -guarded[-1] in active_selectors
        ]
        variables = sorted({abs(literal) for clause in active_clauses for literal in clause})
        for values in product((False, True), repeat=len(variables)):
            assignment = dict(zip(variables, values, strict=True))
            if all(
                any(assignment[abs(literal)] == (literal > 0) for literal in clause)
                for clause in active_clauses
            ):
                return True
        return False
