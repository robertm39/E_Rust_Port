#!/usr/bin/env python3
"""Inventory CASC nonlinear arithmetic and probe a pinned Z3 decision boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass
from decimal import Decimal
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable, Sequence


sys.setrecursionlimit(max(sys.getrecursionlimit(), 10_000))


EXPECTED_Z3_COMMIT = "2d48fd119ce5074b880944c2b1c59e537c99cd46"
NUMERIC_SORTS = {"$int", "$rat", "$real"}
ARITHMETIC_FUNCTIONS = {
    "$sum",
    "$difference",
    "$product",
    "$quotient",
    "$quotient_e",
    "$quotient_t",
    "$quotient_f",
    "$remainder_e",
    "$remainder_t",
    "$remainder_f",
    "$uminus",
    "$floor",
    "$ceiling",
    "$truncate",
    "$round",
    "$to_int",
    "$to_rat",
    "$to_real",
    "$abs",
}
ARITHMETIC_RELATIONS = {
    "$less",
    "$lesseq",
    "$greater",
    "$greatereq",
    "$is_int",
    "$is_rat",
}
POLYNOMIAL_FUNCTIONS = {
    "$sum",
    "$difference",
    "$product",
    "$quotient",
    "$uminus",
}
ORDERED_RELATIONS = {"$less", "$lesseq", "$greater", "$greatereq"}
AXIOM_ROLES = {
    "axiom",
    "hypothesis",
    "lemma",
    "theorem",
    "definition",
    "assumption",
}
CONJECTURE_ROLES = {"conjecture", "negated_conjecture"}
ACCEPTED_ROLES = AXIOM_ROLES | CONJECTURE_ROLES
NUMBER_RE = re.compile(
    r"^[+-]?(?:\d+/\d+|\d+\.\d+(?:[Ee][+-]?\d+)?|"
    r"\d+[Ee][+-]?\d+|\d+)$"
)
ARITHMETIC_TEXT_RE = re.compile(
    r"\$(?:int|rat|real|sum|difference|product|quotient(?:_[etf])?|"
    r"remainder_[etf]|uminus|floor|ceiling|truncate|round|to_(?:int|rat|real)|"
    r"abs|less|lesseq|greater|greatereq|is_int|is_rat)\b"
    r"|(?<![A-Za-z0-9_])[-+]?\d+(?:/\d+|\.\d+(?:[Ee][-+]?\d+)?|"
    r"[Ee][-+]?\d+)?(?![A-Za-z0-9_])"
)
VARIABLE_RE = re.compile(r"^[A-Z][A-Za-z0-9_]*$")
TOKEN_RE = re.compile(
    r"""
    \s+
  | %[^\r\n]*
  | /\*.*?\*/
  | <=> | <~> | => | <= | != | :=
  | [+-]?\d+/\d+
  | [+-]?(?:\d+\.\d+(?:[Ee][+-]?\d+)?|\d+[Ee][+-]?\d+)
  | [+-]?\d+
  | \$[A-Za-z][A-Za-z0-9_]*
  | [A-Za-z][A-Za-z0-9_]*
  | '(?:[^'\\]|\\.)*'
  | "(?:[^"\\]|\\.)*"
  | [()[\]{},.:=&|~!?*^>@]
    """,
    re.VERBOSE | re.DOTALL,
)


class ExperimentError(RuntimeError):
    """A stable experiment or parser failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class Term:
    kind: str
    value: str
    arguments: tuple["Term", ...] = ()


@dataclass(frozen=True)
class Formula:
    kind: str
    value: str = ""
    children: tuple["Formula", ...] = ()
    terms: tuple[Term, ...] = ()
    binders: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class Annotated:
    dialect: str
    name: str
    role: str
    body_tokens: tuple[str, ...]


@dataclass(frozen=True)
class TermFacts:
    degree: int
    symbols: frozenset[str]


@dataclass
class ProblemAnalysis:
    path: str
    problem_id: str
    category: str
    division: str
    family: str
    split: str
    expected_class: str
    arithmetic_active: bool
    nonlinear_active: bool
    whole_real_polynomial: bool
    fragment: str
    exclusion_reason: str | None
    formula_count: int
    quantifier_count: int
    max_degree: int
    query_sha256: str | None
    expected_status: str | None
    solver_runs: list[dict[str, Any]]


