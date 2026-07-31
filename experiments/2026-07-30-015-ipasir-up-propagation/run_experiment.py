#!/usr/bin/env python3
"""Run the frozen IPASIR-UP-style propagation simulation."""

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
from pathlib import Path
from typing import Any

import simulator

EXPERIMENT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = EXPERIMENT_DIR.parents[1]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_tests() -> dict[str, Any]:
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
    match = re.search(r"Ran (\d+) tests?", combined)
    if completed.returncode or match is None:
        raise RuntimeError(combined)
    return {
        "count": int(match.group(1)),
        "output_sha256": hashlib.sha256(combined.encode()).hexdigest(),
    }


def outcome_semantic(outcome: simulator.Outcome) -> Any:
    return {
        "treatment": outcome.treatment.value,
        "decision": outcome.decision,
        "reason": outcome.reason,
        "model": list(outcome.model) if outcome.model is not None else None,
        "metrics": outcome.metrics.semantic(),
        "learned": [list(clause) for clause in outcome.learned],
        "events": list(outcome.events),
        "semantic_sha256": outcome.semantic_sha256,
    }


def mutation_probes(case: simulator.Case) -> dict[str, bool]:
    valid = simulator.LearnEvent(
        "propagation",
        (-2, -1),
        -2,
        ((1, True),),
        (simulator.TrailEntry(1, 1, "decision"),),
    )
    cross_group = simulator.LearnEvent(
        "conflict",
        (-1, -4),
        None,
        ((1, True), (4, True)),
        (),
    )
    missing_literal = dataclasses.replace(valid, propagated=-4)
    stale = (simulator.TrailEntry(1, 1, "stale"),)
    return {
        "valid_reason_accepted": simulator.validate_reason(case, valid),
        "cross_group_reason_rejected": not simulator.validate_reason(
            case, cross_group
        ),
        "missing_propagated_literal_rejected": not simulator.validate_reason(
            case, missing_literal
        ),
        "nonroot_backtrack_rejected": not simulator.validate_root_backtrack(
            valid, 1, ()
        ),
        "stale_post_trail_rejected": not simulator.validate_root_backtrack(
            valid, 0, stale
        ),
    }


