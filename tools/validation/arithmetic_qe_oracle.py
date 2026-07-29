#!/usr/bin/env python3
"""Independent exact oracle and SMT differential gate for bounded LIRA.

This module deliberately does not import Umlaut.  It provides a small
paper-derived arithmetic AST, exact ``fractions.Fraction`` semantics, a
complete bounded one-variable cell decomposition, an external SMT-LIB process
adapter, outcome classification, and a deterministic structural shrinker.

The supported arithmetic language is rational affine arithmetic with nested
``floor`` and ``ceil``.  Bounded real queries are complete because the oracle
enumerates every rounding discontinuity, every atom zero, every boundary
point, and one representative of each remaining open cell.  Bounded integer
queries are decided by exact enumeration.
"""

from __future__ import annotations

import dataclasses
import enum
import re
import subprocess
from collections.abc import Callable, Iterable, Iterator, Sequence
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path
from typing import Any


_SYMBOL_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def as_fraction(value: int | Fraction) -> Fraction:
    """Return ``value`` as an exact rational."""

    return value if isinstance(value, Fraction) else Fraction(value)


def floor_fraction(value: Fraction) -> int:
    """Return mathematical floor, including for negative rationals."""

    return value.numerator // value.denominator


def ceil_fraction(value: Fraction) -> int:
    """Return mathematical ceiling, including for negative rationals."""

    return -floor_fraction(-value)


def quotient_remainder(value: Fraction, period: Fraction) -> tuple[int, Fraction]:
    """Return the Euclidean quotient and remainder for a positive period."""

    if period <= 0:
        raise ValueError("period must be positive")
    quotient = floor_fraction(value / period)
    remainder = value - period * quotient
    assert Fraction(0) <= remainder < period
    return quotient, remainder


def rational_lcm(values: Iterable[Fraction]) -> Fraction:
    """Return the least positive rational divisible by every nonzero input."""

    from math import gcd, lcm

    numerators: list[int] = []
    denominators: list[int] = []
    for value in values:
        if value == 0:
            continue
        positive = abs(value)
        numerators.append(positive.numerator)
        denominators.append(positive.denominator)
    if not numerators:
        return Fraction(0)
    numerator_lcm = 1
    denominator_gcd = denominators[0]
    for numerator in numerators:
        numerator_lcm = lcm(numerator_lcm, numerator)
    for denominator in denominators[1:]:
        denominator_gcd = gcd(denominator_gcd, denominator)
    return Fraction(numerator_lcm, denominator_gcd)


class Relation(str, enum.Enum):
    """Supported normalized and convenience relations."""

    EQ = "eq"
    NE = "ne"
    GT = "gt"
    GE = "ge"
    LT = "lt"
    LE = "le"

    def evaluate(self, left: Fraction, right: Fraction) -> bool:
        """Evaluate the relation exactly."""

        if self is Relation.EQ:
            return left == right
        if self is Relation.NE:
            return left != right
        if self is Relation.GT:
            return left > right
        if self is Relation.GE:
            return left >= right
        if self is Relation.LT:
            return left < right
        return left <= right

    @property
    def smt_operator(self) -> str:
        """Return the SMT-LIB operator for non-disequality relations."""

        return {
            Relation.EQ: "=",
            Relation.GT: ">",
            Relation.GE: ">=",
            Relation.LT: "<",
            Relation.LE: "<=",
        }.get(self, "=")


class ExprKind(str, enum.Enum):
    """Arithmetic expression node kinds."""

    CONSTANT = "constant"
    VARIABLE = "variable"
    ADD = "add"
    SCALE = "scale"
    FLOOR = "floor"
    CEIL = "ceil"


