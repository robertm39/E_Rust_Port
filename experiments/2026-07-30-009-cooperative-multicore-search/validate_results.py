#!/usr/bin/env python3
"""Independently validate raw cooperative multicore result artifacts."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Sequence

from common import (
    PROOF_STATUSES,
    ExperimentError,
    atomic_json,
    canonical_json,
    final_status,
    proof_step_count,
    sha256_bytes,
    sha256_file,
)
from run_experiment import ARMS


def verify_hash(path_value: str | None, expected: str | None, label: str) -> None:
    if path_value is None or expected is None:
        if path_value is not None or expected is not None:
            raise ExperimentError(f"{label} incomplete path/hash pair")
        return
    path = Path(path_value)
    if not path.is_file() or sha256_file(path) != expected:
        raise ExperimentError(f"{label} hash mismatch: {path}")


def proof_gate(
    *,
    validation_gate: Path,
    proofcheck: Path,
    problem: Path,
    proof: Path,
    report: Path,
) -> dict[str, Any]:
    command = [
        "python3",
        str(validation_gate),
        str(problem),
        str(proof),
        "--proof-command-json",
        json.dumps([str(proofcheck), "-p", "{problem}", "{artifact}"]),
        "--report",
        str(report),
    ]
    completed = subprocess.run(
        command, check=False, capture_output=True, text=True, timeout=180
    )
    return {
        "command": command,
        "return_code": completed.returncode,
        "stderr": completed.stderr,
        "stdout": completed.stdout,
        "verified": completed.returncode == 0,
    }


def validate_coordinate(
    *,
    path: Path,
    proofcheck: Path,
    validation_gate: Path,
    replay_root: Path,
) -> tuple[dict[str, Any], list[str]]:
    result = json.loads(path.read_text(encoding="utf-8"))
    failures: list[str] = []
    if result.get("correctness_failures"):
        failures.extend(str(value) for value in result["correctness_failures"])
    problem = Path(result["problem_path"])
    verify_hash(str(problem), result["problem_sha256"], "problem")
    for wave in result["waves"]:
        if wave["surviving_processes"] != 0:
            failures.append("recorded_survivor")
        for worker in wave["workers"]:
            verify_hash(
                worker["stdout_path"], worker["stdout_sha256"], "worker stdout"
            )
            verify_hash(
                worker["stderr_path"], worker["stderr_sha256"], "worker stderr"
            )
            verify_hash(
                worker["telemetry_path"],
                worker["telemetry_sha256"],
                "worker telemetry",
            )
            verify_hash(
                worker["timing_path"], worker["timing_sha256"], "worker timing"
            )
            verify_hash(
                worker["input_path"], worker["input_sha256"], "worker input"
            )
            observed_status = final_status(
                Path(worker["stdout_path"]).read_text(
                    encoding="utf-8", errors="replace"
                )
            )
            if observed_status != worker["status"]:
                failures.append(
                    f"status_mismatch:worker-{worker['index']}:"
                    f"{worker['status']}:{observed_status}"
                )
            if worker["temp_residue"]:
                failures.append(
                    f"temp_residue:worker-{worker['index']}"
                )
    for wrapper in result["exchange"]["wrappers"]:
        verify_hash(wrapper["wrapper_path"], wrapper["sha256"], "wrapper")
        wrapper_path = Path(wrapper["wrapper_path"])
        if wrapper_path.stat().st_size != wrapper["size_bytes"]:
            failures.append("wrapper_size_mismatch")
        text = wrapper_path.read_text(encoding="utf-8", errors="strict")
        if text.count(", watchlist, ") != wrapper["clause_count"]:
            failures.append("wrapper_clause_count_mismatch")
        for clause in wrapper["clauses"]:
            if clause["body"] not in text:
                failures.append("wrapper_clause_missing")
            if int(wrapper["recipient"]) in clause["producers"]:
                failures.append("self_exchange")
    proof = result.get("proof_replay")
    replay = None
    if result["status"] in PROOF_STATUSES:
        if proof is None:
            failures.append("missing_proof_replay")
        else:
            verify_hash(proof["proof_path"], proof["proof_sha256"], "proof")
            verify_hash(
                proof["telemetry_path"],
                proof["telemetry_sha256"],
                "proof telemetry",
            )
            proof_path = Path(proof["proof_path"])
            text = proof_path.read_text(encoding="utf-8", errors="replace")
            if final_status(text) != proof["status"]:
                failures.append("proof_status_mismatch")
            if proof_step_count(text) != proof["proof_steps"]:
                failures.append("proof_step_mismatch")
            if "coop_w" in text:
                failures.append("watchlist_logical_reference")
            coordinate_name = (
                f"{result['problem_id']}-{result['arm']}-"
                f"r{result['repetition']}"
            )
            replay = proof_gate(
                validation_gate=validation_gate,
                proofcheck=proofcheck,
                problem=problem,
                proof=proof_path,
                report=replay_root / f"{coordinate_name}.json",
            )
            if not replay["verified"]:
                failures.append("independent_proof_replay_failed")
            winner_input = Path(proof["command"][-1])
            if winner_input.resolve() != problem.resolve():
                wrapper_replay = proof_gate(
                    validation_gate=validation_gate,
                    proofcheck=proofcheck,
                    problem=winner_input,
                    proof=proof_path,
                    report=replay_root / f"{coordinate_name}-wrapper.json",
                )
                if not wrapper_replay["verified"]:
                    failures.append("independent_wrapper_replay_failed")
            if proof["temp_residue"]:
                failures.append("proof_replay_temp_residue")
    elif proof is not None:
        failures.append("proof_replay_without_proof_status")
    return result, failures


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("train", "validation", "test"), required=True
    )
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--replay-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    root = arguments.root.resolve()
    repetitions = 1 if arguments.phase == "train" else 2
    problems = 16 if arguments.phase == "train" else 8
    expected = problems * repetitions * len(ARMS)
    paths = sorted(root.glob("*/*-r*/result.json"))
    if len(paths) != expected:
        raise ExperimentError(f"found {len(paths)} results, expected {expected}")
    arguments.replay_root.mkdir(parents=True, exist_ok=True)
    failures: list[str] = []
    proof_count = 0
    contracts: set[str] = set()
    for path in paths:
        result, coordinate_failures = validate_coordinate(
            path=path,
            proofcheck=arguments.proofcheck.resolve(),
            validation_gate=arguments.validation_gate.resolve(),
            replay_root=arguments.replay_root.resolve(),
        )
        contracts.add(str(result["contract_id"]))
        proof_count += result["status"] in PROOF_STATUSES
        failures.extend(
            f"{result['problem_id']}:{result['arm']}:"
            f"r{result['repetition']}:{failure}"
            for failure in coordinate_failures
        )
    contract = json.loads((root / "contract.json").read_text(encoding="utf-8"))
    unsigned_contract = dict(contract)
    contract_id = unsigned_contract.pop("contract_id", None)
    if contract_id != sha256_bytes(canonical_json(unsigned_contract)):
        failures.append("contract_hash_mismatch")
    if contracts != {contract["contract_id"]}:
        failures.append("contract_id_mismatch")
    experiment_root = Path(__file__).resolve().parent
    for name, expected_hash in contract["script_hashes"].items():
        script = experiment_root / name
        if not script.is_file() or sha256_file(script) != expected_hash:
            failures.append(f"script_hash_mismatch:{name}")
    for field in ("binary", "proofcheck", "validation_gate"):
        entry = contract[field]
        artifact = Path(entry["path"])
        if (
            not artifact.is_file()
            or sha256_file(artifact) != entry["sha256"]
        ):
            failures.append(f"contract_artifact_hash_mismatch:{field}")
    report = {
        "contract_id": contract["contract_id"],
        "coordinate_count": len(paths),
        "failures": sorted(set(failures)),
        "kind": "cooperative-multicore-validation",
        "phase": arguments.phase,
        "proof_replay_count": proof_count,
        "schema_version": 1,
        "valid": not failures,
    }
    atomic_json(arguments.output.resolve(), report)
    print(json.dumps(report, sort_keys=True, indent=2))
    return 0 if report["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
