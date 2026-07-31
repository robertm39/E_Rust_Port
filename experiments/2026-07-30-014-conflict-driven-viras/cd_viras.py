#!/usr/bin/env python3
"""Bounded clean-room conflict-driven search over base-VIRAS candidates.

The candidate implementation is intentionally limited to the finite,
equality-guarded affine slice frozen in ``PREREGISTRATION.md``.  It imports the
paper-derived exact Experiment 004 kernel for candidate construction, while
learned-clause validation uses the separate Fourier-Motzkin implementation in
this file.
"""

from __future__ import annotations

import dataclasses
import enum
import hashlib
import json
import sys
import time
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable, Sequence

EXPERIMENT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_DIR.parents[1]
BASE_EXPERIMENT_DIR = (
    REPOSITORY_ROOT / "experiments" / "2026-07-30-004-base-viras-qe-prototype"
)
sys.path.insert(0, str(BASE_EXPERIMENT_DIR))

import prototype as base  # noqa: E402


class UnsupportedSlice(RuntimeError):
    """Raised when a formula leaves the preregistered finite affine slice."""


class ResourceLimit(RuntimeError):
    """Raised when a frozen experiment resource bound is exceeded."""


def canonical_json(value: Any) -> str:
    """Return deterministic compact JSON."""

    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def semantic_hash(value: Any) -> str:
    """Hash a semantic record deterministically."""

    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


@dataclass(frozen=True, slots=True)
class Affine:
    """Canonical rational affine expression."""

    coefficients: tuple[tuple[str, Fraction], ...]
    constant: Fraction

    @staticmethod
    def create(
        coefficients: dict[str, Fraction], constant: Fraction = Fraction(0)
    ) -> "Affine":
        return Affine(
            tuple(sorted((name, value) for name, value in coefficients.items() if value)),
            constant,
        )

    def scale(self, coefficient: Fraction) -> "Affine":
        return Affine.create(
            {name: coefficient * value for name, value in self.coefficients},
            coefficient * self.constant,
        )

    def add(self, other: "Affine") -> "Affine":
        coefficients = dict(self.coefficients)
        for name, value in other.coefficients:
            coefficients[name] = coefficients.get(name, Fraction(0)) + value
        return Affine.create(coefficients, self.constant + other.constant)


def affine_from_term(term: base.Term) -> Affine:
    """Extract an exact affine expression, rejecting floors."""

    if term.op is base.TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return Affine.create({}, term.value)
    if term.op is base.TermOp.VAR:
        assert isinstance(term.value, str)
        return Affine.create({term.value: Fraction(1)})
    if term.op is base.TermOp.ADD:
        result = Affine.create({})
        for child in term.args:
            result = result.add(affine_from_term(child))
        return result
    if term.op is base.TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return affine_from_term(term.args[0]).scale(term.value)
    raise UnsupportedSlice("floor terms are outside the finite affine slice")


@dataclass(frozen=True, slots=True)
class Inequality:
    """An affine expression constrained to be nonnegative or positive."""

    expression: Affine
    strict: bool

    def describe(self) -> Any:
        return {
            "coefficients": [
                [name, fraction_text(value)]
                for name, value in self.expression.coefficients
            ],
            "constant": fraction_text(self.expression.constant),
            "strict": self.strict,
        }


def fraction_text(value: Fraction) -> str:
    """Render an exact rational deterministically."""

    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def _normalize_inequality(inequality: Inequality) -> Inequality:
    values = [value for _, value in inequality.expression.coefficients]
    if values:
        divisor = abs(values[0])
    elif inequality.expression.constant:
        divisor = abs(inequality.expression.constant)
    else:
        divisor = Fraction(1)
    return Inequality(inequality.expression.scale(Fraction(1, 1) / divisor), inequality.strict)


def _constant_truth(inequality: Inequality) -> bool | None:
    if inequality.expression.coefficients:
        return None
    value = inequality.expression.constant
    return value > 0 if inequality.strict else value >= 0