@dataclass(frozen=True, slots=True)
class Expr:
    """A rational affine expression with nested rounding."""

    kind: ExprKind
    value: Fraction | str | None = None
    args: tuple["Expr", ...] = ()

    @staticmethod
    def constant(value: int | Fraction) -> "Expr":
        return Expr(ExprKind.CONSTANT, as_fraction(value))

    @staticmethod
    def variable(name: str) -> "Expr":
        if not _SYMBOL_RE.fullmatch(name):
            raise ValueError(f"invalid arithmetic variable name: {name!r}")
        return Expr(ExprKind.VARIABLE, name)

    @staticmethod
    def add(*terms: "Expr") -> "Expr":
        if not terms:
            return Expr.constant(0)
        if len(terms) == 1:
            return terms[0]
        return Expr(ExprKind.ADD, args=tuple(terms))

    @staticmethod
    def scale(coefficient: int | Fraction, term: "Expr") -> "Expr":
        exact = as_fraction(coefficient)
        if exact == 0:
            return Expr.constant(0)
        if exact == 1:
            return term
        return Expr(ExprKind.SCALE, value=exact, args=(term,))

    @staticmethod
    def floor(term: "Expr") -> "Expr":
        return Expr(ExprKind.FLOOR, args=(term,))

    @staticmethod
    def ceil(term: "Expr") -> "Expr":
        return Expr(ExprKind.CEIL, args=(term,))

    @staticmethod
    def negate(term: "Expr") -> "Expr":
        return Expr.scale(-1, term)

    @staticmethod
    def subtract(left: "Expr", right: "Expr") -> "Expr":
        return Expr.add(left, Expr.negate(right))

    def variables(self) -> frozenset[str]:
        if self.kind is ExprKind.VARIABLE:
            assert isinstance(self.value, str)
            return frozenset((self.value,))
        names: set[str] = set()
        for argument in self.args:
            names.update(argument.variables())
        return frozenset(names)

    def complexity(self) -> int:
        return 1 + sum(argument.complexity() for argument in self.args)

    def to_smt2(self) -> str:
        """Render the expression as sort-correct real SMT-LIB."""

        if self.kind is ExprKind.CONSTANT:
            assert isinstance(self.value, Fraction)
            return _fraction_to_smt2(self.value)
        if self.kind is ExprKind.VARIABLE:
            assert isinstance(self.value, str)
            return self.value
        if self.kind is ExprKind.ADD:
            return f"(+ {' '.join(argument.to_smt2() for argument in self.args)})"
        if self.kind is ExprKind.SCALE:
            assert isinstance(self.value, Fraction)
            return f"(* {_fraction_to_smt2(self.value)} {self.args[0].to_smt2()})"
        if self.kind is ExprKind.FLOOR:
            return f"(to_real (to_int {self.args[0].to_smt2()}))"
        assert self.kind is ExprKind.CEIL
        argument = self.args[0].to_smt2()
        return f"(- (to_real (to_int (- {argument}))))"

    def __str__(self) -> str:
        if self.kind is ExprKind.CONSTANT:
            assert isinstance(self.value, Fraction)
            return _fraction_to_text(self.value)
        if self.kind is ExprKind.VARIABLE:
            return str(self.value)
        if self.kind is ExprKind.ADD:
            return "(" + " + ".join(map(str, self.args)) + ")"
        if self.kind is ExprKind.SCALE:
            return f"({_fraction_to_text(self.value)}*{self.args[0]})"
        return f"{self.kind.value}({self.args[0]})"


@dataclass(frozen=True, slots=True)
class ArithmeticSemantics:
    """Injectable rounding semantics used for mutation testing."""

    truncate_negative_rounding: bool = False

    def floor(self, value: Fraction) -> Fraction:
        if self.truncate_negative_rounding:
            return Fraction(int(value))
        return Fraction(floor_fraction(value))

    def ceil(self, value: Fraction) -> Fraction:
        if self.truncate_negative_rounding:
            return Fraction(int(value))
        return Fraction(ceil_fraction(value))


EXACT_SEMANTICS = ArithmeticSemantics()
TRUNCATING_SEMANTICS = ArithmeticSemantics(truncate_negative_rounding=True)


