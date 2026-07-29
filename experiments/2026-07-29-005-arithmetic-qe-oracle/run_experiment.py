#!/usr/bin/env python3
"""Run the independent bounded arithmetic/QE differential experiment."""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import platform
import random
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from fractions import Fraction
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
VALIDATION_ROOT = REPO_ROOT / "tools" / "validation"
sys.path.insert(0, str(VALIDATION_ROOT))

from arithmetic_qe_oracle import (  # noqa: E402
    EXACT_SEMANTICS,
    TRUNCATING_SEMANTICS,
    Atom,
    BoundedQuery,
    Decision,
    DifferentialStatus,
    Expr,
    Formula,
    FormulaKind,
    QuantifierSort,
    Relation,
    SmtLibProcessOracle,
    compare_with_external_solver,
    decide_exact,
    evaluate_expr,
    floor_fraction,
    make_atom,
    quotient_remainder,
    rational_lcm,
    replace_ceil_with_floor,
    shrink_query,
    weaken_strict_relations,
)


def parse_integer(text: str) -> int:
    return int(text, 0)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fraction_text(value: Fraction | None) -> str | None:
    if value is None:
        return None
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def query_outcome(query: BoundedQuery, solver: SmtLibProcessOracle) -> dict[str, Any]:
    differential = compare_with_external_solver(query, solver)
    return {
        "query": query.describe(),
        "status": differential.status.value,
        "exact": {
            "decision": differential.exact.decision.value,
            "reason": differential.exact.reason,
            "witness": fraction_text(differential.exact.witness),
            "checked_cells": differential.exact.checked_cells,
            "critical_points": differential.exact.critical_points,
        },
        "external": {
            "decision": differential.external.decision.value,
            "reason": differential.external.reason,
            "returncode": differential.external.returncode,
            "stdout": differential.external.stdout,
            "stderr": differential.external.stderr,
        },
    }


def exact_helper_checks(generator: random.Random) -> dict[str, Any]:
    vectors = [
        ([Fraction(1, 3), Fraction(1, 2)], Fraction(1)),
        ([Fraction(2, 3), Fraction(4, 5)], Fraction(4)),
        ([Fraction(3, 10), Fraction(9, 14)], Fraction(9, 2)),
    ]
    vector_results = []
    for inputs, expected in vectors:
        actual = rational_lcm(inputs)
        vector_results.append(
            {
                "inputs": [fraction_text(value) for value in inputs],
                "expected": fraction_text(expected),
                "actual": fraction_text(actual),
                "passed": actual == expected,
            }
        )

    property_failures: list[dict[str, str]] = []
    for _ in range(2_000):
        period = Fraction(generator.randint(1, 100), generator.randint(1, 100))
        value = Fraction(generator.randint(-1_000, 1_000), generator.randint(1, 100))
        quotient, remainder = quotient_remainder(value, period)
        if not (
            value == period * quotient + remainder
            and Fraction(0) <= remainder < period
        ):
            property_failures.append(
                {
                    "value": fraction_text(value) or "0",
                    "period": fraction_text(period) or "0",
                }
            )
    return {
        "lcm_vectors": vector_results,
        "quotient_remainder_cases": 2_000,
        "quotient_remainder_failures": property_failures,
        "passed": all(item["passed"] for item in vector_results)
        and not property_failures,
    }


def solver_semantics_probes(solver: SmtLibProcessOracle) -> dict[str, Any]:
    x = Expr.variable("x")
    probes = [
        BoundedQuery.create(
            "x",
            Fraction(-1, 2),
            Fraction(-1, 2),
            make_atom(Expr.floor(x), Relation.EQ, -1),
            name="negative-half-floor-is-minus-one",
        ),
        BoundedQuery.create(
            "x",
            Fraction(-1, 2),
            Fraction(-1, 2),
            make_atom(Expr.floor(x), Relation.EQ, 0),
            name="negative-half-floor-is-not-zero",
        ),
        BoundedQuery.create(
            "x",
            Fraction(-1, 2),
            Fraction(-1, 2),
            make_atom(Expr.ceil(x), Relation.EQ, 0),
            name="negative-half-ceil-is-zero",
        ),
        BoundedQuery.create(
            "x",
            -1,
            0,
            Formula.and_(
                make_atom(x, Relation.GT, -1),
                make_atom(x, Relation.LT, 0),
            ),
            sort=QuantifierSort.INTEGER,
            name="negative-open-unit-interval-has-no-integer",
        ),
    ]
    expected = [
        DifferentialStatus.SAT,
        DifferentialStatus.UNSAT,
        DifferentialStatus.SAT,
        DifferentialStatus.UNSAT,
    ]
    results = [query_outcome(query, solver) for query in probes]
    passed = all(
        result["status"] == expected_status.value
        for result, expected_status in zip(results, expected, strict=True)
    )
    return {"results": results, "passed": passed}


