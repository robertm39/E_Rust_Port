#!/usr/bin/env python3
"""Clean-room exact prototype of base VIRAS for one conjunction.

This module is derived only from the tracked ``viras_docs`` packet.  It is an
experiment-local candidate implementation, deliberately separate from Umlaut
and from the independent oracle in ``tools/validation``.
"""

from __future__ import annotations

import dataclasses
import enum
import json
from dataclasses import dataclass, field
from fractions import Fraction
from math import gcd, lcm
from typing import Any, Iterable, Sequence


def frac(value: int | Fraction) -> Fraction:
    return value if isinstance(value, Fraction) else Fraction(value)


def floor_fraction(value: Fraction) -> int:
    return value.numerator // value.denominator


def ceil_fraction(value: Fraction) -> int:
    return -floor_fraction(-value)


def rational_lcm(values: Iterable[Fraction]) -> Fraction:
    positives = [abs(value) for value in values if value]
    if not positives:
        return Fraction(0)
    numerator = 1
    denominator = positives[0].denominator
    for value in positives:
        numerator = lcm(numerator, value.numerator)
    for value in positives[1:]:
        denominator = gcd(denominator, value.denominator)
    return Fraction(numerator, denominator)


def _fraction_text(value: Fraction) -> str:
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


class TermOp(str, enum.Enum):
    CONST = "const"
    VAR = "var"
    ADD = "add"
    SCALE = "scale"
    FLOOR = "floor"


@dataclass(frozen=True, slots=True)
class Term:
    op: TermOp
    value: Fraction | str | None = None
    args: tuple["Term", ...] = ()

    def variables(self) -> frozenset[str]:
        if self.op is TermOp.VAR:
            assert isinstance(self.value, str)
            return frozenset((self.value,))
        result: set[str] = set()
        for argument in self.args:
            result.update(argument.variables())
        return frozenset(result)

    def contains(self, variable: str) -> bool:
        return variable in self.variables()

    def render(self) -> str:
        if self.op is TermOp.CONST:
            assert isinstance(self.value, Fraction)
            return _fraction_text(self.value)
        if self.op is TermOp.VAR:
            assert isinstance(self.value, str)
            return self.value
        if self.op is TermOp.ADD:
            return "(" + " + ".join(argument.render() for argument in self.args) + ")"
        if self.op is TermOp.SCALE:
            assert isinstance(self.value, Fraction)
            return f"({_fraction_text(self.value)}*{self.args[0].render()})"
        return f"floor({self.args[0].render()})"

    def describe(self) -> Any:
        if self.op in {TermOp.CONST, TermOp.VAR}:
            value = (
                _fraction_text(self.value)
                if isinstance(self.value, Fraction)
                else self.value
            )
            return [self.op.value, value]
        if self.op is TermOp.SCALE:
            assert isinstance(self.value, Fraction)
            return [self.op.value, _fraction_text(self.value), self.args[0].describe()]
        return [self.op.value, *[argument.describe() for argument in self.args]]


def constant(value: int | Fraction) -> Term:
    return Term(TermOp.CONST, frac(value))


def variable(name: str) -> Term:
    if not name or not (name[0].isalpha() or name[0] == "_"):
        raise ValueError(f"invalid variable name: {name!r}")
    if not all(character.isalnum() or character == "_" for character in name):
        raise ValueError(f"invalid variable name: {name!r}")
    return Term(TermOp.VAR, name)


def scale(coefficient: int | Fraction, term: Term) -> Term:
    exact = frac(coefficient)
    if exact == 0:
        return constant(0)
    if term.op is TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return constant(exact * term.value)
    if exact == 1:
        return term
    if term.op is TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return scale(exact * term.value, term.args[0])
    if term.op is TermOp.ADD:
        return add(*(scale(exact, argument) for argument in term.args))
    return Term(TermOp.SCALE, exact, (term,))


def negate(term: Term) -> Term:
    return scale(-1, term)


def add(*terms: Term) -> Term:
    pending: list[Term] = []
    for term in terms:
        if term.op is TermOp.ADD:
            pending.extend(term.args)
        else:
            pending.append(term)

    constant_sum = Fraction(0)
    coefficients: dict[Term, Fraction] = {}
    for term in pending:
        if term.op is TermOp.CONST:
            assert isinstance(term.value, Fraction)
            constant_sum += term.value
            continue
        if term.op is TermOp.SCALE:
            assert isinstance(term.value, Fraction)
            base = term.args[0]
            coefficient = term.value
        else:
            base = term
            coefficient = Fraction(1)
        coefficients[base] = coefficients.get(base, Fraction(0)) + coefficient

    children = [
        scale(coefficient, base)
        for base, coefficient in coefficients.items()
        if coefficient
    ]
    if constant_sum:
        children.append(constant(constant_sum))
    if not children:
        return constant(0)
    children.sort(key=Term.render)
    if len(children) == 1:
        return children[0]
    return Term(TermOp.ADD, args=tuple(children))


def subtract(left: Term, right: Term) -> Term:
    return add(left, negate(right))