def evaluate_expr(
    expression: Expr,
    environment: dict[str, Fraction],
    semantics: ArithmeticSemantics = EXACT_SEMANTICS,
) -> Fraction:
    """Evaluate an expression using exact rational arithmetic."""

    if expression.kind is ExprKind.CONSTANT:
        assert isinstance(expression.value, Fraction)
        return expression.value
    if expression.kind is ExprKind.VARIABLE:
        assert isinstance(expression.value, str)
        try:
            return environment[expression.value]
        except KeyError as error:
            raise ValueError(f"unbound arithmetic variable {expression.value!r}") from error
    if expression.kind is ExprKind.ADD:
        return sum(
            (evaluate_expr(argument, environment, semantics) for argument in expression.args),
            Fraction(0),
        )
    if expression.kind is ExprKind.SCALE:
        assert isinstance(expression.value, Fraction)
        return expression.value * evaluate_expr(
            expression.args[0], environment, semantics
        )
    value = evaluate_expr(expression.args[0], environment, semantics)
    if expression.kind is ExprKind.FLOOR:
        return semantics.floor(value)
    assert expression.kind is ExprKind.CEIL
    return semantics.ceil(value)


@dataclass(frozen=True, slots=True)
class Atom:
    left: Expr
    relation: Relation
    right: Expr = dataclasses.field(default_factory=lambda: Expr.constant(0))

    def evaluate(
        self,
        environment: dict[str, Fraction],
        semantics: ArithmeticSemantics = EXACT_SEMANTICS,
    ) -> bool:
        return self.relation.evaluate(
            evaluate_expr(self.left, environment, semantics),
            evaluate_expr(self.right, environment, semantics),
        )

    def variables(self) -> frozenset[str]:
        return self.left.variables() | self.right.variables()

    def complexity(self) -> int:
        return 1 + self.left.complexity() + self.right.complexity()

    def difference(self) -> Expr:
        return Expr.subtract(self.left, self.right)

    def to_smt2(self) -> str:
        comparison = (
            f"({self.relation.smt_operator} "
            f"{self.left.to_smt2()} {self.right.to_smt2()})"
        )
        if self.relation is Relation.NE:
            return f"(not {comparison})"
        return comparison

    def __str__(self) -> str:
        operator = {
            Relation.EQ: "=",
            Relation.NE: "!=",
            Relation.GT: ">",
            Relation.GE: ">=",
            Relation.LT: "<",
            Relation.LE: "<=",
        }[self.relation]
        return f"{self.left} {operator} {self.right}"


class FormulaKind(str, enum.Enum):
    BOOLEAN = "boolean"
    ATOM = "atom"
    AND = "and"
    OR = "or"
    NOT = "not"


@dataclass(frozen=True, slots=True)
class Formula:
    """A small quantifier-free Boolean formula."""

    kind: FormulaKind
    value: bool | Atom | None = None
    children: tuple["Formula", ...] = ()

    @staticmethod
    def boolean(value: bool) -> "Formula":
        return Formula(FormulaKind.BOOLEAN, bool(value))

    @staticmethod
    def atom(atom: Atom) -> "Formula":
        return Formula(FormulaKind.ATOM, atom)

    @staticmethod
    def and_(*children: "Formula") -> "Formula":
        if not children:
            return Formula.boolean(True)
        if len(children) == 1:
            return children[0]
        return Formula(FormulaKind.AND, children=tuple(children))

    @staticmethod
    def or_(*children: "Formula") -> "Formula":
        if not children:
            return Formula.boolean(False)
        if len(children) == 1:
            return children[0]
        return Formula(FormulaKind.OR, children=tuple(children))

    @staticmethod
    def not_(child: "Formula") -> "Formula":
        return Formula(FormulaKind.NOT, children=(child,))

    def evaluate(
        self,
        environment: dict[str, Fraction],
        semantics: ArithmeticSemantics = EXACT_SEMANTICS,
    ) -> bool:
        if self.kind is FormulaKind.BOOLEAN:
            assert isinstance(self.value, bool)
            return self.value
        if self.kind is FormulaKind.ATOM:
            assert isinstance(self.value, Atom)
            return self.value.evaluate(environment, semantics)
        if self.kind is FormulaKind.AND:
            return all(child.evaluate(environment, semantics) for child in self.children)
        if self.kind is FormulaKind.OR:
            return any(child.evaluate(environment, semantics) for child in self.children)
        assert self.kind is FormulaKind.NOT
        return not self.children[0].evaluate(environment, semantics)

    def atoms(self) -> tuple[Atom, ...]:
        if self.kind is FormulaKind.ATOM:
            assert isinstance(self.value, Atom)
            return (self.value,)
        atoms: list[Atom] = []
        for child in self.children:
            atoms.extend(child.atoms())
        return tuple(atoms)

    def variables(self) -> frozenset[str]:
        names: set[str] = set()
        for atom in self.atoms():
            names.update(atom.variables())
        return frozenset(names)

    def complexity(self) -> int:
        if self.kind is FormulaKind.ATOM:
            assert isinstance(self.value, Atom)
            return self.value.complexity()
        return 1 + sum(child.complexity() for child in self.children)

    def to_smt2(self) -> str:
        if self.kind is FormulaKind.BOOLEAN:
            return "true" if self.value else "false"
        if self.kind is FormulaKind.ATOM:
            assert isinstance(self.value, Atom)
            return self.value.to_smt2()
        if self.kind is FormulaKind.NOT:
            return f"(not {self.children[0].to_smt2()})"
        operator = "and" if self.kind is FormulaKind.AND else "or"
        return f"({operator} {' '.join(child.to_smt2() for child in self.children)})"

    def __str__(self) -> str:
        if self.kind is FormulaKind.BOOLEAN:
            return str(self.value).lower()
        if self.kind is FormulaKind.ATOM:
            return str(self.value)
        if self.kind is FormulaKind.NOT:
            return f"not ({self.children[0]})"
        separator = " and " if self.kind is FormulaKind.AND else " or "
        return "(" + separator.join(map(str, self.children)) + ")"


