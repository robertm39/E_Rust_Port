#!/usr/bin/env python3
"""Independent finite-domain semantics oracle for experiment 023.

This module intentionally does not import ``adapter``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
from typing import Any


TOKEN_RE = re.compile(
    r"""
    \s+ | %[^\r\n]*
  | <=> | <~> | => | <= | !=
  | [+-]?\d+/\d+
  | [+-]?(?:\d+\.\d+(?:[Ee][+-]?\d+)?|\d+[Ee][+-]?\d+)
  | [+-]?\d+
  | \$[A-Za-z][A-Za-z0-9_]* | [A-Za-z][A-Za-z0-9_]*
  | [()[\],.:=&|~!?]
    """,
    re.VERBOSE,
)
NUMBER_RE = re.compile(
    r"^[+-]?(?:\d+/\d+|\d+\.\d+(?:[Ee][+-]?\d+)?|"
    r"\d+[Ee][+-]?\d+|\d+)$"
)
INTEGER_DOMAIN = tuple(Fraction(value) for value in range(-3, 4))
REAL_DOMAIN = tuple(Fraction(value, 2) for value in range(-6, 7))


class OracleError(RuntimeError):
    """The independently parsed formula is malformed or ill typed."""


@dataclass(frozen=True)
class Term:
    name: str
    arguments: tuple["Term", ...] = ()


@dataclass(frozen=True)
class Formula:
    kind: str
    value: str = ""
    children: tuple["Formula", ...] = ()
    terms: tuple[Term, ...] = ()
    binders: tuple[tuple[str, str], ...] = ()


def lex(text: str) -> list[str]:
    tokens = []
    position = 0
    while position < len(text):
        match = TOKEN_RE.match(text, position)
        if match is None:
            raise OracleError(
                f"independent lexer stopped near {text[position:position + 20]!r}"
            )
        token = match.group(0)
        position = match.end()
        if not token.isspace() and not token.startswith("%"):
            tokens.append(token)
    return tokens


class ReferenceParser:
    """A separate recursive-descent parser for the oracle."""

    PRECEDENCE = {
        "<=>": 1,
        "<~>": 1,
        "=>": 2,
        "<=": 2,
        "|": 3,
        "&": 4,
    }

    def __init__(self, text: str) -> None:
        self.tokens = lex(text)
        self.index = 0

    def peek(self) -> str | None:
        return self.tokens[self.index] if self.index < len(self.tokens) else None

    def take(self, expected: str | None = None) -> str:
        token = self.peek()
        if token is None:
            raise OracleError("unexpected end of TFF input")
        if expected is not None and token != expected:
            raise OracleError(f"expected {expected!r}, got {token!r}")
        self.index += 1
        return token

    def document(self) -> Formula:
        self.take("tff")
        self.take("(")
        self.take()
        self.take(",")
        self.take()
        self.take(",")
        formula = self.formula(0)
        self.take(")")
        self.take(".")
        if self.peek() is not None:
            raise OracleError("more than one formula in oracle input")
        return formula

    def formula(self, minimum: int) -> Formula:
        left = self.formula_prefix()
        while True:
            operator = self.peek()
            precedence = self.PRECEDENCE.get(operator or "")
            if precedence is None or precedence < minimum:
                return left
            self.take()
            right_associative = operator in {"<=>", "<~>", "=>", "<="}
            right = self.formula(precedence if right_associative else precedence + 1)
            left = Formula("binary", operator, (left, right))

    def formula_prefix(self) -> Formula:
        token = self.peek()
        if token == "~":
            self.take()
            return Formula("not", children=(self.formula_prefix(),))
        if token in {"!", "?"}:
            return self.quantifier()
        if token == "(":
            self.take("(")
            result = self.formula(0)
            self.take(")")
            return result
        if token in {"$true", "$false"}:
            return Formula("boolean", self.take())
        if token in {
            "$less",
            "$lesseq",
            "$greater",
            "$greatereq",
            "$is_int",
            "$is_rat",
        }:
            name = self.take()
            return Formula("predicate", name, terms=self.arguments())
        left = self.term()
        relation = self.take()
        if relation not in {"=", "!="}:
            raise OracleError(f"expected equality, got {relation!r}")
        return Formula("equality", relation, terms=(left, self.term()))

    def quantifier(self) -> Formula:
        quantifier = self.take()
        self.take("[")
        binders = []
        while True:
            name = self.take()
            self.take(":")
            sort = self.take()
            binders.append((name, sort))
            if self.peek() != ",":
                break
            self.take(",")
        self.take("]")
        self.take(":")
        return Formula(
            "quantifier",
            quantifier,
            (self.formula(0),),
            binders=tuple(binders),
        )

    def arguments(self) -> tuple[Term, ...]:
        self.take("(")
        arguments = []
        if self.peek() != ")":
            while True:
                arguments.append(self.term())
                if self.peek() != ",":
                    break
                self.take(",")
        self.take(")")
        return tuple(arguments)

    def term(self) -> Term:
        if self.peek() == "(":
            self.take("(")
            result = self.term()
            self.take(")")
            return result
        name = self.take()
        if self.peek() == "(":
            return Term(name, self.arguments())
        return Term(name)


def numeric(text: str) -> tuple[str, Fraction]:
    if "/" in text:
        return "$rat", Fraction(text)
    if "." in text or "e" in text.lower():
        return "$real", Fraction(Decimal(text))
    return "$int", Fraction(int(text))


def require_same(left: str, right: str) -> None:
    if left != right:
        raise OracleError(f"independent type mismatch: {left} versus {right}")


def evaluate_term(
    term: Term,
    environment: dict[str, tuple[str, Fraction]],
) -> tuple[str, Fraction]:
    if NUMBER_RE.match(term.name):
        return numeric(term.name)
    if not term.arguments and term.name in environment:
        return environment[term.name]
    arguments = [evaluate_term(child, environment) for child in term.arguments]
    name = term.name
    if name == "$uminus":
        sort, value = arguments[0]
        return sort, -value
    if name in {"$sum", "$difference", "$product", "$quotient"}:
        left_sort, left = arguments[0]
        right_sort, right = arguments[1]
        require_same(left_sort, right_sort)
        if name == "$sum":
            return left_sort, left + right
        if name == "$difference":
            return left_sort, left - right
        if name == "$product":
            return left_sort, left * right
        if right == 0:
            raise OracleError("oracle encountered an unspecified zero quotient")
        return ("$rat" if left_sort == "$int" else left_sort), left / right
    if name in {"$floor", "$ceiling"}:
        sort, value = arguments[0]
        integral = value.numerator // value.denominator
        if name == "$ceiling" and value != integral:
            integral += 1
        return sort, Fraction(integral)
    if name == "$to_int":
        _, value = arguments[0]
        return "$int", Fraction(value.numerator // value.denominator)
    if name == "$to_rat":
        _, value = arguments[0]
        return "$rat", value
    if name == "$to_real":
        _, value = arguments[0]
        return "$real", value
    raise OracleError(f"unsupported independent term {name!r}")


def evaluate_formula(
    formula: Formula,
    environment: dict[str, tuple[str, Fraction]] | None = None,
) -> bool:
    bindings = {} if environment is None else environment
    if formula.kind == "boolean":
        return formula.value == "$true"
    if formula.kind == "not":
        return not evaluate_formula(formula.children[0], bindings)
    if formula.kind == "binary":
        left = evaluate_formula(formula.children[0], bindings)
        right = evaluate_formula(formula.children[1], bindings)
        return {
            "&": left and right,
            "|": left or right,
            "=>": (not left) or right,
            "<=": (not right) or left,
            "<=>": left == right,
            "<~>": left != right,
        }[formula.value]
    if formula.kind == "quantifier":
        def visit(index: int) -> bool:
            if index == len(formula.binders):
                return evaluate_formula(formula.children[0], bindings)
            name, sort = formula.binders[index]
            domain = INTEGER_DOMAIN if sort == "$int" else REAL_DOMAIN
            outcomes = []
            previous = bindings.get(name)
            for value in domain:
                bindings[name] = (sort, value)
                outcomes.append(visit(index + 1))
            if previous is None:
                del bindings[name]
            else:
                bindings[name] = previous
            return all(outcomes) if formula.value == "!" else any(outcomes)

        return visit(0)
    if formula.kind == "equality":
        left_sort, left = evaluate_term(formula.terms[0], bindings)
        right_sort, right = evaluate_term(formula.terms[1], bindings)
        require_same(left_sort, right_sort)
        return left == right if formula.value == "=" else left != right
    if formula.kind == "predicate":
        values = [evaluate_term(term, bindings) for term in formula.terms]
        if formula.value == "$is_int":
            value = values[0][1]
            return value.denominator == 1
        if formula.value == "$is_rat":
            return True
        require_same(values[0][0], values[1][0])
        left, right = values[0][1], values[1][1]
        return {
            "$less": left < right,
            "$lesseq": left <= right,
            "$greater": left > right,
            "$greatereq": left >= right,
        }[formula.value]
    raise OracleError(f"unsupported independent formula {formula.kind!r}")


def evaluate_lira_term(
    term: dict[str, Any],
    environment: dict[str, Fraction],
) -> Fraction:
    kind = term["kind"]
    if kind == "constant":
        return Fraction(term["numerator"], term["denominator"])
    if kind == "variable":
        return environment[term["name"]]
    if kind == "scale":
        coefficient = Fraction(term["numerator"], term["denominator"])
        return coefficient * evaluate_lira_term(term["term"], environment)
    if kind == "add":
        return sum(
            (evaluate_lira_term(child, environment) for child in term["terms"]),
            Fraction(0),
        )
    if kind == "floor":
        value = evaluate_lira_term(term["term"], environment)
        return Fraction(value.numerator // value.denominator)
    raise OracleError(f"unsupported LIRA term {kind!r}")


def evaluate_lira_formula(
    formula: dict[str, Any],
    environment: dict[str, Fraction] | None = None,
) -> bool:
    bindings = {} if environment is None else environment
    kind = formula["kind"]
    if kind == "boolean":
        return bool(formula["value"])
    if kind == "atom":
        value = evaluate_lira_term(formula["term"], bindings)
        return {
            "eq": value == 0,
            "ne": value != 0,
            "gt": value > 0,
            "ge": value >= 0,
        }[formula["relation"]]
    if kind in {"and", "or"}:
        outcomes = [
            evaluate_lira_formula(child, bindings)
            for child in formula["children"]
        ]
        return all(outcomes) if kind == "and" else any(outcomes)
    if kind in {"exists", "forall"}:
        name = formula["variable"]
        previous = bindings.get(name)
        outcomes = []
        for value in REAL_DOMAIN:
            bindings[name] = value
            outcomes.append(evaluate_lira_formula(formula["body"], bindings))
        if previous is None:
            del bindings[name]
        else:
            bindings[name] = previous
        return all(outcomes) if kind == "forall" else any(outcomes)
    raise OracleError(f"unsupported LIRA formula {kind!r}")


def verify_views(source: str, result: dict[str, Any]) -> dict[str, bool]:
    """Compare the source, LIRA JSON, and re-embedded TFF independently."""

    original = evaluate_formula(ReferenceParser(source).document())
    lira = evaluate_lira_formula(result["lira_formula"])
    reembedded = evaluate_formula(
        ReferenceParser(result["reembedded_tff"]).document()
    )
    if len({original, lira, reembedded}) != 1:
        raise OracleError(
            "semantic disagreement: "
            f"source={original}, lira={lira}, reembedded={reembedded}"
        )
    return {
        "source": original,
        "lira": lira,
        "reembedded": reembedded,
    }