def motivating_formula(
    a_value: Fraction,
    c_value: Fraction,
    *,
    ceiling_typo: bool = False,
) -> BoundedQuery:
    x = Expr.variable("x")
    a = Expr.variable("a")
    c = Expr.variable("c")
    floor_a = Expr.floor(a)
    lower = Expr.add(floor_a, Expr.constant(Fraction(1, 3)))
    upper = Expr.add(floor_a, Expr.constant(Fraction(2, 3)))
    rounded_x = Expr.ceil(x)
    if ceiling_typo:
        rounded_x = replace_ceil_with_floor(rounded_x)
    periodic = Expr.subtract(rounded_x, x)
    formula = Formula.and_(
        make_atom(lower, Relation.LE, x),
        make_atom(x, Relation.LE, upper),
        make_atom(periodic, Relation.GE, c),
    )
    center = floor_fraction(a_value)
    return BoundedQuery.create(
        "x",
        center - 1,
        center + 2,
        formula,
        parameters={"a": a_value, "c": c_value},
        name=(
            "motivating-example-floor-typo"
            if ceiling_typo
            else "motivating-example-correct"
        ),
    )


def errata_checks(solver: SmtLibProcessOracle) -> dict[str, Any]:
    x = Expr.variable("x")

    positive_tail = BoundedQuery.create(
        "x",
        0,
        1,
        make_atom(x, Relation.GT, 0),
        name="erratum-1-aperiodic-positive-tail",
    )
    swapped_tail = BoundedQuery.create(
        "x",
        0,
        0,
        make_atom(x, Relation.GT, 0),
        name="erratum-1-printed-swap-base-substitution",
    )
    periodic_term = Expr.subtract(Expr.ceil(x), x)
    periodic_query = BoundedQuery.create(
        "x",
        0,
        1,
        make_atom(periodic_term, Relation.GE, Fraction(3, 4)),
        name="erratum-1-periodic-base-must-be-preserved",
    )

    parameter_values = [
        Fraction(-5, 2),
        Fraction(-1),
        Fraction(-1, 3),
        Fraction(0),
        Fraction(1, 3),
        Fraction(2),
    ]
    c_values = [
        Fraction(-1),
        Fraction(0),
        Fraction(2, 3),
        Fraction(3, 4),
        Fraction(2),
    ]
    motivating_results: list[dict[str, Any]] = []
    typo_disagreements = 0
    correct_failures = 0
    for a_value in parameter_values:
        for c_value in c_values:
            expected = Decision.SAT if c_value <= Fraction(2, 3) else Decision.UNSAT
            correct_query = motivating_formula(a_value, c_value)
            typo_query = motivating_formula(a_value, c_value, ceiling_typo=True)
            correct = decide_exact(correct_query)
            typo = decide_exact(typo_query)
            if correct.decision is not expected:
                correct_failures += 1
            if typo.decision is not expected:
                typo_disagreements += 1
            motivating_results.append(
                {
                    "a": fraction_text(a_value),
                    "c": fraction_text(c_value),
                    "expected": expected.value,
                    "correct": correct.decision.value,
                    "floor_typo": typo.decision.value,
                }
            )

    witness_checks: list[dict[str, Any]] = []
    sign_faults = 0
    for a_value in parameter_values:
        query = motivating_formula(a_value, Fraction(1, 2))
        environment = query.parameter_environment()
        correct_witness = Fraction(floor_fraction(a_value)) + Fraction(1, 3)
        faulty_witness = -Fraction(floor_fraction(a_value)) - Fraction(1, 3)
        environment["x"] = correct_witness
        correct_holds = query.formula.evaluate(environment)
        environment["x"] = faulty_witness
        faulty_holds = query.formula.evaluate(environment)
        if correct_holds and not faulty_holds:
            sign_faults += 1
        witness_checks.append(
            {
                "a": fraction_text(a_value),
                "correct_witness": fraction_text(correct_witness),
                "faulty_witness": fraction_text(faulty_witness),
                "correct_holds": correct_holds,
                "faulty_holds": faulty_holds,
            }
        )

    blocking_samples = []
    blocking_faults = 0
    for sample in [Fraction(-2), Fraction(-1), Fraction(0), Fraction(1), Fraction(2)]:
        correct_disequality = sample != 0
        printed_equality = sample == 0
        if correct_disequality != printed_equality:
            blocking_faults += 1
        blocking_samples.append(
            {
                "sample": fraction_text(sample),
                "correct_disequality": correct_disequality,
                "printed_equality": printed_equality,
            }
        )

    tail_correct = query_outcome(positive_tail, solver)
    tail_faulty = query_outcome(swapped_tail, solver)
    periodic = query_outcome(periodic_query, solver)
    return {
        "erratum_1_periodic_aperiodic_swap": {
            "correct_positive_tail": tail_correct,
            "printed_swap_seed": tail_faulty,
            "periodic_base_preservation": periodic,
            "detected": (
                tail_correct["status"] == DifferentialStatus.SAT.value
                and tail_faulty["status"] == DifferentialStatus.UNSAT.value
                and periodic["status"] == DifferentialStatus.SAT.value
            ),
        },
        "erratum_2_ceiling_changed_to_floor": {
            "matrix": motivating_results,
            "correct_failures": correct_failures,
            "faulty_disagreements": typo_disagreements,
            "detected": correct_failures == 0 and typo_disagreements > 0,
        },
        "erratum_3_lower_witness_sign": {
            "matrix": witness_checks,
            "faults_detected": sign_faults,
            "detected": sign_faults > 0
            and all(item["correct_holds"] for item in witness_checks),
        },
        "erratum_4_blocking_relation": {
            "matrix": blocking_samples,
            "faults_detected": blocking_faults,
            "detected": blocking_faults == len(blocking_samples),
        },
    }