class QuantifierSort(str, enum.Enum):
    REAL = "real"
    INTEGER = "integer"


@dataclass(frozen=True, slots=True)
class BoundedQuery:
    """A closed bounded existential query."""

    variable: str
    lower: Fraction
    upper: Fraction
    formula: Formula
    parameters: tuple[tuple[str, Fraction], ...] = ()
    sort: QuantifierSort = QuantifierSort.REAL
    name: str = "anonymous"

    def __post_init__(self) -> None:
        if not _SYMBOL_RE.fullmatch(self.variable):
            raise ValueError(f"invalid quantified variable name: {self.variable!r}")
        if self.lower > self.upper:
            raise ValueError("query lower bound exceeds upper bound")
        parameter_names = [name for name, _ in self.parameters]
        if len(parameter_names) != len(set(parameter_names)):
            raise ValueError("query has duplicate parameter names")
        invalid_parameters = [
            name for name in parameter_names if not _SYMBOL_RE.fullmatch(name)
        ]
        if invalid_parameters:
            raise ValueError(
                f"invalid parameter names: {sorted(invalid_parameters)}"
            )
        if self.variable in parameter_names:
            raise ValueError("quantified variable also appears as a parameter")
        undeclared = self.formula.variables() - {self.variable, *parameter_names}
        if undeclared:
            raise ValueError(f"query has undeclared variables: {sorted(undeclared)}")

    @staticmethod
    def create(
        variable: str,
        lower: int | Fraction,
        upper: int | Fraction,
        formula: Formula,
        *,
        parameters: dict[str, int | Fraction] | None = None,
        sort: QuantifierSort = QuantifierSort.REAL,
        name: str = "anonymous",
    ) -> "BoundedQuery":
        fixed_parameters = tuple(
            sorted(
                (
                    parameter_name,
                    as_fraction(parameter_value),
                )
                for parameter_name, parameter_value in (parameters or {}).items()
            )
        )
        return BoundedQuery(
            variable=variable,
            lower=as_fraction(lower),
            upper=as_fraction(upper),
            formula=formula,
            parameters=fixed_parameters,
            sort=sort,
            name=name,
        )

    def parameter_environment(self) -> dict[str, Fraction]:
        return dict(self.parameters)

    def with_formula(self, formula: Formula) -> "BoundedQuery":
        return dataclasses.replace(self, formula=formula)

    def describe(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "variable": self.variable,
            "sort": self.sort.value,
            "lower": _fraction_to_text(self.lower),
            "upper": _fraction_to_text(self.upper),
            "parameters": {
                name: _fraction_to_text(value) for name, value in self.parameters
            },
            "formula": str(self.formula),
            "complexity": self.formula.complexity(),
        }


