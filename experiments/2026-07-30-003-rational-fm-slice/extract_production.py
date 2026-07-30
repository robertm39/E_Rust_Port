#!/usr/bin/env python3
"""Extract the frozen rational/real linear clause slice from Umlaut CNF."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from collections import Counter
from fractions import Fraction
from pathlib import Path
from types import ModuleType
from typing import Any

from fm_core import Bounds, SCHEMA, fraction_text, load_corpus


ARITHMETIC_SORTS = {"$rat": "Rat", "$real": "Real"}
DECLARATION_SORT = re.compile(r"\$(?:rat|real)\b")


def load_parser() -> ModuleType:
    path = (
        Path(__file__).resolve().parent.parent
        / "2026-07-30-002-real-ground-theory-traces"
        / "trace_model.py"
    )
    spec = importlib.util.spec_from_file_location("ground_trace_parser", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load the tracked TPTP parser")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def declaration_result_sort(value: str, parser: ModuleType) -> tuple[str, str] | None:
    split = parser.find_top_level_operator(value, (":",))
    if split is None:
        return None
    index, _ = split
    symbol = value[:index].strip()
    expression = parser.strip_wrapping_parentheses(value[index + 1 :])
    matches = list(DECLARATION_SORT.finditer(expression))
    if not matches:
        return None
    last = matches[-1]
    if expression[last.end() :].strip().strip(")") != "":
        return None
    return symbol, last.group()


def add_forms(
    left: tuple[dict[str, Fraction], Fraction],
    right: tuple[dict[str, Fraction], Fraction],
) -> tuple[dict[str, Fraction], Fraction]:
    coefficients = dict(left[0])
    for variable, value in right[0].items():
        coefficients[variable] = coefficients.get(variable, Fraction(0)) + value
        if not coefficients[variable]:
            del coefficients[variable]
    return coefficients, left[1] + right[1]


def scale_form(
    form: tuple[dict[str, Fraction], Fraction],
    scale: Fraction,
) -> tuple[dict[str, Fraction], Fraction]:
    return (
        {
            variable: value * scale
            for variable, value in form[0].items()
            if value * scale
        },
        form[1] * scale,
    )


def linearize(
    term: Any,
    variable_sorts: dict[str, str],
    sort: str,
    parser: ModuleType,
) -> tuple[dict[str, Fraction], Fraction]:
    number = parser.parse_number(term.symbol) if not term.arguments else None
    if number is not None:
        return {}, number
    if not term.arguments and term.symbol in variable_sorts:
        if variable_sorts[term.symbol] != sort:
            raise ValueError("mixed_variable_sort")
        return {term.symbol: Fraction(1)}, Fraction(0)
    if term.symbol == "$sum" and len(term.arguments) == 2:
        return add_forms(
            linearize(term.arguments[0], variable_sorts, sort, parser),
            linearize(term.arguments[1], variable_sorts, sort, parser),
        )
    if term.symbol == "$difference" and len(term.arguments) == 2:
        return add_forms(
            linearize(term.arguments[0], variable_sorts, sort, parser),
            scale_form(
                linearize(term.arguments[1], variable_sorts, sort, parser),
                Fraction(-1),
            ),
        )
    if term.symbol == "$uminus" and len(term.arguments) == 1:
        return scale_form(
            linearize(term.arguments[0], variable_sorts, sort, parser),
            Fraction(-1),
        )
    if term.symbol == "$product" and len(term.arguments) == 2:
        left = linearize(term.arguments[0], variable_sorts, sort, parser)
        right = linearize(term.arguments[1], variable_sorts, sort, parser)
        if not left[0]:
            return scale_form(right, left[1])
        if not right[0]:
            return scale_form(left, right[1])
        raise ValueError("nonlinear_product")
    if term.symbol == "$quotient" and len(term.arguments) == 2:
        numerator = linearize(term.arguments[0], variable_sorts, sort, parser)
        denominator = linearize(term.arguments[1], variable_sorts, sort, parser)
        if denominator[0]:
            raise ValueError("division_by_nonconstant")
        if denominator[1] == 0:
            raise ValueError("division_by_zero")
        return scale_form(numerator, Fraction(1) / denominator[1])
    if term.symbol == "$to_real" and len(term.arguments) == 1:
        argument = term.arguments[0]
        number = parser.parse_number(argument.symbol) if not argument.arguments else None
        if number is not None and sort == "$real":
            return {}, number
    raise ValueError("uninterpreted_arithmetic_term")


def contains_variable(term: Any, variables: set[str]) -> bool:
    return (
        (not term.arguments and term.symbol in variables)
        or any(contains_variable(argument, variables) for argument in term.arguments)
    )


def arithmetic_sort(
    left: Any,
    right: Any,
    variable_sorts: dict[str, str],
    declarations: dict[str, str],
    parser: ModuleType,
) -> str:
    def infer(term: Any) -> str | None:
        if not term.arguments and term.symbol in variable_sorts:
            return variable_sorts[term.symbol]
        number = parser.parse_number(term.symbol) if not term.arguments else None
        if number is not None:
            return None
        if term.symbol == "$to_real":
            return "$real"
        nested = {value for argument in term.arguments if (value := infer(argument))}
        if len(nested) == 1:
            return next(iter(nested))
        if len(nested) > 1:
            raise ValueError("mixed_arithmetic_sort")
        return declarations.get(term.symbol)

    left_sort = infer(left)
    right_sort = infer(right)
    sorts = {value for value in (left_sort, right_sort) if value is not None}
    if len(sorts) != 1:
        raise ValueError("unknown_or_mixed_arithmetic_sort")
    sort = next(iter(sorts))
    if sort not in ARITHMETIC_SORTS:
        raise ValueError("unsupported_arithmetic_sort")
    return sort


def arithmetic_literal(
    literal: Any,
    variable_sorts: dict[str, str],
    declarations: dict[str, str],
    parser: ModuleType,
) -> dict[str, Any]:
    if literal.atom.relation not in {"lt", "le", "gt", "ge"}:
        raise ValueError("not_an_order_literal")
    left, right = literal.atom.arguments
    sort = arithmetic_sort(left, right, variable_sorts, declarations, parser)
    form = add_forms(
        linearize(left, variable_sorts, sort, parser),
        scale_form(
            linearize(right, variable_sorts, sort, parser),
            Fraction(-1),
        ),
    )
    relation = literal.atom.relation
    if not literal.positive:
        relation = {"lt": "ge", "le": "gt", "gt": "le", "ge": "lt"}[relation]
    if relation in {"lt", "le"}:
        form = scale_form(form, Fraction(-1))
    return {
        "kind": "arith",
        "sort": ARITHMETIC_SORTS[sort],
        "strict": relation in {"lt", "gt"},
        "coefficients": {
            variable: fraction_text(value)
            for variable, value in sorted(form[0].items())
        },
        "constant": fraction_text(form[1]),
    }


def extract_clause(
    literals: list[Any],
    variable_sorts: dict[str, str],
    declarations: dict[str, str],
    parser: ModuleType,
) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    variables = set(variable_sorts)
    for literal in literals:
        if literal.atom.relation in {"lt", "le", "gt", "ge"}:
            result.append(
                arithmetic_literal(
                    literal,
                    variable_sorts,
                    declarations,
                    parser,
                )
            )
            continue
        if literal.atom.relation == "eq":
            raise ValueError("equality_or_disequality")
        if any(contains_variable(argument, variables) for argument in literal.atom.arguments):
            raise ValueError("nonground_opaque_literal")
        result.append(
            {
                "kind": "prop",
                "name": literal.atom.canonical(),
                "positive": literal.positive,
            }
        )
    return result


def parse_transcript(
    text: str,
    parser: ModuleType,
) -> tuple[list[dict[str, Any]], Counter[str], int]:
    declarations: dict[str, str] = {}
    raw_clauses: list[tuple[str, str]] = []
    exclusions: Counter[str] = Counter()
    for statement in parser.split_statements(text):
        prefix, fields = parser.statement_fields(statement)
        if prefix == "tff" and len(fields) >= 3 and fields[1].strip() == "type":
            declaration = declaration_result_sort(fields[2], parser)
            if declaration is not None:
                declarations[declaration[0]] = declaration[1]
            continue
        if prefix not in {"tff", "tcf", "cnf"} or len(fields) < 3:
            continue
        raw_clauses.append((fields[0].strip(), fields[2]))

    clauses: list[dict[str, Any]] = []
    for name, body in raw_clauses:
        try:
            variable_sorts, literal_texts = parser.parse_quantified_clause(body)
            if any(sort not in ARITHMETIC_SORTS for sort in variable_sorts.values()):
                raise ValueError("unsupported_quantified_sort")
            literals = [parser.parse_literal(value) for value in literal_texts]
            extracted = extract_clause(
                literals,
                variable_sorts,
                declarations,
                parser,
            )
            if len(extracted) > Bounds().max_literals_per_clause:
                raise ValueError("literals_per_clause")
            if any(
                literal["kind"] == "arith"
                and len(literal["coefficients"]) > Bounds().max_variables_per_literal
                for literal in extracted
            ):
                raise ValueError("variables_per_literal")
            clauses.append({"id": name, "literals": extracted})
        except (ValueError, parser.TraceError) as error:
            exclusions[str(error)] += 1
    return clauses, exclusions, len(raw_clauses)


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--selection", required=True, type=Path)
    argument_parser.add_argument("--capture-root", required=True, type=Path)
    argument_parser.add_argument("--output", required=True, type=Path)
    arguments = argument_parser.parse_args()
    selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
    capture = json.loads(
        (arguments.capture_root / "capture.json").read_text(encoding="utf-8")
    )
    capture_by_id = {record["problem_id"]: record for record in capture["records"]}
    parser = load_parser()
    workloads: list[dict[str, Any]] = []
    source_reports: list[dict[str, Any]] = []
    all_exclusions: Counter[str] = Counter()
    for source in selection["selected"]:
        record = capture_by_id[source["problem_id"]]
        if record["timed_out"] or record["return_code"] != 0:
            source_reports.append(
                {**source, "status": "capture_failed", "eligible_clauses": 0}
            )
            continue
        transcript = (
            arguments.capture_root / source["problem_id"] / "stdout.txt"
        ).read_text(encoding="utf-8")
        clauses, exclusions, raw_count = parse_transcript(transcript, parser)
        all_exclusions.update(exclusions)
        for chunk_index in range(0, len(clauses), Bounds().max_input_clauses):
            chunk = clauses[
                chunk_index : chunk_index + Bounds().max_input_clauses
            ]
            workloads.append(
                {
                    "id": (
                        f"production_{source['partition']}_"
                        f"{source['problem_id']}_"
                        f"{chunk_index // Bounds().max_input_clauses:02d}"
                    ),
                    "source": "production_subset",
                    "source_problem_id": source["problem_id"],
                    "partition": source["partition"],
                    "template_family": f"production_{source['family']}",
                    "family": source["family"],
                    "expected": "diagnostic",
                    "supported": True,
                    "clauses": chunk,
                }
            )
        source_reports.append(
            {
                **source,
                "status": "extracted",
                "raw_clauses": raw_count,
                "eligible_clauses": len(clauses),
                "workload_chunks": (
                    (len(clauses) + Bounds().max_input_clauses - 1)
                    // Bounds().max_input_clauses
                ),
                "exclusions": dict(sorted(exclusions.items())),
            }
        )
    corpus = {
        "schema": SCHEMA,
        "source": "production_subset",
        "selection_schema": selection["schema"],
        "workloads": workloads,
    }
    load_corpus(corpus)
    result = {
        "corpus": corpus,
        "summary": {
            "selected_sources": len(selection["selected"]),
            "eligible_sources": sum(
                report.get("eligible_clauses", 0) > 0 for report in source_reports
            ),
            "eligible_clauses": sum(
                report.get("eligible_clauses", 0) for report in source_reports
            ),
            "eligible_families": len(
                {
                    report["family"]
                    for report in source_reports
                    if report.get("eligible_clauses", 0) > 0
                }
            ),
            "workloads": len(workloads),
            "exclusions": dict(sorted(all_exclusions.items())),
        },
        "sources": source_reports,
    }
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(result["summary"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
