#!/usr/bin/env python3
"""Exact bounded clause saturation for the frozen Fourier-Motzkin slice."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import math
import time
from fractions import Fraction
from typing import Any, Callable, Iterable, Iterator, Sequence


SCHEMA = "umlaut-rational-fm-corpus-v1"
CERTIFICATE_SCHEMA = "umlaut-rational-fm-certificate-v1"


class FmError(ValueError):
    """The workload or an exact inference violates the frozen protocol."""


@dataclasses.dataclass(frozen=True)
class Bounds:
    max_input_clauses: int = 256
    max_literals_per_clause: int = 64
    max_variables_per_literal: int = 32
    max_retained_clauses: int = 10_000
    max_inference_attempts: int = 100_000
    max_seconds: float = 30.0
    max_integer_bits: int = 256


def canonical_json(value: Any) -> str:
    return json.dumps(
        value,
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    )


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def fraction(value: Any) -> Fraction:
    try:
        return Fraction(str(value))
    except (ValueError, ZeroDivisionError) as error:
        raise FmError(f"invalid exact rational {value!r}") from error


def fraction_text(value: Fraction) -> str:
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def coefficient_bits(value: Fraction) -> int:
    return max(
        abs(value.numerator).bit_length(),
        value.denominator.bit_length(),
    )


def normalize_arithmetic(literal: dict[str, Any]) -> dict[str, Any]:
    if literal.get("kind") != "arith":
        raise FmError("expected arithmetic literal")
    sort = literal.get("sort")
    if sort not in {"Rat", "Real"}:
        raise FmError(f"unsupported arithmetic sort {sort!r}")
    if not isinstance(literal.get("strict"), bool):
        raise FmError("arithmetic strictness must be Boolean")
    raw_coefficients = literal.get("coefficients")
    if not isinstance(raw_coefficients, dict):
        raise FmError("arithmetic coefficients must be an object")
    coefficients = {
        str(variable): fraction(value)
        for variable, value in raw_coefficients.items()
        if fraction(value) != 0
    }
    if any(not variable for variable in coefficients):
        raise FmError("empty arithmetic variable")
    constant = fraction(literal.get("constant", "0"))
    values = [*coefficients.values(), constant]
    nonzero = [value for value in values if value]
    if nonzero:
        denominator_lcm = math.lcm(
            *(value.denominator for value in nonzero)
        )
        integer_values = [
            abs(value.numerator * (denominator_lcm // value.denominator))
            for value in nonzero
        ]
        common = math.gcd(*integer_values)
        scale = Fraction(denominator_lcm, common)
        coefficients = {
            variable: value * scale
            for variable, value in coefficients.items()
        }
        constant *= scale
    return {
        "kind": "arith",
        "sort": sort,
        "strict": literal["strict"],
        "coefficients": {
            variable: fraction_text(value)
            for variable, value in sorted(coefficients.items())
        },
        "constant": fraction_text(constant),
    }


def normalize_proposition(literal: dict[str, Any]) -> dict[str, Any]:
    if literal.get("kind") != "prop":
        raise FmError("expected propositional literal")
    name = literal.get("name")
    positive = literal.get("positive")
    if not isinstance(name, str) or not name:
        raise FmError("proposition name must be nonempty")
    if not isinstance(positive, bool):
        raise FmError("proposition polarity must be Boolean")
    return {"kind": "prop", "name": name, "positive": positive}


def normalize_literal(literal: dict[str, Any]) -> dict[str, Any]:
    kind = literal.get("kind")
    if kind == "arith":
        return normalize_arithmetic(literal)
    if kind == "prop":
        return normalize_proposition(literal)
    raise FmError(f"unsupported literal kind {kind!r}")


def literal_key(literal: dict[str, Any]) -> str:
    return canonical_json(literal)


def literal_shape(literal: dict[str, Any]) -> tuple[Any, ...]:
    if literal["kind"] == "prop":
        return ("prop", literal["name"], literal["positive"])
    coefficients = sorted(
        fraction(value)
        for value in literal["coefficients"].values()
    )
    return (
        "arith",
        literal["sort"],
        literal["strict"],
        fraction(literal["constant"]),
        tuple(coefficients),
    )


def literal_variables(literal: dict[str, Any]) -> set[str]:
    if literal["kind"] != "arith":
        return set()
    return set(literal["coefficients"])


def rename_literal(
    literal: dict[str, Any],
    renaming: dict[str, str],
) -> dict[str, Any]:
    if literal["kind"] != "arith":
        return dict(literal)
    coefficients: dict[str, Fraction] = {}
    for variable, raw_value in literal["coefficients"].items():
        renamed = renaming.get(variable, variable)
        coefficients[renamed] = (
            coefficients.get(renamed, Fraction(0)) + fraction(raw_value)
        )
    return normalize_arithmetic(
        {
            **literal,
            "coefficients": {
                variable: fraction_text(value)
                for variable, value in coefficients.items()
            },
        }
    )


def alpha_normalize(
    literals: Iterable[dict[str, Any]],
) -> list[dict[str, Any]]:
    normalized = [normalize_literal(literal) for literal in literals]
    normalized.sort(key=lambda literal: (literal_shape(literal), literal_key(literal)))
    variable_map: dict[str, str] = {}
    for literal in normalized:
        if literal["kind"] != "arith":
            continue
        ordered = sorted(
            literal["coefficients"],
            key=lambda variable: (
                fraction(literal["coefficients"][variable]),
                variable,
            ),
        )
        for variable in ordered:
            if variable not in variable_map:
                variable_map[variable] = f"v{len(variable_map)}"
    renamed = [
        rename_literal(literal, variable_map)
        for literal in normalized
    ]
    return sorted(renamed, key=literal_key)


def arithmetic_complement(
    left: dict[str, Any],
    right: dict[str, Any],
) -> bool:
    if left["kind"] != "arith" or right["kind"] != "arith":
        return False
    if left["sort"] != right["sort"]:
        return False
    if left["strict"] == right["strict"]:
        return False
    variables = set(left["coefficients"]) | set(right["coefficients"])
    if any(
        fraction(left["coefficients"].get(variable, "0"))
        != -fraction(right["coefficients"].get(variable, "0"))
        for variable in variables
    ):
        return False
    return fraction(left["constant"]) == -fraction(right["constant"])


def constant_truth(literal: dict[str, Any]) -> bool | None:
    if literal["kind"] != "arith" or literal["coefficients"]:
        return None
    value = fraction(literal["constant"])
    return value > 0 if literal["strict"] else value >= 0


def simplify_clause(
    literals: Iterable[dict[str, Any]],
) -> list[dict[str, Any]] | None:
    retained: dict[str, dict[str, Any]] = {}
    propositions: dict[str, set[bool]] = {}
    arithmetic: list[dict[str, Any]] = []
    for raw_literal in literals:
        literal = normalize_literal(raw_literal)
        truth = constant_truth(literal)
        if truth is True:
            return None
        if truth is False:
            continue
        if literal["kind"] == "prop":
            signs = propositions.setdefault(literal["name"], set())
            signs.add(literal["positive"])
            if len(signs) == 2:
                return None
        else:
            if any(
                arithmetic_complement(literal, other)
                for other in arithmetic
            ):
                return None
            arithmetic.append(literal)
        retained[literal_key(literal)] = literal
    return alpha_normalize(retained.values())


def clause_key(clause: Sequence[dict[str, Any]]) -> str:
    return canonical_json(list(clause))


def clause_id(clause: Sequence[dict[str, Any]]) -> str:
    return "c_" + sha256_text(clause_key(clause))[:20]


def clause_variables(clause: Sequence[dict[str, Any]]) -> set[str]:
    return {
        variable
        for literal in clause
        for variable in literal_variables(literal)
    }


def check_bounds_on_clause(clause: Sequence[dict[str, Any]], bounds: Bounds) -> str | None:
    if len(clause) > bounds.max_literals_per_clause:
        return "literals_per_clause"
    for literal in clause:
        if literal["kind"] != "arith":
            continue
        if len(literal["coefficients"]) > bounds.max_variables_per_literal:
            return "variables_per_literal"
        values = [
            fraction(literal["constant"]),
            *(fraction(value) for value in literal["coefficients"].values()),
        ]
        if any(
            coefficient_bits(value) > bounds.max_integer_bits
            for value in values
        ):
            return "coefficient_bits"
    return None


def maximum_coefficient_bits(clause: Sequence[dict[str, Any]]) -> int:
    return max(
        (
            coefficient_bits(fraction(value))
            for literal in clause
            if literal["kind"] == "arith"
            for value in [
                literal["constant"],
                *literal["coefficients"].values(),
            ]
        ),
        default=0,
    )


def subsumes(
    left: Sequence[dict[str, Any]],
    right: Sequence[dict[str, Any]],
) -> bool:
    return {
        literal_key(literal) for literal in left
    }.issubset(literal_key(literal) for literal in right)


def proposition_resolvents(
    left: Sequence[dict[str, Any]],
    right: Sequence[dict[str, Any]],
) -> Iterator[tuple[list[dict[str, Any]], dict[str, Any]]]:
    for left_index, left_literal in enumerate(left):
        if left_literal["kind"] != "prop":
            continue
        for right_index, right_literal in enumerate(right):
            if (
                right_literal["kind"] == "prop"
                and left_literal["name"] == right_literal["name"]
                and left_literal["positive"] != right_literal["positive"]
            ):
                conclusion = simplify_clause(
                    [
                        *left[:left_index],
                        *left[left_index + 1 :],
                        *right[:right_index],
                        *right[right_index + 1 :],
                    ]
                )
                if conclusion is not None:
                    yield conclusion, {
                        "rule": "propositional_resolution",
                        "left_index": left_index,
                        "right_index": right_index,
                        "atom": left_literal["name"],
                    }


def isolated_variable(
    clause: Sequence[dict[str, Any]],
    selected_index: int,
    variable: str,
) -> bool:
    return all(
        variable not in literal_variables(literal)
        for index, literal in enumerate(clause)
        if index != selected_index
    )


def fm_renamings(
    left: Sequence[dict[str, Any]],
    left_variable: str,
    right: Sequence[dict[str, Any]],
    right_variable: str,
) -> tuple[dict[str, str], dict[str, str]]:
    left_map = {
        variable: (
            "pivot"
            if variable == left_variable
            else f"left_{index}"
        )
        for index, variable in enumerate(sorted(clause_variables(left)))
    }
    right_map = {
        variable: (
            "pivot"
            if variable == right_variable
            else f"right_{index}"
        )
        for index, variable in enumerate(sorted(clause_variables(right)))
    }
    return left_map, right_map


def scale_and_add(
    left: dict[str, Any],
    left_scale: Fraction,
    right: dict[str, Any],
    right_scale: Fraction,
) -> dict[str, Any]:
    coefficients: dict[str, Fraction] = {}
    for literal, scale in ((left, left_scale), (right, right_scale)):
        for variable, raw_value in literal["coefficients"].items():
            coefficients[variable] = (
                coefficients.get(variable, Fraction(0))
                + scale * fraction(raw_value)
            )
    constant = (
        left_scale * fraction(left["constant"])
        + right_scale * fraction(right["constant"])
    )
    return normalize_arithmetic(
        {
            "kind": "arith",
            "sort": left["sort"],
            "strict": left["strict"] or right["strict"],
            "coefficients": {
                variable: fraction_text(value)
                for variable, value in coefficients.items()
            },
            "constant": fraction_text(constant),
        }
    )


def fm_resolvents(
    left: Sequence[dict[str, Any]],
    right: Sequence[dict[str, Any]],
) -> Iterator[tuple[list[dict[str, Any]], dict[str, Any]]]:
    for left_index, left_literal in enumerate(left):
        if left_literal["kind"] != "arith":
            continue
        for right_index, right_literal in enumerate(right):
            if (
                right_literal["kind"] != "arith"
                or left_literal["sort"] != right_literal["sort"]
            ):
                continue
            for left_variable, raw_left_coefficient in sorted(
                left_literal["coefficients"].items()
            ):
                left_coefficient = fraction(raw_left_coefficient)
                if left_coefficient <= 0 or not isolated_variable(
                    left, left_index, left_variable
                ):
                    continue
                for right_variable, raw_right_coefficient in sorted(
                    right_literal["coefficients"].items()
                ):
                    right_coefficient = fraction(raw_right_coefficient)
                    if right_coefficient >= 0 or not isolated_variable(
                        right, right_index, right_variable
                    ):
                        continue
                    left_map, right_map = fm_renamings(
                        left,
                        left_variable,
                        right,
                        right_variable,
                    )
                    renamed_left = rename_literal(left_literal, left_map)
                    renamed_right = rename_literal(right_literal, right_map)
                    left_scale = -right_coefficient
                    right_scale = left_coefficient
                    combined = scale_and_add(
                        renamed_left,
                        left_scale,
                        renamed_right,
                        right_scale,
                    )
                    if "pivot" in combined["coefficients"]:
                        raise FmError("Fourier-Motzkin pivot did not cancel")
                    contexts = [
                        rename_literal(literal, left_map)
                        for index, literal in enumerate(left)
                        if index != left_index
                    ]
                    contexts.extend(
                        rename_literal(literal, right_map)
                        for index, literal in enumerate(right)
                        if index != right_index
                    )
                    conclusion = simplify_clause([*contexts, combined])
                    if conclusion is not None:
                        yield conclusion, {
                            "rule": "fourier_motzkin",
                            "left_index": left_index,
                            "right_index": right_index,
                            "left_variable": left_variable,
                            "right_variable": right_variable,
                            "left_multiplier": fraction_text(left_scale),
                            "right_multiplier": fraction_text(right_scale),
                            "left_renaming": dict(sorted(left_map.items())),
                            "right_renaming": dict(sorted(right_map.items())),
                        }


def validate_workload(workload: dict[str, Any], bounds: Bounds) -> None:
    if not isinstance(workload.get("id"), str) or not workload["id"]:
        raise FmError("workload ID must be nonempty")
    supported = workload.get("supported", True)
    if not isinstance(supported, bool):
        raise FmError("workload supported flag must be Boolean")
    clauses = workload.get("clauses")
    if not isinstance(clauses, list):
        raise FmError("workload clauses must be an array")
    if len(clauses) > bounds.max_input_clauses:
        raise FmError("input_clauses")
    if not supported:
        if not isinstance(workload.get("unsupported_reason"), str):
            raise FmError("unsupported workload lacks a reason")
        return
    for raw_clause in clauses:
        if not isinstance(raw_clause, dict) or not isinstance(
            raw_clause.get("literals"), list
        ):
            raise FmError("malformed input clause")
        normalized = simplify_clause(raw_clause["literals"])
        if normalized is None:
            continue
        crossed = check_bounds_on_clause(normalized, bounds)
        if crossed is not None:
            raise FmError(crossed)


def load_corpus(value: dict[str, Any], bounds: Bounds | None = None) -> dict[str, Any]:
    if value.get("schema") != SCHEMA:
        raise FmError("unexpected corpus schema")
    active_bounds = bounds or Bounds()
    identifiers: set[str] = set()
    for workload in value.get("workloads", []):
        validate_workload(workload, active_bounds)
        if workload["id"] in identifiers:
            raise FmError(f"duplicate workload ID {workload['id']!r}")
        identifiers.add(workload["id"])
    return value


def derivation_record(
    clause: Sequence[dict[str, Any]],
    derivation: dict[str, Any],
) -> dict[str, Any]:
    return {
        "id": clause_id(clause),
        "clause_sha256": sha256_text(clause_key(clause)),
        "literals": list(clause),
        "derivation": derivation,
    }


def saturate(
    workload: dict[str, Any],
    *,
    enable_fm: bool,
    bounds: Bounds | None = None,
    cancelled: Callable[[], bool] | None = None,
) -> dict[str, Any]:
    active_bounds = bounds or Bounds()
    validate_workload(workload, active_bounds)
    started = time.perf_counter_ns()
    mode = "native_fm" if enable_fm else "normalize_resolution"
    if not workload.get("supported", True):
        return {
            "schema": CERTIFICATE_SCHEMA,
            "workload_id": workload["id"],
            "mode": mode,
            "outcome": "unknown",
            "empty_clause_id": None,
            "records": [],
            "metrics": {
                "input_clauses": len(workload["clauses"]),
                "retained_clauses": 0,
                "peak_clauses": 0,
                "generated": {
                    "propositional_resolution": 0,
                    "fourier_motzkin": 0,
                },
                "attempts": {
                    "propositional_resolution": 0,
                    "fourier_motzkin": 0,
                },
                "subsumed": 0,
                "elapsed_ns": time.perf_counter_ns() - started,
                "crossed_bound": None,
                "unsupported_reason": workload["unsupported_reason"],
                "max_coefficient_bits": 0,
            },
        }
    if cancelled is not None and cancelled():
        return {
            "schema": CERTIFICATE_SCHEMA,
            "workload_id": workload["id"],
            "mode": mode,
            "outcome": "unknown",
            "empty_clause_id": None,
            "records": [],
            "metrics": {
                "input_clauses": len(workload["clauses"]),
                "retained_clauses": 0,
                "peak_clauses": 0,
                "generated": {
                    "propositional_resolution": 0,
                    "fourier_motzkin": 0,
                },
                "attempts": {
                    "propositional_resolution": 0,
                    "fourier_motzkin": 0,
                },
                "subsumed": 0,
                "elapsed_ns": time.perf_counter_ns() - started,
                "crossed_bound": "cancelled",
                "max_coefficient_bits": 0,
            },
        }
    records: dict[str, dict[str, Any]] = {}
    active: set[str] = set()
    generated = {"propositional_resolution": 0, "fourier_motzkin": 0}
    attempts = {"propositional_resolution": 0, "fourier_motzkin": 0}
    subsumed_count = 0
    crossed_bound = None
    max_seen_bits = 0

    def elapsed_seconds() -> float:
        return (time.perf_counter_ns() - started) / 1_000_000_000

    def add_clause(
        clause: list[dict[str, Any]],
        derivation: dict[str, Any],
    ) -> tuple[bool, str | None]:
        nonlocal subsumed_count, crossed_bound, max_seen_bits
        crossed = check_bounds_on_clause(clause, active_bounds)
        if crossed is not None:
            crossed_bound = crossed
            return False, None
        max_seen_bits = max(
            max_seen_bits,
            maximum_coefficient_bits(clause),
        )
        identifier = clause_id(clause)
        if identifier in records:
            return False, identifier
        if any(subsumes(records[item]["literals"], clause) for item in active):
            subsumed_count += 1
            return False, None
        removed = [
            item
            for item in active
            if subsumes(clause, records[item]["literals"])
        ]
        projected_retained = len(active) - len(removed) + 1
        if projected_retained > active_bounds.max_retained_clauses:
            crossed_bound = "retained_clauses"
            return False, None
        for item in removed:
            active.remove(item)
            subsumed_count += 1
        record = derivation_record(clause, derivation)
        records[identifier] = record
        active.add(identifier)
        return True, identifier

    for source_index, raw_clause in enumerate(workload["clauses"]):
        clause = simplify_clause(raw_clause["literals"])
        if clause is None:
            continue
        added, identifier = add_clause(
            clause,
            {
                "rule": "input",
                "source_index": source_index,
                "source_id": raw_clause.get("id", f"input_{source_index}"),
            },
        )
        if (
            crossed_bound is None
            and added
            and identifier is not None
            and not clause
        ):
            return {
                "schema": CERTIFICATE_SCHEMA,
                "workload_id": workload["id"],
                "mode": mode,
                "outcome": "unsat",
                "empty_clause_id": identifier,
                "records": list(records.values()),
                "metrics": {
                    "input_clauses": len(workload["clauses"]),
                    "retained_clauses": len(active),
                    "peak_clauses": len(active),
                    "generated": generated,
                    "attempts": attempts,
                    "subsumed": subsumed_count,
                    "elapsed_ns": time.perf_counter_ns() - started,
                    "crossed_bound": None,
                    "max_coefficient_bits": max_seen_bits,
                },
            }
        if crossed_bound is not None:
            break

    peak_clauses = len(active)
    attempted: set[tuple[Any, ...]] = set()
    changed = True
    empty_clause_id = None
    while changed and crossed_bound is None and empty_clause_id is None:
        changed = False
        identifiers = sorted(active)
        for left_offset, left_id in enumerate(identifiers):
            for right_id in identifiers[left_offset:]:
                if cancelled is not None and cancelled():
                    crossed_bound = "cancelled"
                    break
                if elapsed_seconds() > active_bounds.max_seconds:
                    crossed_bound = "seconds"
                    break
                left = records[left_id]["literals"]
                right = records[right_id]["literals"]
                rules: list[
                    tuple[
                        str,
                        str,
                        str,
                        Iterator[tuple[list[dict[str, Any]], dict[str, Any]]],
                    ]
                ] = [
                    (
                        "propositional_resolution",
                        left_id,
                        right_id,
                        proposition_resolvents(left, right),
                    )
                ]
                if enable_fm:
                    rules.append(
                        (
                            "fourier_motzkin",
                            left_id,
                            right_id,
                            fm_resolvents(left, right),
                        )
                    )
                    if left_id != right_id:
                        rules.append(
                            (
                                "fourier_motzkin",
                                right_id,
                                left_id,
                                fm_resolvents(right, left),
                            )
                        )
                for rule, first_parent, second_parent, conclusions in rules:
                    for conclusion, evidence in conclusions:
                        signature = (
                            rule,
                            first_parent,
                            second_parent,
                            canonical_json(evidence),
                        )
                        if signature in attempted:
                            continue
                        attempted.add(signature)
                        attempts[rule] += 1
                        if sum(attempts.values()) > active_bounds.max_inference_attempts:
                            crossed_bound = "inference_attempts"
                            break
                        derivation = {
                            **evidence,
                            "parents": [first_parent, second_parent],
                        }
                        added, identifier = add_clause(conclusion, derivation)
                        if crossed_bound is not None:
                            break
                        if added:
                            generated[rule] += 1
                            changed = True
                            peak_clauses = max(peak_clauses, len(active))
                            if identifier is not None and not conclusion:
                                empty_clause_id = identifier
                                break
                    if crossed_bound is not None or empty_clause_id is not None:
                        break
                if crossed_bound is not None or empty_clause_id is not None:
                    break
            if crossed_bound is not None or empty_clause_id is not None:
                break

    outcome = "unsat" if empty_clause_id is not None else "unknown"
    return {
        "schema": CERTIFICATE_SCHEMA,
        "workload_id": workload["id"],
        "mode": mode,
        "outcome": outcome,
        "empty_clause_id": empty_clause_id,
        "records": list(records.values()),
        "metrics": {
            "input_clauses": len(workload["clauses"]),
            "retained_clauses": len(active),
            "peak_clauses": peak_clauses,
            "generated": generated,
            "attempts": attempts,
            "subsumed": subsumed_count,
            "elapsed_ns": time.perf_counter_ns() - started,
            "crossed_bound": crossed_bound,
            "max_coefficient_bits": max_seen_bits,
        },
    }
