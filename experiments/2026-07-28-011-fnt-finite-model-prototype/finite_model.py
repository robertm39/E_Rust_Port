#!/usr/bin/env python3
"""Bounded SAT-based finite-model prototype for function-free TPTP CNF."""

from __future__ import annotations

import argparse
import itertools
import json
import re
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Iterable, Iterator, Sequence


ATOM_RE = re.compile(r"(?:[a-z][A-Za-z0-9_]*|\$[A-Za-z][A-Za-z0-9_]*|'(?:[^'\\]|\\.)*')")
VARIABLE_RE = re.compile(r"[A-Z_][A-Za-z0-9_]*")
SUCCESS_STATUSES = {"SATISFIABLE", "SAT"}
FAILURE_STATUSES = {"UNSATISFIABLE", "UNSAT"}


class PrototypeError(RuntimeError):
    """Base class for controlled prototype failures."""


class UnsupportedInput(PrototypeError):
    """The clausified input is outside the prototype fragment."""


class EncodingLimit(PrototypeError):
    """The bounded encoding exceeded an explicit safety limit."""


@dataclass(frozen=True)
class Term:
    name: str
    variable: bool


@dataclass(frozen=True)
class Literal:
    positive: bool
    predicate: str | None = None
    arguments: tuple[Term, ...] = ()
    equality: tuple[Term, Term] | None = None
    fixed_truth: bool | None = None


@dataclass(frozen=True)
class Clause:
    name: str
    role: str
    literals: tuple[Literal, ...]


@dataclass(frozen=True)
class CnfProblem:
    clauses: tuple[Clause, ...]
    predicates: dict[str, int]
    constants: tuple[str, ...]
    has_conjecture: bool


@dataclass(frozen=True)
class SortLayout:
    sort_count: int
    constant_sorts: dict[str, int]
    predicate_sorts: dict[tuple[str, int], int]
    variable_sorts: dict[tuple[int, str], int]


@dataclass
class BoundReport:
    domain_sizes: list[int]
    propositional_variables: int = 0
    propositional_clauses: int = 0
    ground_instances: int = 0
    encoding_seconds: float = 0.0
    sat_seconds: float = 0.0
    sat_status: str = "not_run"
    conflicts: int | None = None
    decisions: int | None = None
    propagations: int | None = None


@dataclass
class RunReport:
    schema_version: int
    problem: str
    mode: str
    max_size: int
    fragment: str = "function-free-untyped-cnf"
    outcome: str = "unknown"
    claimed_status: str | None = None
    clause_count: int = 0
    predicate_count: int = 0
    constant_count: int = 0
    inferred_sort_count: int = 0
    clausification_seconds: float = 0.0
    bounds: list[BoundReport] = field(default_factory=list)
    reason: str | None = None


class UnionFind:
    def __init__(self) -> None:
        self.parent: dict[tuple[object, ...], tuple[object, ...]] = {}

    def add(self, item: tuple[object, ...]) -> None:
        self.parent.setdefault(item, item)

    def find(self, item: tuple[object, ...]) -> tuple[object, ...]:
        self.add(item)
        parent = self.parent[item]
        if parent != item:
            self.parent[item] = self.find(parent)
        return self.parent[item]

    def union(self, left: tuple[object, ...], right: tuple[object, ...]) -> None:
        left_root = self.find(left)
        right_root = self.find(right)
        if left_root == right_root:
            return
        if repr(left_root) <= repr(right_root):
            self.parent[right_root] = left_root
        else:
            self.parent[left_root] = right_root


def strip_comments(text: str) -> str:
    """Remove TPTP line comments without changing non-comment line content."""

    return "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("%"))


def statements(text: str) -> Iterator[str]:
    """Yield top-level period-terminated TPTP statements."""

    source = strip_comments(text)
    start = 0
    paren_depth = 0
    bracket_depth = 0
    quote: str | None = None
    escaped = False
    for index, char in enumerate(source):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char == "(":
            paren_depth += 1
        elif char == ")":
            paren_depth -= 1
        elif char == "[":
            bracket_depth += 1
        elif char == "]":
            bracket_depth -= 1
        elif char == "." and paren_depth == 0 and bracket_depth == 0:
            statement = source[start : index + 1].strip()
            if statement:
                yield statement
            start = index + 1
    if source[start:].strip():
        raise PrototypeError("unterminated TPTP statement in clausifier output")