class Decision(str, enum.Enum):
    SAT = "sat"
    UNSAT = "unsat"
    UNKNOWN = "unknown"
    ERROR = "error"


@dataclass(frozen=True, slots=True)
class OracleOutcome:
    decision: Decision
    reason: str
    witness: Fraction | None = None
    checked_cells: int = 0
    critical_points: int = 0


def decide_exact(
    query: BoundedQuery,
    *,
    semantics: ArithmeticSemantics = EXACT_SEMANTICS,
    max_cells: int = 100_000,
) -> OracleOutcome:
    """Decide a bounded query using a complete exact oracle."""

    environment = query.parameter_environment()
    if query.sort is QuantifierSort.INTEGER:
        first = ceil_fraction(query.lower)
        last = floor_fraction(query.upper)
        count = max(0, last - first + 1)
        if count > max_cells:
            return OracleOutcome(
                Decision.UNKNOWN,
                f"integer enumeration cap exceeded: {count} > {max_cells}",
            )
        for checked, integer in enumerate(range(first, last + 1), start=1):
            witness = Fraction(integer)
            environment[query.variable] = witness
            if query.formula.evaluate(environment, semantics):
                return OracleOutcome(
                    Decision.SAT,
                    "exact integer witness",
                    witness=witness,
                    checked_cells=checked,
                    critical_points=count,
                )
        return OracleOutcome(
            Decision.UNSAT,
            "complete exact integer enumeration",
            checked_cells=count,
            critical_points=count,
        )

    try:
        points = _real_critical_points(
            query,
            semantics=semantics,
            max_points=max_cells,
        )
    except _OracleLimit as error:
        return OracleOutcome(Decision.UNKNOWN, str(error))

    samples: list[Fraction] = []
    for index, point in enumerate(points):
        samples.append(point)
        if index + 1 < len(points):
            following = points[index + 1]
            samples.append((point + following) / 2)
    if len(samples) > max_cells:
        return OracleOutcome(
            Decision.UNKNOWN,
            f"real cell cap exceeded: {len(samples)} > {max_cells}",
            critical_points=len(points),
        )
    for checked, sample in enumerate(samples, start=1):
        environment[query.variable] = sample
        if query.formula.evaluate(environment, semantics):
            return OracleOutcome(
                Decision.SAT,
                "exact real cell witness",
                witness=sample,
                checked_cells=checked,
                critical_points=len(points),
            )
    return OracleOutcome(
        Decision.UNSAT,
        "complete bounded real cell decomposition",
        checked_cells=len(samples),
        critical_points=len(points),
    )


class _OracleLimit(RuntimeError):
    pass


def _real_critical_points(
    query: BoundedQuery,
    *,
    semantics: ArithmeticSemantics,
    max_points: int,
) -> tuple[Fraction, ...]:
    environment = query.parameter_environment()
    points: set[Fraction] = {query.lower, query.upper}
    rounding_nodes = sorted(
        _rounding_nodes(query.formula),
        key=lambda item: (item[0], str(item[1])),
    )
    for _, expression in rounding_nodes:
        argument = expression.args[0]
        new_points: set[Fraction] = set()
        ordered = sorted(points)
        for left, right in zip(ordered, ordered[1:], strict=False):
            if left == right:
                continue
            slope, intercept = _affine_on_open_interval(
                argument,
                query.variable,
                left,
                right,
                environment,
                semantics,
            )
            if slope == 0:
                continue
            left_value = slope * left + intercept
            right_value = slope * right + intercept
            minimum = min(left_value, right_value)
            maximum = max(left_value, right_value)
            first_integer = floor_fraction(minimum) - 1
            last_integer = ceil_fraction(maximum) + 1
            if last_integer - first_integer + 1 > max_points:
                raise _OracleLimit(
                    "rounding breakpoint cap exceeded in one affine interval"
                )
            for integer in range(first_integer, last_integer + 1):
                root = (Fraction(integer) - intercept) / slope
                if left < root < right:
                    new_points.add(root)
        points.update(new_points)
        if len(points) > max_points:
            raise _OracleLimit(
                f"rounding breakpoint cap exceeded: {len(points)} > {max_points}"
            )

    for atom in query.formula.atoms():
        difference = atom.difference()
        new_points = set()
        ordered = sorted(points)
        for left, right in zip(ordered, ordered[1:], strict=False):
            if left == right:
                continue
            slope, intercept = _affine_on_open_interval(
                difference,
                query.variable,
                left,
                right,
                environment,
                semantics,
            )
            if slope == 0:
                continue
            root = -intercept / slope
            if left < root < right:
                new_points.add(root)
        points.update(new_points)
        if len(points) > max_points:
            raise _OracleLimit(
                f"atom-zero cap exceeded: {len(points)} > {max_points}"
            )
    return tuple(sorted(points))