class FormulaParser:
    """Parser for the first-order TFF formula surface used by the census."""

    PRECEDENCE = {
        "<=>": 1,
        "<~>": 1,
        "=>": 2,
        "<=": 2,
        "|": 3,
        "&": 4,
    }
    RIGHT_ASSOCIATIVE = {"<=>", "<~>", "=>", "<="}

    def __init__(self, tokens: Sequence[str]) -> None:
        self.tokens = list(tokens)
        self.position = 0

    def current(self) -> str | None:
        if self.position == len(self.tokens):
            return None
        return self.tokens[self.position]

    def consume(self, expected: str | None = None) -> str:
        token = self.current()
        if token is None:
            raise ExperimentError("parse_eof", "unexpected end of formula")
        if expected is not None and token != expected:
            raise ExperimentError(
                "parse_token",
                f"expected {expected!r}, found {token!r}",
            )
        self.position += 1
        return token

    def parse(self) -> Formula:
        formula = self.parse_formula(0)
        if self.current() is not None:
            raise ExperimentError(
                "parse_trailing",
                f"unexpected token {self.current()!r}",
            )
        return formula

    def parse_formula(self, minimum_precedence: int) -> Formula:
        left = self.parse_formula_prefix()
        while True:
            operator = self.current()
            precedence = self.PRECEDENCE.get(operator or "")
            if precedence is None or precedence < minimum_precedence:
                break
            self.consume()
            next_precedence = (
                precedence
                if operator in self.RIGHT_ASSOCIATIVE
                else precedence + 1
            )
            right = self.parse_formula(next_precedence)
            left = Formula("binary", operator, (left, right))
        return left

    def parse_formula_prefix(self) -> Formula:
        token = self.current()
        if token == "~":
            self.consume()
            return Formula("not", children=(self.parse_formula_prefix(),))
        if token in {"!", "?"}:
            return self.parse_quantifier()
        if token == "(":
            self.consume("(")
            formula = self.parse_formula(0)
            self.consume(")")
            return formula
        if token in {"$true", "$false"}:
            self.consume()
            return Formula("boolean", token)

        left = self.parse_term()
        relation = self.current()
        if relation in {"=", "!="}:
            self.consume()
            right = self.parse_term()
            return Formula("equality", relation, terms=(left, right))
        if left.kind == "call":
            return Formula("predicate", left.value, terms=left.arguments)
        raise ExperimentError(
            "parse_relation",
            f"expected a relation after {left.value!r}, found {relation!r}",
        )

    def parse_quantifier(self) -> Formula:
        quantifier = self.consume()
        self.consume("[")
        binders = []
        names = set()
        while True:
            name = self.consume()
            if not VARIABLE_RE.match(name):
                raise ExperimentError(
                    "parse_binder",
                    f"invalid quantified variable {name!r}",
                )
            if name in names:
                raise ExperimentError(
                    "parse_binder",
                    f"duplicate quantified variable {name!r}",
                )
            names.add(name)
            self.consume(":")
            sort_tokens = []
            depth = 0
            while True:
                token = self.current()
                if token is None:
                    raise ExperimentError("parse_eof", "unterminated binder")
                if depth == 0 and token in {",", "]"}:
                    break
                token = self.consume()
                sort_tokens.append(token)
                if token == "(":
                    depth += 1
                elif token == ")":
                    depth -= 1
            if len(sort_tokens) != 1:
                raise ExperimentError(
                    "unsupported_binder_type",
                    "only scalar binder types are classified",
                )
            binders.append((name, sort_tokens[0]))
            if self.current() != ",":
                break
            self.consume(",")
        self.consume("]")
        self.consume(":")
        body = self.parse_formula(0)
        return Formula("quantifier", quantifier, (body,), binders=tuple(binders))

    def parse_term(self) -> Term:
        token = self.current()
        if token == "(":
            self.consume("(")
            term = self.parse_term()
            self.consume(")")
            return term
        if token is None:
            raise ExperimentError("parse_eof", "expected a term")
        token = self.consume()
        if NUMBER_RE.match(token):
            return Term("number", token)
        if VARIABLE_RE.match(token):
            return Term("variable", token)
        if self.current() == "(":
            return Term("call", token, self.parse_term_arguments())
        return Term("constant", token)

    def parse_term_arguments(self) -> tuple[Term, ...]:
        self.consume("(")
        arguments = []
        if self.current() != ")":
            while True:
                arguments.append(self.parse_term())
                if self.current() != ",":
                    break
                self.consume(",")
        self.consume(")")
        return tuple(arguments)


def tokenize(text: str) -> list[str]:
    tokens = []
    position = 0
    while position < len(text):
        match = TOKEN_RE.match(text, position)
        if match is None:
            raise ExperimentError(
                "unsupported_token",
                f"unsupported token near {text[position:position + 32]!r}",
            )
        token = match.group(0)
        position = match.end()
        if (
            not token.isspace()
            and not token.startswith("%")
            and not token.startswith("/*")
        ):
            tokens.append(token)
    return tokens


def split_top_level(tokens: Sequence[str], delimiter: str) -> list[list[str]]:
    result: list[list[str]] = []
    current: list[str] = []
    depth = 0
    pairs = {"(": ")", "[": "]", "{": "}"}
    closing = set(pairs.values())
    stack: list[str] = []
    for token in tokens:
        if token in pairs:
            stack.append(pairs[token])
            depth += 1
        elif token in closing:
            if not stack or stack[-1] != token:
                raise ExperimentError("parse_nesting", "mismatched delimiters")
            stack.pop()
            depth -= 1
        if token == delimiter and depth == 0:
            result.append(current)
            current = []
        else:
            current.append(token)
    if stack:
        raise ExperimentError("parse_nesting", "unterminated delimiters")
    result.append(current)
    return result


def parse_document(text: str) -> tuple[list[Annotated], bool]:
    tokens = tokenize(text)
    statements: list[Annotated] = []
    includes = False
    position = 0
    while position < len(tokens):
        dialect = tokens[position]
        position += 1
        if dialect == "include":
            includes = True
        if position >= len(tokens) or tokens[position] != "(":
            raise ExperimentError(
                "parse_document",
                f"expected '(' after {dialect!r}",
            )
        position += 1
        body = []
        depth = 1
        while position < len(tokens) and depth:
            token = tokens[position]
            position += 1
            if token == "(":
                depth += 1
            elif token == ")":
                depth -= 1
                if depth == 0:
                    break
            body.append(token)
        if depth:
            raise ExperimentError("parse_document", "unterminated statement")
        if position >= len(tokens) or tokens[position] != ".":
            raise ExperimentError(
                "parse_document",
                f"expected '.' after {dialect!r} statement",
            )
        position += 1
        if dialect == "include":
            continue
        arguments = split_top_level(body, ",")
        if len(arguments) < 3:
            raise ExperimentError(
                "parse_document",
                f"{dialect!r} statement has fewer than three arguments",
            )
        if len(arguments[0]) != 1 or len(arguments[1]) != 1:
            raise ExperimentError(
                "parse_document",
                "non-atomic statement name or role",
            )
        statements.append(
            Annotated(
                dialect,
                arguments[0][0],
                arguments[1][0],
                tuple(arguments[2]),
            )
        )
    return statements, includes