def floor_term(term: Term) -> Term:
    if term.op is TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return constant(floor_fraction(term.value))

    # Pull an exact integer shift out of a floor.  This local rule is enough
    # to keep the paper vectors and grid terms compact without unsafe algebra.
    if term.op is TermOp.ADD:
        integer_shift = Fraction(0)
        rest: list[Term] = []
        for child in term.args:
            if (
                child.op is TermOp.CONST
                and isinstance(child.value, Fraction)
                and child.value.denominator == 1
            ):
                integer_shift += child.value
            else:
                rest.append(child)
        if integer_shift and rest:
            return add(floor_term(add(*rest)), constant(integer_shift))
    return Term(TermOp.FLOOR, args=(term,))


def ceil_term(term: Term) -> Term:
    return negate(floor_term(negate(term)))


def substitute(term: Term, name: str, replacement: Term) -> Term:
    if term.op is TermOp.VAR:
        return replacement if term.value == name else term
    if term.op in {TermOp.CONST}:
        return term
    if term.op is TermOp.ADD:
        return add(*(substitute(argument, name, replacement) for argument in term.args))
    if term.op is TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return scale(term.value, substitute(term.args[0], name, replacement))
    return floor_term(substitute(term.args[0], name, replacement))


def evaluate_term(
    term: Term,
    environment: dict[str, Fraction],
    *,
    truncate_negative_floor: bool = False,
) -> Fraction:
    if term.op is TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return term.value
    if term.op is TermOp.VAR:
        assert isinstance(term.value, str)
        return environment[term.value]
    if term.op is TermOp.ADD:
        return sum(
            (
                evaluate_term(
                    argument,
                    environment,
                    truncate_negative_floor=truncate_negative_floor,
                )
                for argument in term.args
            ),
            Fraction(0),
        )
    if term.op is TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return term.value * evaluate_term(
            term.args[0],
            environment,
            truncate_negative_floor=truncate_negative_floor,
        )
    value = evaluate_term(
        term.args[0],
        environment,
        truncate_negative_floor=truncate_negative_floor,
    )
    return Fraction(int(value) if truncate_negative_floor else floor_fraction(value))


class Relation(str, enum.Enum):
    EQ = "eq"
    NE = "ne"
    GT = "gt"
    GE = "ge"

    def evaluate(self, value: Fraction) -> bool:
        if self is Relation.EQ:
            return value == 0
        if self is Relation.NE:
            return value != 0
        if self is Relation.GT:
            return value > 0
        return value >= 0


@dataclass(frozen=True, slots=True)
class Literal:
    term: Term
    relation: Relation

    def variables(self) -> frozenset[str]:
        return self.term.variables()

    def render(self) -> str:
        operator = {
            Relation.EQ: "=",
            Relation.NE: "!=",
            Relation.GT: ">",
            Relation.GE: ">=",
        }[self.relation]
        return f"{self.term.render()} {operator} 0"

    def describe(self) -> Any:
        return [self.relation.value, self.term.describe()]


class FormulaOp(str, enum.Enum):
    BOOL = "bool"
    ATOM = "atom"
    AND = "and"
    OR = "or"


@dataclass(frozen=True, slots=True)
class Formula:
    op: FormulaOp
    value: bool | Literal | None = None
    children: tuple["Formula", ...] = ()

    def variables(self) -> frozenset[str]:
        if self.op is FormulaOp.ATOM:
            assert isinstance(self.value, Literal)
            return self.value.variables()
        result: set[str] = set()
        for child in self.children:
            result.update(child.variables())
        return frozenset(result)

    def evaluate(
        self,
        environment: dict[str, Fraction],
        *,
        truncate_negative_floor: bool = False,
    ) -> bool:
        if self.op is FormulaOp.BOOL:
            assert isinstance(self.value, bool)
            return self.value
        if self.op is FormulaOp.ATOM:
            assert isinstance(self.value, Literal)
            return self.value.relation.evaluate(
                evaluate_term(
                    self.value.term,
                    environment,
                    truncate_negative_floor=truncate_negative_floor,
                )
            )
        if self.op is FormulaOp.AND:
            return all(
                child.evaluate(
                    environment,
                    truncate_negative_floor=truncate_negative_floor,
                )
                for child in self.children
            )
        return any(
            child.evaluate(
                environment,
                truncate_negative_floor=truncate_negative_floor,
            )
            for child in self.children
        )

    def render(self) -> str:
        if self.op is FormulaOp.BOOL:
            return "true" if self.value else "false"
        if self.op is FormulaOp.ATOM:
            assert isinstance(self.value, Literal)
            return self.value.render()
        separator = " & " if self.op is FormulaOp.AND else " | "
        return "(" + separator.join(child.render() for child in self.children) + ")"

    def describe(self) -> Any:
        if self.op is FormulaOp.BOOL:
            return [self.op.value, self.value]
        if self.op is FormulaOp.ATOM:
            assert isinstance(self.value, Literal)
            return [self.op.value, self.value.describe()]
        return [self.op.value, *[child.describe() for child in self.children]]