def split_top_level(text: str, separator: str) -> list[str]:
    """Split on one separator outside parentheses, brackets, and quotes."""

    parts: list[str] = []
    start = 0
    paren_depth = 0
    bracket_depth = 0
    quote: str | None = None
    escaped = False
    for index, char in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char == "(":
            paren_depth += 1
        elif char == ")":
            paren_depth -= 1
        elif char == "[":
            bracket_depth += 1
        elif char == "]":
            bracket_depth -= 1
        elif char == separator and paren_depth == 0 and bracket_depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    parts.append(text[start:].strip())
    return parts


def enclosing_parentheses(text: str) -> bool:
    if len(text) < 2 or text[0] != "(" or text[-1] != ")":
        return False
    depth = 0
    quote: str | None = None
    escaped = False
    for index, char in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0 and index != len(text) - 1:
                return False
    return depth == 0


def strip_enclosing_parentheses(text: str) -> str:
    result = text.strip()
    while enclosing_parentheses(result):
        result = result[1:-1].strip()
    return result


def parse_term(text: str) -> Term:
    source = strip_enclosing_parentheses(text)
    if VARIABLE_RE.fullmatch(source):
        return Term(source, True)
    if source.startswith('"') or re.fullmatch(r"[+-]?\d+(?:\.\d+)?", source):
        raise UnsupportedInput("distinct objects and numeric terms are not supported")
    if ATOM_RE.fullmatch(source):
        if source.startswith("$"):
            raise UnsupportedInput(f"interpreted term {source} is not supported")
        return Term(source, False)
    if "(" in source:
        name = source.split("(", 1)[0].strip()
        raise UnsupportedInput(f"positive-arity function symbol {name} is not supported")
    raise UnsupportedInput(f"unsupported term syntax: {source}")


def find_top_level_equality(text: str) -> tuple[int, str] | None:
    depth = 0
    quote: str | None = None
    escaped = False
    index = 0
    while index < len(text):
        char = text[index]
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif depth == 0:
            if text.startswith("!=", index):
                return index, "!="
            if char == "=":
                return index, "="
        index += 1
    return None


def parse_literal(text: str) -> Literal:
    source = strip_enclosing_parentheses(text)
    positive = True
    while source.startswith("~"):
        positive = not positive
        source = strip_enclosing_parentheses(source[1:].strip())

    if source in {"$true", "$false"}:
        truth = source == "$true"
        return Literal(positive=True, fixed_truth=truth if positive else not truth)

    equality = find_top_level_equality(source)
    if equality is not None:
        index, operator = equality
        left = parse_term(source[:index])
        right = parse_term(source[index + len(operator) :])
        if operator == "!=":
            positive = not positive
        return Literal(positive=positive, equality=(left, right))

    opening = source.find("(")
    if opening < 0:
        name = source
        arguments: tuple[Term, ...] = ()
    else:
        if not source.endswith(")"):
            raise UnsupportedInput(f"malformed predicate literal: {source}")
        name = source[:opening].strip()
        argument_text = source[opening + 1 : -1]
        arguments = tuple(parse_term(part) for part in split_top_level(argument_text, ","))
    if not ATOM_RE.fullmatch(name):
        raise UnsupportedInput(f"unsupported predicate name: {name}")
    if name.startswith("$"):
        raise UnsupportedInput(f"interpreted predicate {name} is not supported")
    return Literal(positive=positive, predicate=name, arguments=arguments)


def parse_cnf(text: str) -> CnfProblem:
    clauses: list[Clause] = []
    predicates: dict[str, int] = {}
    constants: set[str] = set()
    has_conjecture = False

    for statement in statements(text):
        if not statement.lower().startswith("cnf"):
            continue
        opening = statement.find("(")
        if opening < 0 or not statement.endswith(")."):
            raise PrototypeError(f"malformed cnf statement: {statement[:80]}")
        fields = split_top_level(statement[opening + 1 : -2], ",")
        if len(fields) < 3:
            raise PrototypeError(f"cnf statement has fewer than three fields: {statement[:80]}")
        name, role = fields[0], fields[1].lower()
        body = strip_enclosing_parentheses(fields[2])
        literal_texts = [] if body == "$false" else split_top_level(body, "|")
        literals = tuple(parse_literal(item) for item in literal_texts)
        for literal in literals:
            if literal.predicate is not None:
                arity = len(literal.arguments)
                previous = predicates.setdefault(literal.predicate, arity)
                if previous != arity:
                    raise PrototypeError(
                        f"predicate {literal.predicate} has arities {previous} and {arity}"
                    )
                constants.update(term.name for term in literal.arguments if not term.variable)
            if literal.equality is not None:
                constants.update(term.name for term in literal.equality if not term.variable)
        has_conjecture |= role in {"conjecture", "negated_conjecture"}
        clauses.append(Clause(name=name, role=role, literals=literals))

    if not clauses:
        raise PrototypeError("clausifier output contains no cnf clauses")
    return CnfProblem(
        clauses=tuple(clauses),
        predicates=predicates,
        constants=tuple(sorted(constants)),
        has_conjecture=has_conjecture,
    )