def _simplify_inequalities(
    inequalities: Iterable[Inequality],
) -> tuple[tuple[Inequality, ...], bool]:
    """Deduplicate constraints and report an immediate contradiction."""

    strongest: dict[Affine, bool] = {}
    for raw in inequalities:
        inequality = _normalize_inequality(raw)
        truth = _constant_truth(inequality)
        if truth is False:
            return (), False
        if truth is True:
            continue
        strongest[inequality.expression] = (
            strongest.get(inequality.expression, False) or inequality.strict
        )
    ordered = tuple(
        sorted(
            (
                Inequality(expression, strict)
                for expression, strict in strongest.items()
            ),
            key=lambda item: canonical_json(item.describe()),
        )
    )
    return ordered, True


def literal_inequalities(literal: base.Literal) -> tuple[Inequality, ...]:
    """Translate one supported literal to conjunctive affine inequalities."""

    expression = affine_from_term(literal.term)
    if literal.relation is base.Relation.EQ:
        return (
            Inequality(expression, False),
            Inequality(expression.scale(Fraction(-1)), False),
        )
    if literal.relation is base.Relation.GE:
        return (Inequality(expression, False),)
    if literal.relation is base.Relation.GT:
        return (Inequality(expression, True),)
    raise UnsupportedSlice("disequality is outside the convex affine checker")


@dataclass(frozen=True, slots=True)
class ClauseComponent:
    """One plain blocking atom ``variable != term``."""

    variable: str
    term: base.Term

    def describe(self) -> Any:
        return {"variable": self.variable, "term": self.term.describe()}


@dataclass(frozen=True, slots=True)
class Decision:
    """A finite virtual assignment and its complete generation context."""

    variable: str
    term: base.Term
    origin_index: int
    origin_kind: str
    prefix_depth: int

    @property
    def component(self) -> ClauseComponent:
        return ClauseComponent(self.variable, self.term)

    def describe(self) -> Any:
        return {
            "variable": self.variable,
            "term": self.term.describe(),
            "origin_index": self.origin_index,
            "origin_kind": self.origin_kind,
            "prefix_depth": self.prefix_depth,
        }


def component_inequalities(component: ClauseComponent) -> tuple[Inequality, ...]:
    """Translate the negation of a blocking atom, ``x = term``."""

    difference = base.subtract(base.variable(component.variable), component.term)
    expression = affine_from_term(difference)
    return (
        Inequality(expression, False),
        Inequality(expression.scale(Fraction(-1)), False),
    )


@dataclass(slots=True)
class AffineCheck:
    """Result and work count for exact affine feasibility."""

    feasible: bool
    combinations: int
    peak_constraints: int


def affine_feasible(
    literals: Sequence[base.Literal],
    components: Sequence[ClauseComponent] = (),
    *,
    max_combinations: int = 100_000,
) -> AffineCheck:
    """Decide exact rational affine feasibility by Fourier-Motzkin elimination."""

    constraints: list[Inequality] = []
    for literal in literals:
        constraints.extend(literal_inequalities(literal))
    for component in components:
        constraints.extend(component_inequalities(component))
    current, consistent = _simplify_inequalities(constraints)
    peak = len(current)
    if not consistent:
        return AffineCheck(False, 0, peak)

    variables = sorted(
        {
            name
            for inequality in current
            for name, _ in inequality.expression.coefficients
        }
    )
    combinations = 0
    for variable in variables:
        positive: list[tuple[Fraction, Inequality]] = []
        negative: list[tuple[Fraction, Inequality]] = []
        zero: list[Inequality] = []
        for inequality in current:
            coefficients = dict(inequality.expression.coefficients)
            coefficient = coefficients.pop(variable, Fraction(0))
            remainder = Inequality(
                Affine.create(coefficients, inequality.expression.constant),
                inequality.strict,
            )
            if coefficient > 0:
                positive.append((coefficient, remainder))
            elif coefficient < 0:
                negative.append((coefficient, remainder))
            else:
                zero.append(remainder)

        generated = list(zero)
        for positive_coefficient, lower in positive:
            for negative_coefficient, upper in negative:
                combinations += 1
                if combinations > max_combinations:
                    raise ResourceLimit(
                        "affine-combination limit exceeded: "
                        f"{combinations}>{max_combinations}"
                    )
                expression = lower.expression.scale(-negative_coefficient).add(
                    upper.expression.scale(positive_coefficient)
                )
                generated.append(
                    Inequality(expression, lower.strict or upper.strict)
                )
        current, consistent = _simplify_inequalities(generated)
        peak = max(peak, len(current))
        if not consistent:
            return AffineCheck(False, combinations, peak)
    return AffineCheck(True, combinations, peak)


