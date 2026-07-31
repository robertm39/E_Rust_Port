#!/usr/bin/env python3
"""Run the frozen Experiment 014 comparison and emit auditable evidence."""

from __future__ import annotations

import argparse
import dataclasses
import gzip
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence

import cd_viras
import corpus

base = cd_viras.base
EXPERIMENT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_DIR.parents[1]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fraction_smt(value: Fraction) -> str:
    negative = value < 0
    magnitude = abs(value)
    if magnitude.denominator == 1:
        rendered = str(magnitude.numerator)
    else:
        rendered = f"(/ {magnitude.numerator} {magnitude.denominator})"
    return f"(- {rendered})" if negative else rendered


def term_smt(term: base.Term) -> str:
    if term.op is base.TermOp.CONST:
        assert isinstance(term.value, Fraction)
        return fraction_smt(term.value)
    if term.op is base.TermOp.VAR:
        assert isinstance(term.value, str)
        return term.value
    if term.op is base.TermOp.ADD:
        return f"(+ {' '.join(term_smt(child) for child in term.args)})"
    if term.op is base.TermOp.SCALE:
        assert isinstance(term.value, Fraction)
        return f"(* {fraction_smt(term.value)} {term_smt(term.args[0])})"
    raise cd_viras.UnsupportedSlice("floor cannot be rendered as QF_LRA")


def literal_smt(literal: base.Literal) -> str:
    operator = {
        base.Relation.EQ: "=",
        base.Relation.GE: ">=",
        base.Relation.GT: ">",
    }.get(literal.relation)
    if operator is None:
        raise cd_viras.UnsupportedSlice("disequality is outside QF_LRA corpus")
    return f"({operator} {term_smt(literal.term)} 0)"


@dataclass(frozen=True, slots=True)
class Z3Query:
    query_id: str
    kind: str
    literals: tuple[base.Literal, ...]
    expected: str
    case_id: str
    clause_key: str | None = None


def run_z3_queries(
    z3_path: Path,
    queries: Sequence[Z3Query],
    *,
    timeout_seconds: int,
) -> tuple[dict[str, str], str]:
    variables = sorted(
        {
            variable
            for query in queries
            for literal in query.literals
            for variable in literal.variables()
        }
    )
    lines = [
        "(set-option :print-success false)",
        "(set-logic QF_LRA)",
        *[f"(declare-const {variable} Real)" for variable in variables],
    ]
    for query in queries:
        lines.extend(
            [
                f'(echo "{query.query_id}")',
                "(push 1)",
                *[f"(assert {literal_smt(literal)})" for literal in query.literals],
                "(check-sat)",
                "(pop 1)",
            ]
        )
    content = "\n".join(lines) + "\n"
    completed = subprocess.run(
        [str(z3_path), "-in"],
        input=content,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout_seconds,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"Z3 exited {completed.returncode}: {completed.stderr.strip()}"
        )
    if completed.stderr.strip():
        raise RuntimeError(f"Z3 emitted stderr: {completed.stderr.strip()}")

    expected_ids = {query.query_id for query in queries}
    statuses: dict[str, str] = {}
    current: str | None = None
    for raw in completed.stdout.splitlines():
        line = raw.strip().strip('"')
        if line in expected_ids:
            current = line
        elif line in {"sat", "unsat", "unknown"}:
            if current is None:
                raise RuntimeError(f"unlabelled Z3 status: {line}")
            statuses[current] = line
            current = None
        elif line:
            raise RuntimeError(f"unexpected Z3 output: {line}")
    missing = expected_ids - statuses.keys()
    if missing:
        raise RuntimeError(f"Z3 omitted {len(missing)} query statuses")
    return statuses, hashlib.sha256(content.encode("utf-8")).hexdigest()