def parse_number(token: str) -> Fraction:
    if "/" in token:
        return Fraction(token)
    if "." in token or "e" in token.lower():
        return Fraction(Decimal(token))
    return Fraction(int(token))


def numeric_constant(term: Term) -> Fraction | None:
    if term.kind == "number":
        return parse_number(term.value)
    if term.kind != "call":
        return None
    values = [numeric_constant(argument) for argument in term.arguments]
    if any(value is None for value in values):
        return None
    exact = [value for value in values if value is not None]
    if term.value == "$uminus" and len(exact) == 1:
        return -exact[0]
    if term.value == "$sum" and len(exact) == 2:
        return exact[0] + exact[1]
    if term.value == "$difference" and len(exact) == 2:
        return exact[0] - exact[1]
    if term.value == "$product" and len(exact) == 2:
        return exact[0] * exact[1]
    if term.value == "$quotient" and len(exact) == 2 and exact[1] != 0:
        return exact[0] / exact[1]
    return None


def iter_terms(formula: Formula) -> Iterable[Term]:
    for term in formula.terms:
        yield term
        yield from iter_subterms(term)
    for child in formula.children:
        yield from iter_terms(child)


def iter_subterms(term: Term) -> Iterable[Term]:
    for argument in term.arguments:
        yield argument
        yield from iter_subterms(argument)


def term_has_nonlinear_syntax(term: Term) -> bool:
    if term.kind == "call":
        if term.value == "$product" and len(term.arguments) == 2:
            if (
                numeric_constant(term.arguments[0]) is None
                and numeric_constant(term.arguments[1]) is None
            ):
                return True
        elif term.value == "$quotient" and len(term.arguments) == 2:
            if numeric_constant(term.arguments[1]) is None:
                return True
        elif term.value in ARITHMETIC_FUNCTIONS - POLYNOMIAL_FUNCTIONS:
            return True
    return any(term_has_nonlinear_syntax(child) for child in term.arguments)


def tokens_have_nonlinear_syntax(tokens: Sequence[str]) -> bool:
    stack: list[int] = []
    close_by_open: dict[int, int] = {}
    comma_by_open: dict[int, int] = {}
    for index, token in enumerate(tokens):
        if token == "(":
            stack.append(index)
        elif token == "," and stack:
            comma_by_open.setdefault(stack[-1], index)
        elif token == ")":
            if not stack:
                continue
            close_by_open[stack.pop()] = index

    punctuation = {
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        ",",
        ".",
        ":",
        "=",
        "!=",
        "&",
        "|",
        "~",
        "!",
        "?",
        "=>",
        "<=",
        "<=>",
        "<~>",
    }
    prefix_symbol_count = [0]
    for token in tokens:
        is_symbol = (
            token not in punctuation
            and token not in ARITHMETIC_FUNCTIONS
            and token not in ARITHMETIC_RELATIONS
            and token not in NUMERIC_SORTS
            and token not in {"$true", "$false"}
            and NUMBER_RE.match(token) is None
        )
        prefix_symbol_count.append(
            prefix_symbol_count[-1] + int(is_symbol)
        )

    def has_symbol(start: int, end: int) -> bool:
        return prefix_symbol_count[end] > prefix_symbol_count[start]

    for index, token in enumerate(tokens):
        if token in ARITHMETIC_FUNCTIONS - POLYNOMIAL_FUNCTIONS:
            return True
        if token not in {"$product", "$quotient"}:
            continue
        open_index = index + 1
        close_index = close_by_open.get(open_index)
        comma_index = comma_by_open.get(open_index)
        if (
            open_index >= len(tokens)
            or tokens[open_index] != "("
            or close_index is None
            or comma_index is None
        ):
            continue
        if token == "$product":
            if has_symbol(
                open_index + 1,
                comma_index,
            ) and has_symbol(comma_index + 1, close_index):
                return True
        elif has_symbol(comma_index + 1, close_index):
            return True
    return False


def scalar_declarations(
    statements: Sequence[Annotated],
) -> tuple[dict[str, str], str | None]:
    declarations: dict[str, str] = {}
    for statement in statements:
        if statement.role != "type":
            continue
        tokens = statement.body_tokens
        if len(tokens) == 3 and tokens[1] == ":":
            symbol, sort = tokens[0], tokens[2]
            if symbol in declarations and declarations[symbol] != sort:
                return {}, "conflicting_declaration"
            declarations[symbol] = sort
    return declarations, None