def clause_soundness(
    original: Sequence[base.Literal],
    components: Sequence[ClauseComponent],
    *,
    max_combinations: int = 100_000,
) -> AffineCheck:
    """Prove ``F -> OR(x_i != t_i)`` by refuting its negation."""

    check = affine_feasible(
        original, components, max_combinations=max_combinations
    )
    return AffineCheck(not check.feasible, check.combinations, check.peak_constraints)


def clause_progress(
    components: Sequence[ClauseComponent], prefix: Sequence[Decision]
) -> bool:
    """Check that a rejecting stack makes a learned disjunction ground false."""

    for component in components:
        difference = base.subtract(base.variable(component.variable), component.term)
        for decision in prefix:
            difference = base.substitute(
                difference, decision.variable, decision.term
            )
        if difference.variables() or base.evaluate_term(difference, {}) != 0:
            return False
    return True


def clause_false_under(
    components: Sequence[ClauseComponent], prefix: Sequence[Decision]
) -> bool:
    """Return whether a learned clause is trivially false under a prefix."""

    return clause_progress(components, prefix)


@dataclass(frozen=True, slots=True)
class LearnedClause:
    """A validated learned blocking disjunction."""

    components: tuple[ClauseComponent, ...]
    kind: str
    source_index: int | None
    soundness_combinations: int
    peak_constraints: int

    @property
    def key(self) -> str:
        return canonical_json([component.describe() for component in self.components])

    def describe(self) -> Any:
        return {
            "components": [component.describe() for component in self.components],
            "kind": self.kind,
            "source_index": self.source_index,
            "soundness": True,
            "progress": True,
            "soundness_combinations": self.soundness_combinations,
            "peak_constraints": self.peak_constraints,
        }


@dataclass(frozen=True, slots=True)
class TrackedLiteral:
    """An original literal paired with its current residual."""

    origin_index: int
    original: base.Literal
    residual: base.Literal

    def describe(self) -> Any:
        return {
            "origin_index": self.origin_index,
            "original": self.original.describe(),
            "residual": self.residual.describe(),
        }


@dataclass(slots=True)
class Metrics:
    """Treatment counters."""

    candidate_generations: int = 0
    virtual_substitutions: int = 0
    explored_leaves: int = 0
    learned_generated: int = 0
    learned_inserted: int = 0
    admissibility_prunes: int = 0
    affine_combinations: int = 0
    affine_peak_constraints: int = 0
    peak_learned_clauses: int = 0
    steps: int = 0
    elapsed_seconds: float = 0.0

    def semantic_description(self) -> Any:
        result = dataclasses.asdict(self)
        result.pop("elapsed_seconds")
        return result


class Treatment(str, enum.Enum):
    EAGER = "eager"
    BASIC = "basic"
    FOCUSED = "focused"


@dataclass(frozen=True, slots=True)
class SearchOutcome:
    """One treatment result."""

    treatment: Treatment
    supported: bool
    decision: bool | None
    reason: str
    metrics: Metrics
    clauses: tuple[LearnedClause, ...]
    trace: tuple[Any, ...]
    semantic_trace_sha256: str

    def summary(self) -> Any:
        return {
            "treatment": self.treatment.value,
            "supported": self.supported,
            "decision": self.decision,
            "reason": self.reason,
            "metrics": dataclasses.asdict(self.metrics),
            "clauses": [clause.describe() for clause in self.clauses],
            "semantic_trace_sha256": self.semantic_trace_sha256,
        }