def _rounding_nodes(formula: Formula) -> set[tuple[int, Expr]]:
    nodes: set[tuple[int, Expr]] = set()
    for atom in formula.atoms():
        for expression in (atom.left, atom.right):
            _collect_rounding_nodes(expression, nodes)
    return nodes


def _collect_rounding_nodes(
    expression: Expr,
    nodes: set[tuple[int, Expr]],
) -> int:
    child_depth = 0
    for argument in expression.args:
        child_depth = max(child_depth, _collect_rounding_nodes(argument, nodes))
    if expression.kind in {ExprKind.FLOOR, ExprKind.CEIL}:
        depth = child_depth + 1
        nodes.add((depth, expression))
        return depth
    return child_depth


def _affine_on_open_interval(
    expression: Expr,
    variable: str,
    left: Fraction,
    right: Fraction,
    parameter_environment: dict[str, Fraction],
    semantics: ArithmeticSemantics,
) -> tuple[Fraction, Fraction]:
    width = right - left
    first = left + width / 3
    second = left + 2 * width / 3
    first_environment = dict(parameter_environment)
    second_environment = dict(parameter_environment)
    first_environment[variable] = first
    second_environment[variable] = second
    first_value = evaluate_expr(expression, first_environment, semantics)
    second_value = evaluate_expr(expression, second_environment, semantics)
    slope = (second_value - first_value) / (second - first)
    intercept = first_value - slope * first
    return slope, intercept


@dataclass(frozen=True, slots=True)
class SolverOutcome:
    decision: Decision
    reason: str
    stdout: str = ""
    stderr: str = ""
    returncode: int | None = None


@dataclass(frozen=True, slots=True)
class SmtLibProcessOracle:
    """Run a caller-supplied SMT-LIB solver as an isolated process."""

    command: tuple[str, ...]
    timeout_seconds: float = 5.0
    solver_timeout_ms: int = 4_000

    @staticmethod
    def z3(executable: str | Path, *, timeout_seconds: float = 5.0) -> "SmtLibProcessOracle":
        return SmtLibProcessOracle(
            (str(executable), "-in", "-smt2"),
            timeout_seconds=timeout_seconds,
        )

    def check(self, query: BoundedQuery) -> SolverOutcome:
        if not self.command:
            return SolverOutcome(Decision.UNKNOWN, "external solver is not configured")
        script = render_smt2(query, timeout_ms=self.solver_timeout_ms)
        try:
            completed = subprocess.run(
                self.command,
                input=script,
                text=True,
                capture_output=True,
                timeout=self.timeout_seconds,
                check=False,
            )
        except FileNotFoundError:
            return SolverOutcome(
                Decision.UNKNOWN,
                f"external solver is unavailable: {self.command[0]}",
            )
        except subprocess.TimeoutExpired as error:
            return SolverOutcome(
                Decision.UNKNOWN,
                f"external solver process timed out after {self.timeout_seconds}s",
                stdout=_coerce_subprocess_text(error.stdout),
                stderr=_coerce_subprocess_text(error.stderr),
            )
        stdout = completed.stdout
        stderr = completed.stderr
        token = next(
            (
                line.strip().lower()
                for line in stdout.splitlines()
                if line.strip().lower() in {"sat", "unsat", "unknown"}
            ),
            None,
        )
        if token == "sat":
            decision = Decision.SAT
        elif token == "unsat":
            decision = Decision.UNSAT
        elif token == "unknown":
            decision = Decision.UNKNOWN
        else:
            return SolverOutcome(
                Decision.ERROR,
                "external solver produced no check-sat verdict",
                stdout=stdout,
                stderr=stderr,
                returncode=completed.returncode,
            )
        if completed.returncode != 0:
            return SolverOutcome(
                Decision.ERROR,
                f"external solver exited with code {completed.returncode}",
                stdout=stdout,
                stderr=stderr,
                returncode=completed.returncode,
            )
        return SolverOutcome(
            decision,
            "external SMT-LIB verdict",
            stdout=stdout,
            stderr=stderr,
            returncode=completed.returncode,
        )