def term_node(clause_index: int, term: Term) -> tuple[object, ...]:
    return ("v", clause_index, term.name) if term.variable else ("c", term.name)


def infer_sorts(problem: CnfProblem, mode: str) -> SortLayout:
    if mode == "naive":
        return SortLayout(
            sort_count=1,
            constant_sorts={name: 0 for name in problem.constants},
            predicate_sorts={(name, index): 0 for name, arity in problem.predicates.items() for index in range(arity)},
            variable_sorts={
                (clause_index, term.name): 0
                for clause_index, clause in enumerate(problem.clauses)
                for literal in clause.literals
                for term in literal.arguments
                + (() if literal.equality is None else literal.equality)
                if term.variable
            },
        )

    union_find = UnionFind()
    predicate_nodes: dict[tuple[str, int], tuple[object, ...]] = {}
    constant_nodes = {name: ("c", name) for name in problem.constants}
    for node in constant_nodes.values():
        union_find.add(node)

    variable_nodes: dict[tuple[int, str], tuple[object, ...]] = {}
    for clause_index, clause in enumerate(problem.clauses):
        for literal in clause.literals:
            if literal.predicate is not None:
                for position, term in enumerate(literal.arguments):
                    position_key = (literal.predicate, position)
                    position_node = predicate_nodes.setdefault(
                        position_key, ("p", literal.predicate, position)
                    )
                    variable_key = (clause_index, term.name)
                    node = (
                        variable_nodes.setdefault(variable_key, ("v", *variable_key))
                        if term.variable
                        else constant_nodes[term.name]
                    )
                    union_find.union(position_node, node)
            if literal.equality is not None:
                left, right = literal.equality
                nodes: list[tuple[object, ...]] = []
                for term in (left, right):
                    variable_key = (clause_index, term.name)
                    nodes.append(
                        variable_nodes.setdefault(variable_key, ("v", *variable_key))
                        if term.variable
                        else constant_nodes[term.name]
                    )
                union_find.union(nodes[0], nodes[1])

    if not union_find.parent:
        union_find.add(("domain",))
    roots = sorted({union_find.find(node) for node in union_find.parent}, key=repr)
    root_numbers = {root: index for index, root in enumerate(roots)}

    def number(node: tuple[object, ...]) -> int:
        return root_numbers[union_find.find(node)]

    return SortLayout(
        sort_count=len(roots),
        constant_sorts={name: number(node) for name, node in constant_nodes.items()},
        predicate_sorts={key: number(node) for key, node in predicate_nodes.items()},
        variable_sorts={key: number(node) for key, node in variable_nodes.items()},
    )


