#!/usr/bin/env python3
"""Independently recognize one generated lower-bounded induction schema."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
ADAPTER_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "proof_adapter.py"
)
VARIABLE_RE = re.compile(r"^[A-Z][A-Za-z0-9_]*$")
INTEGER_RE = re.compile(r"^[0-9]+$")


class VerificationError(RuntimeError):
    """The augmented problem is not the claimed induction transformation."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise VerificationError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ADAPTER = load_module("integer_induction_verifier_adapter", ADAPTER_PATH)


def strip_parentheses(tokens: Sequence[str]) -> list[str]:
    result = list(tokens)
    while len(result) >= 2 and result[0] == "(" and result[-1] == ")":
        depth = 0
        closes_early = False
        for index, token in enumerate(result):
            if token == "(":
                depth += 1
            elif token == ")":
                depth -= 1
                if depth == 0 and index != len(result) - 1:
                    closes_early = True
                    break
        if closes_early or depth != 0:
            break
        result = result[1:-1]
    return result


def split_top(tokens: Sequence[str], delimiter: str) -> list[list[str]]:
    result: list[list[str]] = []
    current: list[str] = []
    depth = 0
    for token in tokens:
        if token in {"(", "[", "{"}:
            depth += 1
        elif token in {")", "]", "}"}:
            depth -= 1
        if depth < 0:
            raise VerificationError("unbalanced formula")
        if token == delimiter and depth == 0:
            result.append(strip_parentheses(current))
            current = []
        else:
            current.append(token)
    if depth != 0:
        raise VerificationError("unbalanced formula")
    result.append(strip_parentheses(current))
    return result


def parse_integer(tokens: Sequence[str]) -> tuple[str, ...] | None:
    value = strip_parentheses(tokens)
    if len(value) == 1 and INTEGER_RE.fullmatch(value[0]):
        return (value[0],)
    if (
        len(value) == 2
        and value[0] == "-"
        and INTEGER_RE.fullmatch(value[1])
    ):
        return ("-", value[1])
    return None


def parse_quantifier(
    tokens: Sequence[str], marker: str
) -> tuple[str, list[str]] | None:
    value = strip_parentheses(tokens)
    if not value or value[0] != marker:
        return None
    if (
        len(value) < 8
        or value[1] != "["
        or not VARIABLE_RE.fullmatch(value[2])
        or value[3:7] != [":", "$int", "]", ":"]
    ):
        raise VerificationError("unsupported quantified formula")
    return value[2], strip_parentheses(value[7:])


def lower_guard(
    tokens: Sequence[str], variable: str
) -> tuple[str, ...] | None:
    value = strip_parentheses(tokens)
    if len(value) < 6 or value[1] != "(" or value[-1] != ")":
        return None
    depth = 0
    for index, token in enumerate(value[1:], start=1):
        if token == "(":
            depth += 1
        elif token == ")":
            depth -= 1
            if depth == 0 and index != len(value) - 1:
                return None
        if depth < 0:
            return None
    if depth != 0:
        return None
    arguments = split_top(value[2:-1], ",")
    if len(arguments) != 2:
        return None
    if value[0] == "$greatereq" and arguments[0] == [variable]:
        return parse_integer(arguments[1])
    if value[0] == "$lesseq" and arguments[1] == [variable]:
        return parse_integer(arguments[0])
    return None


def replace_token(
    tokens: Sequence[str], old: str, replacement: Sequence[str]
) -> list[str]:
    result: list[str] = []
    for token in tokens:
        result.extend(replacement if token == old else [token])
    return result


def normalized_source_property(problem_text: str) -> tuple[tuple[str, ...], list[str]]:
    conjectures = []
    for statement in ADAPTER.split_tptp_statements(problem_text):
        formula = ADAPTER.parse_annotated(statement)
        if formula is not None and formula.role == "conjecture":
            conjectures.append(formula)
    if len(conjectures) != 1 or conjectures[0].kind != "tff":
        raise VerificationError("source does not have one TFF conjecture")
    tokens = ADAPTER.tokenize_formula(conjectures[0].body)
    direct = parse_quantifier(tokens, "!")
    if direct is not None:
        variable, body = direct
        implication = split_top(body, "=>")
        if len(implication) != 2:
            raise VerificationError("source universal is not lower bounded")
        bound = lower_guard(implication[0], variable)
        if bound is None:
            raise VerificationError("source universal has no literal lower bound")
        return bound, replace_token(implication[1], variable, ["$IND"])

    value = strip_parentheses(tokens)
    if not value or value[0] != "~":
        raise VerificationError("source has unsupported conjecture shape")
    existential = parse_quantifier(strip_parentheses(value[1:]), "?")
    if existential is None:
        raise VerificationError("source negation is not existential")
    variable, body = existential
    conjuncts = split_top(body, "&")
    guards = [
        (index, lower_guard(part, variable))
        for index, part in enumerate(conjuncts)
    ]
    guards = [(index, bound) for index, bound in guards if bound is not None]
    if len(guards) != 1:
        raise VerificationError("source existential has ambiguous lower guard")
    guard_index, bound = guards[0]
    remaining = [
        part for index, part in enumerate(conjuncts) if index != guard_index
    ]
    if not remaining:
        raise VerificationError("source existential has no violation")
    violation: list[str] = []
    for index, part in enumerate(remaining):
        if index:
            violation.append("&")
        violation.extend(["(", *part, ")"])
    property_tokens = ["~", "(", *strip_parentheses(violation), ")"]
    return bound, replace_token(property_tokens, variable, ["$IND"])


