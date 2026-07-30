#!/usr/bin/env python3
"""Conservative typed-TPTP importer for the experiment-only LIRA AST."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence


TOKEN_RE = re.compile(
    r"""
    \s+
  | %[^\r\n]*
  | <=> | <~> | => | <= | !=
  | [+-]?\d+/\d+
  | [+-]?(?:\d+\.\d+(?:[Ee][+-]?\d+)?|\d+[Ee][+-]?\d+)
  | [+-]?\d+
  | \$[A-Za-z][A-Za-z0-9_]*
  | [A-Za-z][A-Za-z0-9_]*
  | [()[\],.:=&|~!?]
    """,
    re.VERBOSE,
)
VARIABLE_RE = re.compile(r"^[A-Z][A-Za-z0-9_]*$")
NUMBER_RE = re.compile(
    r"^[+-]?(?:\d+/\d+|\d+\.\d+(?:[Ee][+-]?\d+)?|"
    r"\d+[Ee][+-]?\d+|\d+)$"
)
DEFINED_UNSUPPORTED = {
    "$quotient_e",
    "$quotient_t",
    "$quotient_f",
    "$remainder_e",
    "$remainder_t",
    "$remainder_f",
    "$truncate",
    "$round",
    "$abs",
}


class AdapterError(RuntimeError):
    """A stable, fail-closed adapter rejection."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class ParsedTerm:
    kind: str
    value: str
    arguments: tuple["ParsedTerm", ...] = ()


@dataclass(frozen=True)
class ParsedFormula:
    kind: str
    value: str = ""
    children: tuple["ParsedFormula", ...] = ()
    terms: tuple[ParsedTerm, ...] = ()
    binders: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class ParsedDocument:
    name: str
    role: str
    formula: ParsedFormula


LTerm = tuple[Any, ...]
LFormula = tuple[Any, ...]


def tokenize(text: str) -> list[str]:
    tokens = []
    position = 0
    while position < len(text):
        match = TOKEN_RE.match(text, position)
        if match is None:
            raise AdapterError(
                "MALFORMED_INPUT",
                f"unsupported token near {text[position:position + 24]!r}",
            )
        token = match.group(0)
        position = match.end()
        if not token.isspace() and not token.startswith("%"):
            tokens.append(token)
    return tokens


