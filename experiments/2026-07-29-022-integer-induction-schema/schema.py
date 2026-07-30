#!/usr/bin/env python3
"""Generate one narrowly validated lower-bounded integer-induction axiom."""

from __future__ import annotations

import hashlib
import importlib.util
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
ADAPTER_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "proof_adapter.py"
)
QUANTIFIERS = {"!", "?", "^", "@"}
INTEGER_RE = re.compile(r"^[0-9]+$")
VARIABLE_RE = re.compile(r"^[A-Z][A-Za-z0-9_]*$")
SAFE_NAME_RE = re.compile(r"[^a-zA-Z0-9_]")
ARITHMETIC_DECLARATIONS = (
    (
        "$sum",
        "tff(umlaut_arith_sum_type,type,"
        "$sum:( $int * $int ) > $int).",
    ),
    (
        "$difference",
        "tff(umlaut_arith_difference_type,type,"
        "$difference:( $int * $int ) > $int).",
    ),
    (
        "$product",
        "tff(umlaut_arith_product_type,type,"
        "$product:( $int * $int ) > $int).",
    ),
    (
        "$greatereq",
        "tff(umlaut_arith_greatereq_type,type,"
        "$greatereq:( $int * $int ) > $o).",
    ),
    (
        "$lesseq",
        "tff(umlaut_arith_lesseq_type,type,"
        "$lesseq:( $int * $int ) > $o).",
    ),
)


class SchemaError(RuntimeError):
    """The problem is outside the restricted induction contract."""


@dataclass(frozen=True)
class InductionTarget:
    """One normalized lower-bounded integer conjecture."""

    conjecture_name: str
    variable: str
    bound_tokens: tuple[str, ...]
    property_tokens: tuple[str, ...]
    source_form: str

    @property
    def bound(self) -> str:
        return tokens_to_text(self.bound_tokens)

    @property
    def property(self) -> str:
        return tokens_to_text(self.property_tokens)


@dataclass(frozen=True)
class GeneratedSchema:
    """A generated schema and its mechanically checked identity."""

    name: str
    statement: str
    schema_id: str
    target: InductionTarget


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SchemaError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ADAPTER = load_module("integer_induction_proof_adapter", ADAPTER_PATH)


def strip_outer_parentheses(tokens: Sequence[str]) -> list[str]:
    result = list(tokens)
    while len(result) >= 2 and result[0] == "(" and result[-1] == ")":
        depth = 0
        encloses_all = True
        for index, token in enumerate(result):
            if token == "(":
                depth += 1
            elif token == ")":
                depth -= 1
                if depth == 0 and index != len(result) - 1:
                    encloses_all = False
                    break
        if not encloses_all or depth != 0:
            break
        result = result[1:-1]
    return result


def split_top_level(tokens: Sequence[str], delimiter: str) -> list[list[str]]:
    fields: list[list[str]] = []
    current: list[str] = []
    depth = 0
    for token in tokens:
        if token in {"(", "[", "{"}:
            depth += 1
        elif token in {")", "]", "}"}:
            depth -= 1
            if depth < 0:
                raise SchemaError("unbalanced formula tokens")
        if token == delimiter and depth == 0:
            fields.append(strip_outer_parentheses(current))
            current = []
        else:
            current.append(token)
    if depth != 0:
        raise SchemaError("unbalanced formula tokens")
    fields.append(strip_outer_parentheses(current))
    return fields


def parse_quantifier(
    tokens: Sequence[str], marker: str
) -> tuple[str, list[str]] | None:
    candidate = strip_outer_parentheses(tokens)
    if not candidate or candidate[0] != marker:
        return None
    expected = [marker, "[", None, ":", "$int", "]", ":"]
    if len(candidate) < len(expected):
        raise SchemaError("truncated integer quantifier")
    if (
        candidate[1] != "["
        or not VARIABLE_RE.fullmatch(candidate[2])
        or candidate[3:7] != [":", "$int", "]", ":"]
    ):
        raise SchemaError("quantifier is not one monomorphic integer binder")
    return candidate[2], strip_outer_parentheses(candidate[7:])


def integer_literal(tokens: Sequence[str]) -> tuple[str, ...] | None:
    candidate = strip_outer_parentheses(tokens)
    if len(candidate) == 1 and INTEGER_RE.fullmatch(candidate[0]):
        return (candidate[0],)
    if (
        len(candidate) == 2
        and candidate[0] == "-"
        and INTEGER_RE.fullmatch(candidate[1])
    ):
        return ("-", candidate[1])
    return None


