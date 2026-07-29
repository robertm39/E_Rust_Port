#!/usr/bin/env python3
"""Bounded typed finite-model worker with incremental function-table SAT."""

from __future__ import annotations

import argparse
import itertools
import json
import queue
import re
import subprocess
import threading
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable, Iterator, Sequence


class WorkerError(Exception):
    """Base class for controlled worker failures."""


class UnsupportedInput(WorkerError):
    """The input is outside the deliberately supported fragment."""


class EncodingLimit(WorkerError):
    """The configured encoding limit was reached."""


class SolverFailure(WorkerError):
    """The incremental SAT process violated its protocol."""


@dataclass(frozen=True)
class SymbolType:
    arguments: tuple[str, ...]
    result: str


@dataclass(frozen=True)
class Term:
    name: str
    arguments: tuple["Term", ...]
    sort: str
    variable: bool = False


@dataclass(frozen=True)
class Literal:
    predicate: str | None
    arguments: tuple[Term, ...]
    left: Term | None
    right: Term | None
    negated: bool
    truth: bool | None = None


@dataclass(frozen=True)
class Clause:
    name: str
    role: str
    literals: tuple[Literal, ...]
    variables: tuple[tuple[str, str], ...]


@dataclass
class TypedProblem:
    clauses: tuple[Clause, ...]
    functions: dict[str, SymbolType]
    predicates: dict[str, SymbolType]
    sorts: tuple[str, ...]
    has_conjecture: bool


@dataclass
class BoundReport:
    sizes: dict[str, int]
    new_ground_instances: int
    cumulative_ground_instances: int
    new_clauses: int
    cumulative_clauses: int
    propositional_variables: int
    grounding_seconds: float
    insertion_seconds: float
    sat_seconds: float
    sat_status: str
    model_variables: int


IDENTIFIER_RE = re.compile(
    r"""(?x)
    (?:
        [a-z][A-Za-z0-9_]*
        |\$[A-Za-z][A-Za-z0-9_]*
        |'(?:\\.|[^'\\])*'
        |[A-Z][A-Za-z0-9_]*
    )
    """
)
INTERPRETED_SORTS = {"$o", "$tType", "$int", "$rat", "$real"}


def split_top_level(text: str, separator: str = ",") -> list[str]:
    """Split at a one-character separator outside brackets and quotes."""

    parts: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    escaped = False
    for index, character in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "([":
            depth += 1
        elif character in ")]":
            depth -= 1
            if depth < 0:
                raise UnsupportedInput("unbalanced closing delimiter")
        elif character == separator and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    if quote is not None or depth != 0:
        raise UnsupportedInput("unterminated quote or delimiter")
    parts.append(text[start:].strip())
    return parts


def find_top_level(text: str, operators: Sequence[str]) -> tuple[int, str] | None:
    depth = 0
    quote: str | None = None
    escaped = False
    for index, character in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "([":
            depth += 1
        elif character in ")]":
            depth -= 1
        elif depth == 0:
            for operator in operators:
                if text.startswith(operator, index):
                    return index, operator
    return None


def strip_outer_parentheses(text: str) -> str:
    result = text.strip()
    while result.startswith("(") and result.endswith(")"):
        depth = 0
        quote: str | None = None
        escaped = False
        closes_at_end = False
        for index, character in enumerate(result):
            if quote is not None:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == quote:
                    quote = None
                continue
            if character in {"'", '"'}:
                quote = character
            elif character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
                if depth == 0:
                    closes_at_end = index == len(result) - 1
                    break
        if not closes_at_end:
            break
        result = result[1:-1].strip()
    return result


def statements(text: str) -> list[str]:
    logical = "\n".join(
        line for line in text.splitlines() if not line.lstrip().startswith("%")
    )
    result: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    escaped = False
    for index, character in enumerate(logical):
        if quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "([":
            depth += 1
        elif character in ")]":
            depth -= 1
        elif character == "." and depth == 0:
            item = logical[start : index + 1].strip()
            if item:
                result.append(item)
            start = index + 1
    if logical[start:].strip():
        raise UnsupportedInput("unterminated TSTP statement")
    return result


def parse_atomic_type(text: str) -> str:
    value = strip_outer_parentheses(text)
    if not IDENTIFIER_RE.fullmatch(value):
        raise UnsupportedInput(f"unsupported type expression {text!r}")
    if value in {"$int", "$rat", "$real"}:
        raise UnsupportedInput(f"interpreted sort {value} is unsupported")
    if value.startswith("'"):
        raise UnsupportedInput("quoted native sort names are unsupported")
    return value


def parse_symbol_type(text: str) -> SymbolType:
    value = strip_outer_parentheses(text)
    arrow = find_top_level(value, (">",))
    if arrow is None:
        return SymbolType((), parse_atomic_type(value))
    index, _ = arrow
    domain = strip_outer_parentheses(value[:index])
    result = parse_atomic_type(value[index + 1 :])
    arguments = tuple(
        parse_atomic_type(item) for item in split_top_level(domain, "*")
    )
    if not arguments or any(sort in {"$o", "$tType"} for sort in arguments):
        raise UnsupportedInput(f"non-first-order symbol type {text!r}")
    return SymbolType(arguments, result)