class Encoding:
    def __init__(
        self,
        problem: CnfProblem,
        layout: SortLayout,
        sort_sizes: tuple[int, ...],
        symmetry: bool,
        max_ground_instances: int,
    ) -> None:
        self.problem = problem
        self.layout = layout
        self.sort_sizes = sort_sizes
        self.symmetry = symmetry
        self.max_ground_instances = max_ground_instances
        self.next_variable = 1
        self.clauses: list[list[int]] = []
        self.constant_variables: dict[tuple[str, int], int] = {}
        self.predicate_variables: dict[tuple[str, tuple[int, ...]], int] = {}
        self.ground_instances = 0

    def sort_size(self, sort: int) -> int:
        return self.sort_sizes[sort]

    def fresh_variable(self) -> int:
        result = self.next_variable
        self.next_variable += 1
        return result

    def constant_variable(self, name: str, value: int) -> int:
        key = (name, value)
        if key not in self.constant_variables:
            self.constant_variables[key] = self.fresh_variable()
        return self.constant_variables[key]

    def predicate_variable(self, name: str, arguments: tuple[int, ...]) -> int:
        key = (name, arguments)
        if key not in self.predicate_variables:
            self.predicate_variables[key] = self.fresh_variable()
        return self.predicate_variables[key]

    def add_exactly_one(self, variables: Sequence[int]) -> None:
        self.clauses.append(list(variables))
        for left_index, left in enumerate(variables):
            for right in variables[left_index + 1 :]:
                self.clauses.append([-left, -right])

    def add_symbol_variables(self) -> None:
        constants_by_sort: dict[int, list[str]] = {
            sort: [] for sort in range(self.layout.sort_count)
        }
        for constant in self.problem.constants:
            sort = self.layout.constant_sorts[constant]
            constants_by_sort[sort].append(constant)
            variables = [
                self.constant_variable(constant, value)
                for value in range(self.sort_size(sort))
            ]
            self.add_exactly_one(variables)

        if self.symmetry:
            for sort, constants in constants_by_sort.items():
                for index, constant in enumerate(sorted(constants)):
                    maximum = min(index, self.sort_size(sort) - 1)
                    for value in range(maximum + 1, self.sort_size(sort)):
                        self.clauses.append([-self.constant_variable(constant, value)])

        for predicate, arity in sorted(self.problem.predicates.items()):
            argument_ranges = [
                range(self.sort_size(self.layout.predicate_sorts[(predicate, position)]))
                for position in range(arity)
            ]
            for arguments in itertools.product(*argument_ranges):
                self.predicate_variable(predicate, tuple(arguments))

    def term_value(
        self,
        clause_index: int,
        term: Term,
        variable_values: dict[str, int],
        constant_values: dict[str, int],
    ) -> int:
        del clause_index
        return variable_values[term.name] if term.variable else constant_values[term.name]

    def literal_value(
        self,
        clause_index: int,
        literal: Literal,
        variable_values: dict[str, int],
        constant_values: dict[str, int],
    ) -> bool | int:
        if literal.fixed_truth is not None:
            return literal.fixed_truth
        if literal.equality is not None:
            left, right = literal.equality
            equal = self.term_value(
                clause_index, left, variable_values, constant_values
            ) == self.term_value(clause_index, right, variable_values, constant_values)
            return equal if literal.positive else not equal
        assert literal.predicate is not None
        arguments = tuple(
            self.term_value(clause_index, term, variable_values, constant_values)
            for term in literal.arguments
        )
        variable = self.predicate_variable(literal.predicate, arguments)
        return variable if literal.positive else -variable

    def add_ground_clause(
        self,
        clause_index: int,
        clause: Clause,
        variable_values: dict[str, int],
        constant_values: dict[str, int],
    ) -> None:
        encoded = [
            -self.constant_variable(name, value)
            for name, value in constant_values.items()
        ]
        seen = set(encoded)
        for literal in clause.literals:
            value = self.literal_value(
                clause_index, literal, variable_values, constant_values
            )
            if value is True:
                return
            if value is False:
                continue
            assert isinstance(value, int)
            if -value in seen:
                return
            if value not in seen:
                encoded.append(value)
                seen.add(value)
        self.clauses.append(encoded)

    def add_ground_instances(self) -> None:
        for clause_index, clause in enumerate(self.problem.clauses):
            variables = sorted(
                {
                    term.name
                    for literal in clause.literals
                    for term in literal.arguments
                    + (() if literal.equality is None else literal.equality)
                    if term.variable
                }
            )
            constants = sorted(
                {
                    term.name
                    for literal in clause.literals
                    for term in literal.arguments
                    + (() if literal.equality is None else literal.equality)
                    if not term.variable
                }
            )
            variable_ranges = [
                range(self.sort_size(self.layout.variable_sorts[(clause_index, name)]))
                for name in variables
            ]
            constant_ranges = [
                range(self.sort_size(self.layout.constant_sorts[name]))
                for name in constants
            ]
            instance_count = 1
            for values in (*variable_ranges, *constant_ranges):
                instance_count *= len(values)
            self.ground_instances += instance_count
            if self.ground_instances > self.max_ground_instances:
                raise EncodingLimit(
                    "ground-instance limit exceeded "
                    f"({self.ground_instances} > {self.max_ground_instances})"
                )
            for variable_tuple in itertools.product(*variable_ranges):
                variable_values = dict(zip(variables, variable_tuple, strict=True))
                for constant_tuple in itertools.product(*constant_ranges):
                    constant_values = dict(zip(constants, constant_tuple, strict=True))
                    self.add_ground_clause(
                        clause_index, clause, variable_values, constant_values
                    )

    def build(self) -> None:
        self.add_symbol_variables()
        self.add_ground_instances()

    @property
    def variable_count(self) -> int:
        return self.next_variable - 1

    def dimacs(self) -> str:
        lines = [f"p cnf {self.variable_count} {len(self.clauses)}"]
        lines.extend(" ".join(map(str, clause)) + " 0" for clause in self.clauses)
        return "\n".join(lines) + "\n"


