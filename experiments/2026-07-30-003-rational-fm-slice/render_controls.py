#!/usr/bin/env python3
"""Render supported workloads to canonical SMT-LIB and TPTP controls."""

from __future__ import annotations

import argparse
import hashlib
import json
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable, Sequence

from fm_core import FmError, fraction, load_corpus


def safe_name(prefix: str, value: str) -> str:
    digest = hashlib.sha256(value.encode("utf-8")).hexdigest()[:16]
    return f"{prefix}_{digest}"


def all_propositions(workload: dict[str, Any]) -> list[str]:
    return sorted(
        {
            literal["name"]
            for clause in workload["clauses"]
            for literal in clause["literals"]
            if literal.get("kind") == "prop"
        }
    )


def rational_smt(value: Fraction) -> str:
    numerator = abs(value.numerator)
    if value.denominator == 1:
        atom = str(numerator)
    else:
        atom = f"(/ {numerator} {value.denominator})"
    return f"(- {atom})" if value < 0 else atom


def sum_smt(terms: Sequence[str]) -> str:
    if not terms:
        return "0"
    if len(terms) == 1:
        return terms[0]
    return f"(+ {' '.join(terms)})"


def arithmetic_smt(
    literal: dict[str, Any],
    variable_names: dict[str, str],
) -> str:
    terms: list[str] = []
    for variable, raw_coefficient in sorted(literal["coefficients"].items()):
        coefficient = fraction(raw_coefficient)
        symbol = variable_names[variable]
        if coefficient == 1:
            terms.append(symbol)
        elif coefficient == -1:
            terms.append(f"(- {symbol})")
        else:
            terms.append(f"(* {rational_smt(coefficient)} {symbol})")
    constant = fraction(literal["constant"])
    if constant:
        terms.append(rational_smt(constant))
    relation = ">" if literal["strict"] else ">="
    return f"({relation} {sum_smt(terms)} 0)"


def render_smt(workload: dict[str, Any]) -> str:
    proposition_names = {
        name: safe_name("p", name) for name in all_propositions(workload)
    }
    lines = [
        "(set-option :produce-models false)",
        "(set-option :random-seed 0)",
        "(set-logic AUFLIRA)",
    ]
    lines.extend(
        f"(declare-const {symbol} Bool)"
        for symbol in proposition_names.values()
    )
    for clause_index, clause in enumerate(workload["clauses"]):
        variables = sorted(
            {
                variable
                for literal in clause["literals"]
                if literal["kind"] == "arith"
                for variable in literal["coefficients"]
            }
        )
        sorts: dict[str, str] = {}
        for literal in clause["literals"]:
            if literal["kind"] != "arith":
                continue
            for variable in literal["coefficients"]:
                previous = sorts.setdefault(variable, literal["sort"])
                if previous != literal["sort"]:
                    raise FmError("a clause uses one variable at mixed sorts")
        variable_names = {
            variable: f"x_{clause_index}_{index}"
            for index, variable in enumerate(variables)
        }
        rendered_literals: list[str] = []
        for literal in clause["literals"]:
            if literal["kind"] == "prop":
                atom = proposition_names[literal["name"]]
                rendered_literals.append(
                    atom if literal["positive"] else f"(not {atom})"
                )
            elif literal["kind"] == "arith":
                rendered_literals.append(arithmetic_smt(literal, variable_names))
            else:
                raise FmError("unsupported literal reached SMT renderer")
        if not rendered_literals:
            body = "false"
        elif len(rendered_literals) == 1:
            body = rendered_literals[0]
        else:
            body = f"(or {' '.join(rendered_literals)})"
        if variables:
            declarations = " ".join(
                f"({variable_names[variable]} Real)"
                for variable in variables
            )
            body = f"(forall ({declarations}) {body})"
        lines.append(f"(assert (! {body} :named clause_{clause_index}))")
    lines.extend(["(check-sat)", "(exit)"])
    return "\n".join(lines) + "\n"


def rational_tptp(value: Fraction, sort: str) -> str:
    if value.denominator == 1:
        base = str(value.numerator)
        return f"$to_rat({base})" if sort == "Rat" else f"$to_real({base})"
    base = f"{value.numerator}/{value.denominator}"
    return base if sort == "Rat" else f"$to_real({base})"


