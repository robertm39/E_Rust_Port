#!/usr/bin/env python3
"""Independently validate deterministic adaptive-probe artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Sequence

import analyze
import common
import run


def current_temp_residue(search: dict[str, Any]) -> list[str]:
    root = Path(search["output_path"]).parent / "tmp"
    if not root.is_dir():
        return []
    return sorted(
        str(path.relative_to(root))
        for path in root.rglob("*")
        if path.is_file() or path.is_symlink()
    )


def independent_result_failures(result: dict[str, Any]) -> list[str]:
    failures = []
    for index, search in enumerate(result["phases"], start=1):
        output = Path(search["output_path"]).read_text(
            encoding="utf-8", errors="replace"
        )
        stderr = Path(search["stderr_path"]).read_text(
            encoding="utf-8", errors="replace"
        )
        if common.final_status(output, stderr) != search["szs_status"]:
            failures.append(f"phase-{index}:status_reparse_mismatch")
        if (
            common.processed_clause_count(output)
            != search["processed_clauses"]
        ):
            failures.append(f"phase-{index}:processed_reparse_mismatch")
        timing_path = search["timing_path"]
        reparsed_timing = (
            run.parse_timing(Path(timing_path))
            if timing_path is not None
            else None
        )
        if reparsed_timing != search["timing"]:
            failures.append(f"phase-{index}:timing_reparse_mismatch")
        if current_temp_residue(search) != search["temp_residue"]:
            failures.append(f"phase-{index}:temp_residue_mismatch")
    proof_phases = [
        search
        for search in result["phases"]
        if search["szs_status"] in common.PROOF_STATUSES
    ]
    final_phase = proof_phases[0] if proof_phases else result["phases"][-1]
    if final_phase["szs_status"] != result["szs_status"]:
        failures.append("final_status_mismatch")
    timings = [search["timing"] for search in result["phases"]]
    expected_resources = {
        "total_cpu_seconds": (
            sum(float(item["total_cpu_seconds"]) for item in timings)
            if all(item is not None for item in timings)
            else None
        ),
        "wall_seconds": sum(
            float(search["controller_wall_seconds"])
            for search in result["phases"]
        ),
        "peak_rss_kib": (
            max(int(item["peak_rss_kib"]) for item in timings)
            if all(item is not None for item in timings)
            else None
        ),
    }
    if expected_resources != result["resources"]:
        failures.append("aggregate_resources_mismatch")
    replay = result["proof_replay"]
    if proof_phases:
        if replay is None or not replay["reproduced"]:
            failures.append("proof_replay_not_reproduced")
    elif replay is not None:
        failures.append("proof_replay_without_proof_status")
    if result["correctness_failures"]:
        failures.extend(
            f"recorded:{failure}"
            for failure in result["correctness_failures"]
        )
    return failures


def contract_artifact_failures(contract: dict[str, Any]) -> list[str]:
    failures = []
    for name, expected in contract["script_hashes"].items():
        path = run.EXPERIMENT_ROOT / name
        if not path.is_file() or common.sha256_file(path) != expected:
            failures.append(f"script_hash_mismatch:{name}")
    for field in ("binary", "proofcheck", "validation_gate", "corpus_report"):
        entry = contract[field]
        path = Path(entry["path"])
        if not path.is_file() or common.sha256_file(path) != entry["sha256"]:
            failures.append(f"contract_artifact_hash_mismatch:{field}")
    corpus = contract["corpus"]
    corpus_path = Path(corpus["path"])
    if (
        not corpus_path.is_file()
        or common.sha256_file(corpus_path) != corpus["sha256"]
    ):
        failures.append("contract_artifact_hash_mismatch:corpus")
    validation = contract["validation_report"]
    if validation is not None:
        path = Path(validation["path"])
        if (
            not path.is_file()
            or common.sha256_file(path) != validation["sha256"]
        ):
            failures.append(
                "contract_artifact_hash_mismatch:validation_report"
            )
    return failures


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("train", "validation", "test"), required=True
    )
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--replay-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    root = arguments.root.resolve()
    contract = analyze.load_contract(root, arguments.phase)
    results = analyze.load_results(root, contract)
    failures = contract_artifact_failures(contract)
    proofcheck = arguments.proofcheck.resolve()
    validation_gate = arguments.validation_gate.resolve()
    if (
        common.sha256_file(proofcheck)
        != contract["proofcheck"]["sha256"]
    ):
        failures.append("argument_proofcheck_hash_mismatch")
    if (
        common.sha256_file(validation_gate)
        != contract["validation_gate"]["sha256"]
    ):
        failures.append("argument_validation_gate_hash_mismatch")
    corpus_report = run.load_corpus_report(
        Path(contract["corpus_report"]["path"])
    )
    run.verify_problem_inputs(
        problem_root=arguments.problem_root.resolve(),
        records=contract["records"],
        corpus_report=corpus_report,
    )
    arguments.replay_root.mkdir(parents=True, exist_ok=True)
    proof_count = 0
    for result in results:
        failures.extend(
            f"{result['policy']}:{result['problem_id']}:"
            f"r{result['repetition']}:{failure}"
            for failure in independent_result_failures(result)
        )
        replay = result["proof_replay"]
        if replay is None:
            continue
        proof_count += 1
        proof = Path(replay["output_path"])
        problem = (
            arguments.problem_root.resolve()
            / next(
                record["path"]
                for record in contract["records"]
                if record["problem_id"] == result["problem_id"]
            )
        )
        name = (
            f"{result['policy']}-{result['problem_id']}-"
            f"r{result['repetition']}.json"
        )
        gate = run.run_validation_gate(
            validation_gate=validation_gate,
            proofcheck=proofcheck,
            problem=problem,
            proof=proof,
            report=arguments.replay_root.resolve() / name,
        )
        if not gate["verified"]:
            failures.append(
                f"independent_proof_replay_failed:{result['policy']}:"
                f"{result['problem_id']}:r{result['repetition']}"
            )
    report = {
        "schema_version": 1,
        "kind": "deterministic-adaptive-probe-validation",
        "phase": arguments.phase,
        "contract_id": contract["contract_id"],
        "coordinate_count": len(results),
        "proof_replay_count": proof_count,
        "failures": sorted(set(failures)),
        "valid": not failures,
    }
    common.atomic_json(arguments.output.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["valid"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except common.ExperimentError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