def classification_checks() -> dict[str, Any]:
    x = Expr.variable("x")
    sat_query = BoundedQuery.create(
        "x",
        -1,
        1,
        make_atom(x, Relation.EQ, 0),
        name="classification-sat",
    )
    unsat_query = BoundedQuery.create(
        "x",
        -1,
        1,
        Formula.and_(
            make_atom(x, Relation.GT, 0),
            make_atom(x, Relation.LE, 0),
        ),
        name="classification-unsat",
    )

    def fixture(verdict: str) -> SmtLibProcessOracle:
        code = (
            "import sys;"
            "sys.stdin.read();"
            f"print({verdict!r})"
        )
        return SmtLibProcessOracle((sys.executable, "-c", code))

    outcomes = {
        "sat": compare_with_external_solver(sat_query, fixture("sat")).status.value,
        "unsat": compare_with_external_solver(
            unsat_query, fixture("unsat")
        ).status.value,
        "unknown": compare_with_external_solver(
            sat_query, fixture("unknown")
        ).status.value,
        "disagreement": compare_with_external_solver(
            sat_query, fixture("unsat")
        ).status.value,
        "error": compare_with_external_solver(
            sat_query, fixture("malformed")
        ).status.value,
    }
    expected = {
        "sat": "sat",
        "unsat": "unsat",
        "unknown": "unknown",
        "disagreement": "disagreement",
        "error": "error",
    }
    return {"outcomes": outcomes, "expected": expected, "passed": outcomes == expected}


def random_fraction(generator: random.Random, *, allow_zero: bool = True) -> Fraction:
    while True:
        value = Fraction(generator.randint(-4, 4), generator.randint(1, 4))
        if allow_zero or value != 0:
            return value