def fold_binary(name: str, terms: Iterable[str], empty: str) -> str:
    iterator = iter(terms)
    try:
        result = next(iterator)
    except StopIteration:
        return empty
    for term in iterator:
        result = f"{name}({result},{term})"
    return result


def arithmetic_tptp(
    literal: dict[str, Any],
    variable_names: dict[str, str],
) -> str:
    terms: list[str] = []
    sort = literal["sort"]
    for variable, raw_coefficient in sorted(literal["coefficients"].items()):
        coefficient = fraction(raw_coefficient)
        symbol = variable_names[variable]
        if coefficient == 1:
            terms.append(symbol)
        elif coefficient == -1:
            terms.append(f"$uminus({symbol})")
        else:
            terms.append(
                f"$product({rational_tptp(coefficient, sort)},{symbol})"
            )
    constant = fraction(literal["constant"])
    if constant:
        terms.append(rational_tptp(constant, sort))
    zero = rational_tptp(Fraction(0), sort)
    polynomial = fold_binary("$sum", terms, zero)
    relation = "$greater" if literal["strict"] else "$greatereq"
    return f"{relation}({polynomial},{zero})"


def render_tptp(workload: dict[str, Any]) -> str:
    proposition_names = {
        name: safe_name("p", name) for name in all_propositions(workload)
    }
    lines = [
        f"tff({symbol}_type,type,{symbol}:$o)."
        for symbol in proposition_names.values()
    ]
    for clause_index, clause in enumerate(workload["clauses"]):
        variables = sorted(
            {
                variable
                for literal in clause["literals"]
                if literal["kind"] == "arith"
                for variable in literal["coefficients"]
            }
        )
        sorts: dict[str, str] = {}
        for literal in clause["literals"]:
            if literal["kind"] != "arith":
                continue
            for variable in literal["coefficients"]:
                previous = sorts.setdefault(variable, literal["sort"])
                if previous != literal["sort"]:
                    raise FmError("a clause uses one variable at mixed sorts")
        variable_names = {
            variable: f"X_{clause_index}_{index}"
            for index, variable in enumerate(variables)
        }
        rendered_literals: list[str] = []
        for literal in clause["literals"]:
            if literal["kind"] == "prop":
                atom = proposition_names[literal["name"]]
                rendered_literals.append(
                    atom if literal["positive"] else f"~ {atom}"
                )
            elif literal["kind"] == "arith":
                rendered_literals.append(arithmetic_tptp(literal, variable_names))
            else:
                raise FmError("unsupported literal reached TPTP renderer")
        body = " $false " if not rendered_literals else " | ".join(rendered_literals)
        if variables:
            declarations = ",".join(
                f"{variable_names[variable]}:${sorts[variable].lower()}"
                for variable in variables
            )
            body = f"! [{declarations}] : ({body})"
        lines.append(f"tff(clause_{clause_index},axiom,({body})).")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", type=Path)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument(
        "--partition",
        action="append",
        choices=["train", "validation", "test"],
    )
    arguments = parser.parse_args()
    loaded = json.loads(arguments.corpus.read_text(encoding="utf-8"))
    corpus = load_corpus(loaded.get("corpus", loaded))
    selected = set(arguments.partition or ["train", "validation", "test"])
    smt_directory = arguments.output_directory / "smt2"
    tptp_directory = arguments.output_directory / "tptp"
    smt_directory.mkdir(parents=True, exist_ok=True)
    tptp_directory.mkdir(parents=True, exist_ok=True)
    manifest: list[dict[str, Any]] = []
    for workload in corpus["workloads"]:
        if (
            workload["partition"] not in selected
            or not workload.get("supported", True)
        ):
            continue
        smt_path = smt_directory / f"{workload['id']}.smt2"
        tptp_path = tptp_directory / f"{workload['id']}.p"
        smt_path.write_text(render_smt(workload), encoding="utf-8")
        tptp_path.write_text(render_tptp(workload), encoding="utf-8")
        manifest.append(
            {
                "id": workload["id"],
                "partition": workload["partition"],
                "expected": workload["expected"],
                "smt2": smt_path.relative_to(
                    arguments.output_directory
                ).as_posix(),
                "tptp": tptp_path.relative_to(
                    arguments.output_directory
                ).as_posix(),
            }
        )
    manifest_path = arguments.output_directory / "control_manifest.json"
    manifest_path.write_text(
        json.dumps({"workloads": manifest}, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({"rendered": len(manifest)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