@dataclass(frozen=True)
class SatResult:
    status: str
    assignment: frozenset[int]
    seconds: float
    conflicts: int | None
    decisions: int | None
    propagations: int | None
    output: str


def parse_statistic(output: str, name: str) -> int | None:
    match = re.search(rf"(?im)^\s*c\s+{re.escape(name)}\s*:\s*([0-9]+)", output)
    return int(match.group(1)) if match else None


def run_sat_solver(
    executable: Path, encoding: Encoding, timeout_seconds: float
) -> SatResult:
    with tempfile.TemporaryDirectory(prefix="umlaut-fmb-") as temporary:
        dimacs = Path(temporary) / "model.cnf"
        dimacs.write_text(encoding.dimacs(), encoding="utf-8", newline="\n")
        started = time.monotonic()
        try:
            completed = subprocess.run(
                [str(executable), str(dimacs)],
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=timeout_seconds,
            )
        except subprocess.TimeoutExpired as error:
            output = (error.stdout or "") + "\n" + (error.stderr or "")
            return SatResult(
                "timeout", frozenset(), time.monotonic() - started, None, None, None, output
            )
        seconds = time.monotonic() - started
        output = completed.stdout + "\n" + completed.stderr
        status_matches = re.findall(r"(?im)^\s*s\s+([A-Z]+)", output)
        status = status_matches[-1] if status_matches else "ERROR"
        assignment: set[int] = set()
        if status in SUCCESS_STATUSES:
            for line in output.splitlines():
                if line.startswith("v "):
                    assignment.update(
                        value
                        for value in map(int, line[2:].split())
                        if value > 0
                    )
        elif status not in FAILURE_STATUSES:
            status = "error"
        return SatResult(
            status=status.lower(),
            assignment=frozenset(assignment),
            seconds=seconds,
            conflicts=parse_statistic(output, "conflicts"),
            decisions=parse_statistic(output, "decisions"),
            propagations=parse_statistic(output, "propagations"),
            output=output[-16_384:],
        )


def choose_domain_prefix(problem: CnfProblem) -> str:
    symbols = set(problem.constants) | set(problem.predicates)
    prefix = "umlaut_fmb_d"
    while any(symbol.startswith(prefix) for symbol in symbols):
        prefix += "x"
    return prefix


def selected_constant_values(
    encoding: Encoding, assignment: frozenset[int]
) -> dict[str, int]:
    result: dict[str, int] = {}
    for constant in encoding.problem.constants:
        sort = encoding.layout.constant_sorts[constant]
        selected = [
            value
            for value in range(encoding.sort_size(sort))
            if encoding.constant_variable(constant, value) in assignment
        ]
        if len(selected) != 1:
            raise PrototypeError(
                f"SAT assignment gives {len(selected)} values to constant {constant}"
            )
        result[constant] = selected[0]
    return result


def conjunction(items: Iterable[str]) -> str:
    values = list(items)
    if not values:
        return "$true"
    if len(values) == 1:
        return values[0]
    return "( " + "\n    & ".join(values) + " )"