class DifferentialStatus(str, enum.Enum):
    SAT = "sat"
    UNSAT = "unsat"
    UNKNOWN = "unknown"
    DISAGREEMENT = "disagreement"
    ERROR = "error"


@dataclass(frozen=True, slots=True)
class DifferentialOutcome:
    status: DifferentialStatus
    exact: OracleOutcome
    external: SolverOutcome


def compare_with_external_solver(
    query: BoundedQuery,
    external: SmtLibProcessOracle,
    *,
    max_cells: int = 100_000,
) -> DifferentialOutcome:
    """Classify exact/external agreement without treating unknown as false."""

    exact = decide_exact(query, max_cells=max_cells)
    solver = external.check(query)
    if exact.decision is Decision.ERROR or solver.decision is Decision.ERROR:
        status = DifferentialStatus.ERROR
    elif (
        exact.decision is Decision.UNKNOWN
        or solver.decision is Decision.UNKNOWN
    ):
        status = DifferentialStatus.UNKNOWN
    elif exact.decision is not solver.decision:
        status = DifferentialStatus.DISAGREEMENT
    elif exact.decision is Decision.SAT:
        status = DifferentialStatus.SAT
    else:
        status = DifferentialStatus.UNSAT
    return DifferentialOutcome(status, exact, solver)


def render_smt2(query: BoundedQuery, *, timeout_ms: int = 4_000) -> str:
    """Render a closed bounded query as an SMT-LIB check."""

    lines = [
        "(set-logic ALL)",
        f"(set-option :timeout {timeout_ms})",
    ]
    names = sorted(
        query.formula.variables()
        | {query.variable}
        | {name for name, _ in query.parameters}
    )
    for name in names:
        lines.append(f"(declare-const {name} Real)")
    for name, value in query.parameters:
        lines.append(f"(assert (= {name} {_fraction_to_smt2(value)}))")
    lines.extend(
        (
            f"(assert (<= {_fraction_to_smt2(query.lower)} {query.variable}))",
            f"(assert (<= {query.variable} {_fraction_to_smt2(query.upper)}))",
        )
    )
    if query.sort is QuantifierSort.INTEGER:
        lines.append(
            f"(assert (= {query.variable} "
            f"(to_real (to_int {query.variable}))))"
        )
    lines.append(f"(assert {query.formula.to_smt2()})")
    lines.extend(("(check-sat)", "(exit)", ""))
    return "\n".join(lines)


def shrink_query(
    query: BoundedQuery,
    still_fails: Callable[[BoundedQuery], bool],
    *,
    max_attempts: int = 10_000,
) -> tuple[BoundedQuery, int]:
    """Greedily shrink a failing query while preserving the predicate."""

    if not still_fails(query):
        raise ValueError("initial query does not satisfy the failure predicate")
    current = query
    attempts = 0
    while attempts < max_attempts:
        reduced = False
        for candidate_formula in _formula_shrinks(current.formula):
            attempts += 1
            candidate = current.with_formula(candidate_formula)
            if (
                candidate.formula.complexity() < current.formula.complexity()
                and still_fails(candidate)
            ):
                current = candidate
                reduced = True
                break
            if attempts >= max_attempts:
                break
        if not reduced:
            break
    return current, attempts


