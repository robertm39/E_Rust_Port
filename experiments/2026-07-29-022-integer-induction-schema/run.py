#!/usr/bin/env python3
"""Run the preregistered restricted integer-induction experiment on Linux."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence

import schema
import verify_schema


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parent.parent
BASE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "run.py"
)
SELECTION_PATH = EXPERIMENT_ROOT / "selected-problems.json"
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
TCF_RE = re.compile(r"^tcf\s*\(", re.MULTILINE)
FIFO = "FIFOWeight(ConstPrio)"
ORIENT = "OrientLMaxWeight(ConstPrio,2,1,2,1,1)"
GOAL = "Refinedweight(PreferGoals,1,1,1.5,1.1,1.1)"
COMMON_ARGS = [
    f"--expert-heuristic=(5*{ORIENT},2*{GOAL},1*{FIFO})",
    "--term-ordering=KBO6",
    "--literal-selection-strategy=NoSelection",
    "--disable-eq-factoring",
    "--forward-demod-level=2",
    "--presat-simplify=true",
]
PHASES = {
    "calibration": {
        "kind": "fixture",
        "repetitions": 1,
        "budget": {"soft_cpu_seconds": 2, "hard_cpu_seconds": 4},
        "proof_objects": False,
    },
    "validation": {
        "kind": "fixture",
        "repetitions": 2,
        "budget": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6},
        "proof_objects": False,
    },
    "test": {
        "kind": "fixture",
        "repetitions": 2,
        "budget": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10},
        "proof_objects": True,
    },
    "transfer": {
        "kind": "casc",
        "repetitions": 2,
        "budget": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10},
        "proof_objects": True,
    },
}
STRATEGIES = ("baseline", "induction")


class ExperimentError(RuntimeError):
    """The frozen experiment contract or an execution is invalid."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ExperimentError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("integer_induction_base_runner", BASE_PATH)


def fixture_records(phase: str) -> list[dict[str, Any]]:
    directory = EXPERIMENT_ROOT / "fixtures" / phase
    records = []
    for path in sorted(directory.glob("*.p")):
        records.append(
            {
                "problem_id": path.stem,
                "path": path.relative_to(REPO_ROOT).as_posix(),
                "sha256": BASE.sha256_file(path),
                "family": f"targeted-{phase}",
                "category": "targeted-induction",
                "division": "TFA",
                "holdout_split": phase,
                "difficulty_band": "targeted",
                "expected_class": "theorem",
                "source_kind": "constructed",
            }
        )
    if len(records) != 2:
        raise ExperimentError(f"{phase}: expected two fixtures, found {len(records)}")
    return records


def transfer_records(
    manifest_path: Path, problem_root: Path
) -> list[dict[str, Any]]:
    _, records = BASE.load_manifest(manifest_path)
    by_id = {record["problem_id"]: record for record in records}
    selection = json.loads(SELECTION_PATH.read_text(encoding="utf-8"))
    selected = []
    for pinned in selection["records"]:
        record = by_id.get(pinned["problem_id"])
        if record is None:
            raise ExperimentError(f"missing selected problem {pinned['problem_id']}")
        if record["path"] != pinned["path"] or record["sha256"] != pinned["sha256"]:
            raise ExperimentError(f"manifest drift for {record['problem_id']}")
        path = problem_root / record["path"]
        if BASE.sha256_file(path) != record["sha256"]:
            raise ExperimentError(f"corpus hash mismatch for {record['problem_id']}")
        selected.append({**record, "source_kind": "casc30-transfer"})
    if [record["problem_id"] for record in selected] != selection["problem_ids"]:
        raise ExperimentError("selection order or identifiers drifted")
    return selected


def phase_records(
    phase: str, manifest_path: Path, problem_root: Path
) -> list[dict[str, Any]]:
    return (
        fixture_records(phase)
        if PHASES[phase]["kind"] == "fixture"
        else transfer_records(manifest_path, problem_root)
    )


def atomic_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(text, encoding="utf-8")
    temporary.replace(path)