def random_term(
    generator: random.Random,
    variables: tuple[Expr, ...],
    depth: int,
) -> Expr:
    if depth <= 0 or generator.random() < 0.30:
        choice = generator.randrange(len(variables) + 1)
        if choice < len(variables):
            return variables[choice]
        return Expr.constant(random_fraction(generator))
    branch = generator.randrange(5)
    if branch == 0:
        return Expr.add(
            random_term(generator, variables, depth - 1),
            random_term(generator, variables, depth - 1),
        )
    if branch == 1:
        return Expr.scale(
            random_fraction(generator, allow_zero=False),
            random_term(generator, variables, depth - 1),
        )
    if branch == 2:
        return Expr.floor(random_term(generator, variables, depth - 1))
    if branch == 3:
        return Expr.ceil(random_term(generator, variables, depth - 1))
    return Expr.add(
        random_term(generator, variables, depth - 1),
        Expr.scale(
            random_fraction(generator, allow_zero=False),
            random_term(generator, variables, depth - 1),
        ),
    )


def random_formula(generator: random.Random) -> tuple[Formula, dict[str, Fraction]]:
    x = Expr.variable("x")
    a = Expr.variable("a")
    variables = (x, a)
    atoms: list[Formula] = []
    relations = tuple(Relation)
    for _ in range(generator.randint(1, 3)):
        left = random_term(generator, variables, generator.randint(1, 3))
        right = random_term(generator, variables, generator.randint(0, 2))
        atoms.append(make_atom(left, generator.choice(relations), right))
    if len(atoms) == 1:
        formula = atoms[0]
    elif generator.random() < 0.75:
        formula = Formula.and_(*atoms)
    else:
        formula = Formula.or_(*atoms)
    if generator.random() < 0.15:
        formula = Formula.not_(formula)
    return formula, {"a": random_fraction(generator)}


def random_differential_checks(
    generator: random.Random,
    solver: SmtLibProcessOracle,
    cases: int,
) -> dict[str, Any]:
    counts: Counter[str] = Counter()
    failures: list[dict[str, Any]] = []
    maximum_points = 0
    maximum_cells = 0
    for index in range(cases):
        formula, parameters = random_formula(generator)
        query = BoundedQuery.create(
            "x",
            -3,
            3,
            formula,
            parameters=parameters,
            sort=(
                QuantifierSort.INTEGER
                if generator.random() < 0.20
                else QuantifierSort.REAL
            ),
            name=f"generated-{index:04d}",
        )
        outcome = query_outcome(query, solver)
        counts[outcome["status"]] += 1
        maximum_points = max(
            maximum_points,
            outcome["exact"]["critical_points"],
        )
        maximum_cells = max(maximum_cells, outcome["exact"]["checked_cells"])
        if outcome["status"] not in {
            DifferentialStatus.SAT.value,
            DifferentialStatus.UNSAT.value,
        }:
            failures.append(outcome)
    return {
        "cases": cases,
        "counts": dict(sorted(counts.items())),
        "maximum_critical_points": maximum_points,
        "maximum_checked_cells": maximum_cells,
        "non_agreements": failures,
        "passed": not failures,
    }


def metamorphic_checks(generator: random.Random) -> dict[str, Any]:
    x = Expr.variable("x")
    a = Expr.variable("a")
    failures: list[dict[str, Any]] = []
    evaluation_count = 0
    for index in range(300):
        term = random_term(generator, (x, a), 3)
        ceil_rewrite = Expr.negate(Expr.floor(Expr.negate(term)))
        commuted = Expr.add(Expr.constant(0), term)
        environment = {
            "x": random_fraction(generator),
            "a": random_fraction(generator),
        }
        direct = evaluate_expr(Expr.ceil(term), environment)
        rewritten = evaluate_expr(ceil_rewrite, environment)
        zero_inserted = evaluate_expr(commuted, environment)
        original = evaluate_expr(term, environment)
        evaluation_count += 2
        if direct != rewritten or original != zero_inserted:
            failures.append(
                {
                    "case": index,
                    "term": str(term),
                    "environment": {
                        name: fraction_text(value)
                        for name, value in environment.items()
                    },
                    "ceil": fraction_text(direct),
                    "ceil_rewrite": fraction_text(rewritten),
                    "original": fraction_text(original),
                    "zero_inserted": fraction_text(zero_inserted),
                }
            )

    query_failures = []
    for index in range(200):
        formula, parameters = random_formula(generator)
        query = BoundedQuery.create(
            "x",
            -3,
            3,
            formula,
            parameters=parameters,
            name=f"metamorphic-{index:04d}",
        )
        original = decide_exact(query).decision
        transformed = formula
        if formula.kind is FormulaKind.AND:
            transformed = Formula.and_(
                *reversed(formula.children),
                formula.children[0],
            )
        elif formula.kind is FormulaKind.OR:
            transformed = Formula.or_(
                *reversed(formula.children),
                formula.children[0],
            )
        duplicate = decide_exact(query.with_formula(transformed)).decision
        if original is not duplicate:
            query_failures.append(
                {
                    "query": query.describe(),
                    "transformed": str(transformed),
                    "original": original.value,
                    "transformed_decision": duplicate.value,
                }
            )
    return {
        "expression_evaluations": evaluation_count,
        "query_cases": 200,
        "expression_failures": failures,
        "query_failures": query_failures,
        "passed": not failures and not query_failures,
    }