def run(args: argparse.Namespace) -> dict[str, Any]:
    tests = run_tests()
    cases = (
        *simulator.hand_cases(),
        *simulator.generated_cases(args.seed, args.cases),
    )
    case_sha = simulator.semantic_hash([case.describe() for case in cases])
    first: dict[str, dict[str, simulator.Outcome]] = {}
    repetitions: list[dict[str, Any]] = []
    trace_path = Path(args.trace_output)
    trace_path.parent.mkdir(parents=True, exist_ok=True)
    trace_records = 0
    with trace_path.open("wb") as raw:
        with gzip.GzipFile(
            filename="experiment-015-traces.jsonl",
            mode="wb",
            fileobj=raw,
            mtime=0,
        ) as compressed:
            for repetition in range(args.repetitions):
                started = time.perf_counter()
                semantic_records = []
                elapsed = {treatment.value: 0.0 for treatment in simulator.Treatment}
                for case in cases:
                    saved: dict[str, simulator.Outcome] = {}
                    for treatment in simulator.Treatment:
                        outcome = simulator.run_case(
                            case, treatment, args.max_steps
                        )
                        semantic = outcome_semantic(outcome)
                        semantic_records.append(
                            {"case_id": case.case_id, "outcome": semantic}
                        )
                        elapsed[treatment.value] += outcome.metrics.elapsed_seconds
                        if repetition == 0:
                            saved[treatment.value] = outcome
                            compressed.write(
                                (
                                    simulator.canonical_json(
                                        {
                                            "case": case.describe(),
                                            "outcome": semantic,
                                        }
                                    )
                                    + "\n"
                                ).encode()
                            )
                            trace_records += 1
                    if repetition == 0:
                        first[case.case_id] = saved
                repetitions.append(
                    {
                        "index": repetition + 1,
                        "semantic_sha256": simulator.semantic_hash(
                            semantic_records
                        ),
                        "elapsed_by_treatment": elapsed,
                        "wall_seconds": time.perf_counter() - started,
                    }
                )

    errors: list[str] = []
    reason_events = 0
    backtracks = 0
    for case in cases:
        exact = simulator.exhaustive_oracle(case)
        if exact != case.expected:
            errors.append(f"{case.case_id}: oracle disagrees with frozen label")
        for treatment in simulator.Treatment:
            outcome = first[case.case_id][treatment.value]
            if outcome.decision != exact:
                errors.append(
                    f"{case.case_id}/{treatment.value}: decision mismatch"
                )
            if outcome.decision is None:
                errors.append(
                    f"{case.case_id}/{treatment.value}: resource limit"
                )
            if outcome.model is not None and not simulator.validate_model(
                case, outcome.model
            ):
                errors.append(f"{case.case_id}/{treatment.value}: invalid model")
            reason_events += len(outcome.events)
            backtracks += outcome.metrics.root_backtracks
            if len(outcome.events) != outcome.metrics.root_backtracks:
                errors.append(
                    f"{case.case_id}/{treatment.value}: backtrack log mismatch"
                )

    deterministic = len(
        {record["semantic_sha256"] for record in repetitions}
    ) == 1
    if not deterministic:
        errors.append("semantic repetition hashes differ")
    mutations = mutation_probes(simulator.hand_cases()[0])
    if not all(mutations.values()):
        errors.append("mutation probe survived")

    generated_unsat = [
        case
        for case in cases
        if case.family == "unsat_4_3"
    ]
    decisions = {
        treatment.value: sum(
            first[case.case_id][treatment.value].metrics.decisions
            for case in generated_unsat
        )
        for treatment in simulator.Treatment
    }
    improved_conflict = sum(
        first[case.case_id]["propagate"].metrics.decisions
        < first[case.case_id]["conflict"].metrics.decisions
        for case in generated_unsat
    )
    improved_lazy = sum(
        first[case.case_id]["propagate"].metrics.decisions
        < first[case.case_id]["lazy"].metrics.decisions
        for case in generated_unsat
    )
    propagation_conflict_ratio = decisions["propagate"] / decisions["conflict"]
    propagation_lazy_ratio = decisions["propagate"] / decisions["lazy"]
    improve_conflict_share = improved_conflict / len(generated_unsat)
    improve_lazy_share = improved_lazy / len(generated_unsat)
    elapsed_ratio = statistics.median(
        record["elapsed_by_treatment"]["propagate"] for record in repetitions
    ) / statistics.median(
        record["elapsed_by_treatment"]["conflict"] for record in repetitions
    )

    correctness_gate = not errors
    corpus_gate = (
        sum(case.family == "sat_4_4" for case in cases) >= 40
        and len(generated_unsat) >= 40
    )
    reduction_gate = (
        propagation_conflict_ratio <= 0.70
        and propagation_lazy_ratio <= 0.30
        and improve_conflict_share >= 0.80
        and improve_lazy_share >= 0.80
    )
    overhead_gate = elapsed_ratio <= 1.5
    experiment_decision = (
        "stop"
        if not correctness_gate
        else "prototype-supported"
        if corpus_gate and reduction_gate and overhead_gate
        else "defer"
    )

    source_files = [
        EXPERIMENT_DIR / name
        for name in (
            "PREREGISTRATION.md",
            "simulator.py",
            "run_experiment.py",
            "test_simulator.py",
        )
    ]
    return {
        "schema": "umlaut-ipasir-up-propagation-v1",
        "configuration": {
            "seed": args.seed,
            "generated_cases": args.cases,
            "repetitions": args.repetitions,
            "max_steps": args.max_steps,
            "preregistered": (
                args.seed == simulator.FROZEN_SEED
                and args.cases >= 100
                and args.repetitions >= 2
            ),
        },
        "environment": {
            "platform": platform.platform(),
            "python": sys.version,
            "cpu_count": os.cpu_count(),
            "source_revision": args.source_revision,
        },
        "source_sha256": {
            str(path.relative_to(REPOSITORY_ROOT)): sha256_file(path)
            for path in source_files
        },
        "tests": tests,
        "corpus": {
            "sha256": case_sha,
            "hand": len(simulator.hand_cases()),
            "generated": args.cases,
            "sat": sum(case.expected for case in cases),
            "unsat": sum(not case.expected for case in cases),
        },
        "validation": {
            "reason_events": reason_events,
            "root_backtracks": backtracks,
            "mutations": mutations,
        },
        "repetitions": repetitions,
        "trace": {
            "records": trace_records,
            "bytes": trace_path.stat().st_size,
            "sha256": sha256_file(trace_path),
        },
        "metrics": {
            "generated_unsat_decisions": decisions,
            "propagation_conflict_ratio": propagation_conflict_ratio,
            "propagation_lazy_ratio": propagation_lazy_ratio,
            "improved_vs_conflict": improved_conflict,
            "improved_vs_conflict_share": improve_conflict_share,
            "improved_vs_lazy": improved_lazy,
            "improved_vs_lazy_share": improve_lazy_share,
            "propagation_conflict_elapsed_ratio": elapsed_ratio,
        },
        "gates": {
            "corpus": corpus_gate,
            "correctness": correctness_gate,
            "deterministic": deterministic,
            "reduction": reduction_gate,
            "overhead": overhead_gate,
            "errors": errors,
        },
        "decision": {
            "experiment": experiment_decision,
            "production": "defer",
            "reason": (
                "live callbacks, proof integration, stable production atom "
                "identities, and real theory-bearing traces remain required"
            ),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    parser.add_argument("--trace-output", required=True)
    parser.add_argument("--source-revision")
    parser.add_argument(
        "--seed", type=lambda value: int(value, 0), default=simulator.FROZEN_SEED
    )
    parser.add_argument("--cases", type=int, default=100)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--max-steps", type=int, default=1_000_000)
    parser.add_argument("--smoke", action="store_true")
    args = parser.parse_args()
    if not args.smoke and (args.cases < 100 or args.repetitions < 2):
        parser.error("frozen run requires at least 100 cases and 2 repetitions")
    report = run(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(
        json.dumps(
            {
                "decision": report["decision"],
                "gates": report["gates"],
                "metrics": report["metrics"],
                "semantic_sha256": report["repetitions"][0][
                    "semantic_sha256"
                ],
            },
            sort_keys=True,
        )
    )
    return 0 if report["gates"]["correctness"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