class Parser:
    """Parse the deliberately small, pure-arithmetic TFF surface."""

    PRECEDENCE = {
        "<=>": 1,
        "<~>": 1,
        "=>": 2,
        "<=": 2,
        "|": 3,
        "&": 4,
    }
    RIGHT_ASSOCIATIVE = {"<=>", "<~>", "=>", "<="}

    def __init__(self, text: str) -> None:
        self.tokens = tokenize(text)
        self.position = 0

    def current(self) -> str | None:
        if self.position == len(self.tokens):
            return None
        return self.tokens[self.position]

    def consume(self, expected: str | None = None) -> str:
        token = self.current()
        if token is None:
            raise AdapterError("MALFORMED_INPUT", "unexpected end of input")
        if expected is not None and token != expected:
            raise AdapterError(
                "MALFORMED_INPUT",
                f"expected {expected!r}, found {token!r}",
            )
        self.position += 1
        return token

    def parse(self) -> ParsedDocument:
        dialect = self.consume()
        if dialect != "tff":
            raise AdapterError(
                "UNSUPPORTED_DIALECT",
                f"only tff is supported, found {dialect!r}",
            )
        self.consume("(")
        name = self.consume()
        self.consume(",")
        role = self.consume()
        if role not in {"axiom", "conjecture"}:
            raise AdapterError(
                "UNSUPPORTED_ROLE",
                f"unsupported TFF role {role!r}",
            )
        self.consume(",")
        formula = self.parse_formula(0)
        self.consume(")")
        self.consume(".")
        if self.current() is not None:
            raise AdapterError(
                "UNSUPPORTED_DOCUMENT",
                "the adapter accepts exactly one annotated formula",
            )
        return ParsedDocument(name, role, formula)

    def parse_formula(self, minimum_precedence: int) -> ParsedFormula:
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
            left = ParsedFormula("binary", operator, (left, right))
        return left

    def parse_formula_prefix(self) -> ParsedFormula:
        token = self.current()
        if token == "~":
            self.consume()
            return ParsedFormula("not", children=(self.parse_formula_prefix(),))
        if token in {"!", "?"}:
            return self.parse_quantifier()
        if token == "(":
            self.consume("(")
            formula = self.parse_formula(0)
            self.consume(")")
            return formula
        if token in {"$true", "$false"}:
            self.consume()
            return ParsedFormula("boolean", token)
        if token in {
            "$less",
            "$lesseq",
            "$greater",
            "$greatereq",
            "$is_int",
            "$is_rat",
        }:
            name = self.consume()
            arguments = self.parse_term_arguments()
            return ParsedFormula("predicate", name, terms=arguments)
        left = self.parse_term()
        relation = self.current()
        if relation not in {"=", "!="}:
            raise AdapterError(
                "MALFORMED_INPUT",
                f"expected arithmetic relation, found {relation!r}",
            )
        self.consume()
        right = self.parse_term()
        return ParsedFormula("equality", relation, terms=(left, right))

    def parse_quantifier(self) -> ParsedFormula:
        quantifier = self.consume()
        self.consume("[")
        binders = []
        binder_names = set()
        while True:
            name = self.consume()
            if not VARIABLE_RE.match(name):
                raise AdapterError(
                    "MALFORMED_INPUT",
                    f"invalid quantified variable {name!r}",
                )
            if name in binder_names:
                raise AdapterError(
                    "MALFORMED_INPUT",
                    f"duplicate quantified variable {name!r}",
                )
            binder_names.add(name)
            self.consume(":")
            sort = self.consume()
            if sort not in {"$int", "$rat", "$real"}:
                raise AdapterError(
                    "UNSUPPORTED_SORT",
                    f"unsupported quantified sort {sort!r}",
                )
            binders.append((name, sort))
            if self.current() != ",":
                break
            self.consume(",")
        self.consume("]")
        self.consume(":")
        body = self.parse_formula(0)
        return ParsedFormula("quantifier", quantifier, (body,), binders=tuple(binders))

    def parse_term_arguments(self) -> tuple[ParsedTerm, ...]:
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

    def parse_term(self) -> ParsedTerm:
        token = self.current()
        if token == "(":
            self.consume("(")
            term = self.parse_term()
            self.consume(")")
            return term
        if token is None:
            raise AdapterError("MALFORMED_INPUT", "expected a term")
        token = self.consume()
        if NUMBER_RE.match(token):
            return ParsedTerm("number", token)
        if VARIABLE_RE.match(token):
            return ParsedTerm("variable", token)
        if self.current() == "(":
            return ParsedTerm("call", token, self.parse_term_arguments())
        raise AdapterError(
            "MALFORMED_INPUT",
            f"unsupported term atom {token!r}",
        )


def parse_number(text: str) -> tuple[str, Fraction]:
    if "/" in text:
        return "$rat", Fraction(text)
    if "." in text or "e" in text.lower():
        return "$real", Fraction(Decimal(text))
    return "$int", Fraction(int(text))


def constant(value: Fraction | int) -> LTerm:
    number = Fraction(value)
    return ("constant", number.numerator, number.denominator)


def constant_value(term: LTerm) -> Fraction | None:
    if term[0] == "constant":
        return Fraction(term[1], term[2])
    return None