def qe_golden_checks() -> dict[str, Any]:
    x = Expr.variable("x")
    a = Expr.variable("a")
    b = Expr.variable("b")
    lra_matrix = []
    lra_failures = 0
    values = [
        Fraction(-2),
        Fraction(-1, 2),
        Fraction(0),
        Fraction(1, 2),
        Fraction(2),
    ]
    for a_value in values:
        for b_value in values:
            query = BoundedQuery.create(
                "x",
                -4,
                4,
                Formula.and_(
                    make_atom(a, Relation.LT, x),
                    make_atom(x, Relation.LE, b),
                ),
                parameters={"a": a_value, "b": b_value},
                name="pure-lra-qe",
            )
            decision = decide_exact(query).decision
            expected = Decision.SAT if a_value < b_value else Decision.UNSAT
            if decision is not expected:
                lra_failures += 1
            lra_matrix.append(
                {
                    "a": fraction_text(a_value),
                    "b": fraction_text(b_value),
                    "expected": expected.value,
                    "actual": decision.value,
                }
            )

    integer_matrix = []
    integer_failures = 0
    for a_value in values:
        integer_query = BoundedQuery.create(
            "x",
            -4,
            4,
            Formula.and_(
                make_atom(a, Relation.LT, x),
                make_atom(x, Relation.LT, Expr.add(a, Expr.constant(1))),
            ),
            parameters={"a": a_value},
            sort=QuantifierSort.INTEGER,
            name="typed-integer-open-interval",
        )
        encoded_query = dataclasses.replace(
            integer_query,
            sort=QuantifierSort.REAL,
            formula=Formula.and_(
                make_atom(x, Relation.EQ, Expr.floor(x)),
                integer_query.formula,
            ),
            name="floor-encoded-open-interval",
        )
        integer_decision = decide_exact(integer_query).decision
        encoded_decision = decide_exact(encoded_query).decision
        if integer_decision is not encoded_decision:
            integer_failures += 1
        integer_matrix.append(
            {
                "a": fraction_text(a_value),
                "typed_integer": integer_decision.value,
                "floor_encoded_real": encoded_decision.value,
            }
        )
    return {
        "pure_lra": {
            "matrix": lra_matrix,
            "failures": lra_failures,
        },
        "typed_integer_adapter": {
            "matrix": integer_matrix,
            "failures": integer_failures,
        },
        "passed": lra_failures == 0 and integer_failures == 0,
    }