def parse_type_declaration(body: str) -> tuple[str, SymbolType]:
    colon = find_top_level(body, (":",))
    if colon is None:
        raise UnsupportedInput(f"malformed type declaration {body!r}")
    index, _ = colon
    name = body[:index].strip()
    if not IDENTIFIER_RE.fullmatch(name) or name[0].isupper():
        raise UnsupportedInput(f"unsupported declared symbol {name!r}")
    return name, parse_symbol_type(body[index + 1 :])


def term_suffix(text: str) -> tuple[str, str]:
    locations: list[int] = []
    depth = 0
    quote: str | None = None
    escaped = False
    for index, character in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
        elif character == ":" and depth == 0:
            locations.append(index)
    if len(locations) != 1:
        raise UnsupportedInput(f"term lacks one explicit type suffix: {text!r}")
    index = locations[0]
    return text[:index].strip(), parse_atomic_type(text[index + 1 :])


def parse_term(text: str) -> Term:
    core, sort = term_suffix(strip_outer_parentheses(text))
    core = core.strip()
    if core.startswith('"'):
        raise UnsupportedInput("distinct objects are unsupported")
    if core and core[0].isupper():
        if not IDENTIFIER_RE.fullmatch(core):
            raise UnsupportedInput(f"malformed variable {core!r}")
        return Term(core, (), sort, True)
    open_index = core.find("(")
    if open_index < 0:
        name = core
        arguments: tuple[Term, ...] = ()
    else:
        if not core.endswith(")"):
            raise UnsupportedInput(f"malformed application {core!r}")
        name = core[:open_index].strip()
        raw_arguments = core[open_index + 1 : -1]
        arguments = tuple(
            parse_term(item) for item in split_top_level(raw_arguments)
        )
    if not IDENTIFIER_RE.fullmatch(name) or name[0].isupper():
        raise UnsupportedInput(f"unsupported term symbol {name!r}")
    if name.startswith("$") and name not in {"$true", "$false"}:
        raise UnsupportedInput(f"interpreted symbol {name} is unsupported")
    return Term(name, arguments, sort)


def register_symbol(
    target: dict[str, SymbolType],
    other: dict[str, SymbolType],
    name: str,
    symbol_type: SymbolType,
    kind: str,
) -> None:
    if name in other:
        raise UnsupportedInput(f"{name} is used as both function and predicate")
    previous = target.get(name)
    if previous is not None and previous != symbol_type:
        raise UnsupportedInput(
            f"inconsistent {kind} type for {name}: {previous} versus {symbol_type}"
        )
    target[name] = symbol_type


def validate_term(
    term: Term,
    functions: dict[str, SymbolType],
    predicates: dict[str, SymbolType],
    variables: dict[str, str],
) -> None:
    if term.variable:
        previous = variables.setdefault(term.name, term.sort)
        if previous != term.sort:
            raise UnsupportedInput(
                f"variable {term.name} has both {previous} and {term.sort}"
            )
        return
    if term.name in {"$true", "$false"}:
        if term.arguments or term.sort != "$o":
            raise UnsupportedInput(f"malformed truth constant {term}")
        return
    for argument in term.arguments:
        validate_term(argument, functions, predicates, variables)
    symbol_type = SymbolType(
        tuple(argument.sort for argument in term.arguments), term.sort
    )
    register_symbol(functions, predicates, term.name, symbol_type, "function")


def parse_literal(
    text: str,
    functions: dict[str, SymbolType],
    predicates: dict[str, SymbolType],
    variables: dict[str, str],
) -> Literal:
    value = strip_outer_parentheses(text)
    negated = value.startswith("~")
    if negated:
        value = strip_outer_parentheses(value[1:])
    if value in {"$true", "$false"}:
        return Literal(
            None,
            (),
            None,
            None,
            negated,
            value == "$true",
        )
    equality = find_top_level(value, ("!=", "="))
    if equality is not None:
        index, operator = equality
        left = parse_term(value[:index])
        right = parse_term(value[index + len(operator) :])
        validate_term(left, functions, predicates, variables)
        validate_term(right, functions, predicates, variables)
        if left.sort != right.sort or left.sort in {"$o", "$tType"}:
            raise UnsupportedInput(
                f"ill-sorted equality between {left.sort} and {right.sort}"
            )
        return Literal(
            None,
            (),
            left,
            right,
            negated ^ (operator == "!="),
        )

    atom = parse_term(value)
    if atom.name in {"$true", "$false"}:
        truth = atom.name == "$true"
        return Literal(None, (), None, None, negated, truth)
    if atom.variable or atom.sort != "$o":
        raise UnsupportedInput(f"non-Boolean clause atom {value!r}")
    for argument in atom.arguments:
        validate_term(argument, functions, predicates, variables)
    signature = SymbolType(tuple(arg.sort for arg in atom.arguments), "$o")
    register_symbol(predicates, functions, atom.name, signature, "predicate")
    return Literal(atom.name, atom.arguments, None, None, negated)