def scale(coefficient: Fraction | int, term: LTerm) -> LTerm:
    value = Fraction(coefficient)
    if value == 0:
        return constant(0)
    if value == 1:
        return term
    term_constant = constant_value(term)
    if term_constant is not None:
        return constant(value * term_constant)
    if term[0] == "scale":
        nested = Fraction(term[1], term[2])
        return scale(value * nested, term[3])
    return ("scale", value.numerator, value.denominator, term)


def split_scale(term: LTerm) -> tuple[Fraction, LTerm]:
    if term[0] == "scale":
        return Fraction(term[1], term[2]), term[3]
    return Fraction(1), term


def add(*terms: LTerm) -> LTerm:
    pending = list(terms)
    total = Fraction(0)
    coefficients: dict[LTerm, Fraction] = {}
    while pending:
        term = pending.pop()
        if term[0] == "add":
            pending.extend(term[1])
            continue
        term_constant = constant_value(term)
        if term_constant is not None:
            total += term_constant
            continue
        coefficient, base = split_scale(term)
        coefficients[base] = coefficients.get(base, Fraction(0)) + coefficient
    normalized = [
        scale(coefficient, base)
        for base, coefficient in coefficients.items()
        if coefficient
    ]
    if total:
        normalized.append(constant(total))
    normalized.sort(key=repr)
    if not normalized:
        return constant(0)
    if len(normalized) == 1:
        return normalized[0]
    return ("add", tuple(normalized))