def parse_lower_guard(
    tokens: Sequence[str], variable: str
) -> tuple[str, ...] | None:
    candidate = strip_outer_parentheses(tokens)
    if len(candidate) < 6:
        return None
    if candidate[1] != "(" or candidate[-1] != ")":
        return None
    depth = 0
    for index, token in enumerate(candidate[1:], start=1):
        if token == "(":
            depth += 1
        elif token == ")":
            depth -= 1
            if depth == 0 and index != len(candidate) - 1:
                return None
        if depth < 0:
            return None
    if depth != 0:
        return None
    arguments = split_top_level(candidate[2:-1], ",")
    if len(arguments) != 2:
        return None
    if candidate[0] == "$greatereq" and arguments[0] == [variable]:
        return integer_literal(arguments[1])
    if candidate[0] == "$lesseq" and arguments[1] == [variable]:
        return integer_literal(arguments[0])
    return None


def joined_conjunction(parts: Sequence[Sequence[str]]) -> list[str]:
    if not parts:
        raise SchemaError("empty conjunction")
    result: list[str] = []
    for index, part in enumerate(parts):
        if index:
            result.append("&")
        result.extend(["(", *strip_outer_parentheses(part), ")"])
    return strip_outer_parentheses(result)


def validate_property(tokens: Sequence[str], variable: str) -> tuple[str, ...]:
    property_tokens = tuple(strip_outer_parentheses(tokens))
    if not property_tokens:
        raise SchemaError("empty induction property")
    if any(token in QUANTIFIERS for token in property_tokens):
        raise SchemaError("induction property contains a nested quantifier")
    if variable not in property_tokens:
        raise SchemaError("bound variable does not occur in induction property")
    return property_tokens


def target_from_body(name: str, body: str) -> InductionTarget:
    tokens = ADAPTER.tokenize_formula(body)
    direct = parse_quantifier(tokens, "!")
    if direct is not None:
        variable, quantified_body = direct
        implication = split_top_level(quantified_body, "=>")
        if len(implication) != 2:
            raise SchemaError("universal target is not one lower-bound implication")
        bound = parse_lower_guard(implication[0], variable)
        if bound is None:
            raise SchemaError("universal target has no supported lower-bound guard")
        return InductionTarget(
            conjecture_name=name,
            variable=variable,
            bound_tokens=bound,
            property_tokens=validate_property(implication[1], variable),
            source_form="universal_implication",
        )

    candidate = strip_outer_parentheses(tokens)
    if not candidate or candidate[0] != "~":
        raise SchemaError("conjecture is neither supported universal nor negated existential")
    existential = parse_quantifier(strip_outer_parentheses(candidate[1:]), "?")
    if existential is None:
        raise SchemaError("negated target is not one existential integer binder")
    variable, quantified_body = existential
    conjuncts = split_top_level(quantified_body, "&")
    guards = [
        (index, parse_lower_guard(conjunct, variable))
        for index, conjunct in enumerate(conjuncts)
    ]
    guards = [(index, bound) for index, bound in guards if bound is not None]
    if len(guards) != 1:
        raise SchemaError("negated existential needs exactly one lower-bound guard")
    guard_index, bound = guards[0]
    violation = joined_conjunction(
        [part for index, part in enumerate(conjuncts) if index != guard_index]
    )
    property_tokens = ["~", "(", *violation, ")"]
    return InductionTarget(
        conjecture_name=name,
        variable=variable,
        bound_tokens=bound,
        property_tokens=validate_property(property_tokens, variable),
        source_form="negated_existential",
    )


def conjectures(problem_text: str) -> list[object]:
    parsed = []
    for statement in ADAPTER.split_tptp_statements(problem_text):
        formula = ADAPTER.parse_annotated(statement)
        if formula is not None and formula.role == "conjecture":
            parsed.append(formula)
    return parsed


def extract_target(problem_text: str) -> InductionTarget:
    try:
        targets = conjectures(problem_text)
    except ADAPTER.AdapterError as error:
        raise SchemaError(f"cannot parse TPTP source: {error}") from error
    if len(targets) != 1:
        raise SchemaError(f"expected one conjecture, found {len(targets)}")
    formula = targets[0]
    if formula.kind != "tff":
        raise SchemaError(f"expected a TFF conjecture, found {formula.kind}")
    return target_from_body(formula.name, formula.body)


def substitute(
    tokens: Sequence[str], variable: str, replacement: Sequence[str]
) -> list[str]:
    result: list[str] = []
    for token in tokens:
        if token == variable:
            result.extend(replacement)
        else:
            result.append(token)
    return result


def fresh_variable(target: InductionTarget) -> str:
    occupied = set(target.property_tokens) | {target.variable}
    stem = "UMLAUT_IND_N"
    candidate = stem
    suffix = 0
    while candidate in occupied:
        suffix += 1
        candidate = f"{stem}_{suffix}"
    return candidate