def analyze_term(
    term: Term,
    declarations: dict[str, str],
    environment: dict[str, str],
) -> TermFacts:
    if term.kind == "number":
        return TermFacts(0, frozenset())
    if term.kind == "variable":
        sort = environment.get(term.value)
        if sort is None:
            raise ExperimentError(
                "unbound_variable",
                f"unbound variable {term.value!r}",
            )
        if sort != "$real":
            raise ExperimentError(
                "non_real_sort",
                f"variable {term.value!r} has sort {sort!r}",
            )
        return TermFacts(1, frozenset({term.value}))
    if term.kind == "constant":
        sort = declarations.get(term.value)
        if sort is None:
            raise ExperimentError(
                "undeclared_symbol",
                f"undeclared constant {term.value!r}",
            )
        if sort != "$real":
            raise ExperimentError(
                "non_real_sort",
                f"constant {term.value!r} has sort {sort!r}",
            )
        return TermFacts(1, frozenset({term.value}))
    if term.value not in POLYNOMIAL_FUNCTIONS:
        code = (
            "unsupported_arithmetic"
            if term.value.startswith("$")
            else "user_function"
        )
        raise ExperimentError(code, f"unsupported function {term.value!r}")

    arguments = [
        analyze_term(argument, declarations, environment)
        for argument in term.arguments
    ]
    symbols = frozenset().union(*(argument.symbols for argument in arguments))
    if term.value == "$uminus":
        if len(arguments) != 1:
            raise ExperimentError("arity", "$uminus expects one argument")
        return TermFacts(arguments[0].degree, symbols)
    if len(arguments) != 2:
        raise ExperimentError("arity", f"{term.value} expects two arguments")
    if term.value in {"$sum", "$difference"}:
        return TermFacts(max(argument.degree for argument in arguments), symbols)
    if term.value == "$product":
        return TermFacts(sum(argument.degree for argument in arguments), symbols)
    denominator = numeric_constant(term.arguments[1])
    if denominator is None:
        raise ExperimentError(
            "symbolic_division",
            "division by a symbolic term is not polynomial",
        )
    if denominator == 0:
        raise ExperimentError(
            "zero_division",
            "division by zero is outside the candidate fragment",
        )
    return TermFacts(arguments[0].degree, symbols)


def analyze_formula(
    formula: Formula,
    declarations: dict[str, str],
    environment: dict[str, str] | None = None,
) -> tuple[int, int, frozenset[str]]:
    environment = {} if environment is None else environment
    if formula.kind == "boolean":
        return 0, 0, frozenset()
    if formula.kind == "not":
        return analyze_formula(formula.children[0], declarations, environment)
    if formula.kind == "binary":
        left = analyze_formula(formula.children[0], declarations, environment)
        right = analyze_formula(formula.children[1], declarations, environment)
        return (
            max(left[0], right[0]),
            left[1] + right[1],
            left[2] | right[2],
        )
    if formula.kind == "quantifier":
        nested = dict(environment)
        for name, sort in formula.binders:
            if sort != "$real":
                raise ExperimentError(
                    "non_real_sort",
                    f"binder {name!r} has sort {sort!r}",
                )
            nested[name] = sort
        degree, quantifiers, symbols = analyze_formula(
            formula.children[0],
            declarations,
            nested,
        )
        return degree, quantifiers + len(formula.binders), symbols
    if formula.kind == "predicate":
        if formula.value not in ORDERED_RELATIONS:
            code = (
                "unsupported_arithmetic"
                if formula.value.startswith("$")
                else "user_predicate"
            )
            raise ExperimentError(
                code,
                f"unsupported predicate {formula.value!r}",
            )
        if len(formula.terms) != 2:
            raise ExperimentError("arity", f"{formula.value} expects two terms")
    elif formula.kind == "equality":
        if len(formula.terms) != 2:
            raise ExperimentError("arity", "equality expects two terms")
    else:
        raise ExperimentError(
            "unsupported_formula",
            f"unsupported formula kind {formula.kind!r}",
        )
    terms = [
        analyze_term(term, declarations, environment) for term in formula.terms
    ]
    return (
        max(term.degree for term in terms),
        0,
        frozenset().union(*(term.symbols for term in terms)),
    )


def render_fraction(value: Fraction) -> str:
    numerator = value.numerator
    denominator = value.denominator
    if denominator == 1:
        if numerator < 0:
            return f"(- {-numerator})"
        return str(numerator)
    if numerator < 0:
        return f"(- (/ {-numerator} {denominator}))"
    return f"(/ {numerator} {denominator})"