def parse_typed_cnf(text: str) -> TypedProblem:
    declarations: dict[str, SymbolType] = {}
    pending: list[tuple[str, str, str]] = []
    for statement in statements(text):
        match = re.match(r"(?is)^\s*(tff|tcf|cnf)\s*\((.*)\)\s*\.\s*$", statement)
        if match is None:
            raise UnsupportedInput(f"unsupported clausifier record {statement[:100]!r}")
        language, content = match.groups()
        fields = split_top_level(content)
        if len(fields) < 3:
            raise UnsupportedInput(f"malformed annotated record {statement[:100]!r}")
        name, role, body = fields[:3]
        role = role.strip().lower()
        if role == "type":
            if language != "tff":
                raise UnsupportedInput("non-TFF type declaration")
            symbol, symbol_type = parse_type_declaration(body)
            previous = declarations.get(symbol)
            if previous is not None and previous != symbol_type:
                raise UnsupportedInput(f"conflicting declarations for {symbol}")
            declarations[symbol] = symbol_type
        elif language in {"tcf", "cnf"}:
            pending.append((name.strip(), role, strip_outer_parentheses(body)))
        else:
            raise UnsupportedInput("non-clausal formula survived clausification")

    functions: dict[str, SymbolType] = {}
    predicates: dict[str, SymbolType] = {}
    for name, symbol_type in declarations.items():
        if symbol_type.result == "$o":
            register_symbol(predicates, functions, name, symbol_type, "predicate")
        elif symbol_type.result == "$tType":
            raise UnsupportedInput(f"type constructor declaration for {name} survived")
        else:
            register_symbol(functions, predicates, name, symbol_type, "function")

    clauses: list[Clause] = []
    has_conjecture = False
    for name, role, body in pending:
        variables: dict[str, str] = {}
        raw_literals = split_top_level(body, "|")
        parsed_literals = tuple(
            parse_literal(item, functions, predicates, variables)
            for item in raw_literals
        )
        if any(
            literal.truth is not None
            and (not literal.truth if literal.negated else literal.truth)
            for literal in parsed_literals
        ):
            continue
        literals = tuple(
            literal
            for literal in parsed_literals
            if literal.truth is None
            or (not literal.truth if literal.negated else literal.truth)
        )
        clauses.append(
            Clause(name, role, literals, tuple(sorted(variables.items())))
        )
        has_conjecture |= role in {"conjecture", "negated_conjecture"}

    if not clauses:
        raise UnsupportedInput("clausifier emitted no clauses")
    sorts = sorted(
        {
            sort
            for symbol_type in itertools.chain(functions.values(), predicates.values())
            for sort in (*symbol_type.arguments, symbol_type.result)
            if sort not in {"$o", "$tType"}
        }
    )
    if not sorts:
        # A purely propositional problem still uses one harmless individual sort.
        sorts = ["$i"]
    return TypedProblem(
        tuple(clauses), functions, predicates, tuple(sorts), has_conjecture
    )


class ClauseDatabase:
    def __init__(self, maximum_clauses: int) -> None:
        self.clauses: list[list[int]] = []
        self.maximum_clauses = maximum_clauses
        self.session: SatSession | None = None

    def attach(self, session: "SatSession") -> None:
        if self.session is not None:
            raise SolverFailure("a SAT session is already attached")
        self.session = session
        for clause in self.clauses:
            session.add_clause(clause)

    def add(self, clause: Iterable[int]) -> None:
        materialized = list(clause)
        if any(literal == 0 for literal in materialized):
            raise WorkerError("zero is not a propositional literal")
        if len(self.clauses) >= self.maximum_clauses:
            raise EncodingLimit(
                f"propositional clause limit {self.maximum_clauses} reached"
            )
        self.clauses.append(materialized)
        if self.session is not None:
            self.session.add_clause(materialized)


@dataclass(frozen=True)
class ValueVector:
    sort: str
    fixed: int | None
    variables: tuple[int, ...]

    def selections(self) -> Iterator[tuple[int, tuple[int, ...]]]:
        if self.fixed is not None:
            yield self.fixed, ()
        else:
            for value, variable in enumerate(self.variables):
                yield value, (variable,)


