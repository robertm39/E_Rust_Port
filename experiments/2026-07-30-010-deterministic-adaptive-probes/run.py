#!/usr/bin/env python3
"""Run the frozen deterministic adaptive-probe experiment on Linux."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import queue
import shutil
import signal
import subprocess
import sys
import time
from contextlib import contextmanager
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Iterator, Sequence

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
STRATEGIES = {"global": GLOBAL_HEURISTIC, "goal": GOAL_HEURISTIC}
POLICIES = (
    "probe_without_telemetry",
    "probe_with_telemetry",
    "global_full",
    "goal_full",
    "static_global_restart",
    "static_goal",
    "adaptive",
)
BUDGETS = {
    "probe": {"soft_cpu_seconds": 2, "hard_cpu_seconds": 4},
    "continuation": {"soft_cpu_seconds": 3, "hard_cpu_seconds": 5},
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


def phase_records(
    manifest: Path,
    phase: str,
    *,
    smoke: bool,
    smoke_problem: str | None,
) -> list[dict[str, Any]]:
    _, records = common.load_corpus(manifest)
    selected = [
        record
        for record in records
        if record["experiment_split"] == phase
    ]
    selected.sort(
        key=lambda record: (
            str(record["category"]),
            str(record["difficulty_band"]),
            str(record["selection_rank"]),
        )
    )
    if smoke_problem is not None:
        matching = [
            record
            for record in selected
            if record["problem_id"] == smoke_problem
        ]
        if not smoke or len(matching) != 1:
            raise common.ExperimentError(
                "--smoke-problem requires one problem in a smoke phase"
            )
        return matching
    return selected[:1] if smoke else selected


def load_corpus_report(path: Path) -> dict[str, Any]:
    report = json.loads(path.read_text(encoding="utf-8"))
    if (
        report.get("kind")
        != "deterministic-adaptive-probe-prepared-corpus"
        or report.get("problem_count") != 24
    ):
        raise common.ExperimentError("prepared corpus report is invalid")
    return report


def verify_problem_inputs(
    *,
    problem_root: Path,
    records: Sequence[dict[str, Any]],
    corpus_report: dict[str, Any],
) -> None:
    files = {
        str(item["path"]): item
        for item in corpus_report.get("files", [])
    }
    for record in records:
        required = [str(record["path"])]
        required.extend(
            f"problems/casc_2025/{include}"
            for include in record.get("includes", [])
        )
        for relative in required:
            item = files.get(relative)
            path = problem_root / relative
            if item is None or not path.is_file():
                raise common.ExperimentError(
                    f"prepared corpus file is missing: {relative}"
                )
            if common.sha256_file(path) != item["sha256"]:
                raise common.ExperimentError(
                    f"prepared corpus hash mismatch: {relative}"
                )
        if files[str(record["path"])]["sha256"] != record["sha256"]:
            raise common.ExperimentError(
                f"manifest/report mismatch: {record['problem_id']}"
            )


def load_validation_report(path: Path) -> dict[str, Any]:
    report = json.loads(path.read_text(encoding="utf-8"))
    identifier = report.get("analysis_id")
    unsigned = {
        key: value for key, value in report.items() if key != "analysis_id"
    }
    if (
        report.get("kind") != "deterministic-adaptive-probe-analysis"
        or report.get("phase") != "validation"
        or report.get("correctness_failures")
        or identifier != common.sha256_bytes(common.canonical_json(unsigned))
    ):
        raise common.ExperimentError(
            "test requires an accepted validation analysis"
        )
    return report


def script_hashes() -> dict[str, str]:
    paths = sorted(EXPERIMENT_ROOT.glob("*.py")) + [
        EXPERIMENT_ROOT / "PREREGISTRATION.md"
    ]
    return {path.name: common.sha256_file(path) for path in paths}


def contract_body(
    *,
    binary: Path,
    corpus_report_path: Path,
    cpu_list: Sequence[int],
    manifest: Path,
    phase: str,
    proofcheck: Path,
    records: Sequence[dict[str, Any]],
    repetitions: int,
    smoke: bool,
    validation_gate: Path,
    validation_report: Path | None,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "kind": "deterministic-adaptive-probe-contract",
        "experiment": EXPERIMENT_ROOT.name,
        "source_revision": common.SOURCE_REVISION,
        "phase": phase,
        "smoke": smoke,
        "binary": {
            "path": str(binary),
            "sha256": common.sha256_file(binary),
        },
        "proofcheck": {
            "path": str(proofcheck),
            "sha256": common.sha256_file(proofcheck),
        },
        "validation_gate": {
            "path": str(validation_gate),
            "sha256": common.sha256_file(validation_gate),
        },
        "corpus": {
            "path": str(manifest),
            "sha256": common.CORPUS_SHA256,
        },
        "corpus_report": {
            "path": str(corpus_report_path),
            "sha256": common.sha256_file(corpus_report_path),
        },
        "validation_report": (
            None
            if validation_report is None
            else {
                "path": str(validation_report),
                "sha256": common.sha256_file(validation_report),
            }
        ),
        "script_hashes": script_hashes(),
        "policies": list(POLICIES),
        "strategies": STRATEGIES,
        "budgets": SMOKE_BUDGETS if smoke else BUDGETS,
        "processed_limit": common.PROCESSED_LIMIT,
        "minimum_processed": common.MIN_PROCESSED,
        "threshold": common.THRESHOLD,
        "memory_mib": MEMORY_MIB,
        "cpu_list": list(cpu_list),
        "repetitions": repetitions,
        "records": [
            {
                key: record[key]
                for key in (
                    "problem_id",
                    "category",
                    "difficulty_band",
                    "expected_class",
                    "family",
                    "path",
                    "sha256",
                    "selection_rank",
                )
            }
            for record in records
        ],
    }


def initialize_contract(output_root: Path, body: dict[str, Any]) -> dict[str, Any]:
    contract = {
        **body,
        "contract_id": common.sha256_bytes(common.canonical_json(body)),
    }
    path = output_root / "contract.json"
    if path.is_file():
        if json.loads(path.read_text(encoding="utf-8")) != contract:
            raise common.ExperimentError(
                f"output root has a different contract: {path}"
            )
    else:
        common.atomic_json(path, contract)
    return contract


def parse_elapsed(value: str) -> float:
    fields = value.split(":")
    try:
        if len(fields) == 2:
            return float(fields[0]) * 60.0 + float(fields[1])
        if len(fields) == 3:
            return (
                float(fields[0]) * 3600.0
                + float(fields[1]) * 60.0
                + float(fields[2])
            )
    except ValueError as error:
        raise common.ExperimentError(
            f"cannot parse elapsed time: {value}"
        ) from error
    raise common.ExperimentError(f"cannot parse elapsed time: {value}")


def parse_timing(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    values = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        parts = line.strip().rsplit(": ", 1)
        if len(parts) == 2:
            values[parts[0]] = parts[1]
    try:
        user = float(values["User time (seconds)"])
        system = float(values["System time (seconds)"])
        return {
            "user_cpu_seconds": user,
            "system_cpu_seconds": system,
            "total_cpu_seconds": user + system,
            "wall_seconds": parse_elapsed(
                values["Elapsed (wall clock) time (h:mm:ss or m:ss)"]
            ),
            "peak_rss_kib": int(
                values["Maximum resident set size (kbytes)"]
            ),
        }
    except (KeyError, ValueError):
        return None


def process_group_alive(group: int) -> bool:
    try:
        os.killpg(group, 0)
        return True
    except ProcessLookupError:
        return False


def terminate_group(process: subprocess.Popen[bytes]) -> float:
    started = time.monotonic()
    if process.poll() is not None:
        return 0.0
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return time.monotonic() - started
    try:
        process.wait(timeout=1.0)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=5.0)
    return time.monotonic() - started


def telemetry_record(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not path.is_file():
        return None, None
    digest = common.sha256_file(path)
    try:
        return json.loads(path.read_text(encoding="utf-8")), digest
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None, digest


def search_arguments(
    *,
    budget: dict[str, int],
    phase_kind: str,
    strategy: str,
    telemetry_path: Path | None,
    proof: bool,
    problem: Path,
) -> list[str]:
    arguments = [
        f"--expert-heuristic={STRATEGIES[strategy]}",
        "--term-ordering=KBO6",
        f"--soft-cpu-limit={budget['soft_cpu_seconds']}",
        f"--cpu-limit={budget['hard_cpu_seconds']}",
        f"--memory-limit={MEMORY_MIB}",
        "--tstp-out",
        "--print-statistics",
    ]
    if phase_kind == "probe":
        arguments.append(
            f"--processed-clauses-limit={common.PROCESSED_LIMIT}"
        )
    if telemetry_path is not None:
        arguments.append(f"--search-telemetry={telemetry_path}")
    if proof:
        arguments.extend(("--proof-object=1", "--force-deriv=2"))
    arguments.append(str(problem))
    return arguments


def execute_search(
    *,
    binary: Path,
    budget: dict[str, int],
    cpu: int,
    environment: dict[str, str],
    phase_kind: str,
    root: Path,
    strategy: str,
    telemetry_enabled: bool,
    proof: bool,
    problem: Path,
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    output_path = root / ("proof.tstp" if proof else "stdout.txt")
    stderr_path = root / "stderr.txt"
    timing_path = root / "timing.txt"
    telemetry_path = root / "telemetry.json" if telemetry_enabled else None
    for path in (output_path, stderr_path, timing_path, telemetry_path):
        if path is not None:
            path.unlink(missing_ok=True)
    temporary_root = root / "tmp"
    if temporary_root.exists():
        shutil.rmtree(temporary_root)
    temporary_root.mkdir()
    phase_environment = dict(environment)
    phase_environment["TMPDIR"] = str(temporary_root)
    umlaut_arguments = search_arguments(
        budget=budget,
        phase_kind=phase_kind,
        strategy=strategy,
        telemetry_path=telemetry_path,
        proof=proof,
        problem=problem,
    )
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(timing_path),
        "taskset",
        "--cpu-list",
        str(cpu),
        str(binary),
        *umlaut_arguments,
    ]
    started_at = utc_now()
    started = time.monotonic()
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=phase_environment,
        start_new_session=True,
    )
    external_timeout = False
    cleanup_seconds = 0.0
    try:
        stdout, stderr = process.communicate(
            timeout=budget["hard_cpu_seconds"] + 10
        )
    except subprocess.TimeoutExpired:
        external_timeout = True
        cleanup_seconds = terminate_group(process)
        stdout, stderr = process.communicate()
    wall_seconds = time.monotonic() - started
    output_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    text = stdout.decode("utf-8", errors="replace")
    stderr_text = stderr.decode("utf-8", errors="replace")
    telemetry, telemetry_sha256 = (
        telemetry_record(telemetry_path)
        if telemetry_path is not None
        else (None, None)
    )
    timing = parse_timing(timing_path)
    residue = sorted(
        str(path.relative_to(temporary_root))
        for path in temporary_root.rglob("*")
        if path.is_file() or path.is_symlink()
    )
    return {
        "phase_kind": phase_kind,
        "strategy": strategy,
        "telemetry_enabled": telemetry_enabled,
        "proof": proof,
        "soft_cpu_seconds": budget["soft_cpu_seconds"],
        "hard_cpu_seconds": budget["hard_cpu_seconds"],
        "cpu": cpu,
        "command": command,
        "umlaut_arguments": umlaut_arguments,
        "started_at": started_at,
        "completed_at": utc_now(),
        "return_code": process.returncode,
        "external_timeout": external_timeout,
        "cleanup_seconds": cleanup_seconds,
        "process_group_survived": process_group_alive(process.pid),
        "controller_wall_seconds": wall_seconds,
        "szs_status": common.final_status(text, stderr_text),
        "processed_clauses": common.processed_clause_count(text),
        "output_path": str(output_path),
        "output_sha256": common.sha256_file(output_path),
        "stderr_path": str(stderr_path),
        "stderr_sha256": common.sha256_file(stderr_path),
        "timing_path": str(timing_path) if timing_path.is_file() else None,
        "timing_sha256": (
            common.sha256_file(timing_path)
            if timing_path.is_file()
            else None
        ),
        "timing": timing,
        "telemetry_path": (
            str(telemetry_path)
            if telemetry_path is not None and telemetry_path.is_file()
            else None
        ),
        "telemetry_sha256": telemetry_sha256,
        "temp_residue": residue,
        "_telemetry": telemetry,
    }


def run_validation_gate(
    *,
    validation_gate: Path,
    proofcheck: Path,
    problem: Path,
    proof: Path,
    report: Path,
) -> dict[str, Any]:
    stdout_path = report.with_suffix(".stdout.txt")
    stderr_path = report.with_suffix(".stderr.txt")
    for path in (report, stdout_path, stderr_path):
        path.unlink(missing_ok=True)
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
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    return {
        "command": command,
        "report_path": str(report) if report.is_file() else None,
        "report_sha256": (
            common.sha256_file(report) if report.is_file() else None
        ),
        "return_code": completed.returncode,
        "stdout_path": str(stdout_path),
        "stdout_sha256": common.sha256_file(stdout_path),
        "stderr_path": str(stderr_path),
        "stderr_sha256": common.sha256_file(stderr_path),
        "verified": completed.returncode == 0,
    }


def stored_search(search: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in search.items() if key != "_telemetry"}


def replay_proof(
    *,
    binary: Path,
    cpu: int,
    environment: dict[str, str],
    expected_status: str,
    phase: dict[str, Any],
    problem: Path,
    proofcheck: Path,
    root: Path,
    validation_gate: Path,
) -> dict[str, Any]:
    replay = execute_search(
        binary=binary,
        budget={
            "soft_cpu_seconds": int(phase["soft_cpu_seconds"]),
            "hard_cpu_seconds": int(phase["hard_cpu_seconds"]),
        },
        cpu=cpu,
        environment=environment,
        phase_kind=str(phase["phase_kind"]),
        root=root,
        strategy=str(phase["strategy"]),
        telemetry_enabled=bool(phase["telemetry_enabled"]),
        proof=True,
        problem=problem,
    )
    proof = Path(replay["output_path"])
    gate = run_validation_gate(
        validation_gate=validation_gate,
        proofcheck=proofcheck,
        problem=problem,
        proof=proof,
        report=root / "validation.json",
    )
    return {
        **stored_search(replay),
        "expected_status": expected_status,
        "gate": gate,
        "reproduced": (
            replay["return_code"] == 0
            and replay["szs_status"] == expected_status
            and proof.stat().st_size > 0
            and gate["verified"]
            and not replay["external_timeout"]
            and not replay["process_group_survived"]
            and not replay["temp_residue"]
        ),
    }


def phase_failures(
    phase: dict[str, Any], expected_class: str
) -> list[str]:
    failures = []
    if phase["external_timeout"]:
        failures.append("external_timeout")
    if phase["process_group_survived"]:
        failures.append("surviving_process_group")
    if phase["temp_residue"]:
        failures.append("temporary_file_residue")
    timing = phase["timing"]
    if timing is None:
        failures.append("missing_external_timing")
    elif (
        float(timing["total_cpu_seconds"])
        > float(phase["hard_cpu_seconds"]) + 0.5
    ):
        failures.append("hard_cpu_budget_violation")
    if not common.status_is_acceptable(
        phase["szs_status"], expected_class
    ):
        failures.append(f"unexpected_status:{phase['szs_status']}")
    return failures


def result_is_resumable(
    path: Path,
    *,
    binary_sha256: str,
    contract_id: str,
    problem_sha256: str,
) -> bool:
    if not path.is_file():
        return False
    try:
        result = json.loads(path.read_text(encoding="utf-8"))
        if (
            result["binary_sha256"] != binary_sha256
            or result["contract_id"] != contract_id
            or result["problem_sha256"] != problem_sha256
        ):
            return False
        searches = [*result["phases"]]
        if result["proof_replay"] is not None:
            searches.append(result["proof_replay"])
        for search in searches:
            for path_key, hash_key in (
                ("output_path", "output_sha256"),
                ("stderr_path", "stderr_sha256"),
                ("timing_path", "timing_sha256"),
                ("telemetry_path", "telemetry_sha256"),
            ):
                artifact = search.get(path_key)
                expected = search.get(hash_key)
                if artifact is None or expected is None:
                    if artifact is not None or expected is not None:
                        return False
                    continue
                if common.sha256_file(Path(artifact)) != expected:
                    return False
        replay = result["proof_replay"]
        if replay is not None:
            gate = replay["gate"]
            for path_key, hash_key in (
                ("report_path", "report_sha256"),
                ("stdout_path", "stdout_sha256"),
                ("stderr_path", "stderr_sha256"),
            ):
                if common.sha256_file(Path(gate[path_key])) != gate[hash_key]:
                    return False
        return True
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError):
        return False


@contextmanager
def cpu_slot(slots: queue.Queue[int]) -> Iterator[int]:
    cpu = slots.get()
    try:
        yield cpu
    finally:
        slots.put(cpu)


def run_policy(
    *,
    binary: Path,
    binary_sha256: str,
    budgets: dict[str, dict[str, int]],
    contract_id: str,
    cpu_slots: queue.Queue[int],
    environment: dict[str, str],
    output_root: Path,
    phase_name: str,
    policy: str,
    problem_root: Path,
    proofcheck: Path,
    record: dict[str, Any],
    repetition: int,
    validation_gate: Path,
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
        binary_sha256=binary_sha256,
        contract_id=contract_id,
        problem_sha256=str(record["sha256"]),
    ):
        return {"resumed": True, "result_path": str(result_path)}
    run_dir.mkdir(parents=True, exist_ok=True)
    problem = problem_root / str(record["path"])
    phases: list[dict[str, Any]] = []
    decision: dict[str, Any] | None = None
    decision_cpu_seconds = 0.0
    decision_wall_seconds = 0.0
    with cpu_slot(cpu_slots) as cpu:
        if policy in {
            "probe_without_telemetry",
            "probe_with_telemetry",
        }:
            phases.append(
                execute_search(
                    binary=binary,
                    budget=budgets["probe"],
                    cpu=cpu,
                    environment=environment,
                    phase_kind="probe",
                    root=run_dir / "phase-1",
                    strategy="global",
                    telemetry_enabled=policy == "probe_with_telemetry",
                    proof=False,
                    problem=problem,
                )
            )
        elif policy in {"global_full", "goal_full"}:
            phases.append(
                execute_search(
                    binary=binary,
                    budget=budgets["full"],
                    cpu=cpu,
                    environment=environment,
                    phase_kind="full",
                    root=run_dir / "phase-1",
                    strategy=(
                        "global" if policy == "global_full" else "goal"
                    ),
                    telemetry_enabled=True,
                    proof=False,
                    problem=problem,
                )
            )
        else:
            probe = execute_search(
                binary=binary,
                budget=budgets["probe"],
                cpu=cpu,
                environment=environment,
                phase_kind="probe",
                root=run_dir / "phase-1",
                strategy="global",
                telemetry_enabled=True,
                proof=False,
                problem=problem,
            )
            phases.append(probe)
            if probe["szs_status"] not in common.PROOF_STATUSES:
                if policy == "static_global_restart":
                    branch = "global"
                elif policy == "static_goal":
                    branch = "goal"
                elif policy == "adaptive":
                    cpu_started = time.process_time_ns()
                    wall_started = time.monotonic_ns()
                    decision = common.choose_branch(probe["_telemetry"])
                    decision_cpu_seconds = (
                        time.process_time_ns() - cpu_started
                    ) / 1.0e9
                    decision_wall_seconds = (
                        time.monotonic_ns() - wall_started
                    ) / 1.0e9
                    branch = str(decision["branch"])
                else:
                    raise common.ExperimentError(
                        f"unknown restart policy: {policy}"
                    )
                phases.append(
                    execute_search(
                        binary=binary,
                        budget=budgets["continuation"],
                        cpu=cpu,
                        environment=environment,
                        phase_kind="continuation",
                        root=run_dir / "phase-2",
                        strategy=branch,
                        telemetry_enabled=True,
                        proof=False,
                        problem=problem,
                    )
                )
            elif policy == "adaptive":
                decision = {
                    "threshold": common.THRESHOLD,
                    "branch": "probe_solved",
                    **common.signal_from_telemetry(probe["_telemetry"]),
                }
        proof_phases = [
            item
            for item in phases
            if item["szs_status"] in common.PROOF_STATUSES
        ]
        final_phase = proof_phases[0] if proof_phases else phases[-1]
        proof_replay = (
            replay_proof(
                binary=binary,
                cpu=cpu,
                environment=environment,
                expected_status=str(final_phase["szs_status"]),
                phase=final_phase,
                problem=problem,
                proofcheck=proofcheck,
                root=run_dir / "proof-replay",
                validation_gate=validation_gate,
            )
            if proof_phases
            else None
        )
    correctness_failures = [
        f"phase-{index}:{failure}"
        for index, phase in enumerate(phases, start=1)
        for failure in phase_failures(
            phase, str(record["expected_class"])
        )
    ]
    if proof_phases and (
        proof_replay is None or not proof_replay["reproduced"]
    ):
        correctness_failures.append("proof_replay_failed")
    if not proof_phases and proof_replay is not None:
        correctness_failures.append("proof_replay_without_proof")
    if policy == "adaptive" and decision is None:
        correctness_failures.append("adaptive_decision_missing")
    timing = [item["timing"] for item in phases]
    result = {
        "schema_version": 1,
        "kind": "deterministic-adaptive-probe-result",
        "contract_id": contract_id,
        "phase": phase_name,
        "policy": policy,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "category": record["category"],
        "difficulty_band": record["difficulty_band"],
        "family": record["family"],
        "expected_class": record["expected_class"],
        "repetition": repetition,
        "binary_sha256": binary_sha256,
        "configured_soft_cpu_seconds": sum(
            int(item["soft_cpu_seconds"]) for item in phases
        ),
        "resources": {
            "total_cpu_seconds": (
                sum(float(item["total_cpu_seconds"]) for item in timing)
                if all(item is not None for item in timing)
                else None
            ),
            "wall_seconds": sum(
                float(item["controller_wall_seconds"]) for item in phases
            ),
            "peak_rss_kib": (
                max(int(item["peak_rss_kib"]) for item in timing)
                if all(item is not None for item in timing)
                else None
            ),
        },
        "szs_status": final_phase["szs_status"],
        "decision": decision,
        "decision_cpu_seconds": decision_cpu_seconds,
        "decision_wall_seconds": decision_wall_seconds,
        "phases": [stored_search(item) for item in phases],
        "proof_replay": proof_replay,
        "correctness_failures": sorted(set(correctness_failures)),
    }
    common.atomic_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path)}


def validate_cpu_list(cpu_list: Sequence[int]) -> None:
    if len(cpu_list) != 4 or len(set(cpu_list)) != 4:
        raise common.ExperimentError("exactly four distinct CPUs are required")
    available = os.sched_getaffinity(0)
    missing = set(cpu_list) - available
    if missing:
        raise common.ExperimentError(
            f"requested CPUs are unavailable: {sorted(missing)}"
        )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=CORPUS_PATH)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--corpus-report", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("train", "validation", "test"), required=True
    )
    parser.add_argument("--validation-report", type=Path)
    parser.add_argument("--cpu-list", default="0,1,2,3")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--smoke-problem")
    parser.add_argument("--contract-preview", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise common.ExperimentError(
            "prover experiments may run only on Linux"
        )
    if arguments.source_revision != common.SOURCE_REVISION:
        raise common.ExperimentError(
            "source revision differs from preregistration"
        )
    cpu_list = [int(value) for value in arguments.cpu_list.split(",")]
    validate_cpu_list(cpu_list)
    manifest = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    corpus_report_path = arguments.corpus_report.resolve()
    binary = arguments.binary.resolve()
    proofcheck = arguments.proofcheck.resolve()
    validation_gate = arguments.validation_gate.resolve()
    output_root = arguments.output_root.resolve()
    for path, label in (
        (binary, "Umlaut binary"),
        (proofcheck, "ProofCheck"),
        (validation_gate, "validation gate"),
        (corpus_report_path, "corpus report"),
    ):
        if not path.is_file():
            raise common.ExperimentError(f"{label} is missing: {path}")
    records = phase_records(
        manifest,
        arguments.phase,
        smoke=arguments.smoke,
        smoke_problem=arguments.smoke_problem,
    )
    corpus_report = load_corpus_report(corpus_report_path)
    verify_problem_inputs(
        problem_root=problem_root,
        records=records,
        corpus_report=corpus_report,
    )
    validation_report = None
    if arguments.phase == "test":
        if (
            arguments.validation_report is None
            or not arguments.validation_report.is_file()
        ):
            raise common.ExperimentError(
                "test requires --validation-report"
            )
        validation_report = arguments.validation_report.resolve()
        load_validation_report(validation_report)
    elif arguments.validation_report is not None:
        raise common.ExperimentError(
            "--validation-report is accepted only for test"
        )
    repetitions = 1 if arguments.phase == "train" else 2
    if arguments.smoke:
        repetitions = 1
    body = contract_body(
        binary=binary,
        corpus_report_path=corpus_report_path,
        cpu_list=cpu_list,
        manifest=manifest,
        phase=arguments.phase,
        proofcheck=proofcheck,
        records=records,
        repetitions=repetitions,
        smoke=arguments.smoke,
        validation_gate=validation_gate,
        validation_report=validation_report,
    )
    if arguments.contract_preview:
        print(json.dumps(body, indent=2, sort_keys=True))
        return 0
    output_root.mkdir(parents=True, exist_ok=True)
    contract = initialize_contract(output_root, body)
    environment = dict(os.environ)
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    slots: queue.Queue[int] = queue.Queue()
    for cpu in cpu_list:
        slots.put(cpu)
    budgets = SMOKE_BUDGETS if arguments.smoke else BUDGETS
    binary_sha256 = common.sha256_file(binary)
    jobs = [
        (policy, record, repetition)
        for policy in POLICIES
        for record in records
        for repetition in range(1, repetitions + 1)
    ]
    completed = 0
    resumed = 0
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=len(cpu_list)
    ) as executor:
        futures = [
            executor.submit(
                run_policy,
                binary=binary,
                binary_sha256=binary_sha256,
                budgets=budgets,
                contract_id=contract["contract_id"],
                cpu_slots=slots,
                environment=environment,
                output_root=output_root,
                phase_name=arguments.phase,
                policy=policy,
                problem_root=problem_root,
                proofcheck=proofcheck,
                record=record,
                repetition=repetition,
                validation_gate=validation_gate,
            )
            for policy, record, repetition in jobs
        ]
        for future in concurrent.futures.as_completed(futures):
            outcome = future.result()
            resumed += int(outcome["resumed"])
            completed += int(not outcome["resumed"])
    print(
        json.dumps(
            {
                "completed": completed,
                "contract_id": contract["contract_id"],
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