def clausify(
    *,
    binary: Path,
    input_path: Path,
    output_path: Path,
    environment: dict[str, str],
) -> int:
    completed = subprocess.run(
        [str(binary), "--cnf", str(input_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        timeout=30,
        check=False,
    )
    output_path.write_bytes(completed.stdout)
    output_path.with_suffix(".stderr.txt").write_bytes(completed.stderr)
    if completed.returncode != 0:
        raise ExperimentError(
            f"CNF gate failed for {input_path}: return {completed.returncode}"
        )
    text = completed.stdout.decode("utf-8", errors="replace")
    if "% CNFization successful!" not in text:
        raise ExperimentError(f"CNF success marker missing for {input_path}")
    return len(TCF_RE.findall(text))


def materialize_inputs(
    *,
    binary: Path,
    phase_root: Path,
    problem_root: Path,
    records: Sequence[dict[str, Any]],
) -> dict[str, dict[str, dict[str, Any]]]:
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    materialized: dict[str, dict[str, dict[str, Any]]] = {}
    for record in records:
        source_path = problem_root / record["path"]
        source_text = source_path.read_text(encoding="utf-8")
        prepared = schema.prepare_problem(source_text)
        augmented, generated = schema.augment_problem(source_text)
        verification = verify_schema.verify_structure(source_text, augmented)
        if verification["schema_id"] != generated.schema_id:
            raise ExperimentError(f"schema verifier drift for {record['problem_id']}")
        by_strategy = {}
        for strategy, text in (
            ("baseline", prepared),
            ("induction", augmented),
        ):
            input_path = (
                phase_root / "inputs" / record["problem_id"] / f"{strategy}.p"
            )
            atomic_text(input_path, text)
            cnf_path = input_path.with_suffix(".cnf.txt")
            clause_count = clausify(
                binary=binary,
                input_path=input_path,
                output_path=cnf_path,
                environment=environment,
            )
            by_strategy[strategy] = {
                "path": str(input_path),
                "sha256": BASE.sha256_file(input_path),
                "cnf_path": str(cnf_path),
                "cnf_sha256": BASE.sha256_file(cnf_path),
                "clausified_clauses": clause_count,
                "schema_id": generated.schema_id if strategy == "induction" else None,
                "schema_name": generated.name if strategy == "induction" else None,
            }
        if (
            by_strategy["induction"]["clausified_clauses"]
            <= by_strategy["baseline"]["clausified_clauses"]
        ):
            raise ExperimentError(
                f"schema produced no clause growth for {record['problem_id']}"
            )
        materialized[record["problem_id"]] = by_strategy
    return materialized


def result_is_resumable(
    result_path: Path,
    *,
    contract_id: str,
    binary_sha256: str,
    input_sha256: str,
) -> bool:
    if not result_path.is_file():
        return False
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
        run_dir = result_path.parent
        return (
            result["contract_id"] == contract_id
            and result["binary_sha256"] == binary_sha256
            and result["input_sha256"] == input_sha256
            and BASE.sha256_file(run_dir / "stdout.txt") == result["stdout_sha256"]
            and BASE.sha256_file(run_dir / "stderr.txt") == result["stderr_sha256"]
            and (
                result["telemetry_sha256"] is None
                or BASE.sha256_file(run_dir / "telemetry.json")
                == result["telemetry_sha256"]
            )
        )
    except (KeyError, OSError, ValueError, json.JSONDecodeError):
        return False


def run_one(
    *,
    binary: Path,
    binary_sha256: str,
    problem_root: Path,
    phase_root: Path,
    contract_id: str,
    phase: str,
    record: dict[str, Any],
    input_record: dict[str, Any],
    strategy: str,
    repetition: int,
    memory_mib: int,
) -> dict[str, Any]:
    phase_config = PHASES[phase]
    budget = phase_config["budget"]
    run_dir = (
        phase_root
        / "runs"
        / strategy
        / record["family"]
        / record["problem_id"]
        / f"rep-{repetition}"
    )
    result_path = run_dir / "result.json"
    if result_is_resumable(
        result_path,
        contract_id=contract_id,
        binary_sha256=binary_sha256,
        input_sha256=input_record["sha256"],
    ):
        return {"resumed": True, "result_path": str(result_path)}

    run_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = run_dir / "stdout.txt"
    stderr_path = run_dir / "stderr.txt"
    telemetry_path = run_dir / "telemetry.json"
    telemetry_path.unlink(missing_ok=True)
    proof_args = (
        ["--tstp-out", "--proof-object=1"]
        if phase_config["proof_objects"]
        else []
    )
    command = [
        str(binary),
        *COMMON_ARGS,
        *proof_args,
        f"--soft-cpu-limit={budget['soft_cpu_seconds']}",
        f"--cpu-limit={budget['hard_cpu_seconds']}",
        f"--memory-limit={memory_mib}",
        f"--search-telemetry={telemetry_path}",
        input_record["path"],
    ]
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    started_at = BASE.utc_now()
    started = time.monotonic()
    external_timeout = False
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=budget["hard_cpu_seconds"] + 10,
            check=False,
        )
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        external_timeout = True
        return_code = None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    wall_seconds = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    status = BASE.final_status(stdout.decode("utf-8", errors="replace"))
    telemetry, telemetry_sha256, telemetry_error = BASE.load_optional_telemetry(
        telemetry_path
    )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "phase": phase,
        "problem_id": record["problem_id"],
        "source_sha256": record["sha256"],
        "source_kind": record["source_kind"],
        "family": record["family"],
        "category": record["category"],
        "strategy": strategy,
        "repetition": repetition,
        "soft_cpu_seconds": budget["soft_cpu_seconds"],
        "hard_cpu_seconds": budget["hard_cpu_seconds"],
        "binary_sha256": binary_sha256,
        "input_sha256": input_record["sha256"],
        "input_clausified_clauses": input_record["clausified_clauses"],
        "schema_id": input_record["schema_id"],
        "command": command,
        "started_at": started_at,
        "completed_at": BASE.utc_now(),
        "return_code": return_code,
        "external_timeout": external_timeout,
        "wall_seconds": wall_seconds,
        "szs_status": status,
        "expected_status_match": status in PROOF_STATUSES,
        "telemetry_present": telemetry is not None,
        "telemetry_sha256": telemetry_sha256,
        "telemetry_error": telemetry_error,
        "stdout_sha256": BASE.sha256_file(stdout_path),
        "stderr_sha256": BASE.sha256_file(stderr_path),
    }
    BASE.atomic_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path)}


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=tuple(PHASES), required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--source-snapshot-sha256", required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--memory-mib", type=int, default=1536)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("prover experiments may run only on Linux")
    if arguments.workers < 1:
        raise ExperimentError("--workers must be positive")
    if arguments.memory_mib < 256:
        raise ExperimentError("--memory-mib must be at least 256")

    phase = arguments.phase
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    output_root = arguments.output_root.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(f"binary is missing or not executable: {binary}")
    records = phase_records(phase, manifest_path, problem_root)
    phase_root = output_root / phase
    materialized = materialize_inputs(
        binary=binary,
        phase_root=phase_root,
        problem_root=problem_root,
        records=records,
    )
    binary_sha256 = BASE.sha256_file(binary)
    phase_config = PHASES[phase]
    contract_body = {
        "schema_version": 1,
        "phase": phase,
        "phase_config": phase_config,
        "strategies": list(STRATEGIES),
        "common_args": COMMON_ARGS,
        "manifest_sha256": BASE.sha256_file(manifest_path),
        "selection_sha256": BASE.sha256_file(SELECTION_PATH),
        "selected_problem_ids": [record["problem_id"] for record in records],
        "selected_source_sha256": {
            record["problem_id"]: record["sha256"] for record in records
        },
        "materialized_inputs": materialized,
        "binary_sha256": binary_sha256,
        "source_snapshot_sha256": arguments.source_snapshot_sha256,
        "harness_sha256": BASE.sha256_file(Path(__file__).resolve()),
        "schema_generator_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "schema.py"
        ),
        "schema_verifier_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "verify_schema.py"
        ),
        "base_harness_sha256": BASE.sha256_file(BASE_PATH),
        "resources": {
            "workers": arguments.workers,
            "memory_mib": arguments.memory_mib,
        },
    }
    contract_id = hashlib.sha256(BASE.canonical_json(contract_body)).hexdigest()
    contract = {
        **contract_body,
        "contract_id": contract_id,
        "created_at": BASE.utc_now(),
        "host": {
            "hostname": socket.gethostname(),
            "platform": platform.platform(),
            "cpu_count": os.cpu_count(),
        },
    }
    contract_path = phase_root / "contract.json"
    if contract_path.is_file():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        existing_body = {
            key: value
            for key, value in existing.items()
            if key not in {"created_at", "host"}
        }
        current_body = {
            key: value
            for key, value in contract.items()
            if key not in {"created_at", "host"}
        }
        if existing_body != current_body:
            raise ExperimentError(f"incompatible existing contract: {contract_path}")
    else:
        BASE.atomic_json(contract_path, contract)

    coordinates = [
        (record, strategy, repetition)
        for record in records
        for strategy in STRATEGIES
        for repetition in range(1, phase_config["repetitions"] + 1)
    ]
    coordinates.sort(
        key=lambda value: hashlib.sha256(
            f"{contract_id}:{value[0]['problem_id']}:{value[1]}:{value[2]}".encode(
                "utf-8"
            )
        ).digest()
    )
    resumed = 0
    completed = 0
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=arguments.workers
    ) as executor:
        futures = [
            executor.submit(
                run_one,
                binary=binary,
                binary_sha256=binary_sha256,
                problem_root=problem_root,
                phase_root=phase_root,
                contract_id=contract_id,
                phase=phase,
                record=record,
                input_record=materialized[record["problem_id"]][strategy],
                strategy=strategy,
                repetition=repetition,
                memory_mib=arguments.memory_mib,
            )
            for record, strategy, repetition in coordinates
        ]
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            resumed += int(result["resumed"])
            completed += int(not result["resumed"])
            print(
                f"{'resumed' if result['resumed'] else 'completed'} "
                f"{result['result_path']}"
            )

    print(
        f"OK: phase {phase}; contract {contract_id}; "
        f"{completed} completed, {resumed} resumed"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        ExperimentError,
        OSError,
        KeyError,
        ValueError,
        json.JSONDecodeError,
        subprocess.TimeoutExpired,
    ) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