class Encoding:
    def __init__(
        self,
        problem: TypedProblem,
        maximum_size: int,
        maximum_clauses: int,
        maximum_ground_instances: int,
    ) -> None:
        self.problem = problem
        self.maximum_size = maximum_size
        self.maximum_ground_instances = maximum_ground_instances
        self.database = ClauseDatabase(maximum_clauses)
        self.next_variable = 1
        self.activity: dict[tuple[str, int], int] = {}
        self.function_tables: dict[tuple[str, tuple[int, ...], int], int] = {}
        self.predicate_tables: dict[tuple[str, tuple[int, ...]], int] = {}
        self.term_cache: dict[tuple[Any, ...], ValueVector] = {}
        self.atom_cache: dict[tuple[Any, ...], int | bool] = {}
        self.grounded: set[tuple[int, tuple[int, ...]]] = set()
        self._build_global_tables()

    @property
    def variable_count(self) -> int:
        return self.next_variable - 1

    def new_variable(self) -> int:
        variable = self.next_variable
        self.next_variable += 1
        return variable

    def exactly_one(self, variables: Sequence[int]) -> None:
        if not variables:
            self.database.add([])
            return
        self.database.add(variables)
        for index, left in enumerate(variables):
            for right in variables[index + 1 :]:
                self.database.add((-left, -right))

    def active(self, sort: str, value: int) -> int:
        return self.activity[(sort, value)]

    def _build_global_tables(self) -> None:
        for sort in self.problem.sorts:
            for value in range(self.maximum_size):
                self.activity[(sort, value)] = self.new_variable()
            self.database.add((self.active(sort, 0),))
            for value in range(1, self.maximum_size):
                self.database.add(
                    (-self.active(sort, value), self.active(sort, value - 1))
                )

        for name, signature in sorted(self.problem.functions.items()):
            argument_ranges = [
                range(self.maximum_size) for _ in signature.arguments
            ]
            for arguments in itertools.product(*argument_ranges):
                row = []
                for output in range(self.maximum_size):
                    variable = self.new_variable()
                    self.function_tables[(name, tuple(arguments), output)] = variable
                    row.append(variable)
                self.exactly_one(row)
                for output, variable in enumerate(row):
                    clause = [
                        -self.active(sort, value)
                        for sort, value in zip(signature.arguments, arguments)
                    ]
                    clause.extend(
                        (self.active(signature.result, output), -variable)
                    )
                    self.database.add(clause)

        for name, signature in sorted(self.problem.predicates.items()):
            argument_ranges = [
                range(self.maximum_size) for _ in signature.arguments
            ]
            for arguments in itertools.product(*argument_ranges):
                self.predicate_tables[(name, tuple(arguments))] = self.new_variable()

    def table_row(self, name: str, arguments: tuple[int, ...]) -> tuple[int, ...]:
        return tuple(
            self.function_tables[(name, arguments, output)]
            for output in range(self.maximum_size)
        )

    def ground_term_key(
        self, term: Term, assignment: dict[str, int]
    ) -> tuple[Any, ...]:
        if term.variable:
            return ("element", term.sort, assignment[term.name])
        return (
            "term",
            term.name,
            term.sort,
            tuple(self.ground_term_key(arg, assignment) for arg in term.arguments),
        )

    def term_values(self, term: Term, assignment: dict[str, int]) -> ValueVector:
        if term.variable:
            return ValueVector(term.sort, assignment[term.name], ())
        key = self.ground_term_key(term, assignment)
        cached = self.term_cache.get(key)
        if cached is not None:
            return cached
        arguments = tuple(self.term_values(arg, assignment) for arg in term.arguments)
        if not arguments or all(argument.fixed is not None for argument in arguments):
            indices = tuple(
                int(argument.fixed) for argument in arguments
            )
            result = ValueVector(
                term.sort, None, self.table_row(term.name, indices)
            )
            self.term_cache[key] = result
            return result

        outputs = tuple(self.new_variable() for _ in range(self.maximum_size))
        self.exactly_one(outputs)
        for selection in itertools.product(
            *(tuple(argument.selections()) for argument in arguments)
        ):
            indices = tuple(item[0] for item in selection)
            selectors = tuple(
                literal for item in selection for literal in item[1]
            )
            row = self.table_row(term.name, indices)
            for output, result_variable in enumerate(outputs):
                table_variable = row[output]
                self.database.add(
                    (
                        *(-selector for selector in selectors),
                        -table_variable,
                        result_variable,
                    )
                )
                self.database.add(
                    (
                        *(-selector for selector in selectors),
                        -result_variable,
                        table_variable,
                    )
                )
        result = ValueVector(term.sort, None, outputs)
        self.term_cache[key] = result
        return result

    @staticmethod
    def negated(value: int | bool) -> int | bool:
        return not value if isinstance(value, bool) else -value

    def predicate_truth(
        self, name: str, terms: tuple[Term, ...], assignment: dict[str, int]
    ) -> int | bool:
        key = (
            "predicate",
            name,
            tuple(self.ground_term_key(term, assignment) for term in terms),
        )
        cached = self.atom_cache.get(key)
        if cached is not None:
            return cached
        arguments = tuple(self.term_values(term, assignment) for term in terms)
        if not arguments or all(argument.fixed is not None for argument in arguments):
            indices = tuple(int(argument.fixed) for argument in arguments)
            result = self.predicate_tables[(name, indices)]
            self.atom_cache[key] = result
            return result
        truth = self.new_variable()
        for selection in itertools.product(
            *(tuple(argument.selections()) for argument in arguments)
        ):
            indices = tuple(item[0] for item in selection)
            selectors = tuple(
                literal for item in selection for literal in item[1]
            )
            table = self.predicate_tables[(name, indices)]
            self.database.add(
                (*(-selector for selector in selectors), -truth, table)
            )
            self.database.add(
                (*(-selector for selector in selectors), truth, -table)
            )
        self.atom_cache[key] = truth
        return truth

    def equality_truth(
        self, left: Term, right: Term, assignment: dict[str, int]
    ) -> int | bool:
        left_key = self.ground_term_key(left, assignment)
        right_key = self.ground_term_key(right, assignment)
        ordered = tuple(sorted((left_key, right_key), key=repr))
        key = ("equality", *ordered)
        cached = self.atom_cache.get(key)
        if cached is not None:
            return cached
        left_values = self.term_values(left, assignment)
        right_values = self.term_values(right, assignment)
        if left_values.fixed is not None and right_values.fixed is not None:
            result = left_values.fixed == right_values.fixed
            self.atom_cache[key] = result
            return result
        truth = self.new_variable()
        for left_value, left_selectors in left_values.selections():
            for right_value, right_selectors in right_values.selections():
                selectors = (*left_selectors, *right_selectors)
                consequence = truth if left_value == right_value else -truth
                self.database.add(
                    (*(-selector for selector in selectors), consequence)
                )
        self.atom_cache[key] = truth
        return truth

    def literal_truth(
        self, literal: Literal, assignment: dict[str, int]
    ) -> int | bool:
        if literal.truth is not None:
            value: int | bool = literal.truth
        elif literal.predicate is not None:
            value = self.predicate_truth(
                literal.predicate, literal.arguments, assignment
            )
        else:
            assert literal.left is not None and literal.right is not None
            value = self.equality_truth(literal.left, literal.right, assignment)
        return self.negated(value) if literal.negated else value

    def ground_clause(
        self,
        clause_index: int,
        clause: Clause,
        variable_values: tuple[int, ...],
    ) -> None:
        key = (clause_index, variable_values)
        if key in self.grounded:
            return
        if len(self.grounded) >= self.maximum_ground_instances:
            raise EncodingLimit(
                f"ground-instance limit {self.maximum_ground_instances} reached"
            )
        assignment = {
            name: value
            for (name, _), value in zip(clause.variables, variable_values)
        }
        guard = [
            -self.active(sort, assignment[name])
            for name, sort in clause.variables
        ]
        propositional: list[int] = []
        for literal in clause.literals:
            value = self.literal_truth(literal, assignment)
            if value is True:
                self.grounded.add(key)
                return
            if value is not False:
                propositional.append(value)
        self.database.add((*guard, *propositional))
        self.grounded.add(key)

    def extend_grounding(self, sizes: dict[str, int]) -> int:
        before = len(self.grounded)
        for clause_index, clause in enumerate(self.problem.clauses):
            ranges = [range(sizes[sort]) for _, sort in clause.variables]
            for values in itertools.product(*ranges):
                self.ground_clause(clause_index, clause, tuple(values))
        return len(self.grounded) - before

    def assumptions(self, sizes: dict[str, int]) -> list[int]:
        return [
            self.active(sort, value)
            if value < sizes[sort]
            else -self.active(sort, value)
            for sort in self.problem.sorts
            for value in range(self.maximum_size)
        ]


