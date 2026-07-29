#!/usr/bin/env python3
"""Restricted TPTP CNF decomposition for the bounded AVATAR experiment."""

from __future__ import annotations

import hashlib
import re
from pathlib import Path
from typing import Any, Iterable


class SplitError(ValueError):
    """The input is outside the deliberately narrow experiment fragment."""


ANNOTATED_PREFIXES = ("cnf", "fof", "tff", "thf", "tpi")
VARIABLE_RE = re.compile(r"[A-Z_][A-Za-z0-9_]*")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


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
                raise SplitError("unterminated block comment")
            output.extend((" ", " "))
            index += 2
            continue
        output.append(character)
        index += 1
    if quote is not None:
        raise SplitError("unterminated quoted token")
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
                raise SplitError("unbalanced closing parenthesis")
        elif character == "." and depth == 0:
            statement = clean[start : index + 1].strip()
            if statement:
                statements.append(statement)
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise SplitError("unbalanced statement")
    if clean[start:].strip():
        raise SplitError("trailing text without statement terminator")
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
                raise SplitError("unbalanced nested expression")
        elif character == separator and depth == 0:
            pieces.append(value[start:index].strip())
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise SplitError("unbalanced nested expression")
    pieces.append(value[start:].strip())
    return pieces


def wrapping_parentheses(value: str) -> bool:
    if not (value.startswith("(") and value.endswith(")")):
        return False
    depth = 0
    quote: str | None = None
    for index, character in enumerate(value):
        if quote is not None:
            if character == quote:
                quote = None
            elif character == "\\":
                continue
            continue
        if character in ("'", '"'):
            quote = character
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0 and index != len(value) - 1:
                return False
    return depth == 0


def strip_wrapping_parentheses(value: str) -> str:
    value = value.strip()
    while wrapping_parentheses(value):
        value = value[1:-1].strip()
    return value


def parse_cnf_statement(statement: str, statement_index: int) -> dict[str, Any]:
    prefix, separator, rest = statement.partition("(")
    if not separator or prefix.strip().lower() != "cnf":
        raise SplitError("statement is not CNF")
    if not rest.rstrip().endswith(")."):
        raise SplitError("malformed CNF statement")
    body = rest.rstrip()[:-2]
    arguments = split_top_level(body, ",")
    if len(arguments) < 3:
        raise SplitError("CNF statement has fewer than three fields")
    formula = strip_wrapping_parentheses(arguments[2])
    literals = split_top_level(formula, "|")
    if any(not literal for literal in literals):
        raise SplitError("CNF statement contains an empty literal")
    return {
        "statement_index": statement_index,
        "statement": statement,
        "name": arguments[0],
        "role": arguments[1],
        "formula": formula,
        "literals": literals,
    }