def mutation_and_shrinking_checks() -> dict[str, Any]:
    x = Expr.variable("x")
    negative_floor = BoundedQuery.create(
        "x",
        Fraction(-1, 2),
        Fraction(-1, 2),
        Formula.and_(
            make_atom(Expr.floor(x), Relation.EQ, -1),
            make_atom(x, Relation.LT, 0),
            make_atom(x, Relation.LE, 0),
            make_atom(Expr.add(x, Expr.constant(0)), Relation.LT, 1),
        ),
        name="seed-negative-floor-truncation",
    )

    def rounding_disagreement(query: BoundedQuery) -> bool:
        correct = decide_exact(query, semantics=EXACT_SEMANTICS)
        faulty = decide_exact(query, semantics=TRUNCATING_SEMANTICS)
        return correct.decision is not faulty.decision

    minimized, attempts = shrink_query(negative_floor, rounding_disagreement)

    strict = BoundedQuery.create(
        "x",
        0,
        0,
        make_atom(x, Relation.GT, 0),
        name="seed-strictness",
    )
    strict_faulty = strict.with_formula(weaken_strict_relations(strict.formula))

    ceil_term = Expr.subtract(Expr.ceil(x), x)
    ceil_query = BoundedQuery.create(
        "x",
        0,
        1,
        make_atom(ceil_term, Relation.GE, Fraction(2, 3)),
        name="seed-ceiling-floor",
    )
    floor_fault = ceil_query.with_formula(
        make_atom(
            replace_ceil_with_floor(ceil_term),
            Relation.GE,
            Fraction(2, 3),
        )
    )

    seeds = {
        "negative_floor_truncation": {
            "correct": decide_exact(negative_floor).decision.value,
            "faulty": decide_exact(
                negative_floor,
                semantics=TRUNCATING_SEMANTICS,
            ).decision.value,
            "detected": rounding_disagreement(negative_floor),
        },
        "strictness_weakened": {
            "correct": decide_exact(strict).decision.value,
            "faulty": decide_exact(strict_faulty).decision.value,
            "detected": (
                decide_exact(strict).decision
                is not decide_exact(strict_faulty).decision
            ),
        },
        "ceiling_changed_to_floor": {
            "correct": decide_exact(ceil_query).decision.value,
            "faulty": decide_exact(floor_fault).decision.value,
            "detected": (
                decide_exact(ceil_query).decision
                is not decide_exact(floor_fault).decision
            ),
        },
    }
    return {
        "seeds": seeds,
        "shrink": {
            "before": negative_floor.describe(),
            "after": minimized.describe(),
            "attempts": attempts,
            "preserved_disagreement": rounding_disagreement(minimized),
        },
        "passed": all(seed["detected"] for seed in seeds.values())
        and minimized.formula.complexity() < negative_floor.formula.complexity()
        and rounding_disagreement(minimized),
    }


def solver_version(executable: Path) -> str:
    completed = subprocess.run(
        (str(executable), "-version"),
        text=True,
        capture_output=True,
        timeout=10,
        check=True,
    )
    return (completed.stdout or completed.stderr).strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--z3", type=Path, required=True)
    parser.add_argument("--seed", type=parse_integer, default=0x5A172026)
    parser.add_argument("--cases", type=int, default=500)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    if arguments.cases <= 0:
        parser.error("--cases must be positive")
    z3 = arguments.z3.resolve()
    if not z3.is_file():
        parser.error(f"--z3 is not a file: {z3}")

    generator = random.Random(arguments.seed)
    solver = SmtLibProcessOracle.z3(z3, timeout_seconds=10.0)
    sections = {
        "exact_helpers": exact_helper_checks(generator),
        "solver_semantics": solver_semantics_probes(solver),
        "classification": classification_checks(),
        "known_errata": errata_checks(solver),
        "qe_golden": qe_golden_checks(),
        "mutations_and_shrinking": mutation_and_shrinking_checks(),
        "metamorphic": metamorphic_checks(generator),
        "random_differential": random_differential_checks(
            generator,
            solver,
            arguments.cases,
        ),
    }
    errata_passed = all(
        item["detected"] for item in sections["known_errata"].values()
    )
    passed = (
        all(
            section["passed"]
            for name, section in sections.items()
            if name not in {"known_errata"}
        )
        and errata_passed
    )
    report = {
        "schema_version": 1,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "bead": "E_Rust_Port-9jt.5.5",
        "independence": {
            "imports_umlaut": False,
            "uses_viras_implementation": False,
            "exact_oracle": "Python fractions plus bounded cell decomposition",
            "external_oracle": "separate SMT-LIB process",
        },
        "environment": {
            "platform": platform.platform(),
            "python": sys.version,
            "z3_path": str(z3),
            "z3_sha256": sha256(z3),
            "z3_version": solver_version(z3),
            "oracle_sha256": sha256(
                VALIDATION_ROOT / "arithmetic_qe_oracle.py"
            ),
        },
        "configuration": {
            "seed": arguments.seed,
            "seed_hex": hex(arguments.seed),
            "random_differential_cases": arguments.cases,
        },
        "sections": sections,
        "passed": passed,
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "random_differential": sections["random_differential"]["counts"],
                "errata_detected": errata_passed,
                "output": str(arguments.output),
            },
            sort_keys=True,
        )
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