def _formula_shrinks(formula: Formula) -> Iterator[Formula]:
    seen: set[Formula] = set()

    def emit(candidate: Formula) -> Iterator[Formula]:
        if candidate != formula and candidate not in seen:
            seen.add(candidate)
            yield candidate

    if formula.kind in {FormulaKind.AND, FormulaKind.OR}:
        constructor = Formula.and_ if formula.kind is FormulaKind.AND else Formula.or_
        for index in range(len(formula.children)):
            yield from emit(
                constructor(
                    *(
                        child
                        for child_index, child in enumerate(formula.children)
                        if child_index != index
                    )
                )
            )
        for index, child in enumerate(formula.children):
            for child_candidate in _formula_shrinks(child):
                children = list(formula.children)
                children[index] = child_candidate
                yield from emit(constructor(*children))
    elif formula.kind is FormulaKind.NOT:
        yield from emit(formula.children[0])
        for child_candidate in _formula_shrinks(formula.children[0]):
            yield from emit(Formula.not_(child_candidate))
    elif formula.kind is FormulaKind.ATOM:
        assert isinstance(formula.value, Atom)
        atom = formula.value
        for left in _expr_shrinks(atom.left):
            yield from emit(Formula.atom(Atom(left, atom.relation, atom.right)))
        for right in _expr_shrinks(atom.right):
            yield from emit(Formula.atom(Atom(atom.left, atom.relation, right)))


def _expr_shrinks(expression: Expr) -> Iterator[Expr]:
    seen: set[Expr] = set()

    def emit(candidate: Expr) -> Iterator[Expr]:
        if candidate != expression and candidate not in seen:
            seen.add(candidate)
            yield candidate

    yield from emit(Expr.constant(0))
    for argument in expression.args:
        yield from emit(argument)
    if expression.kind is ExprKind.ADD:
        for index in range(len(expression.args)):
            yield from emit(
                Expr.add(
                    *(
                        argument
                        for argument_index, argument in enumerate(expression.args)
                        if argument_index != index
                    )
                )
            )
    if expression.kind is ExprKind.SCALE:
        assert isinstance(expression.value, Fraction)
        yield from emit(Expr.scale(1 if expression.value > 0 else -1, expression.args[0]))
    for index, argument in enumerate(expression.args):
        for candidate_argument in _expr_shrinks(argument):
            arguments = list(expression.args)
            arguments[index] = candidate_argument
            yield from emit(dataclasses.replace(expression, args=tuple(arguments)))


def replace_ceil_with_floor(expression: Expr) -> Expr:
    """Seed the paper's motivating-example ceiling/floor typo."""

    arguments = tuple(replace_ceil_with_floor(argument) for argument in expression.args)
    if expression.kind is ExprKind.CEIL:
        return Expr.floor(arguments[0])
    return dataclasses.replace(expression, args=arguments)


def weaken_strict_relations(formula: Formula) -> Formula:
    """Seed a common strict/non-strict normalization defect."""

    if formula.kind is FormulaKind.ATOM:
        assert isinstance(formula.value, Atom)
        relation = {
            Relation.GT: Relation.GE,
            Relation.LT: Relation.LE,
        }.get(formula.value.relation, formula.value.relation)
        return Formula.atom(dataclasses.replace(formula.value, relation=relation))
    return dataclasses.replace(
        formula,
        children=tuple(weaken_strict_relations(child) for child in formula.children),
    )


def _fraction_to_smt2(value: Fraction) -> str:
    if value.denominator == 1:
        if value.numerator < 0:
            return f"(- {-value.numerator})"
        return str(value.numerator)
    numerator = (
        f"(- {-value.numerator})" if value.numerator < 0 else str(value.numerator)
    )
    return f"(/ {numerator} {value.denominator})"


def _fraction_to_text(value: Fraction | str | None) -> str:
    assert isinstance(value, Fraction)
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def _coerce_subprocess_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def make_atom(
    left: Expr,
    relation: Relation,
    right: Expr | int | Fraction = 0,
) -> Formula:
    """Convenience constructor for a formula atom."""

    right_expression = right if isinstance(right, Expr) else Expr.constant(right)
    return Formula.atom(Atom(left, relation, right_expression))


def sum_expr(terms: Sequence[Expr]) -> Expr:
    """Construct a sum from a sequence."""

    return Expr.add(*terms)
