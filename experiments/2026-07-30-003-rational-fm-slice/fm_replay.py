#!/usr/bin/env python3
"""Independent structural replay for rational FM experiment certificates."""

from __future__ import annotations

import argparse
import json
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence

from fm_core import (
    CERTIFICATE_SCHEMA,
    FmError,
    canonical_json,
    clause_id,
    fraction,
    fraction_text,
    rename_literal,
    sha256_text,
    simplify_clause,
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise FmError(message)


def variables(clause: Sequence[dict[str, Any]]) -> set[str]:
    return {
        variable
        for literal in clause
        if literal["kind"] == "arith"
        for variable in literal["coefficients"]
    }


def is_isolated(
    clause: Sequence[dict[str, Any]],
    selected_index: int,
    variable: str,
) -> bool:
    return all(
        literal["kind"] != "arith"
        or variable not in literal["coefficients"]
        for index, literal in enumerate(clause)
        if index != selected_index
    )


def expected_renaming(
    clause: Sequence[dict[str, Any]],
    selected_variable: str,
    side: str,
) -> dict[str, str]:
    return {
        variable: (
            "pivot"
            if variable == selected_variable
            else f"{side}_{index}"
        )
        for index, variable in enumerate(sorted(variables(clause)))
    }


def replay_resolution(
    left: Sequence[dict[str, Any]],
    right: Sequence[dict[str, Any]],
    derivation: dict[str, Any],
) -> list[dict[str, Any]] | None:
    left_index = derivation.get("left_index")
    right_index = derivation.get("right_index")
    require(isinstance(left_index, int), "resolution left index is not an integer")
    require(isinstance(right_index, int), "resolution right index is not an integer")
    require(0 <= left_index < len(left), "resolution left index is out of range")
    require(0 <= right_index < len(right), "resolution right index is out of range")
    left_literal = left[left_index]
    right_literal = right[right_index]
    require(
        left_literal["kind"] == right_literal["kind"] == "prop",
        "resolution pivot is not propositional",
    )
    require(
        left_literal["name"] == right_literal["name"],
        "resolution atom differs between parents",
    )
    require(
        left_literal["positive"] != right_literal["positive"],
        "resolution polarities are not complementary",
    )
    require(
        derivation.get("atom") == left_literal["name"],
        "resolution evidence names the wrong atom",
    )
    return simplify_clause(
        [
            *left[:left_index],
            *left[left_index + 1 :],
            *right[:right_index],
            *right[right_index + 1 :],
        ]
    )


def replay_fm(
    left: Sequence[dict[str, Any]],
    right: Sequence[dict[str, Any]],
    derivation: dict[str, Any],
) -> list[dict[str, Any]] | None:
    left_index = derivation.get("left_index")
    right_index = derivation.get("right_index")
    require(isinstance(left_index, int), "FM left index is not an integer")
    require(isinstance(right_index, int), "FM right index is not an integer")
    require(0 <= left_index < len(left), "FM left index is out of range")
    require(0 <= right_index < len(right), "FM right index is out of range")
    left_literal = left[left_index]
    right_literal = right[right_index]
    require(
        left_literal["kind"] == right_literal["kind"] == "arith",
        "FM selected literal is not arithmetic",
    )
    require(
        left_literal["sort"] == right_literal["sort"],
        "FM selected literals have different sorts",
    )

    left_variable = derivation.get("left_variable")
    right_variable = derivation.get("right_variable")
    require(
        isinstance(left_variable, str)
        and left_variable in left_literal["coefficients"],
        "FM left variable is absent",
    )
    require(
        isinstance(right_variable, str)
        and right_variable in right_literal["coefficients"],
        "FM right variable is absent",
    )
    left_coefficient = fraction(left_literal["coefficients"][left_variable])
    right_coefficient = fraction(right_literal["coefficients"][right_variable])
    require(left_coefficient > 0, "FM left coefficient is not positive")
    require(right_coefficient < 0, "FM right coefficient is not negative")
    require(
        is_isolated(left, left_index, left_variable),
        "FM left pivot occurs in its side clause",
    )
    require(
        is_isolated(right, right_index, right_variable),
        "FM right pivot occurs in its side clause",
    )

    left_map = expected_renaming(left, left_variable, "left")
    right_map = expected_renaming(right, right_variable, "right")
    require(
        derivation.get("left_renaming") == left_map,
        "FM left standardization-apart map differs",
    )
    require(
        derivation.get("right_renaming") == right_map,
        "FM right standardization-apart map differs",
    )
    left_multiplier = -right_coefficient
    right_multiplier = left_coefficient
    require(
        fraction(derivation.get("left_multiplier")) == left_multiplier,
        "FM left multiplier differs",
    )
    require(
        fraction(derivation.get("right_multiplier")) == right_multiplier,
        "FM right multiplier differs",
    )
    require(
        left_multiplier > 0 and right_multiplier > 0,
        "FM multipliers are not positive",
    )

    renamed_left = rename_literal(left_literal, left_map)
    renamed_right = rename_literal(right_literal, right_map)
    coefficients: dict[str, Fraction] = {}
    for literal, multiplier in (
        (renamed_left, left_multiplier),
        (renamed_right, right_multiplier),
    ):
        for variable, raw_value in literal["coefficients"].items():
            coefficients[variable] = (
                coefficients.get(variable, Fraction(0))
                + multiplier * fraction(raw_value)
            )
    require(coefficients.get("pivot", Fraction(0)) == 0, "FM pivot did not cancel")
    coefficients.pop("pivot", None)
    constant = (
        left_multiplier * fraction(renamed_left["constant"])
        + right_multiplier * fraction(renamed_right["constant"])
    )
    combined = {
        "kind": "arith",
        "sort": left_literal["sort"],
        "strict": left_literal["strict"] or right_literal["strict"],
        "coefficients": {
            variable: fraction_text(value)
            for variable, value in coefficients.items()
            if value
        },
        "constant": fraction_text(constant),
    }
    context = [
        rename_literal(literal, left_map)
        for index, literal in enumerate(left)
        if index != left_index
    ]
    context.extend(
        rename_literal(literal, right_map)
        for index, literal in enumerate(right)
        if index != right_index
    )
    return simplify_clause([*context, combined])


def replay(
    workload: dict[str, Any],
    certificate: dict[str, Any],
) -> dict[str, Any]:
    require(
        certificate.get("schema") == CERTIFICATE_SCHEMA,
        "unexpected certificate schema",
    )
    require(
        certificate.get("workload_id") == workload.get("id"),
        "certificate workload ID differs",
    )
    require(
        certificate.get("mode") in {"normalize_resolution", "native_fm"},
        "unexpected certificate mode",
    )
    records = certificate.get("records")
    require(isinstance(records, list), "certificate records are not an array")
    seen: dict[str, list[dict[str, Any]]] = {}
    rule_counts: dict[str, int] = {}

    for record_index, record in enumerate(records):
        require(isinstance(record, dict), f"record {record_index} is not an object")
        literals = record.get("literals")
        require(isinstance(literals, list), f"record {record_index} lacks literals")
        normalized = simplify_clause(literals)
        require(normalized is not None, f"record {record_index} is a tautology")
        require(normalized == literals, f"record {record_index} is not canonical")
        identifier = clause_id(literals)
        require(record.get("id") == identifier, f"record {record_index} ID differs")
        require(identifier not in seen, f"duplicate record ID {identifier}")
        require(
            record.get("clause_sha256")
            == sha256_text(canonical_json(literals)),
            f"record {identifier} digest differs",
        )
        derivation = record.get("derivation")
        require(isinstance(derivation, dict), f"record {identifier} lacks derivation")
        rule = derivation.get("rule")

        if rule == "input":
            source_index = derivation.get("source_index")
            require(isinstance(source_index, int), "input source index is not integer")
            require(
                0 <= source_index < len(workload["clauses"]),
                "input source index is out of range",
            )
            source = workload["clauses"][source_index]
            expected = simplify_clause(source["literals"])
            require(expected == literals, "input record differs from source clause")
            require(
                derivation.get("source_id")
                == source.get("id", f"input_{source_index}"),
                "input source ID differs",
            )
        else:
            parents = derivation.get("parents")
            require(
                isinstance(parents, list) and len(parents) == 2,
                f"record {identifier} does not have two parents",
            )
            require(
                all(parent in seen for parent in parents),
                f"record {identifier} has a missing or forward parent",
            )
            left, right = (seen[parent] for parent in parents)
            if rule == "propositional_resolution":
                expected = replay_resolution(left, right, derivation)
            elif rule == "fourier_motzkin":
                require(
                    certificate["mode"] == "native_fm",
                    "FM inference appears in baseline certificate",
                )
                expected = replay_fm(left, right, derivation)
            else:
                raise FmError(f"record {identifier} has unknown rule {rule!r}")
            require(expected is not None, f"record {identifier} derives a tautology")
            require(expected == literals, f"record {identifier} conclusion differs")
        seen[identifier] = literals
        rule_counts[str(rule)] = rule_counts.get(str(rule), 0) + 1

    empty_identifiers = [
        identifier for identifier, literals in seen.items() if not literals
    ]
    outcome = certificate.get("outcome")
    require(outcome in {"unsat", "unknown"}, "unexpected certificate outcome")
    if outcome == "unsat":
        require(len(empty_identifiers) == 1, "UNSAT certificate lacks unique empty clause")
        require(
            certificate.get("empty_clause_id") == empty_identifiers[0],
            "UNSAT certificate points to the wrong empty clause",
        )
    else:
        require(not empty_identifiers, "unknown certificate contains an empty clause")
        require(
            certificate.get("empty_clause_id") is None,
            "unknown certificate names an empty clause",
        )
    return {
        "workload_id": workload["id"],
        "mode": certificate["mode"],
        "outcome": outcome,
        "records": len(records),
        "rule_counts": dict(sorted(rule_counts.items())),
        "certificate_sha256": sha256_text(canonical_json(certificate)),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workload", type=Path)
    parser.add_argument("certificate", type=Path)
    arguments = parser.parse_args()
    workload = json.loads(arguments.workload.read_text(encoding="utf-8"))
    certificate = json.loads(arguments.certificate.read_text(encoding="utf-8"))
    print(json.dumps(replay(workload, certificate), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