def run_unit_tests() -> dict[str, Any]:
    command = [
        sys.executable,
        "-m",
        "unittest",
        "discover",
        "-s",
        str(EXPERIMENT_DIR),
        "-p",
        "test_*.py",
        "-v",
    ]
    completed = subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    combined = completed.stdout + completed.stderr
    matched = re.search(r"Ran (\d+) tests?", combined)
    if completed.returncode != 0 or matched is None:
        raise RuntimeError(f"focused tests failed:\n{combined}")
    return {
        "command": command,
        "count": int(matched.group(1)),
        "returncode": completed.returncode,
        "output_sha256": hashlib.sha256(combined.encode("utf-8")).hexdigest(),
    }


def outcome_semantic(outcome: cd_viras.SearchOutcome) -> Any:
    return {
        "treatment": outcome.treatment.value,
        "supported": outcome.supported,
        "decision": outcome.decision,
        "reason": outcome.reason,
        "metrics": outcome.metrics.semantic_description(),
        "clauses": [clause.describe() for clause in outcome.clauses],
        "semantic_trace_sha256": outcome.semantic_trace_sha256,
    }


def write_trace_record(
    compressed: gzip.GzipFile,
    case: corpus.Case,
    outcome: cd_viras.SearchOutcome,
) -> None:
    """Stream one full first-repetition trace to deterministic gzip."""

    record = {
        "case": case.describe(),
        "treatment": outcome.treatment.value,
        "summary": outcome_semantic(outcome),
        "trace": list(outcome.trace),
    }
    compressed.write(
        (cd_viras.canonical_json(record) + "\n").encode("utf-8")
    )


def mutation_probes() -> dict[str, bool]:
    x = base.variable("x")
    original = (corpus.equality(x, base.constant(0)),)
    sound = (cd_viras.ClauseComponent("x", base.constant(1)),)
    unsound = (cd_viras.ClauseComponent("x", base.constant(0)),)
    prefix = (
        cd_viras.Decision("x", base.constant(1), 0, "linear_zero", 0),
    )
    unsupported = cd_viras.run_search(
        (corpus.gt(x, base.constant(0)),), "focused"
    )
    return {
        "sound_clause_accepted": cd_viras.clause_soundness(
            original, sound
        ).feasible,
        "dropped_blocker_rejected": not cd_viras.clause_soundness(
            original, unsound
        ).feasible,
        "wrong_progress_rejected": not cd_viras.clause_progress(
            unsound, prefix
        ),
        "missing_equality_guard_fails_closed": (
            not unsupported.supported and unsupported.decision is None
        ),
    }


def git_revision() -> str | None:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPOSITORY_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return completed.stdout.strip() if completed.returncode == 0 else None


