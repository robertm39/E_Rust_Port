#!/usr/bin/env python3
"""Run the frozen base-VIRAS clean-room experiment."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import platform
import random
import subprocess
import sys
import unittest
from fractions import Fraction
from pathlib import Path
from typing import Any

EXPERIMENT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_DIR.parents[1]
sys.path.insert(0, str(EXPERIMENT_DIR))

import prototype as p  # noqa: E402
import support  # noqa: E402
import test_prototype  # noqa: E402


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def sha256_json(value: Any) -> str:
    return sha256_bytes(p.canonical_json(value).encode("ascii"))


def parse_seed(text: str) -> int:
    return int(text, 0)


def run_unit_tests() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromModule(test_prototype)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=1).run(suite)
    if not result.wasSuccessful():
        raise RuntimeError("focused unit tests failed:\n" + stream.getvalue())
    return result.testsRun


def run_z3_batch(
    executable: Path, formulas: list[str]
) -> tuple[str, list[str]]:
    version = subprocess.run(
        [str(executable), "-version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    commands = [
        "(set-option :print-success false)",
        "(declare-const x Real)",
        "(push 1)",
        "(assert (= (to_int (/ (- 1.0) 2.0)) (- 1)))",
        "(check-sat)",
        "(pop 1)",
    ]
    for formula in formulas:
        commands.extend(
            (
                "(push 1)",
                f"(assert {formula})",
                "(check-sat)",
                "(pop 1)",
            )
        )
    completed = subprocess.run(
        [str(executable), "-in", "-smt2"],
        input="\n".join(commands) + "\n",
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"Z3 batch failed ({completed.returncode}): "
            f"{completed.stderr}\n{completed.stdout}"
        )
    if "(error" in completed.stdout or "(error" in completed.stderr:
        raise RuntimeError(f"Z3 protocol error: {completed.stdout}\n{completed.stderr}")
    decisions = [
        line.strip()
        for line in completed.stdout.splitlines()
        if line.strip() in {"sat", "unsat", "unknown"}
    ]
    if len(decisions) != len(formulas) + 1:
        raise RuntimeError(
            f"Z3 returned {len(decisions)} decisions for "
            f"{len(formulas) + 1} queries"
        )
    if decisions[0] != "sat":
        raise RuntimeError("Z3 negative-floor semantic probe failed")
    if any(decision == "unknown" for decision in decisions[1:]):
        raise RuntimeError("Z3 returned unknown in the frozen supported fragment")
    return version, decisions[1:]


def grid_coverage(seed: int, cases: int) -> dict[str, Any]:
    rng = random.Random(seed)
    checked_actual_points = 0
    extra_covering_points = 0
    for index in range(cases):
        period = rng.choice(
            (
                Fraction(1, 5),
                Fraction(1, 3),
                Fraction(1, 2),
                Fraction(2, 3),
                Fraction(1),
                Fraction(3, 2),
                Fraction(2),
                Fraction(5, 2),
            )
        )
        base = Fraction(rng.randint(-8, 8), rng.choice((1, 2, 3, 5)))
        lower = Fraction(rng.randint(-12, 12), rng.choice((1, 2, 3, 5)))
        width = Fraction(rng.randint(0, 20), rng.choice((1, 2, 3, 5)))
        lower_closed = bool(rng.getrandbits(1))
        upper_closed = bool(rng.getrandbits(1))
        result = p.Kernel().grid_intersection(
            p.Grid(p.constant(base), period),
            p.constant(lower),
            width,
            lower_closed=lower_closed,
            upper_closed=upper_closed,
        )
        evaluated = {p.evaluate_term(term, {}) for term in result}
        actual = set()
        for integer in range(-1_000, 1_001):
            point = base + period * integer
            lower_ok = point >= lower if lower_closed else point > lower
            upper = lower + width
            upper_ok = point <= upper if upper_closed else point < upper
            if lower_ok and upper_ok:
                actual.add(point)
        if not actual <= evaluated:
            raise AssertionError(f"grid coverage failure at case {index}")
        checked_actual_points += len(actual)
        extra_covering_points += len(evaluated - actual)
    return {
        "cases": cases,
        "actual_points_checked": checked_actual_points,
        "safe_extra_points": extra_covering_points,
    }


def mutation_results() -> dict[str, Any]:
    x = p.variable("x")
    raw_negative_half = p.Term(
        p.TermOp.FLOOR,
        args=(p.constant(Fraction(-1, 2)),),
    )
    negative_floor = {
        "exact": p.evaluate_term(raw_negative_half, {}) == -1,
        "mutated": p.evaluate_term(
            raw_negative_half, {}, truncate_negative_floor=True
        )
        == -1,
    }

    def remainder(term: p.Term, period: int) -> p.Term:
        exact = Fraction(period)
        return p.add(
            term,
            p.scale(
                -exact,
                p.floor_term(p.scale(Fraction(1, period), term)),
            ),
        )

    residue = p.Literal(
        p.subtract(remainder(x, 2), remainder(p.constant(1), 2)),
        p.Relation.EQ,
    )
    infinity_input = [p.Literal(x, p.Relation.GE), residue]
    infinity_baseline = p.eliminate_exists("x", infinity_input)
    infinity_mutated = p.eliminate_exists(
        "x",
        infinity_input,
        mutations=p.Mutations(reverse_infinity_periodicity=True),
    )

    strict_input = [p.Literal(x, p.Relation.GT)]
    strict_baseline = p.eliminate_exists("x", strict_input)
    strict_mutated = p.eliminate_exists(
        "x",
        strict_input,
        mutations=p.Mutations(drop_epsilon_strictness=True),
    )

    omission_input = [
        p.Literal(x, p.Relation.EQ),
        p.Literal(x, p.Relation.GE),
    ]
    omission_baseline = p.eliminate_exists("x", omission_input)
    omission_mutated = p.eliminate_exists(
        "x",
        omission_input,
        mutations=p.Mutations(omit_last_candidate=True),
    )
    outcomes = {
        "negative_floor_truncation": negative_floor,
        "reversed_infinity_periodicity": {
            "baseline": infinity_baseline.formula.evaluate({}),
            "mutated": infinity_mutated.formula.evaluate({}),
        },
        "dropped_epsilon_strictness": {
            "baseline": strict_baseline.formula.evaluate({}),
            "mutated": strict_mutated.formula.evaluate({}),
        },
        "omitted_candidate": {
            "baseline": omission_baseline.formula.evaluate({}),
            "mutated": omission_mutated.formula.evaluate({}),
        },
    }
    if outcomes != {
        "negative_floor_truncation": {"exact": True, "mutated": False},
        "reversed_infinity_periodicity": {
            "baseline": True,
            "mutated": False,
        },
        "dropped_epsilon_strictness": {
            "baseline": True,
            "mutated": False,
        },
        "omitted_candidate": {"baseline": True, "mutated": False},
    }:
        raise AssertionError(f"mutation was not rejected: {outcomes}")
    return outcomes


def fail_closed_results() -> dict[str, Any]:
    x = p.variable("x")
    simple = [p.Literal(x, p.Relation.GE)]
    floor_literal = [p.Literal(p.floor_term(x), p.Relation.EQ)]
    outcomes = {
        "steps": p.eliminate_exists("x", simple, limits=p.Limits(max_steps=0)),
        "candidates": p.eliminate_exists(
            "x", simple, limits=p.Limits(max_candidates=0)
        ),
        "grids": p.eliminate_exists(
            "x", floor_literal, limits=p.Limits(max_grids=0)
        ),
        "grid_points": p.eliminate_exists(
            "x", floor_literal, limits=p.Limits(max_grid_points=0)
        ),
        "formula_nodes": p.eliminate_exists(
            "x", simple, limits=p.Limits(max_formula_nodes=0)
        ),
        "rational_bits": p.eliminate_exists(
            "x",
            [p.Literal(p.scale(2**100, x), p.Relation.GE)],
            limits=p.Limits(max_rational_bits=8),
        ),
        "unsupported": p.eliminate_exists("x", p.boolean(True)),
    }
    for name, outcome in outcomes.items():
        if outcome.status is not p.QEStatus.UNKNOWN or outcome.formula is not None:
            raise AssertionError(f"{name} did not fail closed")
    if outcomes["unsupported"].unknown_kind is not p.UnknownKind.UNSUPPORTED_FRAGMENT:
        raise AssertionError("unsupported formula used the wrong outcome class")
    for name in outcomes.keys() - {"unsupported"}:
        if outcomes[name].unknown_kind is not p.UnknownKind.RESOURCE_LIMIT:
            raise AssertionError(f"{name} used the wrong outcome class")
    return {
        name: {
            "kind": outcome.unknown_kind.value,
            "reason": outcome.reason,
        }
        for name, outcome in outcomes.items()
    }


def run(args: argparse.Namespace) -> dict[str, Any]:
    z3 = Path(args.z3).resolve()
    if not z3.is_file():
        raise FileNotFoundError(z3)
    unit_tests = run_unit_tests()
    generated = support.generate_cases(args.seed, args.cases)
    formulas = [case.oracle_formula.to_smt2() for case in generated]
    z3_version, z3_decisions = run_z3_batch(z3, formulas)

    records: list[dict[str, Any]] = []
    counts = {"sat": 0, "unsat": 0}
    max_usage = {
        "candidates": 0,
        "grids": 0,
        "grid_points": 0,
        "steps": 0,
    }
    for case, z3_decision, smt_formula in zip(
        generated, z3_decisions, formulas, strict=True
    ):
        exact = support.exact_oracle_decision(case)
        outcome = p.eliminate_exists("x", case.literals)
        if outcome.status is not p.QEStatus.SUCCESS:
            raise AssertionError(
                f"{case.case_id}: candidate returned {outcome.status}: {outcome.reason}"
            )
        assert outcome.formula is not None
        candidate = outcome.formula.evaluate({})
        expected_z3 = z3_decision == "sat"
        if candidate != exact or candidate != expected_z3:
            raise AssertionError(
                f"{case.case_id}: candidate={candidate}, exact={exact}, "
                f"z3={z3_decision}"
            )
        reversed_outcome = p.eliminate_exists(
            "x", tuple(reversed(case.literals))
        )
        duplicate_outcome = p.eliminate_exists(
            "x", (*case.literals, case.literals[-1])
        )
        assert reversed_outcome.formula is not None
        assert duplicate_outcome.formula is not None
        if (
            reversed_outcome.formula.evaluate({}) != candidate
            or duplicate_outcome.formula.evaluate({}) != candidate
        ):
            raise AssertionError(f"{case.case_id}: metamorphic disagreement")
        if outcome.formula.variables():
            raise AssertionError(f"{case.case_id}: output retains a variable")
        rendered = outcome.formula.render()
        if any(marker in rendered for marker in ("epsilon", "infinity", "*Z")):
            raise AssertionError(f"{case.case_id}: virtual marker leaked")

        decision = "sat" if candidate else "unsat"
        counts[decision] += 1
        usage = outcome.derivation["resource_usage"]
        for field in max_usage:
            max_usage[field] = max(max_usage[field], int(usage[field]))
        records.append(
            {
                "case_id": case.case_id,
                "input_sha256": sha256_bytes(smt_formula.encode("ascii")),
                "decision": decision,
                "candidate_formula_sha256": sha256_json(
                    outcome.formula.describe()
                ),
                "derivation_sha256": sha256_json(outcome.derivation),
                "candidate_count": len(outcome.derivation["candidates"]),
                "flatten_cases": [
                    item["case"]
                    for item in outcome.derivation["grid_flattening"]
                ],
                "metamorphic_order": True,
                "metamorphic_duplicate": True,
            }
        )
    if not counts["sat"] or not counts["unsat"]:
        raise AssertionError("generated corpus must exercise both decisions")

    source_files = (
        EXPERIMENT_DIR / "prototype.py",
        EXPERIMENT_DIR / "support.py",
        EXPERIMENT_DIR / "test_prototype.py",
        EXPERIMENT_DIR / "run_experiment.py",
        EXPERIMENT_DIR / "PREREGISTRATION.md",
        REPOSITORY_ROOT / "tools" / "validation" / "arithmetic_qe_oracle.py",
    )
    report = {
        "schema": "umlaut-base-viras-qe-prototype-v1",
        "bead": "E_Rust_Port-9jt.5.2",
        "passed": True,
        "declared_fragment": (
            "one existential real variable over a nonempty conjunction of "
            "normalized exact LIRA literals; free real parameters allowed"
        ),
        "excluded": [
            "arbitrary Boolean and quantifier wrapper",
            "typed TPTP import/export",
            "conflict-driven VIRAS",
            "production integration",
        ],
        "independence": {
            "candidate_imports_umlaut": False,
            "candidate_imports_independent_oracle": False,
            "unlicensed_viras_used": False,
            "source_hashes": {
                str(path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"):
                    sha256_file(path)
                for path in source_files
            },
        },
        "environment": {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "z3_version": z3_version,
            "z3_sha256": sha256_file(z3),
            "negative_floor_probe": "sat",
        },
        "configuration": {
            "seed": hex(args.seed),
            "generated_cases": args.cases,
            "explicit_bound": "[-8,8]",
        },
        "focused_tests": {
            "tests_run": unit_tests,
            "passed": True,
        },
        "grid_coverage": grid_coverage(args.seed ^ 0x641D, 1_000),
        "generated_differential": {
            "candidate_exact_oracle_disagreements": 0,
            "candidate_z3_disagreements": 0,
            "metamorphic_disagreements": 0,
            "counts": counts,
            "max_resource_usage": max_usage,
            "case_aggregate_sha256": sha256_json(records),
            "cases": records,
        },
        "falsification": mutation_results(),
        "fail_closed": fail_closed_results(),
    }
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--z3", required=True)
    parser.add_argument("--seed", type=parse_seed, default=0xB451E2026)
    parser.add_argument("--cases", type=int, default=1_000)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.cases < 1_000:
        parser.error("--cases must be at least the preregistered 1000")
    report = run(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    content = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(content, encoding="utf-8", newline="\n")
    print(
        json.dumps(
            {
                "passed": report["passed"],
                "cases": report["configuration"]["generated_cases"],
                "sat": report["generated_differential"]["counts"]["sat"],
                "unsat": report["generated_differential"]["counts"]["unsat"],
                "case_aggregate_sha256": report["generated_differential"][
                    "case_aggregate_sha256"
                ],
                "report_sha256": sha256_bytes(content.encode("utf-8")),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