def boolean(value: bool) -> Formula:
    return Formula(FormulaOp.BOOL, bool(value))


def atom(literal: Literal) -> Formula:
    if not literal.variables():
        value = evaluate_term(literal.term, {})
        return boolean(literal.relation.evaluate(value))
    return Formula(FormulaOp.ATOM, literal)


def _formula_sort_key(formula: Formula) -> str:
    return json.dumps(formula.describe(), sort_keys=True, separators=(",", ":"))


def conjunction(*children: Formula) -> Formula:
    flattened: list[Formula] = []
    for child in children:
        if child.op is FormulaOp.BOOL:
            if child.value is False:
                return boolean(False)
            continue
        if child.op is FormulaOp.AND:
            flattened.extend(child.children)
        else:
            flattened.append(child)
    unique = sorted(set(flattened), key=_formula_sort_key)
    if not unique:
        return boolean(True)
    if len(unique) == 1:
        return unique[0]
    return Formula(FormulaOp.AND, children=tuple(unique))


def disjunction(*children: Formula) -> Formula:
    flattened: list[Formula] = []
    for child in children:
        if child.op is FormulaOp.BOOL:
            if child.value is True:
                return boolean(True)
            continue
        if child.op is FormulaOp.OR:
            flattened.extend(child.children)
        else:
            flattened.append(child)
    unique = sorted(set(flattened), key=_formula_sort_key)
    if not unique:
        return boolean(False)
    if len(unique) == 1:
        return unique[0]
    return Formula(FormulaOp.OR, children=tuple(unique))


@dataclass(frozen=True, slots=True)
class Grid:
    base: Term
    period: Fraction

    def __post_init__(self) -> None:
        if self.period <= 0:
            raise ValueError("grid period must be positive")

    def describe(self) -> Any:
        return [self.base.describe(), _fraction_text(self.period)]


class InfinitySign(str, enum.Enum):
    NEGATIVE = "negative"
    POSITIVE = "positive"

    @property
    def factor(self) -> int:
        return -1 if self is InfinitySign.NEGATIVE else 1


@dataclass(frozen=True, slots=True)
class VirtualTerm:
    base: Term = field(default_factory=lambda: constant(0))
    epsilon: bool = False
    grid_period: Fraction | None = None
    infinity: InfinitySign | None = None

    def __post_init__(self) -> None:
        if self.grid_period is not None and self.grid_period <= 0:
            raise ValueError("virtual grid period must be positive")
        if self.grid_period is not None and self.infinity is not None:
            raise ValueError("a virtual term cannot contain grid and infinity")

    def with_epsilon(self) -> "VirtualTerm":
        return dataclasses.replace(self, epsilon=True)

    def describe(self) -> Any:
        return {
            "base": self.base.describe(),
            "epsilon": self.epsilon,
            "grid_period": (
                _fraction_text(self.grid_period)
                if self.grid_period is not None
                else None
            ),
            "infinity": self.infinity.value if self.infinity is not None else None,
        }


@dataclass(frozen=True, slots=True)
class Profile:
    outer_slope: Fraction
    segment_slope: Fraction
    period: Fraction
    delta_y: Fraction
    dist_y_minus: Term
    right_limit: Term

    @property
    def dist_y_plus(self) -> Term:
        return add(self.dist_y_minus, constant(self.delta_y))


@dataclass(frozen=True, slots=True)
class Limits:
    max_steps: int = 1_000_000
    max_candidates: int = 20_000
    max_grids: int = 20_000
    max_grid_points: int = 50_000
    max_formula_nodes: int = 200_000
    max_rational_bits: int = 4_096


class UnknownKind(str, enum.Enum):
    RESOURCE_LIMIT = "ResourceLimit"
    UNSUPPORTED_FRAGMENT = "UnsupportedFragment"


class _Unknown(RuntimeError):
    def __init__(self, kind: UnknownKind, reason: str):
        super().__init__(reason)
        self.kind = kind
        self.reason = reason


@dataclass(slots=True)
class Budget:
    limits: Limits
    steps: int = 0
    candidates: int = 0
    grids: int = 0
    grid_points: int = 0

    def tick(self, label: str, count: int = 1) -> None:
        self.steps += count
        if self.steps > self.limits.max_steps:
            raise _Unknown(
                UnknownKind.RESOURCE_LIMIT,
                f"step limit exceeded during {label}: "
                f"{self.steps}>{self.limits.max_steps}",
            )

    def check_fraction(self, value: Fraction, label: str) -> None:
        bits = max(abs(value.numerator).bit_length(), value.denominator.bit_length())
        if bits > self.limits.max_rational_bits:
            raise _Unknown(
                UnknownKind.RESOURCE_LIMIT,
                f"rational bit limit exceeded during {label}: "
                f"{bits}>{self.limits.max_rational_bits}",
            )

    def add_candidate(self) -> None:
        self.candidates += 1
        if self.candidates > self.limits.max_candidates:
            raise _Unknown(
                UnknownKind.RESOURCE_LIMIT,
                f"candidate limit exceeded: "
                f"{self.candidates}>{self.limits.max_candidates}",
            )

    def add_grid(self) -> None:
        self.grids += 1
        if self.grids > self.limits.max_grids:
            raise _Unknown(
                UnknownKind.RESOURCE_LIMIT,
                f"grid limit exceeded: {self.grids}>{self.limits.max_grids}",
            )

    def add_grid_point(self) -> None:
        self.grid_points += 1
        if self.grid_points > self.limits.max_grid_points:
            raise _Unknown(
                UnknownKind.RESOURCE_LIMIT,
                f"grid-point limit exceeded: "
                f"{self.grid_points}>{self.limits.max_grid_points}",
            )


