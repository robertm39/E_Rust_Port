#!/usr/bin/env python3
"""Audit run coordinates, hashes, proof checks, and repetition semantics."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Sequence

import connection_common as common


def artifact_hashes(root: Path) -> dict[str, str]:
    return {
        str(path.relative_to(root)): common.sha256_file(path)
        for path in sorted(root.rglob("*"))
        if path.is_file() and path.name != "result.json"
    }


def theorem_claim(result: dict[str, Any]) -> bool:
    if result["method"] == "connection":
        return result["status"] == "Theorem"
    return result["status"] in common.PROOF_STATUSES


def audit(root: Path, phase: str, corpus: Path) -> dict[str, Any]:
    contract_path = root / "contract.json"
    results_path = root / "results.jsonl"
    if not contract_path.is_file() or not results_path.is_file():
        raise common.ExperimentError("run root lacks contract or results")
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    results = common.read_jsonl(results_path)
    _header, records = common.load_corpus(corpus)
    selected = [
        record for record in records if record["experiment_split"] == phase
    ]
    expected = {
        (record["problem_id"], method, repetition)
        for record in selected
        for method in common.METHODS
        for repetition in range(1, common.REPETITIONS[phase] + 1)
    }
    observed = {
        (result["problem_id"], result["method"], result["repetition"])
        for result in results
    }
    failures: list[str] = []
    if contract.get("phase") != phase:
        failures.append("contract_phase_mismatch")
    if contract.get("contract_id") is None:
        failures.append("missing_contract_id")
    if expected != observed or len(results) != len(expected):
        failures.append("coordinate_matrix_mismatch")

    result_files = sorted((root / "runs").rglob("result.json"))
    if len(result_files) != len(expected):
        failures.append("run_result_file_count_mismatch")
    indexed_files: dict[tuple[str, str, int], dict[str, Any]] = {}
    for path in result_files:
        try:
            item = json.loads(path.read_text(encoding="utf-8"))
            coordinate = (
                item["problem_id"],
                item["method"],
                item["repetition"],
            )
            indexed_files[coordinate] = item
            if artifact_hashes(path.parent) != item["artifact_hashes"]:
                failures.append(
                    "artifact_hash_mismatch:" + "/".join(map(str, coordinate))
                )
        except (OSError, UnicodeError, KeyError, json.JSONDecodeError) as error:
            failures.append(f"malformed_result_file:{path}:{error}")

    allowed_no_claim = common.NO_CLAIM_STATUSES | {None}
    for result in results:
        coordinate = (
            result["problem_id"],
            result["method"],
            result["repetition"],
        )
        if indexed_files.get(coordinate) != result:
            failures.append(
                "results_export_mismatch:" + "/".join(map(str, coordinate))
            )
        if result.get("contract_id") != contract.get("contract_id"):
            failures.append(
                "contract_id_mismatch:" + "/".join(map(str, coordinate))
            )
        if result.get("phase") != phase:
            failures.append(
                "result_phase_mismatch:" + "/".join(map(str, coordinate))
            )
        if result.get("correctness_failures"):
            failures.extend(
                f"run_correctness:{'/'.join(map(str, coordinate))}:{failure}"
                for failure in result["correctness_failures"]
            )
        claim = theorem_claim(result)
        if claim and not result.get("proof_verified"):
            failures.append(
                "unverified_theorem:" + "/".join(map(str, coordinate))
            )
        if result["method"] == "connection":
            if result.get("status") not in {"Theorem", "Unknown"}:
                failures.append(
                    "connection_polarity:" + "/".join(map(str, coordinate))
                )
        elif (
            result.get("status") not in common.PROOF_STATUSES
            and result.get("status") not in allowed_no_claim
        ):
            failures.append(
                "saturation_polarity:" + "/".join(map(str, coordinate))
            )

    repetition_disagreements: list[dict[str, Any]] = []
    if common.REPETITIONS[phase] > 1:
        for problem in (record["problem_id"] for record in selected):
            for method in common.METHODS:
                coordinates = [
                    result for result in results
                    if result["problem_id"] == problem
                    and result["method"] == method
                ]
                terminal = [theorem_claim(result) for result in coordinates]
                if len(set(terminal)) != 1:
                    repetition_disagreements.append(
                        {
                            "problem_id": problem,
                            "method": method,
                            "terminal": terminal,
                            "statuses": [
                                result.get("status") for result in coordinates
                            ],
                        }
                    )
        if repetition_disagreements:
            failures.append("heldout_repetition_disagreement")

    return {
        "schema_version": 1,
        "phase": phase,
        "contract_id": contract.get("contract_id"),
        "expected_coordinates": len(expected),
        "observed_coordinates": len(results),
        "artifact_files_checked": sum(
            len(result.get("artifact_hashes", {})) for result in results
        ),
        "verified_theorem_runs": sum(
            theorem_claim(result) and bool(result.get("proof_verified"))
            for result in results
        ),
        "repetition_disagreements": repetition_disagreements,
        "failures": sorted(set(failures)),
        "valid": not failures,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--phase", choices=common.REPETITIONS, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    report = audit(
        arguments.root.resolve(),
        arguments.phase,
        arguments.corpus.resolve(),
    )
    common.atomic_json(arguments.output.resolve(), report)
    print(json.dumps(report, sort_keys=True))
    return 0 if report["valid"] else 1


if __name__ == "__main__":
    sys.exit(main())