def schema_formula(augmented_text: str) -> object:
    formulas = []
    for statement in ADAPTER.split_tptp_statements(augmented_text):
        formula = ADAPTER.parse_annotated(statement)
        if (
            formula is not None
            and formula.name.startswith("umlaut_integer_induction_")
        ):
            formulas.append(formula)
    if len(formulas) != 1:
        raise VerificationError(
            f"expected one generated schema, found {len(formulas)}"
        )
    formula = formulas[0]
    if formula.kind != "tff" or formula.role != "axiom":
        raise VerificationError("generated schema is not a TFF axiom")
    return formula


def verify_structure(
    source_text: str, augmented_text: str
) -> dict[str, Any]:
    formula = schema_formula(augmented_text)
    root = split_top(ADAPTER.tokenize_formula(formula.body), "=>")
    if len(root) != 2:
        raise VerificationError("schema root is not one implication")
    antecedent, conclusion = root

    conclusion_quantifier = parse_quantifier(conclusion, "!")
    if conclusion_quantifier is None:
        raise VerificationError("schema conclusion is not universally quantified")
    variable, conclusion_body = conclusion_quantifier
    conclusion_implication = split_top(conclusion_body, "=>")
    if len(conclusion_implication) != 2:
        raise VerificationError("schema conclusion is not lower bounded")
    bound = lower_guard(conclusion_implication[0], variable)
    if bound is None:
        raise VerificationError("schema conclusion has no literal lower bound")
    property_at_variable = strip_parentheses(conclusion_implication[1])
    abstract_property = replace_token(
        property_at_variable, variable, ["$IND"]
    )

    source_bound, source_property = normalized_source_property(source_text)
    if bound != source_bound or abstract_property != source_property:
        raise VerificationError("schema conclusion differs from source conjecture")

    antecedent_parts = split_top(antecedent, "&")
    if len(antecedent_parts) != 2:
        raise VerificationError("schema antecedent is not base and step")
    expected_base = replace_token(
        abstract_property, "$IND", list(bound)
    )
    if strip_parentheses(antecedent_parts[0]) != strip_parentheses(expected_base):
        raise VerificationError("schema base is not the property at the bound")

    step_quantifier = parse_quantifier(antecedent_parts[1], "!")
    if step_quantifier is None:
        raise VerificationError("schema step is not universally quantified")
    step_variable, step_body = step_quantifier
    step_implication = split_top(step_body, "=>")
    if len(step_implication) != 2:
        raise VerificationError("schema step is not one implication")
    step_premises = split_top(step_implication[0], "&")
    if len(step_premises) != 2:
        raise VerificationError("schema step lacks guard or hypothesis")
    step_bound = lower_guard(step_premises[0], step_variable)
    if step_bound != bound:
        raise VerificationError("schema step bound differs from conclusion")
    expected_current = replace_token(
        abstract_property, "$IND", [step_variable]
    )
    if strip_parentheses(step_premises[1]) != strip_parentheses(expected_current):
        raise VerificationError("schema step hypothesis is not P(N)")
    successor = ["$sum", "(", step_variable, ",", "1", ")"]
    expected_successor = replace_token(
        abstract_property, "$IND", successor
    )
    if strip_parentheses(step_implication[1]) != strip_parentheses(
        expected_successor
    ):
        raise VerificationError("schema step conclusion is not P(N+1)")

    schema_tokens = ADAPTER.tokenize_formula(formula.body)
    schema_id = hashlib.sha256(
        "\0".join(schema_tokens).encode("utf-8")
    ).hexdigest()
    return {
        "schema_version": 1,
        "schema_name": formula.name,
        "schema_id": schema_id,
        "bound": " ".join(bound),
        "property_tokens": abstract_property,
        "checks": {
            "single_visible_tff_axiom": True,
            "source_conclusion_match": True,
            "base_instance_match": True,
            "step_hypothesis_match": True,
            "successor_instance_match": True,
        },
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--augmented", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    report = verify_structure(
        arguments.source.resolve().read_text(encoding="utf-8"),
        arguments.augmented.resolve().read_text(encoding="utf-8"),
    )
    if arguments.output is not None:
        output = arguments.output.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(
            json.dumps(report, sort_keys=True, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
    print(
        f"OK: verified {report['schema_name']} "
        f"({report['schema_id']})"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        OSError,
        VerificationError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
