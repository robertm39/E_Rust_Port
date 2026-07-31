#!/usr/bin/env python3
"""Run every experiment arm on one frozen training problem."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
from typing import Sequence

from common import ExperimentError, load_corpus, sha256_file
from run_experiment import (
    ARMS,
    cnf_bodies,
    preprocess_audit,
    run_coordinate,
    validate_cpu_list,
)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--cpu-list", default="0,1,2,3")
    parser.add_argument("--problem-id", default="LAT265-2")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    cpu_list = [int(value) for value in arguments.cpu_list.split(",")]
    validate_cpu_list(cpu_list)
    _, records = load_corpus(arguments.manifest.resolve())
    matches = [
        record
        for record in records
        if record["problem_id"] == arguments.problem_id
        and record["experiment_split"] == "train"
    ]
    if len(matches) != 1:
        raise ExperimentError("smoke problem must identify one train record")
    record = matches[0]
    problem_root = arguments.problem_root.resolve()
    original = problem_root / str(record["path"])
    if sha256_file(original) != record["sha256"]:
        raise ExperimentError("smoke problem hash mismatch")
    environment = dict(os.environ)
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    output_root = arguments.output_root.resolve()
    audit = preprocess_audit(
        binary=arguments.binary.resolve(),
        cpu_list=cpu_list,
        problem=original,
        root=output_root / "_preprocess" / str(record["problem_id"]),
        environment=environment,
    )
    canonical = Path(audit["single"]["stdout_path"])
    original_bodies = cnf_bodies(
        canonical.read_text(encoding="utf-8", errors="replace")
    )
    selection = {
        "selection_id": "smoke-selection",
        "worker_budgets": [7, 4, 3, 2],
    }
    for arm in ARMS:
        result = run_coordinate(
            arm=arm,
            binary=arguments.binary.resolve(),
            contract_id="smoke-contract",
            cpu_list=cpu_list,
            original=original,
            original_bodies=original_bodies,
            output_root=output_root,
            proofcheck=arguments.proofcheck.resolve(),
            record=record,
            repetition=1,
            selection=selection,
            validation_gate=arguments.validation_gate.resolve(),
            environment=environment,
        )
        if result["correctness_failures"]:
            raise ExperimentError(
                f"{arm}: {', '.join(result['correctness_failures'])}"
            )
        print(f"{arm}: {result['status']}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