def expected_schema_tokens(target: InductionTarget) -> list[str]:
    variable = fresh_variable(target)
    bound = list(target.bound_tokens)
    property_at_bound = substitute(
        target.property_tokens, target.variable, bound
    )
    property_at_variable = substitute(
        target.property_tokens, target.variable, [variable]
    )
    successor = ["$sum", "(", variable, ",", "1", ")"]
    property_at_successor = substitute(
        target.property_tokens, target.variable, successor
    )
    guard = ["$greatereq", "(", variable, ",", *bound, ")"]
    return [
        "(",
        "(",
        "(",
        *property_at_bound,
        ")",
        "&",
        "!",
        "[",
        variable,
        ":",
        "$int",
        "]",
        ":",
        "(",
        "(",
        *guard,
        "&",
        "(",
        *property_at_variable,
        ")",
        ")",
        "=>",
        "(",
        *property_at_successor,
        ")",
        ")",
        ")",
        "=>",
        "!",
        "[",
        variable,
        ":",
        "$int",
        "]",
        ":",
        "(",
        *guard,
        "=>",
        "(",
        *property_at_variable,
        ")",
        ")",
        ")",
    ]


def tokens_to_text(tokens: Sequence[str]) -> str:
    return " ".join(tokens)


def schema_name(target: InductionTarget) -> str:
    safe = SAFE_NAME_RE.sub("_", target.conjecture_name).strip("_").lower()
    digest = hashlib.sha256(
        "\0".join(
            [
                target.conjecture_name,
                target.variable,
                target.bound,
                target.property,
            ]
        ).encode("utf-8")
    ).hexdigest()[:12]
    return f"umlaut_integer_induction_{safe}_{digest}"


def generate_schema(problem_text: str) -> GeneratedSchema:
    target = extract_target(problem_text)
    name = schema_name(target)
    body_tokens = expected_schema_tokens(target)
    statement = f"tff({name},axiom,\n    {tokens_to_text(body_tokens)} )."
    try:
        parsed = ADAPTER.parse_annotated(statement)
    except ADAPTER.AdapterError as error:
        raise SchemaError(f"generated schema is unparsable: {error}") from error
    if parsed is None or parsed.kind != "tff" or parsed.role != "axiom":
        raise SchemaError("generated schema failed annotated-formula reconstruction")
    actual_tokens = ADAPTER.tokenize_formula(parsed.body)
    expected_tokens = strip_outer_parentheses(body_tokens)
    if actual_tokens != expected_tokens:
        raise SchemaError("generated schema changed during token reconstruction")
    schema_id = hashlib.sha256(
        "\0".join(actual_tokens).encode("utf-8")
    ).hexdigest()
    return GeneratedSchema(name, statement, schema_id, target)


def explicitly_declared(problem_text: str) -> set[str]:
    result = set()
    for statement in ADAPTER.split_tptp_statements(problem_text):
        formula = ADAPTER.parse_annotated(statement)
        if formula is None or formula.kind != "tff" or formula.role != "type":
            continue
        body_tokens = ADAPTER.tokenize_formula(formula.body)
        for symbol, _ in ARITHMETIC_DECLARATIONS:
            if body_tokens[:2] == [symbol, ":"]:
                result.add(symbol)
    return result


def prepare_problem(problem_text: str) -> str:
    """Add redundant standard integer symbol types to either treatment."""

    if "$rat" in problem_text or "$real" in problem_text:
        raise SchemaError("mixed rational/real arithmetic is outside the prototype")
    try:
        declared = explicitly_declared(problem_text)
    except ADAPTER.AdapterError as error:
        raise SchemaError(f"cannot inspect arithmetic declarations: {error}") from error
    declarations = [
        declaration
        for symbol, declaration in ARITHMETIC_DECLARATIONS
        if symbol not in declared
    ]
    if not declarations:
        return problem_text
    return (
        "% Redundant standard integer types added for Umlaut parsing.\n"
        + "\n".join(declarations)
        + "\n\n"
        + problem_text
    )


def augment_problem(problem_text: str) -> tuple[str, GeneratedSchema]:
    schema = generate_schema(problem_text)
    prepared = prepare_problem(problem_text)
    augmented = (
        prepared.rstrip()
        + "\n\n% Added by the restricted Umlaut integer-induction prototype.\n"
        + schema.statement
        + "\n"
    )
    try:
        augmented_conjectures = conjectures(augmented)
    except ADAPTER.AdapterError as error:
        raise SchemaError(f"augmented problem is unparsable: {error}") from error
    if len(augmented_conjectures) != 1:
        raise SchemaError("augmentation changed the conjecture count")
    generated = [
        ADAPTER.parse_annotated(statement)
        for statement in ADAPTER.split_tptp_statements(augmented)
    ]
    matching = [
        formula
        for formula in generated
        if formula is not None and formula.name == schema.name
    ]
    if len(matching) != 1:
        raise SchemaError("augmented problem does not contain exactly one schema")
    return augmented, schema