class SmtRenderer:
    def __init__(self, declarations: dict[str, str]) -> None:
        real_constants = sorted(
            symbol for symbol, sort in declarations.items() if sort == "$real"
        )
        self.constants = {
            symbol: f"c_{index}" for index, symbol in enumerate(real_constants)
        }
        self.next_variable = 0

    def render_term(self, term: Term, environment: dict[str, str]) -> str:
        if term.kind == "number":
            return render_fraction(parse_number(term.value))
        if term.kind == "variable":
            return environment[term.value]
        if term.kind == "constant":
            return self.constants[term.value]
        operator = {
            "$sum": "+",
            "$difference": "-",
            "$product": "*",
            "$quotient": "/",
            "$uminus": "-",
        }[term.value]
        arguments = " ".join(
            self.render_term(argument, environment)
            for argument in term.arguments
        )
        return f"({operator} {arguments})"

    def render_formula(
        self,
        formula: Formula,
        environment: dict[str, str] | None = None,
    ) -> str:
        environment = {} if environment is None else environment
        if formula.kind == "boolean":
            return "true" if formula.value == "$true" else "false"
        if formula.kind == "not":
            return f"(not {self.render_formula(formula.children[0], environment)})"
        if formula.kind == "binary":
            left = self.render_formula(formula.children[0], environment)
            right = self.render_formula(formula.children[1], environment)
            if formula.value == "<=>":
                return f"(= {left} {right})"
            if formula.value == "<~>":
                return f"(xor {left} {right})"
            if formula.value == "=>":
                return f"(=> {left} {right})"
            if formula.value == "<=":
                return f"(=> {right} {left})"
            operator = {"&": "and", "|": "or"}[formula.value]
            return f"({operator} {left} {right})"
        if formula.kind == "quantifier":
            nested = dict(environment)
            binders = []
            for name, _sort in formula.binders:
                rendered = f"v_{self.next_variable}"
                self.next_variable += 1
                nested[name] = rendered
                binders.append(f"({rendered} Real)")
            quantifier = "forall" if formula.value == "!" else "exists"
            body = self.render_formula(formula.children[0], nested)
            return f"({quantifier} ({' '.join(binders)}) {body})"
        terms = [
            self.render_term(term, environment) for term in formula.terms
        ]
        if formula.kind == "equality":
            relation = "=" if formula.value == "=" else "distinct"
        else:
            relation = {
                "$less": "<",
                "$lesseq": "<=",
                "$greater": ">",
                "$greatereq": ">=",
            }[formula.value]
        return f"({relation} {' '.join(terms)})"

    def problem_script(
        self,
        formulas: Sequence[tuple[str, Formula]],
        fragment: str,
        timeout_ms: int,
    ) -> str:
        lines = [
            f"(set-option :timeout {timeout_ms})",
            "(set-option :print-success false)",
            "(set-logic NRA)",
        ]
        for rendered in self.constants.values():
            lines.append(f"(declare-fun {rendered} () Real)")
        for index, (role, formula) in enumerate(formulas):
            rendered = self.render_formula(formula)
            if role == "conjecture":
                rendered = f"(not {rendered})"
            lines.append(f"(assert (! {rendered} :named f_{index}))")
        tactic = "qfnra-nlsat" if fragment == "whole_qf_nra" else "nlqsat"
        lines.append(f"(check-sat-using {tactic})")
        lines.append("(exit)")
        return "\n".join(lines) + "\n"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    records = [json.loads(line) for line in path.read_text().splitlines()]
    if not records or records[0].get("record_type") != "manifest":
        raise ExperimentError("manifest", "missing manifest header")
    problems = [
        record for record in records[1:] if record.get("record_type") == "problem"
    ]
    if len(problems) != records[0].get("problem_count"):
        raise ExperimentError("manifest", "problem count does not match header")
    return records[0], problems


def arithmetic_lexical_activity(tokens: Sequence[str]) -> bool:
    return any(
        token in NUMERIC_SORTS
        or token in ARITHMETIC_FUNCTIONS
        or token in ARITHMETIC_RELATIONS
        or NUMBER_RE.match(token)
        for token in tokens
    )


def arithmetic_text_activity(text: str) -> bool:
    without_comments = re.sub(
        r"%[^\r\n]*|/\*.*?\*/",
        " ",
        text,
        flags=re.DOTALL,
    )
    return ARITHMETIC_TEXT_RE.search(without_comments) is not None


def expected_status(expected_class: str) -> str | None:
    return {
        "theorem": "unsat",
        "countersatisfiable": "sat",
    }.get(expected_class)