@dataclass(frozen=True, slots=True)
class Mutations:
    reverse_infinity_periodicity: bool = False
    drop_epsilon_strictness: bool = False
    omit_last_candidate: bool = False


@dataclass(frozen=True, slots=True)
class Candidate:
    virtual: VirtualTerm
    literal_index: int
    origin_kind: str

    def describe(self) -> Any:
        return {
            "virtual": self.virtual.describe(),
            "literal_index": self.literal_index,
            "origin_kind": self.origin_kind,
        }


class QEStatus(str, enum.Enum):
    SUCCESS = "success"
    UNKNOWN = "unknown"


@dataclass(frozen=True, slots=True)
class QEOutcome:
    status: QEStatus
    formula: Formula | None
    unknown_kind: UnknownKind | None
    reason: str
    derivation: dict[str, Any]

    def describe(self) -> dict[str, Any]:
        return {
            "status": self.status.value,
            "formula": self.formula.describe() if self.formula is not None else None,
            "unknown_kind": (
                self.unknown_kind.value if self.unknown_kind is not None else None
            ),
            "reason": self.reason,
            "derivation": self.derivation,
        }


class Kernel:
    def __init__(
        self,
        *,
        limits: Limits | None = None,
        mutations: Mutations | None = None,
    ) -> None:
        self.budget = Budget(limits or Limits())
        self.mutations = mutations or Mutations()
        self._profiles: dict[tuple[Term, str], Profile] = {}
        self._breaks: dict[tuple[Term, str], tuple[Grid, ...]] = {}
        self.flatten_records: list[dict[str, Any]] = []

    def profile(self, term: Term, eliminated: str) -> Profile:
        key = (term, eliminated)
        if key in self._profiles:
            return self._profiles[key]
        self.budget.tick("profile")

        if term.op is TermOp.CONST:
            assert isinstance(term.value, Fraction)
            result = Profile(
                Fraction(0),
                Fraction(0),
                Fraction(0),
                Fraction(0),
                term,
                term,
            )
        elif term.op is TermOp.VAR:
            is_eliminated = term.value == eliminated
            result = Profile(
                Fraction(int(is_eliminated)),
                Fraction(int(is_eliminated)),
                Fraction(0),
                Fraction(0),
                constant(0) if is_eliminated else term,
                term,
            )
        elif term.op is TermOp.SCALE:
            assert isinstance(term.value, Fraction)
            coefficient = term.value
            inner = self.profile(term.args[0], eliminated)
            lower = (
                scale(coefficient, inner.dist_y_minus)
                if coefficient >= 0
                else scale(coefficient, inner.dist_y_plus)
            )
            result = Profile(
                coefficient * inner.outer_slope,
                coefficient * inner.segment_slope,
                inner.period,
                abs(coefficient) * inner.delta_y,
                lower,
                scale(coefficient, inner.right_limit),
            )
        elif term.op is TermOp.ADD:
            parts = [self.profile(argument, eliminated) for argument in term.args]
            result = Profile(
                sum((part.outer_slope for part in parts), Fraction(0)),
                sum((part.segment_slope for part in parts), Fraction(0)),
                rational_lcm(part.period for part in parts),
                sum((part.delta_y for part in parts), Fraction(0)),
                add(*(part.dist_y_minus for part in parts)),
                add(*(part.right_limit for part in parts)),
            )
        else:
            inner = self.profile(term.args[0], eliminated)
            if inner.period == 0 and inner.outer_slope == 0:
                period = Fraction(0)
            elif inner.period == 0:
                period = Fraction(1, 1) / abs(inner.outer_slope)
            else:
                period = Fraction(
                    abs(inner.period.numerator) * inner.outer_slope.denominator
                )
            if inner.segment_slope >= 0:
                right_limit = floor_term(inner.right_limit)
            else:
                right_limit = add(ceil_term(inner.right_limit), constant(-1))
            result = Profile(
                inner.outer_slope,
                Fraction(0),
                period,
                inner.delta_y + 1,
                add(inner.dist_y_minus, constant(-1)),
                right_limit,
            )

        for rational in (
            result.outer_slope,
            result.segment_slope,
            result.period,
            result.delta_y,
        ):
            self.budget.check_fraction(rational, "profile")
        if result.period < 0 or result.delta_y < 0:
            raise AssertionError("profile period and width must be nonnegative")
        self._profiles[key] = result
        return result

    def segment_zero(self, term: Term, eliminated: str, base: Term) -> Term:
        profile = self.profile(term, eliminated)
        if profile.segment_slope == 0:
            raise AssertionError("segment zero needs nonzero segment slope")
        if base.contains(eliminated):
            raise AssertionError("segment-zero base contains eliminated variable")
        limit_at_base = substitute(profile.right_limit, eliminated, base)
        return add(base, scale(-1 / profile.segment_slope, limit_at_base))

    def core_interval(
        self, literal: Literal, eliminated: str
    ) -> tuple[Term, Term, Fraction]:
        profile = self.profile(literal.term, eliminated)
        if profile.outer_slope == 0:
            raise AssertionError("periodic literal has no aperiodic core")
        signed = (
            profile.dist_y_plus
            if profile.outer_slope > 0
            else profile.dist_y_minus
        )
        lower = scale(-1 / profile.outer_slope, signed)
        width = profile.delta_y / abs(profile.outer_slope)
        self.budget.check_fraction(width, "core interval")
        return lower, add(lower, constant(width)), width

    def limit_truth(
        self, literal: Literal, eliminated: str, sign: InfinitySign
    ) -> bool:
        profile = self.profile(literal.term, eliminated)
        if profile.outer_slope == 0:
            raise AssertionError("periodic literal has no constant infinity limit")
        if literal.relation is Relation.EQ:
            return False
        if literal.relation is Relation.NE:
            return True
        return sign.factor * profile.outer_slope > 0

    def _rem(self, term: Term, period: Fraction) -> Term:
        if period <= 0:
            raise AssertionError("remainder period must be positive")
        return add(
            term,
            scale(-period, floor_term(scale(Fraction(1, 1) / period, term))),
        )

    def grid_ceil(self, grid: Grid, term: Term) -> Term:
        return add(term, self._rem(subtract(grid.base, term), grid.period))

    def grid_floor(self, grid: Grid, term: Term) -> Term:
        return subtract(term, self._rem(subtract(term, grid.base), grid.period))

    def grid_intersection(
        self,
        grid: Grid,
        lower: Term,
        width: Fraction,
        *,
        lower_closed: bool,
        upper_closed: bool,
    ) -> tuple[Term, ...]:
        self.budget.tick("grid intersection")
        self.budget.check_fraction(width, "grid intersection")
        if width < 0:
            raise AssertionError("grid intersection width must be nonnegative")
        start = (
            self.grid_ceil(grid, lower)
            if lower_closed
            else self.grid_floor(grid, add(lower, constant(grid.period)))
        )
        quotient = width / grid.period
        upper_index = (
            floor_fraction(quotient)
            if upper_closed
            else ceil_fraction(quotient) - 1
        )
        if upper_index < 0:
            return ()
        result = []
        for index in range(upper_index + 1):
            self.budget.add_grid_point()
            result.append(add(start, constant(index * grid.period)))
        return tuple(dict.fromkeys(result))

    def breaks(self, term: Term, eliminated: str) -> tuple[Grid, ...]:
        key = (term, eliminated)
        if key in self._breaks:
            return self._breaks[key]
        self.budget.tick("break construction")

        if term.op in {TermOp.CONST, TermOp.VAR}:
            result: tuple[Grid, ...] = ()
        elif term.op is TermOp.SCALE:
            result = self.breaks(term.args[0], eliminated)
        elif term.op is TermOp.ADD:
            grids: set[Grid] = set()
            for argument in term.args:
                grids.update(self.breaks(argument, eliminated))
            result = tuple(sorted(grids, key=lambda item: json.dumps(item.describe())))
        else:
            inner_term = term.args[0]
            inner_profile = self.profile(inner_term, eliminated)
            inner_breaks = self.breaks(inner_term, eliminated)
            if inner_profile.segment_slope == 0:
                result = inner_breaks
            elif not inner_breaks:
                period = self.profile(term, eliminated).period
                if period <= 0:
                    raise AssertionError("nonconstant floor needs positive period")
                result = (
                    Grid(
                        self.segment_zero(
                            inner_term, eliminated, constant(0)
                        ),
                        period,
                    ),
                )
            else:
                period = self.profile(term, eliminated).period
                minimum_period = min(grid.period for grid in inner_breaks)
                reciprocal_slope = abs(
                    Fraction(1, 1) / inner_profile.segment_slope
                )
                generated: set[Grid] = set(inner_breaks)
                for source in inner_breaks:
                    segment_bases = self.grid_intersection(
                        source,
                        source.base,
                        period,
                        lower_closed=True,
                        upper_closed=False,
                    )
                    for segment_base in segment_bases:
                        zero_grid = Grid(
                            self.segment_zero(
                                inner_term, eliminated, segment_base
                            ),
                            reciprocal_slope,
                        )
                        breaks_in_segment = self.grid_intersection(
                            zero_grid,
                            segment_base,
                            minimum_period,
                            lower_closed=True,
                            upper_closed=False,
                        )
                        for break_base in breaks_in_segment:
                            generated.add(Grid(break_base, period))
                result = tuple(
                    sorted(generated, key=lambda item: json.dumps(item.describe()))
                )

        for _ in result:
            self.budget.add_grid()
        self._breaks[key] = result
        return result

    def _candidate(
        self,
        output: list[Candidate],
        virtual: VirtualTerm,
        literal_index: int,
        origin: str,
    ) -> None:
        self.budget.add_candidate()
        output.append(Candidate(virtual, literal_index, origin))

    def literal_candidates(
        self, literal: Literal, eliminated: str, literal_index: int
    ) -> list[Candidate]:
        profile = self.profile(literal.term, eliminated)
        breaks = self.breaks(literal.term, eliminated)
        output: list[Candidate] = []

        if not breaks:
            if profile.segment_slope == 0:
                self._candidate(
                    output,
                    VirtualTerm(infinity=InfinitySign.NEGATIVE),
                    literal_index,
                    "negative_tail",
                )
            else:
                zero = self.segment_zero(literal.term, eliminated, constant(0))
                if literal.relation is Relation.NE:
                    self._candidate(
                        output,
                        VirtualTerm(infinity=InfinitySign.NEGATIVE),
                        literal_index,
                        "negative_tail",
                    )
                    self._candidate(
                        output,
                        VirtualTerm(zero, epsilon=True),
                        literal_index,
                        "linear_zero_right",
                    )
                elif literal.relation is Relation.EQ:
                    self._candidate(
                        output, VirtualTerm(zero), literal_index, "linear_zero"
                    )
                elif profile.segment_slope > 0:
                    self._candidate(
                        output,
                        VirtualTerm(
                            zero, epsilon=literal.relation is Relation.GT
                        ),
                        literal_index,
                        "linear_lower_bound",
                    )
                else:
                    self._candidate(
                        output,
                        VirtualTerm(infinity=InfinitySign.NEGATIVE),
                        literal_index,
                        "negative_tail",
                    )
            return output

        periodic = profile.outer_slope == 0
        break_terms: list[VirtualTerm] = []
        if periodic:
            break_terms = [
                VirtualTerm(grid.base, grid_period=grid.period) for grid in breaks
            ]
        else:
            lower, _, width = self.core_interval(literal, eliminated)
            for grid in breaks:
                break_terms.extend(
                    VirtualTerm(base)
                    for base in self.grid_intersection(
                        grid,
                        lower,
                        width,
                        lower_closed=False,
                        upper_closed=False,
                    )
                )
        for virtual in break_terms:
            self._candidate(
                output, virtual, literal_index, "discontinuity"
            )

        zero_terms: list[VirtualTerm] = []
        if periodic and profile.segment_slope != 0:
            zero_terms = [
                VirtualTerm(
                    self.segment_zero(literal.term, eliminated, grid.base),
                    grid_period=grid.period,
                )
                for grid in breaks
            ]
        elif profile.segment_slope != 0:
            lower, _, width = self.core_interval(literal, eliminated)
            if profile.outer_slope == profile.segment_slope:
                zero_terms = [
                    VirtualTerm(
                        self.segment_zero(literal.term, eliminated, grid.base)
                    )
                    for grid in breaks
                ]
            else:
                for grid in breaks:
                    zero_period = abs(
                        (
                            Fraction(1)
                            - profile.outer_slope / profile.segment_slope
                        )
                        * grid.period
                    )
                    if zero_period == 0:
                        raise AssertionError("zero-grid period must be positive")
                    zero_grid = Grid(
                        self.segment_zero(
                            literal.term, eliminated, grid.base
                        ),
                        zero_period,
                    )
                    zero_terms.extend(
                        VirtualTerm(base)
                        for base in self.grid_intersection(
                            zero_grid,
                            lower,
                            width,
                            lower_closed=False,
                            upper_closed=False,
                        )
                    )

        segment_terms: list[VirtualTerm] = []
        if profile.segment_slope == 0 or (
            profile.segment_slope < 0
            and literal.relation in {Relation.GT, Relation.GE}
        ):
            segment_terms = [virtual.with_epsilon() for virtual in break_terms]
        elif profile.segment_slope > 0 and literal.relation is Relation.GE:
            segment_terms = [
                virtual.with_epsilon() for virtual in break_terms
            ] + zero_terms
        elif profile.segment_slope > 0 and literal.relation is Relation.GT:
            segment_terms = [
                virtual.with_epsilon() for virtual in break_terms
            ] + [virtual.with_epsilon() for virtual in zero_terms]
        elif (
            profile.segment_slope != 0
            and literal.relation is Relation.NE
        ):
            segment_terms = [
                virtual.with_epsilon()
                for virtual in [*break_terms, *zero_terms]
            ]
        elif (
            profile.segment_slope != 0
            and literal.relation is Relation.EQ
        ):
            segment_terms = zero_terms
        for virtual in segment_terms:
            self._candidate(
                output, virtual, literal_index, "segment_candidate"
            )

        if not periodic:
            lower, upper, _ = self.core_interval(literal, eliminated)
            positive = self.limit_truth(
                literal, eliminated, InfinitySign.POSITIVE
            )
            negative = self.limit_truth(
                literal, eliminated, InfinitySign.NEGATIVE
            )
            self._candidate(
                output, VirtualTerm(upper), literal_index, "core_upper"
            )
            if positive:
                self._candidate(
                    output,
                    VirtualTerm(upper, epsilon=True),
                    literal_index,
                    "core_upper_right",
                )
            self._candidate(
                output, VirtualTerm(lower), literal_index, "core_lower"
            )
            if negative:
                self._candidate(
                    output,
                    VirtualTerm(infinity=InfinitySign.NEGATIVE),
                    literal_index,
                    "negative_tail",
                )
        return output

    def flatten_grid(
        self,
        literals: Sequence[Literal],
        eliminated: str,
        virtual: VirtualTerm,
    ) -> tuple[VirtualTerm, ...]:
        if virtual.grid_period is None:
            raise AssertionError("flatten_grid requires a grid virtual term")
        self.budget.tick("grid flatten")
        periodic = [
            literal
            for literal in literals
            if self.profile(literal.term, eliminated).outer_slope == 0
        ]
        aperiodic = [
            literal
            for literal in literals
            if self.profile(literal.term, eliminated).outer_slope != 0
        ]
        common_period = rational_lcm(
            [
                virtual.grid_period,
                *[
                    self.profile(literal.term, eliminated).period
                    for literal in periodic
                    if self.profile(literal.term, eliminated).period
                ],
            ]
        )
        if common_period <= 0:
            raise AssertionError("grid flatten needs a positive common period")
        grid = Grid(virtual.base, virtual.grid_period)

        flattened: list[VirtualTerm] = []
        qualifying_signs = [
            sign
            for sign in (InfinitySign.NEGATIVE, InfinitySign.POSITIVE)
            if all(
                self.limit_truth(literal, eliminated, sign)
                for literal in aperiodic
            )
        ]
        if qualifying_signs:
            representatives = self.grid_intersection(
                grid,
                virtual.base,
                common_period,
                lower_closed=True,
                upper_closed=False,
            )
            for sign in qualifying_signs:
                flattened.extend(
                    VirtualTerm(
                        representative,
                        epsilon=virtual.epsilon,
                        infinity=sign,
                    )
                    for representative in representatives
                )
            case = "V1"
        else:
            equalities = [
                literal
                for literal in aperiodic
                if literal.relation is Relation.EQ
            ]
            if equalities:
                chosen = min(
                    equalities,
                    key=lambda literal: self.core_interval(
                        literal, eliminated
                    )[2],
                )
                lower, _, width = self.core_interval(chosen, eliminated)
                representatives = self.grid_intersection(
                    grid,
                    lower,
                    width,
                    lower_closed=True,
                    upper_closed=True,
                )
                flattened = [
                    VirtualTerm(
                        representative,
                        epsilon=virtual.epsilon,
                    )
                    for representative in representatives
                ]
                case = "V2"
            else:
                for literal in aperiodic:
                    if not self.limit_truth(
                        literal, eliminated, InfinitySign.NEGATIVE
                    ):
                        lower, _, width = self.core_interval(
                            literal, eliminated
                        )
                        representatives = self.grid_intersection(
                            grid,
                            lower,
                            width + common_period,
                            lower_closed=True,
                            upper_closed=True,
                        )
                        flattened.extend(
                            VirtualTerm(
                                representative,
                                epsilon=virtual.epsilon,
                            )
                            for representative in representatives
                        )
                if not flattened:
                    raise AssertionError("V3 needs a negative-tail blocker")
                case = "V3"

        unique = tuple(dict.fromkeys(flattened))
        self.flatten_records.append(
            {
                "case": case,
                "input": virtual.describe(),
                "common_period": _fraction_text(common_period),
                "output": [item.describe() for item in unique],
            }
        )
        return unique

    def _epsilon_literal(
        self, literal: Literal, eliminated: str, base: Term
    ) -> Formula:
        profile = self.profile(literal.term, eliminated)
        limit = substitute(profile.right_limit, eliminated, base)
        if literal.relation is Relation.EQ:
            return (
                boolean(False)
                if profile.segment_slope != 0
                else atom(Literal(limit, Relation.EQ))
            )
        if literal.relation is Relation.NE:
            return (
                boolean(True)
                if profile.segment_slope != 0
                else atom(Literal(limit, Relation.NE))
            )
        if profile.segment_slope > 0:
            relation = (
                literal.relation
                if self.mutations.drop_epsilon_strictness
                else Relation.GE
            )
        elif profile.segment_slope == 0:
            relation = literal.relation
        else:
            relation = Relation.GT
        return atom(Literal(limit, relation))

    def _virtual_literal(
        self, literal: Literal, eliminated: str, virtual: VirtualTerm
    ) -> Formula:
        profile = self.profile(literal.term, eliminated)
        if virtual.infinity is not None:
            is_aperiodic = profile.outer_slope != 0
            if self.mutations.reverse_infinity_periodicity:
                is_aperiodic = not is_aperiodic
            if is_aperiodic:
                if profile.outer_slope == 0:
                    # The paper's printed reversal is deliberately totalized
                    # for mutation testing; this arbitrary constant is expected
                    # to be caught by the frozen periodic-residue corpus.
                    return boolean(literal.relation is Relation.NE)
                return boolean(
                    self.limit_truth(
                        literal, eliminated, virtual.infinity
                    )
                )
            return self._virtual_literal(
                literal,
                eliminated,
                dataclasses.replace(virtual, infinity=None),
            )
        if virtual.epsilon:
            return self._epsilon_literal(literal, eliminated, virtual.base)
        return atom(
            Literal(
                substitute(literal.term, eliminated, virtual.base),
                literal.relation,
            )
        )

    def virtual_substitute(
        self,
        literals: Sequence[Literal],
        eliminated: str,
        virtual: VirtualTerm,
    ) -> Formula:
        self.budget.tick("virtual substitution")
        if virtual.grid_period is not None:
            flattened = self.flatten_grid(literals, eliminated, virtual)
            return disjunction(
                *(
                    self.virtual_substitute(literals, eliminated, finite)
                    for finite in flattened
                )
            )
        return conjunction(
            *(
                self._virtual_literal(literal, eliminated, virtual)
                for literal in literals
            )
        )

    def _check_formula_size(self, formula: Formula) -> None:
        stack = [formula]
        count = 0
        while stack:
            current = stack.pop()
            count += 1
            if count > self.budget.limits.max_formula_nodes:
                raise _Unknown(
                    UnknownKind.RESOURCE_LIMIT,
                    f"formula-node limit exceeded: "
                    f"{count}>{self.budget.limits.max_formula_nodes}",
                )
            stack.extend(current.children)

    def eliminate_exists(
        self, eliminated: str, literals: Sequence[Literal] | object
    ) -> QEOutcome:
        derivation: dict[str, Any] = {
            "calculus": "paper-derived-base-viras-one-conjunction-v1",
            "eliminated": eliminated,
            "candidates": [],
            "grid_flattening": self.flatten_records,
        }
        try:
            if (
                not isinstance(literals, (list, tuple))
                or not literals
                or not all(isinstance(literal, Literal) for literal in literals)
            ):
                raise _Unknown(
                    UnknownKind.UNSUPPORTED_FRAGMENT,
                    "kernel requires a nonempty conjunction of normalized literals",
                )
            assert isinstance(literals, (list, tuple))
            independent = [
                literal
                for literal in literals
                if eliminated not in literal.variables()
            ]
            dependent = [
                literal for literal in literals if eliminated in literal.variables()
            ]
            independent_formula = conjunction(
                *(atom(literal) for literal in independent)
            )
            if not dependent:
                result = independent_formula
                self._check_formula_size(result)
                return QEOutcome(
                    QEStatus.SUCCESS,
                    result,
                    None,
                    "eliminated variable absent",
                    derivation,
                )

            candidates: list[Candidate] = []
            for index, literal in enumerate(dependent):
                candidates.extend(
                    self.literal_candidates(literal, eliminated, index)
                )
            unique: list[Candidate] = []
            seen: set[VirtualTerm] = set()
            for candidate in candidates:
                if candidate.virtual not in seen:
                    seen.add(candidate.virtual)
                    unique.append(candidate)
            if self.mutations.omit_last_candidate and unique:
                unique.pop()
            derivation["candidates"] = [item.describe() for item in unique]
            if not unique:
                result = conjunction(independent_formula, boolean(False))
            else:
                dependent_result = disjunction(
                    *(
                        self.virtual_substitute(
                            dependent, eliminated, candidate.virtual
                        )
                        for candidate in unique
                    )
                )
                result = conjunction(independent_formula, dependent_result)
            if eliminated in result.variables():
                raise AssertionError("successful result retains eliminated variable")
            self._check_formula_size(result)
            derivation["grid_flattening"] = list(self.flatten_records)
            derivation["resource_usage"] = dataclasses.asdict(self.budget)
            return QEOutcome(
                QEStatus.SUCCESS,
                result,
                None,
                "complete finite virtual substitution",
                derivation,
            )
        except _Unknown as error:
            derivation["resource_usage"] = dataclasses.asdict(self.budget)
            return QEOutcome(
                QEStatus.UNKNOWN,
                None,
                error.kind,
                error.reason,
                derivation,
            )


def eliminate_exists(
    eliminated: str,
    literals: Sequence[Literal] | object,
    *,
    limits: Limits | None = None,
    mutations: Mutations | None = None,
) -> QEOutcome:
    return Kernel(limits=limits, mutations=mutations).eliminate_exists(
        eliminated, literals
    )


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