def literal_variables(literal: str) -> set[str]:
    variables: set[str] = set()
    quote: str | None = None
    index = 0
    while index < len(literal):
        character = literal[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(literal) and literal[index + 1] == quote:
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
        match = VARIABLE_RE.match(literal, index)
        if match is not None:
            variables.add(match.group())
            index = match.end()
            continue
        index += 1
    return variables


def decompose_literals(literals: list[str]) -> list[list[str]]:
    variable_sets = [literal_variables(literal) for literal in literals]
    remaining = set(range(len(literals)))
    components: list[list[str]] = []
    while remaining:
        seed = min(remaining)
        remaining.remove(seed)
        members = [seed]
        variables = set(variable_sets[seed])
        changed = True
        while changed:
            changed = False
            for index in sorted(remaining):
                if variables.intersection(variable_sets[index]):
                    remaining.remove(index)
                    members.append(index)
                    variables.update(variable_sets[index])
                    changed = True
        components.append([literals[index] for index in members])
    return components


def canonical_component(literals: Iterable[str]) -> str:
    variable_names: dict[str, str] = {}
    output: list[str] = []
    for literal_index, literal in enumerate(literals):
        if literal_index:
            output.append("|")
        quote: str | None = None
        index = 0
        while index < len(literal):
            character = literal[index]
            if quote is not None:
                output.append(character)
                if character == quote:
                    if index + 1 < len(literal) and literal[index + 1] == quote:
                        output.append(literal[index + 1])
                        index += 2
                        continue
                    quote = None
                elif character == "\\" and index + 1 < len(literal):
                    output.append(literal[index + 1])
                    index += 2
                    continue
                index += 1
                continue
            if character in ("'", '"'):
                quote = character
                output.append(character)
                index += 1
                continue
            match = VARIABLE_RE.match(literal, index)
            if match is not None:
                variable = match.group()
                replacement = variable_names.setdefault(
                    variable, f"V{len(variable_names)}"
                )
                output.append(replacement)
                index = match.end()
                continue
            if not character.isspace():
                output.append(character)
            index += 1
    return "".join(output)


def analyze_problem(text: str, max_split_clauses: int) -> dict[str, Any]:
    if max_split_clauses < 1:
        raise SplitError("max_split_clauses must be positive")
    statements = split_statements(text)
    records: list[dict[str, Any]] = []
    for statement_index, statement in enumerate(statements):
        prefix = statement.partition("(")[0].strip().lower()
        if prefix == "include":
            raise SplitError("include statements are outside the prototype fragment")
        if prefix in ANNOTATED_PREFIXES and prefix != "cnf":
            raise SplitError(f"{prefix} statements are outside the CNF fragment")
        if prefix != "cnf":
            raise SplitError(f"unsupported top-level statement: {prefix or statement}")
        records.append(parse_cnf_statement(statement, statement_index))

    candidates: list[dict[str, Any]] = []
    for record in records:
        components = decompose_literals(record["literals"])
        if len(components) > 1:
            candidates.append({**record, "components": components})
    ranked = sorted(
        candidates,
        key=lambda candidate: (
            -len(candidate["components"]),
            -len(candidate["literals"]),
            candidate["statement_index"],
        ),
    )
    selected_indices = {
        candidate["statement_index"]
        for candidate in ranked[:max_split_clauses]
    }
    selected = [
        candidate
        for candidate in candidates
        if candidate["statement_index"] in selected_indices
    ]

    selector_by_component: dict[str, int] = {}
    component_by_selector: dict[int, dict[str, Any]] = {}
    split_clauses: list[list[int]] = []
    split_records: list[dict[str, Any]] = []
    for candidate in selected:
        selectors: list[int] = []
        rendered_components: list[dict[str, Any]] = []
        for component_literals in candidate["components"]:
            canonical = canonical_component(component_literals)
            selector = selector_by_component.setdefault(
                canonical, len(selector_by_component) + 1
            )
            component = {
                "selector": selector,
                "canonical": canonical,
                "literals": component_literals,
                "formula": " | ".join(component_literals),
            }
            component_by_selector.setdefault(selector, component)
            rendered_components.append(component)
            selectors.append(selector)
        split_clauses.append(selectors)
        split_records.append(
            {
                "statement_index": candidate["statement_index"],
                "name": candidate["name"],
                "role": candidate["role"],
                "formula": candidate["formula"],
                "literals": candidate["literals"],
                "components": rendered_components,
            }
        )

    return {
        "schema_version": 1,
        "statement_count": len(statements),
        "cnf_count": len(records),
        "splittable_clause_count": len(candidates),
        "selected_split_count": len(selected),
        "selector_count": len(selector_by_component),
        "split_clauses": split_clauses,
        "split_records": split_records,
        "base_statements": [
            record["statement"]
            for record in records
            if record["statement_index"] not in selected_indices
        ],
        "components": [
            component_by_selector[selector]
            for selector in sorted(component_by_selector)
        ],
    }


def render_branch(
    abstraction: dict[str, Any],
    active_selectors: Iterable[int],
    *,
    source_sha256: str,
    model_index: int,
) -> str:
    active = sorted(set(active_selectors))
    known = {
        int(component["selector"]): component
        for component in abstraction["components"]
    }
    if any(selector not in known for selector in active):
        raise SplitError("branch activates an unknown selector")
    lines = [
        "% Bounded AVATAR restart branch.",
        "% Status   : Unsatisfiable",
        f"% SourceSHA256 : {source_sha256}",
        f"% ModelIndex   : {model_index}",
        f"% ActiveSelectors : {','.join(map(str, active))}",
        "",
        *abstraction["base_statements"],
    ]
    for selector in active:
        component = known[selector]
        lines.append(
            f"cnf(avatar_component_{selector}, plain, "
            f"({component['formula']}))."
        )
    return "\n".join(lines) + "\n"


def analyze_file(path: Path, max_split_clauses: int) -> dict[str, Any]:
    return analyze_problem(path.read_text(encoding="utf-8"), max_split_clauses)

