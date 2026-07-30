#!/usr/bin/env python3
"""Parse Umlaut CNF output and build deterministic ground-theory traces."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import re
from fractions import Fraction
from typing import Any, Iterable, Sequence


class TraceError(ValueError):
    """The CNF transcript violates the frozen trace contract."""


ARITHMETIC_SORTS = {"$int": "Int", "$real": "Real"}
RELATIONS = {
    "$less": "lt",
    "$lesseq": "le",
    "$greater": "gt",
    "$greatereq": "ge",
}
NUMBER_RE = re.compile(
    r"^[+-]?(?:\d+/\d+|(?:\d+(?:\.\d*)?|\.\d+)(?:[Ee][+-]?\d+)?)$"
)
TOKEN_RE = re.compile(
    r"""
    \s*
    (
        '(?:''|\\.|[^'])*'
      | "(?:""|\\.|[^"])*"
      | [+-]?\d+/\d+
      | [+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[Ee][+-]?\d+)?
      | \$?[A-Za-z_][A-Za-z0-9_]*
      | !=|<=>|<~>|=>|<=
      | [(),:\[\]|~=&>@*+/\-]
    )
    """,
    re.VERBOSE,
)


@dataclasses.dataclass(frozen=True)
class Term:
    symbol: str
    arguments: tuple["Term", ...] = ()

    def canonical(self) -> str:
        if not self.arguments:
            return self.symbol
        return (
            f"{self.symbol}("
            + ",".join(argument.canonical() for argument in self.arguments)
            + ")"
        )


@dataclasses.dataclass(frozen=True)
class Atom:
    relation: str
    arguments: tuple[Term, ...]

    def canonical(self) -> str:
        if self.relation == "eq":
            return f"eq({self.arguments[0].canonical()},{self.arguments[1].canonical()})"
        return (
            f"{self.relation}("
            + ",".join(argument.canonical() for argument in self.arguments)
            + ")"
        )


@dataclasses.dataclass(frozen=True)
class Literal:
    atom: Atom
    positive: bool


@dataclasses.dataclass
class ParsedClause:
    name: str
    role: str
    statement: str
    statement_sha256: str
    variable_sorts: dict[str, str]
    grounding: dict[str, str]
    literals: list[Literal]


@dataclasses.dataclass
class ParsedTranscript:
    declarations: dict[str, str]
    clauses: list[ParsedClause]
    transcript_sha256: str


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def strip_comments(text: str) -> str:
    output: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(text):
        character = text[index]
        if quote is not None:
            output.append(character)
            if character == quote:
                if index + 1 < len(text) and text[index + 1] == quote:
                    output.append(text[index + 1])
                    index += 2
                    continue
                quote = None
            elif character == "\\" and index + 1 < len(text):
                output.append(text[index + 1])
                index += 2
                continue
            index += 1
            continue
        if character in ("'", '"'):
            quote = character
            output.append(character)
            index += 1
            continue
        if character == "%":
            while index < len(text) and text[index] not in "\r\n":
                output.append(" ")
                index += 1
            continue
        if text.startswith("/*", index):
            output.extend((" ", " "))
            index += 2
            while index < len(text) and not text.startswith("*/", index):
                output.append("\n" if text[index] == "\n" else " ")
                index += 1
            if index >= len(text):
                raise TraceError("unterminated block comment")
            output.extend((" ", " "))
            index += 2
            continue
        output.append(character)
        index += 1
    if quote is not None:
        raise TraceError("unterminated quoted token")
    return "".join(output)


def split_statements(text: str) -> list[str]:
    clean = strip_comments(text)
    statements: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(clean):
        character = clean[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(clean) and clean[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in ("'", '"'):
            quote = character
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth < 0:
                raise TraceError("unbalanced closing parenthesis")
        elif character == "." and depth == 0:
            statement = clean[start : index + 1].strip()
            if statement:
                statements.append(statement)
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise TraceError("unbalanced statement")
    if clean[start:].strip():
        raise TraceError("trailing text without statement terminator")
    return statements


def split_top_level(value: str, separator: str) -> list[str]:
    pieces: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(value):
        character = value[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(value) and value[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in ("'", '"'):
            quote = character
        elif character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
            if depth < 0:
                raise TraceError("unbalanced nested expression")
        elif character == separator and depth == 0:
            pieces.append(value[start:index].strip())
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise TraceError("unbalanced nested expression")
    pieces.append(value[start:].strip())
    return pieces


def wrapping_parentheses(value: str) -> bool:
    value = value.strip()
    if not (value.startswith("(") and value.endswith(")")):
        return False
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(value):
        character = value[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(value) and value[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in ("'", '"'):
            quote = character
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0 and index != len(value) - 1:
                return False
        index += 1
    return depth == 0 and quote is None


def strip_wrapping_parentheses(value: str) -> str:
    value = value.strip()
    while wrapping_parentheses(value):
        value = value[1:-1].strip()
    return value


def statement_fields(statement: str) -> tuple[str, list[str]]:
    prefix, separator, rest = statement.partition("(")
    if not separator or not rest.rstrip().endswith(")."):
        raise TraceError("malformed annotated statement")
    body = rest.rstrip()[:-2]
    return prefix.strip().lower(), split_top_level(body, ",")


def tokenize(value: str) -> list[str]:
    tokens: list[str] = []
    position = 0
    while position < len(value):
        match = TOKEN_RE.match(value, position)
        if match is None:
            if value[position:].strip():
                raise TraceError(
                    f"unsupported token near {value[position:position + 24]!r}"
                )
            break
        tokens.append(match.group(1))
        position = match.end()
    return tokens


class TermParser:
    def __init__(self, value: str) -> None:
        self.tokens = tokenize(value)
        self.position = 0

    def current(self) -> str | None:
        if self.position == len(self.tokens):
            return None
        return self.tokens[self.position]

    def consume(self, expected: str | None = None) -> str:
        token = self.current()
        if token is None:
            raise TraceError("unexpected end of term")
        if expected is not None and token != expected:
            raise TraceError(f"expected {expected!r}, found {token!r}")
        self.position += 1
        return token

    def parse_term(self) -> Term:
        symbol = self.consume()
        if symbol in {"(", ")", ",", ":", "[", "]", "|", "~", "=", "!="}:
            raise TraceError(f"invalid term head {symbol!r}")
        arguments: list[Term] = []
        if self.current() == "(":
            self.consume("(")
            if self.current() != ")":
                while True:
                    arguments.append(self.parse_term())
                    if self.current() != ",":
                        break
                    self.consume(",")
            self.consume(")")
        return Term(symbol, tuple(arguments))

    def parse(self) -> Term:
        result = self.parse_term()
        if self.current() is not None:
            raise TraceError(f"trailing term token {self.current()!r}")
        return result


def parse_term(value: str) -> Term:
    return TermParser(value).parse()


def find_top_level_operator(value: str, operators: Sequence[str]) -> tuple[int, str] | None:
    depth = 0
    quote: str | None = None
    index = 0
    ordered = sorted(operators, key=len, reverse=True)
    while index < len(value):
        character = value[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(value) and value[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in ("'", '"'):
            quote = character
            index += 1
            continue
        if character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
        elif depth == 0:
            for operator in ordered:
                if value.startswith(operator, index):
                    return index, operator
        index += 1
    return None


def parse_literal(value: str) -> Literal:
    value = strip_wrapping_parentheses(value)
    positive = True
    while value.startswith("~"):
        positive = not positive
        value = strip_wrapping_parentheses(value[1:])

    equality = find_top_level_operator(value, ("!=", "="))
    if equality is not None:
        index, operator = equality
        left = parse_term(value[:index].strip())
        right = parse_term(value[index + len(operator) :].strip())
        arguments = tuple(sorted((left, right), key=Term.canonical))
        return Literal(Atom("eq", arguments), positive == (operator == "="))

    predicate = parse_term(value)
    relation = RELATIONS.get(predicate.symbol, predicate.symbol)
    if relation in {"lt", "le", "gt", "ge"} and len(predicate.arguments) != 2:
        raise TraceError(f"arithmetic relation {predicate.symbol} is not binary")
    return Literal(Atom(relation, predicate.arguments), positive)


def parse_quantified_clause(
    value: str,
) -> tuple[dict[str, str], list[str]]:
    value = strip_wrapping_parentheses(value)
    variable_sorts: dict[str, str] = {}
    if value.startswith("!"):
        remainder = value[1:].lstrip()
        if not remainder.startswith("["):
            raise TraceError("universal quantifier lacks a binder")
        depth = 0
        end = None
        for index, character in enumerate(remainder):
            if character == "[":
                depth += 1
            elif character == "]":
                depth -= 1
                if depth == 0:
                    end = index
                    break
        if end is None:
            raise TraceError("unterminated universal binder")
        binder = remainder[1:end]
        after = remainder[end + 1 :].lstrip()
        if not after.startswith(":"):
            raise TraceError("universal binder lacks a body separator")
        for declaration in split_top_level(binder, ","):
            name, separator, sort = declaration.partition(":")
            if not separator or not name.strip() or not sort.strip():
                raise TraceError("malformed quantified variable declaration")
            variable_sorts[name.strip()] = sort.strip()
        value = strip_wrapping_parentheses(after[1:])
    literals = split_top_level(value, "|")
    if any(not literal for literal in literals):
        raise TraceError("empty literal in clause")
    return variable_sorts, literals


def declaration_result_sort(value: str) -> tuple[str, str] | None:
    split = find_top_level_operator(value, (":",))
    if split is None:
        return None
    index, _ = split
    symbol = value[:index].strip()
    type_expression = strip_wrapping_parentheses(value[index + 1 :])
    matches = list(re.finditer(r"\$(?:int|real)\b", type_expression))
    if not matches:
        return None
    last = matches[-1]
    if type_expression[last.end() :].strip().strip(")") != "":
        return None
    return symbol, last.group()


def ground_term(term: Term, grounding: dict[str, str]) -> Term:
    if not term.arguments and term.symbol in grounding:
        return Term(grounding[term.symbol])
    return Term(
        term.symbol,
        tuple(ground_term(argument, grounding) for argument in term.arguments),
    )


def ground_atom(atom: Atom, grounding: dict[str, str]) -> Atom:
    arguments = tuple(ground_term(argument, grounding) for argument in atom.arguments)
    if atom.relation == "eq":
        arguments = tuple(sorted(arguments, key=Term.canonical))
    return Atom(atom.relation, arguments)


def grounding_for_variables(
    variable_sorts: dict[str, str],
) -> dict[str, str]:
    ordinals: dict[str, int] = {}
    result: dict[str, str] = {}
    for variable, sort in variable_sorts.items():
        ordinal = ordinals.get(sort, 0)
        ordinals[sort] = ordinal + 1
        sort_name = ARITHMETIC_SORTS.get(sort)
        if sort_name is None:
            sort_name = "u_" + sha256_text(sort)[:8]
        result[variable] = f"ground_{sort_name.lower()}_{ordinal}"
    return result


def parse_transcript(text: str) -> ParsedTranscript:
    declarations: dict[str, str] = {}
    raw_clauses: list[
        tuple[str, str, str, dict[str, str], dict[str, str], list[Literal]]
    ] = []
    for statement in split_statements(text):
        prefix, fields = statement_fields(statement)
        if prefix == "tff" and len(fields) >= 3 and fields[1].strip() == "type":
            declaration = declaration_result_sort(fields[2])
            if declaration is not None:
                declarations[declaration[0]] = declaration[1]
            continue
        if prefix not in {"cnf", "tcf"}:
            continue
        if len(fields) < 3:
            raise TraceError("clause statement has fewer than three fields")
        variable_sorts, literal_texts = parse_quantified_clause(fields[2])
        grounding = grounding_for_variables(variable_sorts)
        literals = [
            Literal(ground_atom(parsed.atom, grounding), parsed.positive)
            for parsed in map(parse_literal, literal_texts)
        ]
        raw_clauses.append(
            (
                fields[0].strip(),
                fields[1].strip(),
                statement,
                variable_sorts,
                grounding,
                literals,
            )
        )

    for _, _, _, variable_sorts, grounding, _ in raw_clauses:
        for variable, ground_symbol in grounding.items():
            declarations[ground_symbol] = variable_sorts[variable]

    clauses = [
        ParsedClause(
            name=name,
            role=role,
            statement=statement,
            statement_sha256=sha256_text(statement),
            variable_sorts=variable_sorts,
            grounding=grounding,
            literals=literals,
        )
        for name, role, statement, variable_sorts, grounding, literals in raw_clauses
    ]
    return ParsedTranscript(
        declarations=declarations,
        clauses=clauses,
        transcript_sha256=sha256_text(text),
    )


def parse_number(symbol: str) -> Fraction | None:
    if not NUMBER_RE.fullmatch(symbol):
        return None
    if "/" in symbol:
        numerator, denominator = symbol.split("/", 1)
        if int(denominator) == 0:
            raise TraceError("zero rational denominator")
        return Fraction(int(numerator), int(denominator))
    return Fraction(symbol)


def infer_sort(term: Term, declarations: dict[str, str]) -> str | None:
    number = parse_number(term.symbol) if not term.arguments else None
    if number is not None:
        return "$real" if any(marker in term.symbol for marker in ".Ee") else "$int"
    if term.symbol == "$to_real" and len(term.arguments) == 1:
        return "$real"
    if term.symbol in {"$to_int", "$floor", "$ceiling", "$truncate", "$round"}:
        return "$int"
    if term.symbol in {
        "$sum",
        "$difference",
        "$uminus",
        "$product",
        "$quotient",
        "$quotient_e",
        "$quotient_t",
        "$quotient_f",
        "$remainder_e",
        "$remainder_t",
        "$remainder_f",
    }:
        sorts = {
            sort
            for argument in term.arguments
            if (sort := infer_sort(argument, declarations)) is not None
        }
        if len(sorts) == 1:
            return next(iter(sorts))
        return None
    return declarations.get(term.symbol)


@dataclasses.dataclass
class LinearForm:
    coefficients: dict[str, Fraction]
    constant: Fraction
    opaque_terms: dict[str, str]

    @classmethod
    def constant_form(cls, value: Fraction) -> "LinearForm":
        return cls({}, value, {})

    def scaled(self, coefficient: Fraction) -> "LinearForm":
        return LinearForm(
            {
                variable: value * coefficient
                for variable, value in self.coefficients.items()
                if value * coefficient
            },
            self.constant * coefficient,
            dict(self.opaque_terms),
        )

    def added(self, other: "LinearForm") -> "LinearForm":
        coefficients = dict(self.coefficients)
        for variable, value in other.coefficients.items():
            coefficients[variable] = coefficients.get(variable, Fraction(0)) + value
            if coefficients[variable] == 0:
                del coefficients[variable]
        opaque_terms = dict(self.opaque_terms)
        opaque_terms.update(other.opaque_terms)
        return LinearForm(
            coefficients,
            self.constant + other.constant,
            opaque_terms,
        )


def opaque_variable(term: Term, sort: str) -> tuple[str, str]:
    canonical = term.canonical()
    digest = sha256_text(f"{sort}\0{canonical}")[:16]
    return f"v_{digest}", canonical


def linearize(term: Term, sort: str, declarations: dict[str, str]) -> LinearForm:
    number = parse_number(term.symbol) if not term.arguments else None
    if number is not None:
        return LinearForm.constant_form(number)
    if term.symbol == "$to_real" and len(term.arguments) == 1:
        argument_number = (
            parse_number(term.arguments[0].symbol)
            if not term.arguments[0].arguments
            else None
        )
        if argument_number is not None:
            return LinearForm.constant_form(argument_number)
        variable, canonical = opaque_variable(term, sort)
        return LinearForm({variable: Fraction(1)}, Fraction(0), {variable: canonical})
    if term.symbol == "$sum" and len(term.arguments) == 2:
        return linearize(term.arguments[0], sort, declarations).added(
            linearize(term.arguments[1], sort, declarations)
        )
    if term.symbol == "$difference" and len(term.arguments) == 2:
        return linearize(term.arguments[0], sort, declarations).added(
            linearize(term.arguments[1], sort, declarations).scaled(Fraction(-1))
        )
    if term.symbol == "$uminus" and len(term.arguments) == 1:
        return linearize(term.arguments[0], sort, declarations).scaled(Fraction(-1))
    if term.symbol == "$product" and len(term.arguments) == 2:
        left = linearize(term.arguments[0], sort, declarations)
        right = linearize(term.arguments[1], sort, declarations)
        if not left.coefficients:
            return right.scaled(left.constant)
        if not right.coefficients:
            return left.scaled(right.constant)
        raise TraceError("NONLINEAR_PRODUCT")
    if term.symbol == "$quotient" and len(term.arguments) == 2:
        numerator = linearize(term.arguments[0], sort, declarations)
        denominator = linearize(term.arguments[1], sort, declarations)
        if denominator.coefficients:
            raise TraceError("NONCONSTANT_QUOTIENT")
        if denominator.constant == 0:
            raise TraceError("ZERO_QUOTIENT")
        return numerator.scaled(Fraction(1) / denominator.constant)
    variable, canonical = opaque_variable(term, sort)
    return LinearForm({variable: Fraction(1)}, Fraction(0), {variable: canonical})


def fraction_text(value: Fraction) -> str:
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def difference_constraint(
    form: LinearForm,
    upper_bound: Fraction,
) -> tuple[dict[str, Any] | None, str | None]:
    coefficients = form.coefficients
    if any(value not in {Fraction(-1), Fraction(1)} for value in coefficients.values()):
        return None, "GENERAL_LINEAR_COEFFICIENT"
    positive = sorted(
        variable for variable, value in coefficients.items() if value == 1
    )
    negative = sorted(
        variable for variable, value in coefficients.items() if value == -1
    )
    if len(positive) > 1 or len(negative) > 1:
        return None, "GENERAL_LINEAR_ARITY"
    return (
        {
            "kind": "difference",
            "lhs": positive[0] if positive else "zero",
            "rhs": negative[0] if negative else "zero",
            "bound": fraction_text(upper_bound - form.constant),
            "opaque_terms": dict(sorted(form.opaque_terms.items())),
        },
        None,
    )


def relation_constraints(
    atom: Atom,
    positive: bool,
    declarations: dict[str, str],
) -> tuple[str | None, list[dict[str, Any]], str | None]:
    if atom.relation not in {"eq", "lt", "le", "gt", "ge"}:
        return None, [], None
    left, right = atom.arguments
    left_sort = infer_sort(left, declarations)
    right_sort = infer_sort(right, declarations)
    if left_sort not in ARITHMETIC_SORTS or right_sort not in ARITHMETIC_SORTS:
        return None, [], "UNKNOWN_ARITHMETIC_SORT"
    if left_sort != right_sort:
        return None, [], "MIXED_ARITHMETIC_SORT"
    sort = ARITHMETIC_SORTS[left_sort]
    try:
        form = linearize(left, left_sort, declarations).added(
            linearize(right, right_sort, declarations).scaled(Fraction(-1))
        )
    except TraceError as error:
        return sort, [], str(error)

    directions: list[tuple[LinearForm, Fraction]]
    if atom.relation == "eq":
        if not positive:
            return sort, [], "DISEQUALITY"
        directions = [
            (form, Fraction(0)),
            (form.scaled(Fraction(-1)), Fraction(0)),
        ]
    else:
        relation = atom.relation
        if not positive:
            relation = {"lt": "ge", "le": "gt", "gt": "le", "ge": "lt"}[relation]
        if relation in {"gt", "ge"}:
            form = form.scaled(Fraction(-1))
            relation = {"gt": "lt", "ge": "le"}[relation]
        if relation == "lt":
            if sort == "Real":
                return sort, [], "STRICT_REAL"
            directions = [(form, Fraction(-1))]
        else:
            directions = [(form, Fraction(0))]

    constraints: list[dict[str, Any]] = []
    for direction, bound in directions:
        constraint, reason = difference_constraint(direction, bound)
        if constraint is None:
            return sort, [], reason
        constraints.append(constraint)
    return sort, constraints, None


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)


def build_abstraction(
    transcript: ParsedTranscript,
    *,
    source_id: str,
    source_sha256: str,
    family: str,
    partition: str,
    max_atoms: int = 256,
    max_clauses: int = 1024,
) -> dict[str, Any]:
    atom_origins: dict[str, list[dict[str, Any]]] = {}
    atom_objects: dict[str, Atom] = {}
    clause_origins: dict[tuple[tuple[str, bool], ...], list[dict[str, Any]]] = {}
    tautologies = 0

    for clause in transcript.clauses:
        literals = {(literal.atom.canonical(), literal.positive) for literal in clause.literals}
        atom_signs: dict[str, set[bool]] = {}
        for atom_key, positive in literals:
            atom_signs.setdefault(atom_key, set()).add(positive)
        if any(len(signs) == 2 for signs in atom_signs.values()):
            tautologies += 1
            continue
        canonical_clause = tuple(sorted(literals))
        origin = {
            "clause_name": clause.name,
            "role": clause.role,
            "statement_sha256": clause.statement_sha256,
            "grounding": dict(sorted(clause.grounding.items())),
            "variable_sorts": dict(sorted(clause.variable_sorts.items())),
        }
        clause_origins.setdefault(canonical_clause, []).append(origin)
        for literal in clause.literals:
            atom_key = literal.atom.canonical()
            atom_objects.setdefault(atom_key, literal.atom)
            atom_origins.setdefault(atom_key, []).append(
                {
                    **origin,
                    "positive": literal.positive,
                }
            )

    canonical_clauses = sorted(clause_origins)
    atom_keys = sorted(
        {
            atom_key
            for clause in canonical_clauses
            for atom_key, _ in clause
        }
    )
    bounds_crossed = []
    if len(atom_keys) > max_atoms:
        bounds_crossed.append("atoms")
    if len(canonical_clauses) > max_clauses:
        bounds_crossed.append("clauses")
    atom_ids = {atom_key: index + 1 for index, atom_key in enumerate(atom_keys)}

    atoms: list[dict[str, Any]] = []
    for atom_key in atom_keys:
        atom = atom_objects[atom_key]
        polarities = {}
        arithmetic = atom.relation in {"eq", "lt", "le", "gt", "ge"}
        for positive in (False, True):
            sort, constraints, reason = relation_constraints(
                atom, positive, transcript.declarations
            )
            polarities["true" if positive else "false"] = {
                "sort": sort,
                "constraints": constraints,
                "unsupported_reason": reason,
            }
        atoms.append(
            {
                "id": atom_ids[atom_key],
                "key": atom_key,
                "relation": atom.relation,
                "arguments": [
                    argument.canonical() for argument in atom.arguments
                ],
                "arithmetic": arithmetic,
                "polarities": polarities,
                "origins": sorted(
                    atom_origins[atom_key],
                    key=canonical_json,
                ),
            }
        )

    clauses = [
        {
            "literals": [
                atom_ids[atom_key] if positive else -atom_ids[atom_key]
                for atom_key, positive in clause
            ],
            "origins": sorted(clause_origins[clause], key=canonical_json),
        }
        for clause in canonical_clauses
    ]
    return {
        "schema": "umlaut-real-ground-abstraction-v1",
        "source_id": source_id,
        "source_sha256": source_sha256,
        "family": family,
        "partition": partition,
        "transcript_sha256": transcript.transcript_sha256,
        "parsed_clause_count": len(transcript.clauses),
        "canonical_clause_count": len(clauses),
        "atom_count": len(atoms),
        "tautology_count": tautologies,
        "bounds_crossed": bounds_crossed,
        "declarations": dict(sorted(transcript.declarations.items())),
        "atoms": atoms,
        "clauses": clauses,
    }


def unit_propagate(
    clauses: Sequence[Sequence[int]],
    assignment: dict[int, bool],
) -> tuple[dict[int, bool], list[dict[str, Any]], list[int] | None]:
    result = dict(assignment)
    steps: list[dict[str, Any]] = []
    while True:
        unit: tuple[int, Sequence[int]] | None = None
        for clause in clauses:
            undecided: list[int] = []
            satisfied = False
            for literal in clause:
                atom = abs(literal)
                if atom not in result:
                    undecided.append(literal)
                elif result[atom] == (literal > 0):
                    satisfied = True
                    break
            if satisfied:
                continue
            if not undecided:
                return result, steps, list(clause)
            if len(undecided) == 1:
                unit = undecided[0], clause
                break
        if unit is None:
            return result, steps, None
        literal, reason = unit
        atom = abs(literal)
        value = literal > 0
        existing = result.get(atom)
        if existing is not None and existing != value:
            return result, steps, list(reason)
        if existing is None:
            result[atom] = value
            steps.append(
                {
                    "atom": atom,
                    "value": value,
                    "reason_clause": list(reason),
                }
            )


def clauses_satisfied(
    clauses: Sequence[Sequence[int]],
    assignment: dict[int, bool],
) -> bool:
    return all(
        any(
            abs(literal) in assignment
            and assignment[abs(literal)] == (literal > 0)
            for literal in clause
        )
        for clause in clauses
    )


def theory_context(
    abstraction: dict[str, Any],
    assignment: dict[int, bool],
) -> dict[str, Any]:
    atoms = {atom["id"]: atom for atom in abstraction["atoms"]}
    constraints: list[dict[str, Any]] = []
    unsupported: list[dict[str, Any]] = []
    sorts: set[str] = set()
    for atom_id, value in sorted(assignment.items()):
        atom = atoms[atom_id]
        if not atom["arithmetic"]:
            continue
        polarity = atom["polarities"]["true" if value else "false"]
        if polarity["sort"] is not None:
            sorts.add(polarity["sort"])
        if polarity["unsupported_reason"] is not None:
            unsupported.append(
                {
                    "atom": atom_id,
                    "value": value,
                    "reason": polarity["unsupported_reason"],
                }
            )
            continue
        for index, constraint in enumerate(polarity["constraints"]):
            constraints.append(
                {
                    **constraint,
                    "label": f"a_{atom_id}_{'t' if value else 'f'}_{index}",
                    "atom": atom_id,
                    "value": value,
                }
            )
    if len(sorts) > 1:
        unsupported.append(
            {
                "atom": 0,
                "value": False,
                "reason": "MIXED_CONTEXT_SORT",
            }
        )
    sort = next(iter(sorts)) if len(sorts) == 1 else None
    fingerprint = sha256_text(
        canonical_json(
            {
                "sort": sort,
                "constraints": constraints,
            }
        )
    )
    return {
        "sort": sort,
        "constraints": constraints,
        "unsupported": unsupported,
        "eligible": sort is not None and len(sorts) == 1 and len(constraints) >= 2,
        "fingerprint": fingerprint,
    }


def build_no_theory_trace(
    abstraction: dict[str, Any],
    *,
    max_nodes: int = 4096,
    max_leaves: int = 1024,
) -> dict[str, Any]:
    if abstraction["bounds_crossed"]:
        return {
            "schema": "umlaut-real-ground-trace-v1",
            "source_id": abstraction["source_id"],
            "status": "bound",
            "bounds_crossed": abstraction["bounds_crossed"],
            "nodes": 0,
            "leaves": 0,
            "propositional_conflicts": 0,
            "eligible_queries": 0,
            "unsupported_contexts": 0,
            "queries": [],
            "events": [],
        }

    clauses = [tuple(clause["literals"]) for clause in abstraction["clauses"]]
    events: list[dict[str, Any]] = []
    queries: list[dict[str, Any]] = []
    node_count = 0
    leaf_count = 0
    conflicts = 0
    unsupported_contexts = 0
    bound_hit: str | None = None
    next_node = 1

    def visit(
        assignment: dict[int, bool],
        decisions: list[dict[str, Any]],
        parent: int | None,
        previous_fingerprint: str | None,
    ) -> None:
        nonlocal node_count, leaf_count, conflicts, unsupported_contexts
        nonlocal bound_hit, next_node
        if bound_hit is not None:
            return
        if node_count >= max_nodes:
            bound_hit = "nodes"
            return
        node_id = next_node
        next_node += 1
        node_count += 1
        propagated, units, conflict = unit_propagate(clauses, assignment)
        event: dict[str, Any] = {
            "node": node_id,
            "parent": parent,
            "depth": len(decisions),
            "decisions": list(decisions),
            "unit_steps": units,
            "assignment": [
                {"atom": atom, "value": value}
                for atom, value in sorted(propagated.items())
            ],
        }
        if conflict is not None:
            conflicts += 1
            event.update({"outcome": "propositional_conflict", "conflict": conflict})
            events.append(event)
            return

        context = theory_context(abstraction, propagated)
        if context["unsupported"]:
            unsupported_contexts += 1
        if context["eligible"] and context["fingerprint"] != previous_fingerprint:
            query_id = f"{abstraction['source_id']}_q_{len(queries) + 1:05d}"
            query = {
                "id": query_id,
                "node": node_id,
                "parent": parent,
                "depth": len(decisions),
                "sort": context["sort"],
                "fingerprint": context["fingerprint"],
                "constraints": context["constraints"],
                "excluded_unsupported": context["unsupported"],
                "assignment": event["assignment"],
                "decisions": list(decisions),
                "unit_steps": units,
            }
            queries.append(query)
            event["theory_query"] = query_id
            previous_fingerprint = context["fingerprint"]
        elif context["unsupported"]:
            event["theory_unknown"] = context["unsupported"]

        if clauses_satisfied(clauses, propagated):
            if leaf_count >= max_leaves:
                bound_hit = "leaves"
                return
            leaf_count += 1
            event["outcome"] = "open_leaf"
            events.append(event)
            return

        undecided = sorted(
            {
                abs(literal)
                for clause in clauses
                for literal in clause
                if abs(literal) not in propagated
            }
        )
        if not undecided:
            raise TraceError("unsatisfied clause set has no undecided atom")
        chosen = undecided[0]
        event.update({"outcome": "branch", "decision_atom": chosen})
        events.append(event)
        for value in (False, True):
            visit(
                {**propagated, chosen: value},
                [
                    *decisions,
                    {
                        "atom": chosen,
                        "value": value,
                        "parent_node": node_id,
                    },
                ],
                node_id,
                previous_fingerprint,
            )

    visit({}, [], None, None)
    return {
        "schema": "umlaut-real-ground-trace-v1",
        "source_id": abstraction["source_id"],
        "status": "bound" if bound_hit is not None else "complete",
        "bounds_crossed": [bound_hit] if bound_hit is not None else [],
        "nodes": node_count,
        "leaves": leaf_count,
        "propositional_conflicts": conflicts,
        "eligible_queries": len(queries),
        "unsupported_contexts": unsupported_contexts,
        "queries": queries,
        "events": events,
    }