def run(args: argparse.Namespace) -> dict[str, Any]:
    tests = run_unit_tests()
    cases = (*corpus.hand_cases(), *corpus.generated_cases(seed=args.seed, count=args.cases))
    case_hash = cd_viras.semantic_hash([case.describe() for case in cases])

    repetitions: list[dict[str, Any]] = []
    first_outcomes: dict[str, dict[str, cd_viras.SearchOutcome]] = {}
    trace_path = Path(args.trace_output)
    trace_path.parent.mkdir(parents=True, exist_ok=True)
    trace_records = 0
    with trace_path.open("wb") as raw_trace:
        with gzip.GzipFile(
            filename="experiment-014-traces.jsonl",
            mode="wb",
            fileobj=raw_trace,
            mtime=0,
        ) as compressed_trace:
            for repetition in range(args.repetitions):
                repetition_started = time.perf_counter()
                elapsed_by_treatment = {
                    treatment.value: 0.0
                    for treatment in cd_viras.Treatment
                }
                semantic_records: list[Any] = []
                for case in cases:
                    treatment_outcomes: dict[str, cd_viras.SearchOutcome] = {}
                    for treatment in cd_viras.Treatment:
                        outcome = cd_viras.run_search(
                            case.literals,
                            treatment,
                            max_steps=args.max_steps,
                            max_affine_combinations=args.max_affine_combinations,
                        )
                        elapsed_by_treatment[
                            treatment.value
                        ] += outcome.metrics.elapsed_seconds
                        semantic_records.append(
                            {
                                "case_id": case.case_id,
                                "outcome": outcome_semantic(outcome),
                            }
                        )
                        if repetition == 0:
                            write_trace_record(compressed_trace, case, outcome)
                            trace_records += 1
                            treatment_outcomes[treatment.value] = dataclasses.replace(
                                outcome, trace=()
                            )
                    if repetition == 0:
                        first_outcomes[case.case_id] = treatment_outcomes
                repetitions.append(
                    {
                        "index": repetition + 1,
                        "semantic_sha256": cd_viras.semantic_hash(semantic_records),
                        "elapsed_by_treatment": elapsed_by_treatment,
                        "wall_seconds": time.perf_counter() - repetition_started,
                    }
                )

    trace_evidence = {
        "path": str(trace_path),
        "records": trace_records,
        "bytes": trace_path.stat().st_size,
        "sha256": sha256_file(trace_path),
    }

    correctness_errors: list[str] = []
    supported_generated: list[corpus.Case] = []
    exact_oracle: dict[str, bool] = {}
    learned_by_case: dict[str, dict[str, cd_viras.LearnedClause]] = {}
    for case in cases:
        outcomes = first_outcomes[case.case_id]
        if case.expected_supported:
            if case.family not in {"ground", "first_variable_exhaustion", "early_sat", "context_lifting", "focused_conflict"}:
                supported_generated.append(case)
            exact = cd_viras.affine_feasible(
                case.literals,
                max_combinations=args.max_affine_combinations,
            ).feasible
            exact_oracle[case.case_id] = exact
            if exact != case.expected_decision:
                correctness_errors.append(
                    f"{case.case_id}: frozen label {case.expected_decision} != exact {exact}"
                )
            for treatment, outcome in outcomes.items():
                if not outcome.supported:
                    correctness_errors.append(
                        f"{case.case_id}/{treatment}: unexpectedly unsupported: {outcome.reason}"
                    )
                elif outcome.decision != exact:
                    correctness_errors.append(
                        f"{case.case_id}/{treatment}: {outcome.decision} != exact {exact}"
                    )
                for clause in outcome.clauses:
                    recheck = cd_viras.clause_soundness(
                        case.literals,
                        clause.components,
                        max_combinations=args.max_affine_combinations,
                    )
                    if not recheck.feasible:
                        correctness_errors.append(
                            f"{case.case_id}/{treatment}: learned clause recheck failed"
                        )
                    learned_by_case.setdefault(case.case_id, {})[clause.key] = clause
        else:
            for treatment, outcome in outcomes.items():
                if outcome.supported or outcome.decision is not None:
                    correctness_errors.append(
                        f"{case.case_id}/{treatment}: unsupported boundary was accepted"
                    )

    z3_queries: list[Z3Query] = []
    query_counter = 0
    for case in cases:
        if not case.expected_supported:
            continue
        query_counter += 1
        z3_queries.append(
            Z3Query(
                f"Q{query_counter:07d}",
                "original",
                case.literals,
                "sat" if exact_oracle[case.case_id] else "unsat",
                case.case_id,
            )
        )
        for clause_key, clause in sorted(
            learned_by_case.get(case.case_id, {}).items()
        ):
            query_counter += 1
            z3_queries.append(
                Z3Query(
                    f"Q{query_counter:07d}",
                    "learned_clause",
                    (
                        *case.literals,
                        *cd_viras.negate_clause_assertions(clause.components),
                    ),
                    "unsat",
                    case.case_id,
                    clause_key,
                )
            )
    z3_statuses, z3_input_sha = run_z3_queries(
        Path(args.z3),
        z3_queries,
        timeout_seconds=args.z3_timeout_seconds,
    )
    z3_disagreements = [
        {
            "query_id": query.query_id,
            "case_id": query.case_id,
            "kind": query.kind,
            "expected": query.expected,
            "actual": z3_statuses[query.query_id],
        }
        for query in z3_queries
        if z3_statuses[query.query_id] != query.expected
    ]
    if z3_disagreements:
        correctness_errors.append(
            f"{len(z3_disagreements)} pinned-Z3 query disagreements"
        )

    semantic_hashes = [item["semantic_sha256"] for item in repetitions]
    deterministic = len(set(semantic_hashes)) == 1
    if not deterministic:
        correctness_errors.append("repetition semantic hashes differ")

    mutation_results = mutation_probes()
    if not all(mutation_results.values()):
        correctness_errors.append("one or more mutation probes survived")

    generated_unsat = [
        case for case in supported_generated if case.expected_decision is False
    ]
    focused_unsat_substitutions = sum(
        first_outcomes[case.case_id]["focused"].metrics.virtual_substitutions
        for case in generated_unsat
    )
    basic_unsat_substitutions = sum(
        first_outcomes[case.case_id]["basic"].metrics.virtual_substitutions
        for case in generated_unsat
    )
    focused_improved = sum(
        first_outcomes[case.case_id]["focused"].metrics.virtual_substitutions
        < first_outcomes[case.case_id]["basic"].metrics.virtual_substitutions
        for case in generated_unsat
    )
    basic_improved = sum(
        first_outcomes[case.case_id]["basic"].metrics.virtual_substitutions
        < first_outcomes[case.case_id]["eager"].metrics.virtual_substitutions
        for case in supported_generated
    )
    focused_basic_ratio = (
        focused_unsat_substitutions / basic_unsat_substitutions
        if basic_unsat_substitutions
        else float("inf")
    )
    focused_improved_share = focused_improved / len(generated_unsat)
    basic_improved_share = basic_improved / len(supported_generated)

    focused_times = [
        item["elapsed_by_treatment"]["focused"] for item in repetitions
    ]
    basic_times = [
        item["elapsed_by_treatment"]["basic"] for item in repetitions
    ]
    elapsed_ratio = statistics.median(focused_times) / statistics.median(
        basic_times
    )

    sat_count = sum(case.expected_decision is True for case in supported_generated)
    unsat_count = sum(case.expected_decision is False for case in supported_generated)
    corpus_gate = sat_count >= 50 and unsat_count >= 50
    correctness_gate = not correctness_errors
    reduction_gate = (
        focused_basic_ratio <= 0.75
        and focused_improved_share >= 0.60
        and basic_improved_share >= 0.60
    )
    overhead_gate = elapsed_ratio <= 2.0
    experiment_decision = (
        "stop"
        if not correctness_gate
        else "prototype-supported"
        if corpus_gate and reduction_gate and overhead_gate
        else "defer"
    )

    version = subprocess.run(
        [str(Path(args.z3)), "-version"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    ).stdout.strip()

    source_files = [
        EXPERIMENT_DIR / "PREREGISTRATION.md",
        EXPERIMENT_DIR / "cd_viras.py",
        EXPERIMENT_DIR / "corpus.py",
        EXPERIMENT_DIR / "run_experiment.py",
        EXPERIMENT_DIR / "test_cd_viras.py",
        cd_viras.BASE_EXPERIMENT_DIR / "prototype.py",
    ]
    learned_inserted = sum(
        outcome.metrics.learned_inserted
        for outcomes in first_outcomes.values()
        for outcome in outcomes.values()
    )
    report_cases = []
    for case in cases:
        report_cases.append(
            {
                "case_id": case.case_id,
                "family": case.family,
                "expected_supported": case.expected_supported,
                "expected_decision": case.expected_decision,
                "treatments": {
                    treatment.value: first_outcomes[case.case_id][
                        treatment.value
                    ].summary()
                    for treatment in cd_viras.Treatment
                },
            }
        )

    return {
        "schema": "umlaut-conflict-driven-viras-feasibility-v1",
        "configuration": {
            "seed": args.seed,
            "generated_cases": args.cases,
            "repetitions": args.repetitions,
            "max_steps": args.max_steps,
            "max_affine_combinations": args.max_affine_combinations,
            "preregistered_run": (
                args.seed == corpus.FROZEN_SEED
                and args.cases >= corpus.FROZEN_GENERATED_CASES
                and args.repetitions >= 2
            ),
        },
        "environment": {
            "platform": platform.platform(),
            "python": sys.version,
            "cpu_count": os.cpu_count(),
            "git_revision": args.source_revision or git_revision(),
            "z3_path": str(Path(args.z3)),
            "z3_version": version,
            "z3_sha256": sha256_file(Path(args.z3)),
        },
        "source_sha256": {
            str(path.relative_to(REPOSITORY_ROOT)): sha256_file(path)
            for path in source_files
        },
        "focused_tests": tests,
        "corpus": {
            "case_sha256": case_hash,
            "hand_cases": len(corpus.hand_cases()),
            "generated_supported": len(supported_generated),
            "generated_sat": sat_count,
            "generated_unsat": unsat_count,
            "cases": report_cases,
        },
        "repetitions": repetitions,
        "trace_evidence": trace_evidence,
        "learned_clause_validation": {
            "inserted_clause_occurrences": learned_inserted,
            "unique_case_clause_queries": sum(
                len(clauses) for clauses in learned_by_case.values()
            ),
            "internal_recheck_failures": sum(
                "learned clause recheck failed" in error
                for error in correctness_errors
            ),
            "z3_queries": len(z3_queries),
            "z3_input_sha256": z3_input_sha,
            "z3_disagreements": z3_disagreements,
        },
        "mutation_probes": mutation_results,
        "metrics": {
            "generated_unsat_basic_substitutions": basic_unsat_substitutions,
            "generated_unsat_focused_substitutions": focused_unsat_substitutions,
            "focused_basic_substitution_ratio": focused_basic_ratio,
            "focused_improved_cases": focused_improved,
            "focused_improved_share": focused_improved_share,
            "basic_improved_cases": basic_improved,
            "basic_improved_share": basic_improved_share,
            "median_focused_basic_elapsed_ratio": elapsed_ratio,
        },
        "gates": {
            "corpus": corpus_gate,
            "correctness": correctness_gate,
            "deterministic": deterministic,
            "reduction": reduction_gate,
            "overhead": overhead_gate,
            "errors": correctness_errors,
        },
        "decision": {
            "experiment": experiment_decision,
            "production": "defer",
            "reason": (
                "finite affine prototype evidence cannot discharge epsilon, "
                "infinity, periodic, grid, or general context-lifting gates"
            ),
            "unsupported_production_branches": [
                "epsilon false-interval lemmas",
                "aperiodic infinity lemmas",
                "periodic residue lemmas",
                "epsilon-plus-infinity lemmas",
                "Z-grid flattening in learned search",
                "general multi-variable lemma-context lifting",
            ],
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--z3", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--trace-output", required=True)
    parser.add_argument("--source-revision")
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=corpus.FROZEN_SEED)
    parser.add_argument("--cases", type=int, default=corpus.FROZEN_GENERATED_CASES)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--max-steps", type=int, default=1_000_000)
    parser.add_argument("--max-affine-combinations", type=int, default=100_000)
    parser.add_argument("--z3-timeout-seconds", type=int, default=300)
    parser.add_argument("--smoke", action="store_true")
    args = parser.parse_args()
    if not args.smoke:
        if args.cases < corpus.FROZEN_GENERATED_CASES:
            parser.error("--cases must be at least the preregistered 300")
        if args.repetitions < 2:
            parser.error("--repetitions must be at least the preregistered 2")
    report = run(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )
    print(
        json.dumps(
            {
                "decision": report["decision"]["experiment"],
                "production": report["decision"]["production"],
                "correctness": report["gates"]["correctness"],
                "reduction": report["gates"]["reduction"],
                "overhead": report["gates"]["overhead"],
                "semantic_sha256": report["repetitions"][0]["semantic_sha256"],
                "trace_sha256": report["trace_evidence"]["sha256"],
            },
            sort_keys=True,
        )
    )
    return 0 if report["gates"]["correctness"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