def analyze_problem(
    root: Path,
    record: dict[str, Any],
    timeout_ms: int,
) -> tuple[ProblemAnalysis, str | None]:
    path = root / Path(record["path"])
    data = path.read_bytes()
    actual_hash = sha256_bytes(data)
    if actual_hash != record["sha256"]:
        raise ExperimentError(
            "problem_hash",
            f"{record['path']}: expected {record['sha256']}, got {actual_hash}",
        )
    text = data.decode("utf-8")
    if record["division"] != "TFA":
        arithmetic_active = arithmetic_text_activity(text)
        return (
            ProblemAnalysis(
                path=record["path"],
                problem_id=record["problem_id"],
                category=record["category"],
                division=record["division"],
                family=record["family"],
                split=record["holdout_split"],
                expected_class=record["expected_class"],
                arithmetic_active=arithmetic_active,
                nonlinear_active=False,
                whole_real_polynomial=False,
                fragment="ineligible",
                exclusion_reason="non_tfa_division",
                formula_count=0,
                quantifier_count=0,
                max_degree=0,
                query_sha256=None,
                expected_status=expected_status(record["expected_class"]),
                solver_runs=[],
            ),
            None,
        )
    raw_tokens = tokenize(text)
    arithmetic_active = arithmetic_lexical_activity(raw_tokens)
    formulas: list[tuple[str, Formula]] = []
    query = None
    exclusion = None
    formula_count = 0
    quantifier_count = 0
    max_degree = 0
    nonlinear_active = False
    try:
        statements, includes = parse_document(text)
        if includes:
            exclusion = "includes"
        elif any(statement.dialect != "tff" for statement in statements):
            exclusion = "non_tff_dialect"
        declarations, declaration_error = scalar_declarations(statements)
        if exclusion is None and declaration_error is not None:
            exclusion = declaration_error
        formula_tokens = [
            token
            for statement in statements
            if statement.role != "type"
            for token in statement.body_tokens
        ]
        declared_types = {
            statement.body_tokens[0]: tuple(statement.body_tokens[2:])
            for statement in statements
            if statement.role == "type"
            and len(statement.body_tokens) >= 3
            and statement.body_tokens[1] == ":"
        }
        used_symbols = set(formula_tokens)
        for symbol, type_tokens in declared_types.items():
            if symbol not in used_symbols:
                continue
            if type_tokens == ("$real",):
                continue
            if "$int" in type_tokens or "$rat" in type_tokens:
                if exclusion is None:
                    exclusion = "non_real_sort"
            elif type_tokens != ("$tType",) and exclusion is None:
                exclusion = "user_function"
        if (
            exclusion is None
            and any(token in {"$int", "$rat"} for token in formula_tokens)
        ):
            exclusion = "non_real_sort"
        unsupported_arithmetic = (
            ARITHMETIC_FUNCTIONS - POLYNOMIAL_FUNCTIONS
        ) | (ARITHMETIC_RELATIONS - ORDERED_RELATIONS)
        if (
            exclusion is None
            and any(token in unsupported_arithmetic for token in formula_tokens)
        ):
            exclusion = "unsupported_arithmetic"
        nonlinear_active = tokens_have_nonlinear_syntax(formula_tokens)
        for statement in statements:
            if statement.role == "type":
                continue
            formula_count += 1
            if exclusion is not None:
                continue
            try:
                formula = FormulaParser(statement.body_tokens).parse()
            except (ExperimentError, RecursionError) as error:
                if exclusion is None:
                    exclusion = (
                        error.code
                        if isinstance(error, ExperimentError)
                        else "resource_depth"
                    )
                continue
            if statement.role not in ACCEPTED_ROLES:
                if exclusion is None:
                    exclusion = "unsupported_role"
                continue
            try:
                degree, quantifiers, _symbols = analyze_formula(
                    formula,
                    declarations,
                )
            except ExperimentError as error:
                if exclusion is None:
                    exclusion = error.code
                continue
            max_degree = max(max_degree, degree)
            quantifier_count += quantifiers
            formulas.append((statement.role, formula))
        if formula_count == 0 and exclusion is None:
            exclusion = "no_formulas"
        if len(formulas) != formula_count and exclusion is None:
            exclusion = "unclassified_formula"
    except ExperimentError as error:
        raw_tokens = []
        arithmetic_active = bool(
            re.search(
                r"\$(?:int|rat|real|sum|difference|product|quotient|"
                r"less|lesseq|greater|greatereq)\b",
                text,
            )
        )
        exclusion = error.code

    whole_real_polynomial = exclusion is None
    if whole_real_polynomial and max_degree < 2:
        fragment = "whole_linear_real"
    elif whole_real_polynomial and quantifier_count == 0:
        fragment = "whole_qf_nra"
    elif whole_real_polynomial:
        fragment = "whole_quantified_nra"
    else:
        fragment = "ineligible"

    if fragment in {"whole_qf_nra", "whole_quantified_nra"}:
        renderer = SmtRenderer(declarations)
        query = renderer.problem_script(formulas, fragment, timeout_ms)

    analysis = ProblemAnalysis(
        path=record["path"],
        problem_id=record["problem_id"],
        category=record["category"],
        division=record["division"],
        family=record["family"],
        split=record["holdout_split"],
        expected_class=record["expected_class"],
        arithmetic_active=arithmetic_active,
        nonlinear_active=nonlinear_active,
        whole_real_polynomial=whole_real_polynomial,
        fragment=fragment,
        exclusion_reason=exclusion,
        formula_count=formula_count,
        quantifier_count=quantifier_count,
        max_degree=max_degree,
        query_sha256=(
            sha256_bytes(query.encode("utf-8")) if query is not None else None
        ),
        expected_status=expected_status(record["expected_class"]),
        solver_runs=[],
    )
    return analysis, query


def normalize_solver_status(stdout: str) -> str | None:
    for line in stdout.splitlines():
        stripped = line.strip()
        if stripped in {"sat", "unsat", "unknown"}:
            return stripped
    return None