class SatSession:
    def __init__(self, executable: Path) -> None:
        try:
            self.process = subprocess.Popen(
                [str(executable)],
                text=True,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                bufsize=1,
            )
        except OSError as error:
            raise SolverFailure(f"could not start SAT probe: {error}") from error
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        assert self.process.stderr is not None
        self.responses: queue.Queue[str | None] = queue.Queue()
        self.stderr_lines: list[str] = []
        threading.Thread(
            target=self._read_stdout, name="fnt-sat-stdout", daemon=True
        ).start()
        threading.Thread(
            target=self._read_stderr, name="fnt-sat-stderr", daemon=True
        ).start()

    def _read_stdout(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            self.responses.put(line)
        self.responses.put(None)

    def _read_stderr(self) -> None:
        assert self.process.stderr is not None
        for line in self.process.stderr:
            if len(self.stderr_lines) < 200:
                self.stderr_lines.append(line.rstrip())

    def add_clause(self, clause: Sequence[int]) -> None:
        if self.process.poll() is not None:
            raise SolverFailure(self.diagnostic("SAT probe exited while adding clauses"))
        assert self.process.stdin is not None
        self.process.stdin.write(
            "a " + " ".join(map(str, clause)) + (" " if clause else "") + "0\n"
        )

    def query(
        self, query_id: str, assumptions: Sequence[int], timeout_seconds: float
    ) -> dict[str, Any]:
        if self.process.poll() is not None:
            raise SolverFailure(self.diagnostic("SAT probe exited before query"))
        assert self.process.stdin is not None
        deadline_microseconds = max(1, round(timeout_seconds * 1_000_000))
        self.process.stdin.write(
            "q "
            + query_id
            + " "
            + str(deadline_microseconds)
            + " "
            + " ".join(map(str, assumptions))
            + (" " if assumptions else "")
            + "0\n"
        )
        self.process.stdin.flush()
        try:
            line = self.responses.get(timeout=timeout_seconds + 2.0)
        except queue.Empty as error:
            self.process.kill()
            raise SolverFailure("SAT probe did not answer before controller deadline") from error
        if line is None:
            raise SolverFailure(self.diagnostic("SAT probe closed its output"))
        try:
            response = json.loads(line)
        except json.JSONDecodeError as error:
            raise SolverFailure(f"invalid SAT probe JSON: {line[:500]!r}") from error
        if response.get("query") != query_id:
            raise SolverFailure(
                f"SAT probe answered query {response.get('query')!r}, expected {query_id!r}"
            )
        return response

    def diagnostic(self, message: str) -> str:
        detail = "\n".join(self.stderr_lines[-20:])
        return message + (f": {detail}" if detail else "")

    def close(self) -> None:
        if self.process.poll() is None:
            try:
                assert self.process.stdin is not None
                self.process.stdin.write("x\n")
                self.process.stdin.flush()
                self.process.wait(timeout=2)
            except (OSError, subprocess.SubprocessError):
                self.process.kill()
                self.process.wait()

    def __enter__(self) -> "SatSession":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def domain_size_vectors(sorts: Sequence[str], maximum: int) -> Iterator[dict[str, int]]:
    for total in range(len(sorts), len(sorts) * maximum + 1):
        for values in itertools.product(range(1, maximum + 1), repeat=len(sorts)):
            if sum(values) == total:
                yield dict(zip(sorts, values))


def positive_assignment(response: dict[str, Any], maximum: int) -> frozenset[int]:
    raw = response.get("model")
    if not isinstance(raw, list) or len(raw) < maximum:
        raise SolverFailure(
            f"SAT probe returned an incomplete model ({len(raw) if isinstance(raw, list) else 'not a list'} < {maximum})"
        )
    values = [int(literal) for literal in raw]
    if any(abs(literal) != index for index, literal in enumerate(values, 1)):
        raise SolverFailure("SAT probe model is not a complete ordered assignment")
    return frozenset(literal for literal in values if literal > 0)


def selected_row(
    variables: Iterable[tuple[int, int]], assignment: frozenset[int], label: str
) -> int:
    selected = [value for value, variable in variables if variable in assignment]
    if len(selected) != 1:
        raise SolverFailure(f"{label} has {len(selected)} selected values")
    return selected[0]


def decode_functions(
    encoding: Encoding,
    sizes: dict[str, int],
    assignment: frozenset[int],
) -> dict[tuple[str, tuple[int, ...]], int]:
    result: dict[tuple[str, tuple[int, ...]], int] = {}
    for name, signature in sorted(encoding.problem.functions.items()):
        ranges = [range(sizes[sort]) for sort in signature.arguments]
        for arguments in itertools.product(*ranges):
            output = selected_row(
                (
                    (value, encoding.function_tables[(name, tuple(arguments), value)])
                    for value in range(encoding.maximum_size)
                ),
                assignment,
                f"{name}{arguments}",
            )
            if output >= sizes[signature.result]:
                raise SolverFailure(f"{name}{arguments} maps outside its active sort")
            result[(name, tuple(arguments))] = output
    return result


def decode_predicates(
    encoding: Encoding,
    sizes: dict[str, int],
    assignment: frozenset[int],
) -> dict[tuple[str, tuple[int, ...]], bool]:
    result: dict[tuple[str, tuple[int, ...]], bool] = {}
    for name, signature in sorted(encoding.problem.predicates.items()):
        ranges = [range(sizes[sort]) for sort in signature.arguments]
        for arguments in itertools.product(*ranges):
            variable = encoding.predicate_tables[(name, tuple(arguments))]
            result[(name, tuple(arguments))] = variable in assignment
    return result


def evaluate_term(
    term: Term,
    variables: dict[str, int],
    functions: dict[tuple[str, tuple[int, ...]], int],
) -> int:
    if term.variable:
        return variables[term.name]
    arguments = tuple(evaluate_term(arg, variables, functions) for arg in term.arguments)
    return functions[(term.name, arguments)]


def evaluate_literal(
    literal: Literal,
    variables: dict[str, int],
    functions: dict[tuple[str, tuple[int, ...]], int],
    predicates: dict[tuple[str, tuple[int, ...]], bool],
) -> bool:
    if literal.truth is not None:
        value = literal.truth
    elif literal.predicate is not None:
        arguments = tuple(
            evaluate_term(term, variables, functions) for term in literal.arguments
        )
        value = predicates[(literal.predicate, arguments)]
    else:
        assert literal.left is not None and literal.right is not None
        value = evaluate_term(literal.left, variables, functions) == evaluate_term(
            literal.right, variables, functions
        )
    return not value if literal.negated else value


def validate_interpretation(
    problem: TypedProblem,
    sizes: dict[str, int],
    functions: dict[tuple[str, tuple[int, ...]], int],
    predicates: dict[tuple[str, tuple[int, ...]], bool],
) -> None:
    for clause in problem.clauses:
        ranges = [range(sizes[sort]) for _, sort in clause.variables]
        for values in itertools.product(*ranges):
            variables = {
                name: value
                for (name, _), value in zip(clause.variables, values)
            }
            if not any(
                evaluate_literal(literal, variables, functions, predicates)
                for literal in clause.literals
            ):
                raise SolverFailure(
                    f"decoded SAT model falsifies {clause.name} at {variables}"
                )


def safe_fragment_name(name: str) -> str:
    value = re.sub(r"[^A-Za-z0-9_]", "_", name)
    return value if value and value[0].isalpha() else f"s_{value}"


def render_model(
    problem_name: str,
    problem: TypedProblem,
    sizes: dict[str, int],
    functions: dict[tuple[str, tuple[int, ...]], int],
    predicates: dict[tuple[str, tuple[int, ...]], bool],
) -> str:
    occupied = set(problem.functions) | set(problem.predicates)
    prefix = "umlaut_fmb_d_"
    while any(name.startswith(prefix) for name in occupied):
        prefix += "x_"

    def element(sort: str, value: int) -> str:
        return f"{prefix}{safe_fragment_name(sort)}_{value}"

    status = "CounterSatisfiable" if problem.has_conjecture else "Satisfiable"
    lines = [
        f"% SZS status {status} for {problem_name}",
        f"% SZS output start FiniteModel for {problem_name}",
    ]
    serial = 0

    def formula(body: str, name: str | None = None) -> None:
        nonlocal serial
        formula_name = name if name is not None else f"umlaut_fmb_{serial}"
        lines.append(f"tff({formula_name},axiom,{body}).")
        serial += 1

    for sort in problem.sorts:
        for value in range(sizes[sort]):
            lines.append(
                f"tff(umlaut_fmb_type_{serial},type,"
                f"{element(sort, value)}:{sort})."
            )
            serial += 1
        domain = " | ".join(
            f"X = {element(sort, value)}" for value in range(sizes[sort])
        )
        label = safe_fragment_name(sort)
        formula(f"! [X:{sort}] : ({domain})", f"finite_domain_{label}")
        inequalities = [
            f"{element(sort, left)} != {element(sort, right)}"
            for left in range(sizes[sort])
            for right in range(left + 1, sizes[sort])
        ]
        if inequalities:
            formula(
                " & ".join(inequalities),
                f"distinct_domain_{label}",
            )

    for (name, arguments), output in sorted(functions.items(), key=repr):
        signature = problem.functions[name]
        left = name
        if arguments:
            left += "(" + ",".join(
                element(sort, value)
                for sort, value in zip(signature.arguments, arguments)
            ) + ")"
        formula(f"{left} = {element(signature.result, output)}")

    for (name, arguments), truth in sorted(predicates.items(), key=repr):
        signature = problem.predicates[name]
        atom = name
        if arguments:
            atom += "(" + ",".join(
                element(sort, value)
                for sort, value in zip(signature.arguments, arguments)
            ) + ")"
        formula(atom if truth else f"~({atom})")

    lines.append(f"% SZS output end FiniteModel for {problem_name}")
    return "\n".join(lines) + "\n"


def run_clausifier(
    executable: Path, problem: Path, timeout_seconds: float
) -> tuple[str, float]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            [
                str(executable),
                "--cnf",
                "--no-preprocessing",
                "--tstp-format",
                "--print-types",
                str(problem),
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout_seconds,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise WorkerError(f"Umlaut clausification failed: {error}") from error
    seconds = time.monotonic() - started
    if completed.returncode != 0:
        raise WorkerError(
            f"Umlaut clausification exited {completed.returncode}: "
            f"{completed.stderr[-2000:]}"
        )
    return completed.stdout, seconds


def query_fresh(
    executable: Path,
    clauses: Sequence[Sequence[int]],
    query_id: str,
    assumptions: Sequence[int],
    timeout_seconds: float,
) -> dict[str, Any]:
    with SatSession(executable) as session:
        for clause in clauses:
            session.add_clause(clause)
        return session.query(query_id, assumptions, timeout_seconds)


def write_report(path: Path | None, report: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def no_claim(problem_name: str, status: str, reason: str) -> str:
    return f"% SZS status {status} for {problem_name}\n% {reason}\n"


def run(args: argparse.Namespace) -> tuple[int, str, dict[str, Any]]:
    problem_name = args.problem.stem
    report: dict[str, Any] = {
        "schema_version": 2,
        "problem": str(args.problem),
        "problem_name": problem_name,
        "solver_mode": args.solver_mode,
        "max_size": args.max_size,
        "max_size_vectors": args.max_size_vectors,
        "max_ground_instances": args.max_ground_instances,
        "max_propositional_clauses": args.max_propositional_clauses,
        "bounds": [],
    }
    try:
        clausified, clausification_seconds = run_clausifier(
            args.umlaut, args.problem, args.clausify_timeout_seconds
        )
        report["clausification_seconds"] = clausification_seconds
        problem = parse_typed_cnf(clausified)
        report.update(
            {
                "sorts": list(problem.sorts),
                "clauses": len(problem.clauses),
                "functions": {
                    name: asdict(signature)
                    for name, signature in sorted(problem.functions.items())
                },
                "predicates": {
                    name: asdict(signature)
                    for name, signature in sorted(problem.predicates.items())
                },
            }
        )
        if args.analyze_only:
            report["outcome"] = "supported"
            return 0, no_claim(problem_name, "Unknown", "supported typed fragment"), report

        encoding = Encoding(
            problem,
            args.max_size,
            args.max_propositional_clauses,
            args.max_ground_instances,
        )
        session: SatSession | None = None
        if args.solver_mode == "incremental":
            session = SatSession(args.sat_probe)
            encoding.database.attach(session)
        try:
            for bound_index, sizes in enumerate(
                domain_size_vectors(problem.sorts, args.max_size)
            ):
                if bound_index >= args.max_size_vectors:
                    report["outcome"] = "resource_out"
                    report["reason"] = "size-vector limit reached"
                    return (
                        2,
                        no_claim(problem_name, "ResourceOut", report["reason"]),
                        report,
                    )
                clauses_before = len(encoding.database.clauses)
                grounding_started = time.monotonic()
                new_instances = encoding.extend_grounding(sizes)
                grounding_seconds = time.monotonic() - grounding_started
                assumptions = encoding.assumptions(sizes)
                if session is not None:
                    response = session.query(
                        str(bound_index), assumptions, args.sat_timeout_seconds
                    )
                else:
                    response = query_fresh(
                        args.sat_probe,
                        encoding.database.clauses,
                        str(bound_index),
                        assumptions,
                        args.sat_timeout_seconds,
                    )
                status = str(response.get("status"))
                bound = BoundReport(
                    sizes=dict(sizes),
                    new_ground_instances=new_instances,
                    cumulative_ground_instances=len(encoding.grounded),
                    new_clauses=len(encoding.database.clauses) - clauses_before,
                    cumulative_clauses=len(encoding.database.clauses),
                    propositional_variables=encoding.variable_count,
                    grounding_seconds=grounding_seconds,
                    insertion_seconds=float(response.get("insertion_ns", 0))
                    / 1_000_000_000,
                    sat_seconds=float(response.get("elapsed_ns", 0)) / 1_000_000_000,
                    sat_status=status,
                    model_variables=int(response.get("model_len", 0)),
                )
                report["bounds"].append(asdict(bound))
                write_report(args.report, report)
                if status == "sat":
                    assignment = positive_assignment(response, encoding.variable_count)
                    functions = decode_functions(encoding, sizes, assignment)
                    predicates = decode_predicates(encoding, sizes, assignment)
                    validate_interpretation(problem, sizes, functions, predicates)
                    model = render_model(
                        problem_name, problem, sizes, functions, predicates
                    )
                    report["outcome"] = "model"
                    report["successful_sizes"] = dict(sizes)
                    report["model_bytes"] = len(model.encode())
                    return 0, model, report
                if status == "unknown":
                    report["outcome"] = "resource_out"
                    report["reason"] = str(response.get("reason", "SAT timeout"))
                    return (
                        2,
                        no_claim(problem_name, "ResourceOut", report["reason"]),
                        report,
                    )
                if status != "unsat":
                    raise SolverFailure(f"SAT probe returned status {status!r}")
        finally:
            if session is not None:
                session.close()

        report["outcome"] = "bounds_exhausted"
        report["reason"] = "no model exists within configured finite bounds"
        return 2, no_claim(problem_name, "GaveUp", report["reason"]), report
    except UnsupportedInput as error:
        report["outcome"] = "unsupported"
        report["reason"] = str(error)
        return 2, no_claim(problem_name, "Inappropriate", str(error)), report
    except EncodingLimit as error:
        report["outcome"] = "resource_out"
        report["reason"] = str(error)
        return 2, no_claim(problem_name, "ResourceOut", str(error)), report
    except (WorkerError, AssertionError, KeyError, ValueError) as error:
        report["outcome"] = "error"
        report["reason"] = str(error)
        return 1, no_claim(problem_name, "Error", str(error)), report


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("problem", type=Path)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--sat-probe", type=Path, required=True)
    parser.add_argument(
        "--solver-mode", choices=("incremental", "fresh"), default="incremental"
    )
    parser.add_argument("--max-size", type=int, default=3)
    parser.add_argument("--max-size-vectors", type=int, default=2048)
    parser.add_argument("--max-ground-instances", type=int, default=5_000_000)
    parser.add_argument("--max-propositional-clauses", type=int, default=10_000_000)
    parser.add_argument("--sat-timeout-seconds", type=float, default=5.0)
    parser.add_argument("--clausify-timeout-seconds", type=float, default=10.0)
    parser.add_argument("--analyze-only", action="store_true")
    parser.add_argument("--report", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.max_size < 1:
        raise SystemExit("--max-size must be positive")
    if args.max_size_vectors < 1:
        raise SystemExit("--max-size-vectors must be positive")
    code, output, report = run(args)
    write_report(args.report, report)
    print(output, end="")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