def render_model(
    problem_name: str,
    problem: CnfProblem,
    layout: SortLayout,
    encoding: Encoding,
    assignment: frozenset[int],
) -> str:
    prefix = choose_domain_prefix(problem)
    domain = [
        f"{prefix}{sort}_{value}"
        for sort in range(layout.sort_count)
        for value in range(encoding.sort_size(sort))
    ]

    def element(sort: int, value: int) -> str:
        return f"{prefix}{sort}_{value}"

    lines = [
        "% SZS status "
        + ("CounterSatisfiable" if problem.has_conjecture else "Satisfiable")
        + f" for {problem_name}",
        f"% SZS output start FiniteModel for {problem_name}",
        "fof(finite_domain,axiom,",
        "    ! [X] : ( " + " | ".join(f"X = {item}" for item in domain) + " ) ).",
    ]
    if len(domain) > 1:
        inequalities = (
            f"{left} != {right}"
            for left_index, left in enumerate(domain)
            for right in domain[left_index + 1 :]
        )
        lines.append(
            "fof(distinct_domain,axiom,\n    "
            + conjunction(inequalities)
            + " )."
        )

    constant_values = selected_constant_values(encoding, assignment)
    for index, constant in enumerate(problem.constants):
        sort = layout.constant_sorts[constant]
        lines.append(
            f"fof(umlaut_constant_{index},axiom,"
            f"{constant} = {element(sort, constant_values[constant])})."
        )

    for predicate_index, (predicate, arity) in enumerate(sorted(problem.predicates.items())):
        rows: list[str] = []
        for global_arguments in itertools.product(domain, repeat=arity):
            local_arguments: list[int] = []
            for position, global_element in enumerate(global_arguments):
                expected_sort = layout.predicate_sorts[(predicate, position)]
                match = re.fullmatch(re.escape(prefix) + r"(\d+)_(\d+)", global_element)
                assert match is not None
                actual_sort, actual_value = map(int, match.groups())
                local_arguments.append(actual_value if actual_sort == expected_sort else 0)
            variable = encoding.predicate_variable(predicate, tuple(local_arguments))
            atom = (
                predicate
                if arity == 0
                else f"{predicate}({','.join(global_arguments)})"
            )
            rows.append(atom if variable in assignment else f"~{atom}")
        lines.append(
            f"fof(umlaut_predicate_{predicate_index},axiom,\n    "
            + conjunction(rows)
            + " )."
        )

    lines.append(f"% SZS output end FiniteModel for {problem_name}")
    return "\n".join(lines) + "\n"