def run_solver(
    z3: Path,
    script: str,
    harness_timeout_seconds: float,
) -> dict[str, Any]:
    started = time.perf_counter()
    try:
        process = subprocess.run(
            [str(z3), "-in", "-smt2"],
            input=script,
            text=True,
            capture_output=True,
            timeout=harness_timeout_seconds,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        return {
            "classification": "timeout",
            "status": None,
            "returncode": None,
            "elapsed_ms": round(elapsed_ms, 3),
            "stdout": error.stdout or "",
            "stderr": error.stderr or "",
        }
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    status = normalize_solver_status(process.stdout)
    if process.returncode != 0:
        classification = "process_error"
    elif status is None:
        classification = "malformed_output"
    else:
        classification = status
    return {
        "classification": classification,
        "status": status,
        "returncode": process.returncode,
        "elapsed_ms": round(elapsed_ms, 3),
        "stdout": process.stdout,
        "stderr": process.stderr,
    }


def solver_version(z3: Path) -> str:
    process = subprocess.run(
        [str(z3), "-version"],
        text=True,
        capture_output=True,
        timeout=10,
        check=True,
    )
    return process.stdout.strip()


def proof_probe(z3: Path) -> dict[str, Any]:
    script = """\
(set-option :produce-proofs true)
(set-option :timeout 10000)
(set-logic QF_NRA)
(declare-fun x () Real)
(assert (> (* x x) 2))
(assert (< (* x x) 1))
(check-sat-using qfnra-nlsat)
(get-proof)
(exit)
"""
    return run_solver(z3, script, 15.0)


def count_source_lines(root: Path, relative_roots: Sequence[str]) -> dict[str, Any]:
    entries = []
    total_files = 0
    total_physical = 0
    total_code = 0
    for relative in relative_roots:
        directory = root / relative
        files = sorted(
            path
            for path in directory.rglob("*")
            if path.is_file() and path.suffix in {".h", ".hpp", ".c", ".cc", ".cpp"}
        )
        physical = 0
        code = 0
        for path in files:
            in_block_comment = False
            for line in path.read_text(
                encoding="utf-8",
                errors="replace",
            ).splitlines():
                physical += 1
                remainder = line
                fragments = []
                while remainder:
                    if in_block_comment:
                        end = remainder.find("*/")
                        if end < 0:
                            remainder = ""
                        else:
                            remainder = remainder[end + 2 :]
                            in_block_comment = False
                    else:
                        block = remainder.find("/*")
                        slash = remainder.find("//")
                        if slash >= 0 and (block < 0 or slash < block):
                            fragments.append(remainder[:slash])
                            remainder = ""
                        elif block >= 0:
                            fragments.append(remainder[:block])
                            remainder = remainder[block + 2 :]
                            in_block_comment = True
                        else:
                            fragments.append(remainder)
                            remainder = ""
                if "".join(fragments).strip():
                    code += 1
        entries.append(
            {
                "path": relative,
                "files": len(files),
                "physical_lines": physical,
                "nonblank_noncomment_lines": code,
            }
        )
        total_files += len(files)
        total_physical += physical
        total_code += code
    return {
        "subsystems": entries,
        "total_files": total_files,
        "total_physical_lines": total_physical,
        "total_nonblank_noncomment_lines": total_code,
        "classification": "large" if total_code > 20_000 else "bounded",
    }


def aggregate(problems: Sequence[ProblemAnalysis]) -> dict[str, Any]:
    def counter(field: str) -> dict[str, int]:
        return dict(
            sorted(
                Counter(str(getattr(problem, field)) for problem in problems).items()
            )
        )

    fragments = Counter(problem.fragment for problem in problems)
    exclusions = Counter(
        problem.exclusion_reason
        for problem in problems
        if problem.exclusion_reason is not None
    )
    eligible = [
        problem
        for problem in problems
        if problem.fragment in {"whole_qf_nra", "whole_quantified_nra"}
    ]
    by_fragment_split: dict[str, Counter[str]] = defaultdict(Counter)
    by_fragment_category: dict[str, Counter[str]] = defaultdict(Counter)
    raw_status = Counter()
    deterministic = 0
    expected = 0
    for problem in eligible:
        by_fragment_split[problem.fragment][problem.split] += 1
        by_fragment_category[problem.fragment][problem.category] += 1
        statuses = [run["status"] for run in problem.solver_runs]
        if statuses:
            raw_status.update(status for status in statuses if status is not None)
            if len(set(statuses)) == 1:
                deterministic += 1
            if statuses[0] == problem.expected_status:
                expected += 1
    return {
        "problem_count": len(problems),
        "division_counts": counter("division"),
        "category_counts": counter("category"),
        "split_counts": counter("split"),
        "arithmetic_active_count": sum(
            problem.arithmetic_active for problem in problems
        ),
        "nonlinear_active_count": sum(
            problem.nonlinear_active for problem in problems
        ),
        "whole_real_polynomial_count": sum(
            problem.whole_real_polynomial for problem in problems
        ),
        "fragment_counts": dict(sorted(fragments.items())),
        "exclusion_counts": dict(sorted(exclusions.items())),
        "eligible_by_fragment_and_split": {
            fragment: dict(sorted(counts.items()))
            for fragment, counts in sorted(by_fragment_split.items())
        },
        "eligible_by_fragment_and_category": {
            fragment: dict(sorted(counts.items()))
            for fragment, counts in sorted(by_fragment_category.items())
        },
        "solver": {
            "eligible_problem_count": len(eligible),
            "raw_status_counts_across_repetitions": dict(
                sorted(raw_status.items())
            ),
            "deterministic_problem_count": deterministic,
            "expected_first_run_problem_count": expected,
            "trusted_problem_count": 0,
            "unknown_baseline_count": len(eligible),
        },
    }


def decision(report: dict[str, Any]) -> dict[str, Any]:
    summary = report["summary"]
    qf_count = summary["fragment_counts"].get("whole_qf_nra", 0)
    eligible = [
        problem
        for problem in report["problems"]
        if problem["fragment"] == "whole_qf_nra"
    ]
    expected = sum(
        bool(problem["solver_runs"])
        and problem["solver_runs"][0]["status"] == problem["expected_status"]
        and len(
            {
                run["status"]
                for run in problem["solver_runs"]
            }
        )
        == 1
        for problem in eligible
    )
    coverage = expected / qf_count if qf_count else 0.0
    gates = {
        "at_least_five_qf_nra": qf_count >= 5,
        "at_least_80_percent_expected_deterministic": coverage >= 0.80,
        "independent_replay_100_percent": False,
        "no_deployment_blocker": False,
        "reimplementation_not_large": (
            report["source_inventory"]["classification"] != "large"
        ),
    }
    if not gates["at_least_five_qf_nra"] or not gates[
        "at_least_80_percent_expected_deterministic"
    ]:
        recommendation = "reject_candidate_boundary"
    elif all(gates.values()):
        recommendation = "pursue_narrow_follow_up"
    else:
        recommendation = "defer"
    return {
        "qf_nra_problem_count": qf_count,
        "qf_nra_expected_deterministic_count": expected,
        "qf_nra_expected_deterministic_fraction": coverage,
        "gates": gates,
        "recommendation": recommendation,
    }


def canonical_json(value: Any) -> bytes:
    return (
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--z3", type=Path)
    parser.add_argument("--z3-source-root", type=Path, required=True)
    parser.add_argument(
        "--z3-commit",
        default=EXPECTED_Z3_COMMIT,
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--query-dir", type=Path)
    parser.add_argument(
        "--inventory-input",
        type=Path,
        help="resume a hash-verified inventory report and execute its queries",
    )
    parser.add_argument("--timeout-ms", type=int, default=10_000)
    parser.add_argument("--harness-timeout-seconds", type=float, default=15.0)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--inventory-only", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if arguments.z3_commit != EXPECTED_Z3_COMMIT:
        raise SystemExit(
            f"refusing unpinned Z3 commit {arguments.z3_commit}; "
            f"expected {EXPECTED_Z3_COMMIT}"
        )
    if arguments.repetitions != 2:
        raise SystemExit("the preregistered protocol requires two repetitions")
    root = arguments.repo_root.resolve()
    manifest = arguments.manifest.resolve()
    z3_source = arguments.z3_source_root.resolve()
    if not z3_source.is_dir():
        raise SystemExit(f"Z3 source root is not a directory: {z3_source}")
    inventory_input = None
    analyses: list[ProblemAnalysis] = []
    queries: dict[str, str] = {}
    if arguments.inventory_input is not None:
        inventory_path = arguments.inventory_input.resolve()
        inventory_input = json.loads(inventory_path.read_text())
        if inventory_input.get("schema") != (
            "umlaut-nonlinear-arithmetic-feasibility-v1"
        ):
            raise SystemExit("inventory input has an unsupported schema")
        protocol = inventory_input.get("protocol", {})
        if not protocol.get("inventory_only"):
            raise SystemExit("inventory input was not produced in inventory-only mode")
        if protocol.get("expected_z3_commit") != EXPECTED_Z3_COMMIT:
            raise SystemExit("inventory input has a different pinned Z3 commit")
        if protocol.get("timeout_ms") != arguments.timeout_ms:
            raise SystemExit("inventory input uses a different solver timeout")
        analyses = [
            ProblemAnalysis(**problem)
            for problem in inventory_input["problems"]
        ]
        if arguments.query_dir is None:
            raise SystemExit("--query-dir is required with --inventory-input")
        for analysis in analyses:
            if analysis.query_sha256 is None:
                continue
            safe_name = re.sub(r"[^A-Za-z0-9_.-]", "_", analysis.problem_id)
            query_path = arguments.query_dir.resolve() / f"{safe_name}.smt2"
            query = query_path.read_text()
            actual_hash = sha256_bytes(query.encode("utf-8"))
            if actual_hash != analysis.query_sha256:
                raise SystemExit(
                    f"query hash mismatch for {analysis.problem_id}: "
                    f"expected {analysis.query_sha256}, got {actual_hash}"
                )
            queries[analysis.problem_id] = query
    else:
        header, records = load_manifest(manifest)
        for record in records:
            analysis, query = analyze_problem(root, record, arguments.timeout_ms)
            analyses.append(analysis)
            if query is not None:
                queries[analysis.problem_id] = query

    z3_metadata = None
    proof = None
    if not arguments.inventory_only:
        if arguments.z3 is None:
            raise SystemExit("--z3 is required unless --inventory-only is used")
        z3 = arguments.z3.resolve()
        if not z3.is_file():
            raise SystemExit(f"Z3 executable is not a file: {z3}")
        z3_metadata = {
            "path": str(z3),
            "sha256": sha256_file(z3),
            "size_bytes": z3.stat().st_size,
            "version": solver_version(z3),
        }
        for analysis in analyses:
            query = queries.get(analysis.problem_id)
            if query is None:
                continue
            for _repetition in range(arguments.repetitions):
                analysis.solver_runs.append(
                    run_solver(
                        z3,
                        query,
                        arguments.harness_timeout_seconds,
                    )
                )
        proof = proof_probe(z3)

    if arguments.query_dir is not None and inventory_input is None:
        query_dir = arguments.query_dir.resolve()
        query_dir.mkdir(parents=True, exist_ok=True)
        for problem_id, query in sorted(queries.items()):
            safe_name = re.sub(r"[^A-Za-z0-9_.-]", "_", problem_id)
            (query_dir / f"{safe_name}.smt2").write_bytes(
                query.encode("utf-8")
            )

    source_inventory = count_source_lines(
        z3_source,
        ["src/nlsat", "src/qe", "src/math/polynomial", "src/math/realclosure"],
    )
    if inventory_input is None:
        input_metadata = {
            "manifest": str(manifest),
            "manifest_sha256": sha256_file(manifest),
            "manifest_problem_archive_sha256": header["sources"][
                "problem_archive_sha256"
            ],
            "z3_source_root": str(z3_source),
        }
    else:
        input_metadata = dict(inventory_input["inputs"])
        input_metadata.update(
            {
                "inventory_report": str(arguments.inventory_input.resolve()),
                "inventory_report_sha256": sha256_file(
                    arguments.inventory_input.resolve()
                ),
                "z3_source_root": str(z3_source),
            }
        )
    report: dict[str, Any] = {
        "schema": "umlaut-nonlinear-arithmetic-feasibility-v1",
        "protocol": {
            "expected_z3_commit": EXPECTED_Z3_COMMIT,
            "z3_commit": arguments.z3_commit,
            "timeout_ms": arguments.timeout_ms,
            "harness_timeout_seconds": arguments.harness_timeout_seconds,
            "repetitions": arguments.repetitions,
            "inventory_only": arguments.inventory_only,
            "trusted_result_policy": "raw_solver_results_are_never_trusted",
        },
        "inputs": input_metadata,
        "z3": z3_metadata,
        "proof_generation_probe": proof,
        "source_inventory": source_inventory,
        "problems": [asdict(analysis) for analysis in analyses],
    }
    report["summary"] = aggregate(analyses)
    report["decision"] = decision(report)
    payload = canonical_json(report)
    arguments.output.resolve().parent.mkdir(parents=True, exist_ok=True)
    arguments.output.resolve().write_bytes(payload)
    print(
        json.dumps(
            {
                "output": str(arguments.output.resolve()),
                "sha256": sha256_bytes(payload),
                "summary": report["summary"],
                "decision": report["decision"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
