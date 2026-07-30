#!/usr/bin/env python3
"""Run the frozen bounded online-adaptation experiment on Linux."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence

import common


EXPERIMENT_ROOT = Path(__file__).resolve().parent
CORPUS_PATH = EXPERIMENT_ROOT / "corpus.jsonl"
GLOBAL_HEURISTIC = (
    "(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),"
    "1*FIFOWeight(ConstPrio))"
)
GOAL_HEURISTIC = (
    "(5*Refinedweight(PreferGoals,2,1,1.5,1.1,1.1),"
    "1*FIFOWeight(ConstPrio))"
)
STRATEGIES = {
    "global": GLOBAL_HEURISTIC,
    "goal": GOAL_HEURISTIC,
}
COMMON_ARGS = (
    "--term-ordering=KBO6",
    "--pcl-out",
    "--proof-object=1",
    "--force-deriv=2",
)
POLICIES = {
    "calibration": (
        "global_full",
        "goal_full",
        "probe",
        "continuation_global",
        "continuation_goal",
    ),
    "validation": (
        "global_full",
        "goal_full",
        "static_global_restart",
        "static_goal",
        "adaptive",
    ),
    "test": (
        "global_full",
        "goal_full",
        "static_global_restart",
        "static_goal",
        "adaptive",
    ),
}
DEFAULT_BUDGETS = {
    "probe": {"soft_cpu_seconds": 1, "hard_cpu_seconds": 3},
    "continuation": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6},
    "full": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
}
SMOKE_BUDGETS = {
    "probe": {"soft_cpu_seconds": 1, "hard_cpu_seconds": 3},
    "continuation": {"soft_cpu_seconds": 1, "hard_cpu_seconds": 3},
    "full": {"soft_cpu_seconds": 1, "hard_cpu_seconds": 3},
}
MEMORY_MIB = 1536


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def load_corpus(
    path: Path, phase: str, *, smoke: bool
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = common.read_jsonl(path)
    if (
        not rows
        or rows[0].get("kind") != "umlaut-online-stagnation-corpus"
        or rows[0].get("source_revision") != common.SOURCE_REVISION
    ):
        raise common.ExperimentError("corpus header violates the contract")
    records = [
        record
        for record in rows[1:]
        if record.get("experiment_split") == phase
    ]
    if len(records) != 8:
        raise common.ExperimentError(
            f"{phase} has {len(records)} records, expected 8"
        )
    records.sort(
        key=lambda record: (
            str(record["category"]),
            str(record["selection_rank"]),
            str(record["problem_id"]),
        )
    )
    if smoke:
        records = records[:1]
    return rows[0], records


def verify_problem_inputs(
    problem_root: Path, records: Sequence[dict[str, Any]]
) -> None:
    for record in records:
        problem = problem_root / str(record["path"])
        if not problem.is_file():
            raise common.ExperimentError(f"missing problem: {problem}")
        observed = common.sha256_file(problem)
        if observed != record["sha256"]:
            raise common.ExperimentError(
                f"problem hash mismatch for {record['problem_id']}: {observed}"
            )
        for include in record.get("includes", []):
            include_path = (
                problem_root / "problems" / "casc_2025" / str(include)
            )
            if not include_path.is_file():
                raise common.ExperimentError(
                    f"missing include for {record['problem_id']}: "
                    f"{include_path}"
                )


def load_selection(path: Path) -> dict[str, Any]:
    selection = json.loads(path.read_text(encoding="utf-8"))
    identifier = selection.get("selection_id")
    body = {
        key: value
        for key, value in selection.items()
        if key != "selection_id"
    }
    if identifier != common.sha256_bytes(common.canonical_json(body)):
        raise common.ExperimentError("threshold selection ID is invalid")
    threshold = float(selection.get("selected_threshold"))
    if threshold not in common.THRESHOLDS:
        raise common.ExperimentError("selection has an unregistered threshold")
    if selection.get("source_revision") != common.SOURCE_REVISION:
        raise common.ExperimentError("selection source revision changed")
    return selection


def load_validation_report(path: Path) -> dict[str, Any]:
    report = json.loads(path.read_text(encoding="utf-8"))
    if report.get("phase") != "validation":
        raise common.ExperimentError("test gate is not a validation report")
    if not report.get("correctness", {}).get("passed"):
        raise common.ExperimentError("validation correctness gate did not pass")
    identifier = report.get("report_id")
    body = {
        key: value
        for key, value in report.items()
        if key != "report_id"
    }
    if identifier != common.sha256_bytes(common.canonical_json(body)):
        raise common.ExperimentError("validation report ID is invalid")
    return report


def contract_body(
    *,
    phase: str,
    smoke: bool,
    binary_sha256: str,
    corpus_sha256: str,
    records: Sequence[dict[str, Any]],
    budgets: dict[str, dict[str, int]],
    selection: dict[str, Any] | None,
    validation_report_sha256: str | None,
    repetitions: int,
    workers: int,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "experiment": "2026-07-29-020-online-stagnation-adaptation",
        "source_revision": common.SOURCE_REVISION,
        "phase": phase,
        "smoke": smoke,
        "binary_sha256": binary_sha256,
        "corpus_sha256": corpus_sha256,
        "harness_sha256": common.sha256_file(Path(__file__).resolve()),
        "common_sha256": common.sha256_file(EXPERIMENT_ROOT / "common.py"),
        "preregistration_sha256": common.sha256_file(
            EXPERIMENT_ROOT / "PREREGISTRATION.md"
        ),
        "policies": list(POLICIES[phase]),
        "strategies": STRATEGIES,
        "common_args": list(COMMON_ARGS),
        "budgets": budgets,
        "memory_mib": MEMORY_MIB,
        "repetitions": repetitions,
        "workers": workers,
        "records": [
            {
                "problem_id": record["problem_id"],
                "category": record["category"],
                "family": record["family"],
                "path": record["path"],
                "sha256": record["sha256"],
                "selection_rank": record["selection_rank"],
            }
            for record in records
        ],
        "selection_id": (
            selection["selection_id"] if selection is not None else None
        ),
        "selected_threshold": (
            float(selection["selected_threshold"])
            if selection is not None
            else None
        ),
        "validation_report_sha256": validation_report_sha256,
    }


def initialize_contract(
    output_root: Path, body: dict[str, Any]
) -> tuple[str, dict[str, Any]]:
    contract_id = common.sha256_bytes(common.canonical_json(body))
    contract = {**body, "contract_id": contract_id}
    path = output_root / "contract.json"
    if path.is_file():
        existing = json.loads(path.read_text(encoding="utf-8"))
        if existing != contract:
            raise common.ExperimentError(
                f"output root has a different contract: {path}"
            )
    else:
        common.atomic_json(path, contract)
    return contract_id, contract


def telemetry_record(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not path.is_file():
        return None, None
    digest = common.sha256_file(path)
    try:
        return json.loads(path.read_text(encoding="utf-8")), digest
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None, digest


def execute_phase(
    *,
    binary: Path,
    problem_root: Path,
    record: dict[str, Any],
    run_dir: Path,
    phase_index: int,
    phase_kind: str,
    strategy: str,
    budgets: dict[str, dict[str, int]],
) -> dict[str, Any]:
    phase_dir = run_dir / f"phase-{phase_index:02d}-{phase_kind}-{strategy}"
    phase_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = phase_dir / "stdout.pcl"
    stderr_path = phase_dir / "stderr.txt"
    telemetry_path = phase_dir / "telemetry.json"
    telemetry_path.unlink(missing_ok=True)
    budget = budgets[phase_kind]
    problem_path = problem_root / str(record["path"])
    command = [
        str(binary),
        f"--expert-heuristic={STRATEGIES[strategy]}",
        *COMMON_ARGS,
        f"--soft-cpu-limit={budget['soft_cpu_seconds']}",
        f"--cpu-limit={budget['hard_cpu_seconds']}",
        f"--memory-limit={MEMORY_MIB}",
        f"--search-telemetry={telemetry_path}",
        str(problem_path),
    ]
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    started_at = utc_now()
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
    stdout_text = stdout.decode("utf-8", errors="replace")
    stderr_text = stderr.decode("utf-8", errors="replace")
    telemetry, telemetry_sha256 = telemetry_record(telemetry_path)
    status = common.final_status(stdout_text, stderr_text)
    return {
        "phase_index": phase_index,
        "phase_kind": phase_kind,
        "strategy": strategy,
        "soft_cpu_seconds": budget["soft_cpu_seconds"],
        "hard_cpu_seconds": budget["hard_cpu_seconds"],
        "command": command,
        "started_at": started_at,
        "completed_at": utc_now(),
        "return_code": return_code,
        "external_timeout": external_timeout,
        "wall_seconds": wall_seconds,
        "szs_status": status,
        "proof_steps": common.proof_step_count(stdout_text),
        "telemetry_present": telemetry is not None,
        "telemetry_sha256": telemetry_sha256,
        "telemetry_cpu_seconds": (
            float(telemetry["resources"]["total_cpu_seconds"])
            if telemetry is not None
            else None
        ),
        "stdout_sha256": common.sha256_file(stdout_path),
        "stderr_sha256": common.sha256_file(stderr_path),
        "artifact_directory": phase_dir.name,
        "_telemetry": telemetry,
    }


def stored_phase(phase: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in phase.items() if key != "_telemetry"}


def result_is_resumable(
    path: Path,
    *,
    contract_id: str,
    problem_sha256: str,
    binary_sha256: str,
) -> bool:
    if not path.is_file():
        return False
    try:
        result = json.loads(path.read_text(encoding="utf-8"))
        if (
            result["contract_id"] != contract_id
            or result["problem_sha256"] != problem_sha256
            or result["binary_sha256"] != binary_sha256
        ):
            return False
        for phase in result["phases"]:
            phase_dir = path.parent / phase["artifact_directory"]
            stdout = phase_dir / "stdout.pcl"
            stderr = phase_dir / "stderr.txt"
            if (
                common.sha256_file(stdout) != phase["stdout_sha256"]
                or common.sha256_file(stderr) != phase["stderr_sha256"]
            ):
                return False
            telemetry_sha256 = phase["telemetry_sha256"]
            if telemetry_sha256 is not None:
                telemetry = phase_dir / "telemetry.json"
                if common.sha256_file(telemetry) != telemetry_sha256:
                    return False
        return True
    except (KeyError, OSError, ValueError, json.JSONDecodeError):
        return False


def run_policy(
    *,
    binary: Path,
    binary_sha256: str,
    problem_root: Path,
    output_root: Path,
    contract_id: str,
    phase: str,
    policy: str,
    record: dict[str, Any],
    repetition: int,
    budgets: dict[str, dict[str, int]],
    threshold: float | None,
) -> dict[str, Any]:
    run_dir = (
        output_root
        / "runs"
        / policy
        / str(record["category"])
        / str(record["problem_id"])
        / f"rep-{repetition}"
    )
    result_path = run_dir / "result.json"
    if result_is_resumable(
        result_path,
        contract_id=contract_id,
        problem_sha256=str(record["sha256"]),
        binary_sha256=binary_sha256,
    ):
        return {"resumed": True, "result_path": str(result_path)}
    run_dir.mkdir(parents=True, exist_ok=True)
    phases: list[dict[str, Any]] = []
    decision: dict[str, Any] | None = None
    decision_cpu_seconds = 0.0
    decision_wall_seconds = 0.0
    started_at = utc_now()
    policy_started = time.monotonic()

    if policy in {"global_full", "goal_full"}:
        strategy = "global" if policy == "global_full" else "goal"
        phases.append(
            execute_phase(
                binary=binary,
                problem_root=problem_root,
                record=record,
                run_dir=run_dir,
                phase_index=1,
                phase_kind="full",
                strategy=strategy,
                budgets=budgets,
            )
        )
    elif phase == "calibration":
        primitive = {
            "probe": ("probe", "global"),
            "continuation_global": ("continuation", "global"),
            "continuation_goal": ("continuation", "goal"),
        }
        if policy not in primitive:
            raise common.ExperimentError(
                f"unknown calibration primitive: {policy}"
            )
        phase_kind, strategy = primitive[policy]
        phases.append(
            execute_phase(
                binary=binary,
                problem_root=problem_root,
                record=record,
                run_dir=run_dir,
                phase_index=1,
                phase_kind=phase_kind,
                strategy=strategy,
                budgets=budgets,
            )
        )
    else:
        probe = execute_phase(
            binary=binary,
            problem_root=problem_root,
            record=record,
            run_dir=run_dir,
            phase_index=1,
            phase_kind="probe",
            strategy="global",
            budgets=budgets,
        )
        phases.append(probe)
        if probe["szs_status"] not in common.PROOF_STATUSES:
            if policy == "static_global_restart":
                branch = "global"
            elif policy == "static_goal":
                branch = "goal"
            elif policy == "adaptive":
                if threshold is None:
                    raise common.ExperimentError(
                        "adaptive policy requires a threshold"
                    )
                cpu_started = time.process_time_ns()
                wall_started = time.monotonic_ns()
                decision = common.choose_branch(
                    probe["_telemetry"], threshold
                )
                decision_cpu_seconds = (
                    time.process_time_ns() - cpu_started
                ) / 1.0e9
                decision_wall_seconds = (
                    time.monotonic_ns() - wall_started
                ) / 1.0e9
                branch = str(decision["branch"])
            else:
                raise common.ExperimentError(f"unknown policy: {policy}")
            phases.append(
                execute_phase(
                    binary=binary,
                    problem_root=problem_root,
                    record=record,
                    run_dir=run_dir,
                    phase_index=2,
                    phase_kind="continuation",
                    strategy=branch,
                    budgets=budgets,
                )
            )
        elif policy == "adaptive":
            decision = {
                "threshold": threshold,
                "branch": "probe_solved",
                "valid": True,
                "fallback_reason": None,
                **common.signal_from_telemetry(probe["_telemetry"]),
            }

    proof_phases = [
        item
        for item in phases
        if item["szs_status"] in common.PROOF_STATUSES
    ]
    final_phase = proof_phases[0] if proof_phases else phases[-1]
    telemetry_cpu = [
        float(item["telemetry_cpu_seconds"])
        for item in phases
        if item["telemetry_cpu_seconds"] is not None
    ]
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "phase": phase,
        "policy": policy,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "category": record["category"],
        "family": record["family"],
        "expected_class": record["expected_class"],
        "repetition": repetition,
        "binary_sha256": binary_sha256,
        "started_at": started_at,
        "completed_at": utc_now(),
        "policy_wall_seconds": time.monotonic() - policy_started,
        "configured_cpu_seconds": sum(
            int(item["soft_cpu_seconds"]) for item in phases
        ),
        "telemetry_cpu_seconds": (
            sum(telemetry_cpu)
            if len(telemetry_cpu) == len(phases)
            else None
        ),
        "szs_status": final_phase["szs_status"],
        "proof_steps": int(final_phase["proof_steps"]),
        "expected_status_match": (
            final_phase["szs_status"] in common.PROOF_STATUSES
        ),
        "external_timeout": any(
            bool(item["external_timeout"]) for item in phases
        ),
        "decision": decision,
        "decision_cpu_seconds": decision_cpu_seconds,
        "decision_wall_seconds": decision_wall_seconds,
        "phases": [stored_phase(item) for item in phases],
    }
    common.atomic_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path)}


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=CORPUS_PATH)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("calibration", "validation", "test"), required=True
    )
    parser.add_argument("--selection", type=Path)
    parser.add_argument("--validation-report", type=Path)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--contract-preview", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise common.ExperimentError(
            "prover experiments may run only on Linux"
        )
    if arguments.source_revision != common.SOURCE_REVISION:
        raise common.ExperimentError("source revision differs from preregistration")
    if arguments.workers < 1 or arguments.repetitions < 1:
        raise common.ExperimentError("workers and repetitions must be positive")
    if arguments.smoke:
        arguments.repetitions = 1
    manifest = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    output_root = arguments.output_root.resolve()
    if not binary.is_file():
        raise common.ExperimentError(f"binary is missing: {binary}")
    header, records = load_corpus(
        manifest, arguments.phase, smoke=arguments.smoke
    )
    del header
    verify_problem_inputs(problem_root, records)
    selection = None
    if arguments.phase == "calibration":
        if arguments.selection is not None:
            raise common.ExperimentError(
                "calibration must not receive a selection"
            )
    else:
        if arguments.selection is None:
            raise common.ExperimentError(
                f"{arguments.phase} requires --selection"
            )
        selection = load_selection(arguments.selection.resolve())
    validation_report_sha256 = None
    if arguments.phase == "test":
        if arguments.validation_report is None:
            raise common.ExperimentError(
                "test requires --validation-report"
            )
        report_path = arguments.validation_report.resolve()
        load_validation_report(report_path)
        validation_report_sha256 = common.sha256_file(report_path)
    elif arguments.validation_report is not None:
        raise common.ExperimentError(
            "--validation-report is accepted only for test"
        )

    budgets = SMOKE_BUDGETS if arguments.smoke else DEFAULT_BUDGETS
    binary_sha256 = common.sha256_file(binary)
    corpus_sha256 = common.sha256_file(manifest)
    body = contract_body(
        phase=arguments.phase,
        smoke=arguments.smoke,
        binary_sha256=binary_sha256,
        corpus_sha256=corpus_sha256,
        records=records,
        budgets=budgets,
        selection=selection,
        validation_report_sha256=validation_report_sha256,
        repetitions=arguments.repetitions,
        workers=arguments.workers,
    )
    if arguments.contract_preview:
        print(json.dumps(body, indent=2, sort_keys=True))
        return 0
    output_root.mkdir(parents=True, exist_ok=True)
    contract_id, _contract = initialize_contract(output_root, body)
    policies = POLICIES[arguments.phase]
    threshold = (
        float(selection["selected_threshold"])
        if selection is not None
        else None
    )
    jobs = [
        (policy, record, repetition)
        for policy in policies
        for record in records
        for repetition in range(1, arguments.repetitions + 1)
    ]
    completed = 0
    resumed = 0
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=arguments.workers
    ) as executor:
        futures = [
            executor.submit(
                run_policy,
                binary=binary,
                binary_sha256=binary_sha256,
                problem_root=problem_root,
                output_root=output_root,
                contract_id=contract_id,
                phase=arguments.phase,
                policy=policy,
                record=record,
                repetition=repetition,
                budgets=budgets,
                threshold=threshold,
            )
            for policy, record, repetition in jobs
        ]
        for future in concurrent.futures.as_completed(futures):
            outcome = future.result()
            if outcome["resumed"]:
                resumed += 1
            else:
                completed += 1
    print(
        json.dumps(
            {
                "contract_id": contract_id,
                "completed": completed,
                "resumed": resumed,
                "total": len(jobs),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except common.ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