def run_clausifier(
    executable: Path, problem: Path, timeout_seconds: float
) -> tuple[str, float]:
    started = time.monotonic()
    completed = subprocess.run(
        [
            str(executable),
            "--cnf",
            "--no-preprocessing",
            "--tstp-format",
            str(problem),
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout_seconds,
    )
    seconds = time.monotonic() - started
    if completed.returncode != 0:
        raise PrototypeError(
            f"Umlaut clausification exited {completed.returncode}: "
            f"{completed.stderr[-2000:]}"
        )
    return completed.stdout, seconds


def write_report(report: RunReport, path: Path | None) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(asdict(report), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def no_claim(problem_name: str, status: str, reason: str) -> str:
    return f"% SZS status {status} for {problem_name}\n% {reason}\n"


def domain_size_vectors(sort_count: int, max_size: int) -> Iterator[tuple[int, ...]]:
    """Enumerate nonempty sort sizes by increasing total size and contour."""

    def bounded_compositions(
        total: int, remaining: int, maximum: int, prefix: tuple[int, ...]
    ) -> Iterator[tuple[int, ...]]:
        if remaining == 1:
            if total <= maximum:
                yield prefix + (total,)
            return
        for value in range(min(total, maximum) + 1):
            yield from bounded_compositions(
                total - value, remaining - 1, maximum, prefix + (value,)
            )

    maximum_extra = max_size - 1
    for total_extra in range(sort_count * maximum_extra + 1):
        for extras in bounded_compositions(total_extra, sort_count, maximum_extra, ()):
            yield tuple(value + 1 for value in extras)


def execute(args: argparse.Namespace) -> tuple[str, RunReport]:
    report = RunReport(
        schema_version=1,
        problem=str(args.problem.resolve()),
        mode=args.mode,
        max_size=args.max_size,
    )
    try:
        cnf_text, report.clausification_seconds = run_clausifier(
            args.umlaut, args.problem, args.clausify_timeout_seconds
        )
        problem = parse_cnf(cnf_text)
        layout = infer_sorts(problem, args.mode)
    except UnsupportedInput as error:
        report.outcome = "unsupported"
        report.reason = str(error)
        return no_claim(args.problem.name, "Inappropriate", str(error)), report
    except (OSError, subprocess.SubprocessError, PrototypeError) as error:
        report.outcome = "input_error"
        report.reason = str(error)
        return no_claim(args.problem.name, "InputError", str(error)), report

    report.clause_count = len(problem.clauses)
    report.predicate_count = len(problem.predicates)
    report.constant_count = len(problem.constants)
    report.inferred_sort_count = layout.sort_count
    if args.analyze_only:
        report.outcome = "supported"
        return no_claim(args.problem.name, "Unknown", "supported fragment"), report

    symmetry = args.mode == "sorted-symmetry"
    vectors: Iterable[tuple[int, ...]]
    if args.mode == "naive":
        vectors = ((size,) for size in range(1, args.max_size + 1))
        vectors_truncated = False
    else:
        total_vectors = args.max_size**layout.sort_count
        vectors_truncated = total_vectors > args.max_size_vectors
        vectors = itertools.islice(
            domain_size_vectors(layout.sort_count, args.max_size),
            args.max_size_vectors,
        )

    for sort_sizes in vectors:
        bound = BoundReport(domain_sizes=list(sort_sizes))
        report.bounds.append(bound)
        write_report(report, args.report)
        try:
            started = time.monotonic()
            encoding = Encoding(
                problem,
                layout,
                sort_sizes,
                symmetry,
                args.max_ground_instances,
            )
            encoding.build()
            bound.encoding_seconds = time.monotonic() - started
        except EncodingLimit as error:
            report.outcome = "resource_out"
            report.reason = str(error)
            return no_claim(args.problem.name, "ResourceOut", str(error)), report
        bound.propositional_variables = encoding.variable_count
        bound.propositional_clauses = len(encoding.clauses)
        bound.ground_instances = encoding.ground_instances
        write_report(report, args.report)

        try:
            result = run_sat_solver(args.sat, encoding, args.sat_timeout_seconds)
        except OSError as error:
            report.outcome = "solver_error"
            report.reason = str(error)
            return no_claim(args.problem.name, "InputError", str(error)), report
        bound.sat_status = result.status
        bound.sat_seconds = result.seconds
        bound.conflicts = result.conflicts
        bound.decisions = result.decisions
        bound.propagations = result.propagations
        write_report(report, args.report)
        if result.status in {"satisfiable", "sat"}:
            claimed = (
                "CounterSatisfiable" if problem.has_conjecture else "Satisfiable"
            )
            report.outcome = "model"
            report.claimed_status = claimed
            return (
                render_model(
                    args.problem.name, problem, layout, encoding, result.assignment
                ),
                report,
            )
        if result.status == "timeout":
            report.outcome = "timeout"
            report.reason = f"SAT timeout at sizes {list(sort_sizes)}"
            return no_claim(args.problem.name, "Timeout", report.reason), report
        if result.status not in {"unsatisfiable", "unsat"}:
            report.outcome = "solver_error"
            report.reason = (
                f"unrecognized SAT result at sizes {list(sort_sizes)}: "
                f"{result.status}"
            )
            return no_claim(args.problem.name, "InputError", report.reason), report

    if vectors_truncated:
        report.outcome = "resource_out"
        report.reason = (
            f"size-vector limit reached ({args.max_size_vectors} of "
            f"{args.max_size**layout.sort_count})"
        )
        return no_claim(args.problem.name, "ResourceOut", report.reason), report
    report.outcome = "bounds_exhausted"
    report.reason = f"no finite model found through per-sort size {args.max_size}"
    return no_claim(args.problem.name, "GaveUp", report.reason), report


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("problem", type=Path)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--sat", type=Path, required=True)
    parser.add_argument(
        "--mode",
        choices=("naive", "sorted", "sorted-symmetry"),
        default="sorted-symmetry",
    )
    parser.add_argument("--max-size", type=int, default=4)
    parser.add_argument("--max-size-vectors", type=int, default=4096)
    parser.add_argument("--max-ground-instances", type=int, default=2_000_000)
    parser.add_argument("--sat-timeout-seconds", type=float, default=30.0)
    parser.add_argument("--clausify-timeout-seconds", type=float, default=30.0)
    parser.add_argument("--analyze-only", action="store_true")
    parser.add_argument("--report", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.max_size < 1:
        raise SystemExit("--max-size must be at least one")
    solution, report = execute(args)
    write_report(report, args.report)
    sys.stdout.write(solution)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