class Search:
    """One deterministic candidate-tree execution."""

    def __init__(
        self,
        original: Sequence[base.Literal],
        treatment: Treatment,
        *,
        max_steps: int = 1_000_000,
        max_affine_combinations: int = 100_000,
    ) -> None:
        self.original = tuple(original)
        self.treatment = treatment
        self.max_steps = max_steps
        self.max_affine_combinations = max_affine_combinations
        self.metrics = Metrics()
        self.learned: list[LearnedClause] = []
        self.learned_keys: set[str] = set()
        self.trace: list[Any] = []

    def _event(self, rule: str, **values: Any) -> None:
        self.metrics.steps += 1
        if self.metrics.steps > self.max_steps:
            raise ResourceLimit(
                f"search-step limit exceeded: {self.metrics.steps}>{self.max_steps}"
            )
        self.trace.append({"rule": rule, **values})

    @staticmethod
    def _state(
        residuals: Sequence[TrackedLiteral],
    ) -> tuple[str, TrackedLiteral | None]:
        active = False
        for tracked in residuals:
            variables = tracked.residual.variables()
            if variables:
                active = True
                continue
            if not tracked.residual.relation.evaluate(
                base.evaluate_term(tracked.residual.term, {})
            ):
                return "conflict", tracked
        return ("active", None) if active else ("success", None)

    @staticmethod
    def _variable(residuals: Sequence[TrackedLiteral]) -> str:
        variables: set[str] = set()
        for tracked in residuals:
            variables.update(tracked.residual.variables())
        if not variables:
            raise AssertionError("active residual has no variables")
        return min(variables)

    def _candidates(
        self,
        residuals: Sequence[TrackedLiteral],
        variable: str,
        prefix: Sequence[Decision],
    ) -> tuple[Decision, ...]:
        kernel = base.Kernel()
        candidates: list[Decision] = []
        seen: set[base.Term] = set()
        for residual_index, tracked in enumerate(residuals):
            literal = tracked.residual
            if (
                literal.relation is not base.Relation.EQ
                or variable not in literal.variables()
            ):
                continue
            profile = kernel.profile(literal.term, variable)
            if profile.segment_slope == 0:
                continue
            for candidate in kernel.literal_candidates(
                literal, variable, residual_index
            ):
                virtual = candidate.virtual
                if (
                    virtual.epsilon
                    or virtual.infinity is not None
                    or virtual.grid_period is not None
                ):
                    raise UnsupportedSlice(
                        "equality-origin candidate is not a plain finite term"
                    )
                affine_from_term(virtual.base)
                if variable in virtual.base.variables():
                    raise AssertionError("candidate retains eliminated variable")
                if virtual.base in seen:
                    continue
                seen.add(virtual.base)
                candidates.append(
                    Decision(
                        variable,
                        virtual.base,
                        tracked.origin_index,
                        candidate.origin_kind,
                        len(prefix),
                    )
                )
        if not candidates:
            raise UnsupportedSlice(
                f"selected variable {variable!r} has no finite equality candidate"
            )
        self.metrics.candidate_generations += len(candidates)
        self._event(
            "Decide",
            variable=variable,
            prefix=[decision.describe() for decision in prefix],
            residual=[tracked.describe() for tracked in residuals],
            candidates=[candidate.describe() for candidate in candidates],
        )
        return tuple(candidates)

    @staticmethod
    def _substitute(
        residuals: Sequence[TrackedLiteral], decision: Decision
    ) -> tuple[TrackedLiteral, ...]:
        return tuple(
            TrackedLiteral(
                tracked.origin_index,
                tracked.original,
                base.Literal(
                    base.substitute(
                        tracked.residual.term, decision.variable, decision.term
                    ),
                    tracked.residual.relation,
                ),
            )
            for tracked in residuals
        )

    @staticmethod
    def _support_components(
        conflict: TrackedLiteral, prefix: Sequence[Decision]
    ) -> tuple[ClauseComponent, ...]:
        needed = set(conflict.original.variables())
        by_variable = {decision.variable: decision for decision in prefix}
        changed = True
        while changed:
            changed = False
            for variable in tuple(sorted(needed)):
                decision = by_variable.get(variable)
                if decision is None:
                    continue
                for dependency in decision.term.variables():
                    if dependency not in needed:
                        needed.add(dependency)
                        changed = True
        return tuple(
            decision.component
            for decision in prefix
            if decision.variable in needed
        )

    def _check_soundness(
        self, components: Sequence[ClauseComponent]
    ) -> AffineCheck:
        check = clause_soundness(
            self.original,
            components,
            max_combinations=self.max_affine_combinations,
        )
        self.metrics.affine_combinations += check.combinations
        self.metrics.affine_peak_constraints = max(
            self.metrics.affine_peak_constraints, check.peak_constraints
        )
        return check

    def _minimize(
        self,
        components: tuple[ClauseComponent, ...],
        prefix: Sequence[Decision],
    ) -> tuple[ClauseComponent, ...]:
        current = list(components)
        index = 0
        while index < len(current):
            trial = tuple(current[:index] + current[index + 1 :])
            check = self._check_soundness(trial)
            if check.feasible and clause_progress(trial, prefix):
                current.pop(index)
            else:
                index += 1
        return tuple(current)

    def _learn(
        self,
        prefix: Sequence[Decision],
        *,
        kind: str,
        conflict: TrackedLiteral | None,
    ) -> LearnedClause:
        self.metrics.learned_generated += 1
        full = tuple(decision.component for decision in prefix)
        components = full
        source_index = conflict.origin_index if conflict is not None else None
        if self.treatment is Treatment.FOCUSED and conflict is not None:
            supported = self._support_components(conflict, prefix)
            supported_check = self._check_soundness(supported)
            if supported_check.feasible and clause_progress(supported, prefix):
                components = supported
        if self.treatment is Treatment.FOCUSED:
            components = self._minimize(components, prefix)

        soundness = self._check_soundness(components)
        if not soundness.feasible:
            raise AssertionError("candidate learned clause failed affine soundness")
        if not clause_progress(components, prefix):
            raise AssertionError("candidate learned clause failed progress")

        clause = LearnedClause(
            tuple(components),
            kind,
            source_index,
            soundness.combinations,
            soundness.peak_constraints,
        )
        inserted = clause.key not in self.learned_keys
        if inserted:
            self.learned_keys.add(clause.key)
            self.learned.append(clause)
            self.metrics.learned_inserted += 1
            self.metrics.peak_learned_clauses = max(
                self.metrics.peak_learned_clauses, len(self.learned)
            )
        self._event(
            "Leaf Conflict" if conflict is not None else "Inner Conflict",
            prefix=[decision.describe() for decision in prefix],
            conflict_origin=source_index,
            clause=clause.describe(),
            inserted=inserted,
        )
        return clause

    def _blocked(self, prefix: Sequence[Decision]) -> LearnedClause | None:
        for clause in self.learned:
            if clause_false_under(clause.components, prefix):
                return clause
        return None

    def _eager(
        self,
        residuals: Sequence[TrackedLiteral],
        prefix: tuple[Decision, ...],
    ) -> bool:
        state, conflict = self._state(residuals)
        if state == "conflict":
            self.metrics.explored_leaves += 1
            assert conflict is not None
            self._event(
                "Eager Conflict",
                prefix=[decision.describe() for decision in prefix],
                conflict=conflict.describe(),
            )
            return False
        if state == "success":
            self.metrics.explored_leaves += 1
            self._event(
                "Eager Success",
                prefix=[decision.describe() for decision in prefix],
            )
            return True

        variable = self._variable(residuals)
        candidates = self._candidates(residuals, variable, prefix)
        any_sat = False
        for candidate in candidates:
            self.metrics.virtual_substitutions += 1
            child = self._substitute(residuals, candidate)
            self._event(
                "Substitute",
                candidate=candidate.describe(),
                prefix=[decision.describe() for decision in prefix],
                residual_after=[tracked.describe() for tracked in child],
            )
            any_sat = self._eager(child, (*prefix, candidate)) or any_sat
        return any_sat

    def _learned_dfs(
        self,
        residuals: Sequence[TrackedLiteral],
        prefix: tuple[Decision, ...],
    ) -> bool:
        blocker = self._blocked(prefix)
        if blocker is not None:
            self.metrics.admissibility_prunes += 1
            self._event(
                "Leaf Backtrack",
                prefix=[decision.describe() for decision in prefix],
                blocker=blocker.describe(),
            )
            return False

        state, conflict = self._state(residuals)
        if state == "conflict":
            self.metrics.explored_leaves += 1
            assert conflict is not None
            self._learn(prefix, kind="leaf", conflict=conflict)
            return False
        if state == "success":
            self.metrics.explored_leaves += 1
            self._event(
                "Succeed",
                prefix=[decision.describe() for decision in prefix],
            )
            return True

        variable = self._variable(residuals)
        candidates = self._candidates(residuals, variable, prefix)
        for candidate in candidates:
            extended = (*prefix, candidate)
            blocker = self._blocked(extended)
            if blocker is not None:
                self.metrics.admissibility_prunes += 1
                self._event(
                    "Substitute Pruned",
                    candidate=candidate.describe(),
                    prefix=[decision.describe() for decision in prefix],
                    blocker=blocker.describe(),
                )
                continue
            self.metrics.virtual_substitutions += 1
            child = self._substitute(residuals, candidate)
            self._event(
                "Substitute",
                candidate=candidate.describe(),
                prefix=[decision.describe() for decision in prefix],
                residual_after=[tracked.describe() for tracked in child],
            )
            if self._learned_dfs(child, extended):
                return True
            prefix_blocker = self._blocked(prefix)
            if prefix_blocker is not None:
                self.metrics.admissibility_prunes += 1
                self._event(
                    "Inner Backtrack",
                    prefix=[decision.describe() for decision in prefix],
                    blocker=prefix_blocker.describe(),
                )
                return False

        self.metrics.explored_leaves += 1
        self._learn(prefix, kind="inner", conflict=None)
        if not prefix:
            self._event("Fail", prefix=[])
        return False

    def run(self) -> SearchOutcome:
        """Execute this treatment and return a deterministic result."""

        started = time.perf_counter()
        try:
            if not self.original:
                raise UnsupportedSlice("empty conjunction is outside the experiment slice")
            for literal in self.original:
                affine_from_term(literal.term)
                literal_inequalities(literal)
            residuals = tuple(
                TrackedLiteral(index, literal, literal)
                for index, literal in enumerate(self.original)
            )
            if self.treatment is Treatment.EAGER:
                decision = self._eager(residuals, ())
            else:
                decision = self._learned_dfs(residuals, ())
            supported = True
            reason = "complete finite equality-guarded search"
        except UnsupportedSlice as error:
            supported = False
            decision = None
            reason = str(error)
            self._event("Unsupported", reason=reason)
        except ResourceLimit as error:
            supported = False
            decision = None
            reason = str(error)
            self._event("ResourceLimit", reason=reason)
        self.metrics.elapsed_seconds = time.perf_counter() - started
        semantic = {
            "treatment": self.treatment.value,
            "supported": supported,
            "decision": decision,
            "reason": reason,
            "metrics": self.metrics.semantic_description(),
            "clauses": [clause.describe() for clause in self.learned],
            "trace": self.trace,
        }
        return SearchOutcome(
            self.treatment,
            supported,
            decision,
            reason,
            self.metrics,
            tuple(self.learned),
            tuple(self.trace),
            semantic_hash(semantic),
        )


def run_search(
    literals: Sequence[base.Literal],
    treatment: Treatment | str,
    *,
    max_steps: int = 1_000_000,
    max_affine_combinations: int = 100_000,
) -> SearchOutcome:
    """Convenience entry point."""

    chosen = treatment if isinstance(treatment, Treatment) else Treatment(treatment)
    return Search(
        literals,
        chosen,
        max_steps=max_steps,
        max_affine_combinations=max_affine_combinations,
    ).run()


def negate_clause_assertions(
    components: Sequence[ClauseComponent],
) -> tuple[base.Literal, ...]:
    """Return the equality conjunction that negates a blocking clause."""

    return tuple(
        base.Literal(
            base.subtract(base.variable(component.variable), component.term),
            base.Relation.EQ,
        )
        for component in components
    )