def floor_term(term: LTerm) -> LTerm:
    value = constant_value(term)
    if value is not None:
        return constant(value.numerator // value.denominator)
    return ("floor", term)


def atom(relation: str, term: LTerm) -> LFormula:
    value = constant_value(term)
    if value is not None:
        outcomes = {
            "eq": value == 0,
            "ne": value != 0,
            "gt": value > 0,
            "ge": value >= 0,
        }
        return ("boolean", outcomes[relation])
    return ("atom", relation, term)


def connective(kind: str, *children: LFormula) -> LFormula:
    flattened = []
    for child in children:
        if child[0] == "boolean":
            if kind == "and" and not child[1]:
                return ("boolean", False)
            if kind == "or" and child[1]:
                return ("boolean", True)
            if (kind == "and" and child[1]) or (kind == "or" and not child[1]):
                continue
        if child[0] == kind:
            flattened.extend(child[1])
        else:
            flattened.append(child)
    unique = sorted(set(flattened), key=repr)
    if not unique:
        return ("boolean", kind == "and")
    if len(unique) == 1:
        return unique[0]
    return (kind, tuple(unique))


def complement(value: LFormula) -> LFormula:
    if value[0] == "boolean":
        return ("boolean", not value[1])
    if value[0] != "atom":
        raise AdapterError(
            "INTERNAL_ERROR",
            "complement called on a non-atomic formula",
        )
    relation, term = value[1], value[2]
    if relation == "eq":
        return atom("ne", term)
    if relation == "ne":
        return atom("eq", term)
    if relation == "gt":
        return atom("ge", scale(-1, term))
    if relation == "ge":
        return atom("gt", scale(-1, term))
    raise AdapterError("INTERNAL_ERROR", f"unknown relation {relation!r}")


class Translator:
    """Type-check and lower a parsed formula into canonical LIRA."""

    def __init__(self) -> None:
        self.bindings: dict[str, tuple[str, str]] = {}
        self.variable_counter = 0
        self.trace: list[dict[str, Any]] = []

    def translate(self, document: ParsedDocument) -> dict[str, Any]:
        formula = self.translate_formula(document.formula, False)
        body = {
            "schema_version": 1,
            "source_name": document.name,
            "source_role": document.role,
            "lira_formula": formula_to_json(formula),
            "reembedded_tff": render_document(
                document.name,
                document.role,
                formula,
            ),
            "trace": self.trace,
        }
        canonical_id = hashlib.sha256(canonical_json(body)).hexdigest()
        return {**body, "canonical_id": canonical_id}

    def translate_formula(
        self,
        formula: ParsedFormula,
        negated: bool,
    ) -> LFormula:
        if formula.kind == "boolean":
            value = formula.value == "$true"
            return ("boolean", not value if negated else value)
        if formula.kind == "not":
            return self.translate_formula(formula.children[0], not negated)
        if formula.kind == "binary":
            left, right = formula.children
            operator = formula.value
            if operator == "&":
                kind = "or" if negated else "and"
                return connective(
                    kind,
                    self.translate_formula(left, negated),
                    self.translate_formula(right, negated),
                )
            if operator == "|":
                kind = "and" if negated else "or"
                return connective(
                    kind,
                    self.translate_formula(left, negated),
                    self.translate_formula(right, negated),
                )
            if operator == "=>":
                if negated:
                    return connective(
                        "and",
                        self.translate_formula(left, False),
                        self.translate_formula(right, True),
                    )
                return connective(
                    "or",
                    self.translate_formula(left, True),
                    self.translate_formula(right, False),
                )
            if operator == "<=":
                reverse = ParsedFormula("binary", "=>", (right, left))
                return self.translate_formula(reverse, negated)
            if operator in {"<=>", "<~>"}:
                xor = negated ^ (operator == "<~>")
                if xor:
                    return connective(
                        "or",
                        connective(
                            "and",
                            self.translate_formula(left, False),
                            self.translate_formula(right, True),
                        ),
                        connective(
                            "and",
                            self.translate_formula(left, True),
                            self.translate_formula(right, False),
                        ),
                    )
                return connective(
                    "or",
                    connective(
                        "and",
                        self.translate_formula(left, False),
                        self.translate_formula(right, False),
                    ),
                    connective(
                        "and",
                        self.translate_formula(left, True),
                        self.translate_formula(right, True),
                    ),
                )
            raise AdapterError(
                "UNSUPPORTED_CONNECTIVE",
                f"unsupported connective {operator!r}",
            )
        if formula.kind == "quantifier":
            return self.translate_quantifier(formula, negated)
        translated = self.translate_atom(formula)
        return complement(translated) if negated else translated

    def translate_quantifier(
        self,
        formula: ParsedFormula,
        negated: bool,
    ) -> LFormula:
        quantifier = formula.value
        if negated:
            quantifier = "?" if quantifier == "!" else "!"
        saved: dict[str, tuple[str, str] | None] = {}
        lowered = []
        for name, sort in formula.binders:
            if sort == "$rat":
                raise AdapterError(
                    "UNSUPPORTED_RAT_QUANTIFIER",
                    "quantified rationals are not representable in LIRA",
                )
            self.variable_counter += 1
            target = f"LIRA_V{self.variable_counter}"
            saved[name] = self.bindings.get(name)
            self.bindings[name] = (target, sort)
            lowering = "integrality_guard" if sort == "$int" else "direct"
            lowered.append((target, sort))
            self.trace.append(
                {
                    "kind": "binder",
                    "source_name": name,
                    "source_sort": sort,
                    "target_name": target,
                    "target_sort": "$real",
                    "lowering": lowering,
                }
            )
        result = self.translate_formula(formula.children[0], negated)
        for target, sort in reversed(lowered):
            variable = ("variable", target)
            if sort == "$int":
                guard = atom("eq", add(variable, scale(-1, floor_term(variable))))
                if quantifier == "?":
                    result = connective("and", guard, result)
                else:
                    result = connective("or", complement(guard), result)
            result = (
                "exists" if quantifier == "?" else "forall",
                target,
                result,
            )
        for name, prior in saved.items():
            if prior is None:
                del self.bindings[name]
            else:
                self.bindings[name] = prior
        return result

    def translate_atom(self, formula: ParsedFormula) -> LFormula:
        if formula.kind == "equality":
            left_sort, left = self.translate_term(formula.terms[0])
            right_sort, right = self.translate_term(formula.terms[1])
            self.require_same_sort(left_sort, right_sort, formula.value)
            relation = "eq" if formula.value == "=" else "ne"
            self.record("relation", source=formula.value, target=relation)
            return atom(relation, add(left, scale(-1, right)))
        if formula.kind != "predicate":
            raise AdapterError(
                "MALFORMED_INPUT",
                f"unsupported formula node {formula.kind!r}",
            )
        name = formula.value
        expected_arity = 1 if name in {"$is_int", "$is_rat"} else 2
        if len(formula.terms) != expected_arity:
            raise AdapterError(
                "ARITY_MISMATCH",
                f"{name} expects {expected_arity} arguments",
            )
        if name in {"$is_int", "$is_rat"}:
            sort, term = self.translate_term(formula.terms[0])
            if name == "$is_rat":
                if sort == "$real":
                    raise AdapterError(
                        "UNSUPPORTED_REAL_RATIONALITY",
                        "$is_rat on a real is outside LIRA",
                    )
                self.record("predicate", source=name, target="true")
                return ("boolean", True)
            if sort == "$int":
                self.record("predicate", source=name, target="true")
                return ("boolean", True)
            self.record("predicate", source=name, target="X = floor(X)")
            return atom("eq", add(term, scale(-1, floor_term(term))))
        left_sort, left = self.translate_term(formula.terms[0])
        right_sort, right = self.translate_term(formula.terms[1])
        self.require_same_sort(left_sort, right_sort, name)
        difference = add(left, scale(-1, right))
        if name == "$greater":
            self.record("relation", source=name, target="gt(lhs-rhs,0)")
            return atom("gt", difference)
        if name == "$greatereq":
            self.record("relation", source=name, target="ge(lhs-rhs,0)")
            return atom("ge", difference)
        if name == "$less":
            self.record("relation", source=name, target="gt(rhs-lhs,0)")
            return atom("gt", scale(-1, difference))
        if name == "$lesseq":
            self.record("relation", source=name, target="ge(rhs-lhs,0)")
            return atom("ge", scale(-1, difference))
        raise AdapterError(
            "UNSUPPORTED_OPERATOR",
            f"unsupported predicate {name!r}",
        )

    def translate_term(self, term: ParsedTerm) -> tuple[str, LTerm]:
        if term.kind == "number":
            sort, value = parse_number(term.value)
            return sort, constant(value)
        if term.kind == "variable":
            binding = self.bindings.get(term.value)
            if binding is None:
                raise AdapterError(
                    "UNBOUND_VARIABLE",
                    f"unbound variable {term.value!r}",
                )
            target, sort = binding
            return sort, ("variable", target)
        if term.kind != "call":
            raise AdapterError(
                "MALFORMED_INPUT",
                f"unsupported term node {term.kind!r}",
            )
        name = term.value
        if not name.startswith("$"):
            raise AdapterError(
                "UNINTERPRETED_ARITHMETIC",
                f"arithmetic-valued uninterpreted function {name!r}",
            )
        if name in DEFINED_UNSUPPORTED:
            code = (
                "UNSUPPORTED_ROUNDING"
                if name in {"$truncate", "$round"}
                else "UNSUPPORTED_OPERATOR"
            )
            raise AdapterError(code, f"unsupported operator {name!r}")
        if name in {"$uminus", "$floor", "$ceiling", "$to_int", "$to_rat", "$to_real"}:
            if len(term.arguments) != 1:
                raise AdapterError(
                    "ARITY_MISMATCH",
                    f"{name} expects one argument",
                )
            source_sort, value = self.translate_term(term.arguments[0])
            if name == "$uminus":
                self.record("term", source=name, target="scale(-1)")
                return source_sort, scale(-1, value)
            if name == "$floor":
                self.record(
                    "term",
                    source=name,
                    target="identity" if source_sort == "$int" else "floor",
                )
                return (
                    source_sort,
                    value if source_sort == "$int" else floor_term(value),
                )
            if name == "$ceiling":
                self.record(
                    "term",
                    source=name,
                    target=(
                        "identity"
                        if source_sort == "$int"
                        else "scale(-1,floor(scale(-1,X)))"
                    ),
                )
                if source_sort == "$int":
                    return source_sort, value
                return source_sort, scale(-1, floor_term(scale(-1, value)))
            if name == "$to_int":
                self.record(
                    "coercion",
                    source=f"{source_sort}->$int",
                    target="identity" if source_sort == "$int" else "floor",
                )
                return (
                    "$int",
                    value if source_sort == "$int" else floor_term(value),
                )
            if name == "$to_rat":
                if source_sort == "$real":
                    raise AdapterError(
                        "UNSUPPORTED_REAL_TO_RAT",
                        "$to_rat from real is underspecified outside a guard",
                    )
                self.record(
                    "coercion",
                    source=f"{source_sort}->$rat",
                    target="value_embedding",
                )
                return "$rat", value
            self.record(
                "coercion",
                source=f"{source_sort}->$real",
                target="value_embedding",
            )
            return "$real", value
        if name in {"$sum", "$difference", "$product", "$quotient"}:
            if len(term.arguments) != 2:
                raise AdapterError(
                    "ARITY_MISMATCH",
                    f"{name} expects two arguments",
                )
            left_sort, left = self.translate_term(term.arguments[0])
            right_sort, right = self.translate_term(term.arguments[1])
            self.require_same_sort(left_sort, right_sort, name)
            if name == "$sum":
                self.record("term", source=name, target="add")
                return left_sort, add(left, right)
            if name == "$difference":
                self.record("term", source=name, target="add(lhs,scale(-1,rhs))")
                return left_sort, add(left, scale(-1, right))
            if name == "$product":
                left_constant = constant_value(left)
                right_constant = constant_value(right)
                if left_constant is not None:
                    self.record("term", source=name, target="scale(left,rhs)")
                    return left_sort, scale(left_constant, right)
                if right_constant is not None:
                    self.record("term", source=name, target="scale(right,lhs)")
                    return left_sort, scale(right_constant, left)
                raise AdapterError(
                    "NONLINEAR_PRODUCT",
                    "product requires a compile-time rational factor",
                )
            divisor = constant_value(right)
            if divisor is None:
                raise AdapterError(
                    "NONCONSTANT_DIVISOR",
                    "quotient requires a compile-time rational divisor",
                )
            if divisor == 0:
                raise AdapterError(
                    "ZERO_DIVISOR",
                    "division by zero is unspecified in TPTP arithmetic",
                )
            result_sort = "$rat" if left_sort == "$int" else left_sort
            self.record(
                "term",
                source=name,
                target="scale(reciprocal(divisor),numerator)",
            )
            return result_sort, scale(Fraction(1, 1) / divisor, left)
        if name.startswith("$quotient_") or name.startswith("$remainder_"):
            raise AdapterError(
                "UNSUPPORTED_OPERATOR",
                f"unsupported operator {name!r}",
            )
        raise AdapterError(
            "UNSUPPORTED_OPERATOR",
            f"unsupported defined function {name!r}",
        )

    @staticmethod
    def require_same_sort(left: str, right: str, context: str) -> None:
        if left != right:
            raise AdapterError(
                "TYPE_MISMATCH",
                f"{context} requires matching sorts, found {left} and {right}",
            )

    def record(self, kind: str, *, source: str, target: str) -> None:
        self.trace.append(
            {
                "kind": kind,
                "source": source,
                "target": target,
            }
        )


def term_to_json(term: LTerm) -> dict[str, Any]:
    if term[0] == "constant":
        return {
            "kind": "constant",
            "numerator": term[1],
            "denominator": term[2],
        }
    if term[0] == "variable":
        return {"kind": "variable", "name": term[1]}
    if term[0] == "scale":
        return {
            "kind": "scale",
            "numerator": term[1],
            "denominator": term[2],
            "term": term_to_json(term[3]),
        }
    if term[0] == "add":
        return {
            "kind": "add",
            "terms": [term_to_json(child) for child in term[1]],
        }
    if term[0] == "floor":
        return {"kind": "floor", "term": term_to_json(term[1])}
    raise AdapterError("INTERNAL_ERROR", f"unknown LIRA term {term[0]!r}")


def formula_to_json(formula: LFormula) -> dict[str, Any]:
    if formula[0] == "boolean":
        return {"kind": "boolean", "value": formula[1]}
    if formula[0] == "atom":
        return {
            "kind": "atom",
            "relation": formula[1],
            "term": term_to_json(formula[2]),
        }
    if formula[0] in {"and", "or"}:
        return {
            "kind": formula[0],
            "children": [formula_to_json(child) for child in formula[1]],
        }
    if formula[0] in {"exists", "forall"}:
        return {
            "kind": formula[0],
            "variable": formula[1],
            "body": formula_to_json(formula[2]),
        }
    raise AdapterError("INTERNAL_ERROR", f"unknown LIRA formula {formula[0]!r}")


def render_constant(numerator: int, denominator: int) -> str:
    value = str(numerator) if denominator == 1 else f"{numerator}/{denominator}"
    return f"$to_real({value})"


def render_term(term: LTerm) -> str:
    if term[0] == "constant":
        return render_constant(term[1], term[2])
    if term[0] == "variable":
        return term[1]
    if term[0] == "scale":
        coefficient = render_constant(term[1], term[2])
        return f"$product({coefficient},{render_term(term[3])})"
    if term[0] == "add":
        rendered = [render_term(child) for child in term[1]]
        result = rendered[-1]
        for child in reversed(rendered[:-1]):
            result = f"$sum({child},{result})"
        return result
    if term[0] == "floor":
        return f"$floor({render_term(term[1])})"
    raise AdapterError("INTERNAL_ERROR", f"unknown LIRA term {term[0]!r}")


def render_formula(formula: LFormula) -> str:
    if formula[0] == "boolean":
        return "$true" if formula[1] else "$false"
    if formula[0] == "atom":
        relation = formula[1]
        term = render_term(formula[2])
        zero = render_constant(0, 1)
        if relation == "eq":
            return f"({term} = {zero})"
        if relation == "ne":
            return f"({term} != {zero})"
        predicate = "$greater" if relation == "gt" else "$greatereq"
        return f"{predicate}({term},{zero})"
    if formula[0] in {"and", "or"}:
        operator = " & " if formula[0] == "and" else " | "
        return "(" + operator.join(render_formula(child) for child in formula[1]) + ")"
    if formula[0] in {"exists", "forall"}:
        quantifier = "?" if formula[0] == "exists" else "!"
        return (
            f"{quantifier} [{formula[1]}:$real] : "
            f"({render_formula(formula[2])})"
        )
    raise AdapterError("INTERNAL_ERROR", f"unknown LIRA formula {formula[0]!r}")


def render_document(name: str, role: str, formula: LFormula) -> str:
    safe_name = re.sub(r"[^A-Za-z0-9_]", "_", name)
    return (
        f"tff(umlaut_lira_{safe_name},{role},\n"
        f"    {render_formula(formula)} ).\n"
    )


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def adapt(text: str) -> dict[str, Any]:
    return Translator().translate(Parser(text).parse())


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        result = adapt(arguments.input.read_text(encoding="utf-8"))
    except (AdapterError, OSError, ValueError) as error:
        code = error.code if isinstance(error, AdapterError) else "IO_ERROR"
        print(f"{code}: {error}")
        return 2
    payload = canonical_json(result) + b"\n"
    if arguments.output is None:
        print(payload.decode("utf-8"), end="")
    else:
        arguments.output.write_bytes(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
