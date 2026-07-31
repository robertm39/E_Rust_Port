#!/usr/bin/env python3
"""Run the frozen four-core portfolio/cooperation comparison on Linux."""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import resource
import signal
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence

from common import (
    BAD_STATUSES,
    CORPUS_SHA256,
    MEMORY_MIB,
    PROOF_STATUSES,
    SOURCE_REVISION,
    ExperimentError,
    atomic_json,
    canonical_json,
    final_status,
    load_corpus,
    normalize_tptp,
    parse_saturated_clauses,
    proof_step_count,
    rank_peer_clauses,
    render_wrapper,
    sha256_bytes,
    sha256_file,
)


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PREREGISTRATION = EXPERIMENT_ROOT / "PREREGISTRATION.md"
ARMS = (
    "independent_equal",
    "independent_unequal",
    "restart_control",
    "share_4",
    "share_16",
    "share_64",
)
SHARING_CAPS = {"share_4": 4, "share_16": 16, "share_64": 64}
CHECKPOINTS = (128, 512)
SEEDS = (
    (11, 13, 17),
    (19, 23, 29),
    (31, 37, 41),
    (43, 47, 53),
)
BASE_EVALUATORS = (
    "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)",
    "Refinedweight(PreferGoals,2,1,1.5,1.1,1.1)",
    "Refinedweight(PreferNonGoals,2,1,1.5,1.1,1.1)",
    "OrientLMaxWeight(ConstPrio,2,1,2,1,1)",
)
TIME_FIELD = re.compile(r"^\s*([^:]+):\s*(.+?)\s*$")


@dataclass
class LiveWorker:
    command: list[str]
    cpu: int
    index: int
    input_path: Path
    popen: subprocess.Popen[bytes]
    stderr_handle: Any
    stderr_path: Path
    stdout_handle: Any
    stdout_path: Path
    telemetry_path: Path
    timing_path: Path


def worker_heuristic(index: int, guidance: bool) -> tuple[str, str]:
    priority = "PreferWatchlist" if guidance else "ConstPrio"
    seed = ",".join(str(value) for value in SEEDS[index])
    weight = (
        f"seeded_random=RandomWeight({priority},1000,0.01,1.0,{seed})"
    )
    heuristic = (
        f"CoopWorker{index}=(5*{BASE_EVALUATORS[index]},"
        "1*seeded_random,1*FIFOWeight(ConstPrio))"
    )
    return weight, heuristic


def worker_args(
    *,
    index: int,
    guidance: bool,
    input_path: Path,
    telemetry_path: Path,
    soft_cpu: int,
    processed_limit: int | None,
    print_saturated: bool,
) -> list[str]:
    weight, heuristic = worker_heuristic(index, guidance)
    arguments = [
        "--term-ordering=KBO6",
        "--forward-demod-level=2",
        f"--define-weight-function={weight}",
        f"--define-heuristic={heuristic}",
        f"--expert-heuristic=CoopWorker{index}",
        f"--soft-cpu-limit={soft_cpu}",
        f"--cpu-limit={soft_cpu + 2}",
        f"--memory-limit={MEMORY_MIB}",
        "--resources-info",
        f"--search-telemetry={telemetry_path}",
        "--tstp-out",
    ]
    if guidance:
        arguments.append("--static-watchlist=Use inline watchlist type")
    if processed_limit is not None:
        arguments.append(f"--processed-clauses-limit={processed_limit}")
    if print_saturated:
        arguments.extend(("--print-saturated=eig", "--print-sat-info"))
    arguments.extend(("--tstp-in", str(input_path)))
    return arguments


def parse_time_report(path: Path) -> dict[str, Any] | None:
    if not path.is_file() or path.stat().st_size == 0:
        return None
    fields: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = TIME_FIELD.match(line)
        if match is not None:
            fields[match.group(1)] = match.group(2)
    try:
        user = float(fields["User time (seconds)"])
        system = float(fields["System time (seconds)"])
        rss = int(fields["Maximum resident set size (kbytes)"])
    except (KeyError, ValueError):
        return None
    return {
        "max_rss_kib": rss,
        "system_cpu_seconds": system,
        "total_cpu_seconds": user + system,
        "user_cpu_seconds": user,
    }


def process_group_rss_kib(groups: set[int]) -> tuple[int, int]:
    total = 0
    processes = 0
    page_kib = os.sysconf("SC_PAGE_SIZE") // 1024
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text(encoding="ascii")
            suffix = stat[stat.rfind(")") + 2 :].split()
            group = int(suffix[2])
            if group not in groups:
                continue
            resident_pages = int(
                (entry / "statm").read_text(encoding="ascii").split()[1]
            )
        except (FileNotFoundError, PermissionError, ValueError, IndexError):
            continue
        total += resident_pages * page_kib
        processes += 1
    return total, processes


def kill_group(worker: LiveWorker, sig: signal.Signals) -> None:
    if worker.popen.poll() is None:
        try:
            os.killpg(worker.popen.pid, sig)
        except ProcessLookupError:
            pass


def close_worker(worker: LiveWorker) -> None:
    worker.stdout_handle.close()
    worker.stderr_handle.close()


def run_wave(
    *,
    binary: Path,
    cpu_list: Sequence[int],
    root: Path,
    inputs: Sequence[Path],
    soft_budgets: Sequence[int],
    guidance: bool,
    processed_limit: int | None,
    print_saturated: bool,
    cancel_on_proof: bool,
    environment: dict[str, str],
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.monotonic()
    live: list[LiveWorker] = []
    for index, (cpu, input_path, soft_cpu) in enumerate(
        zip(cpu_list, inputs, soft_budgets, strict=True)
    ):
        worker_root = root / f"worker-{index}"
        worker_root.mkdir(parents=True, exist_ok=True)
        temporary_root = worker_root / "tmp"
        temporary_root.mkdir(parents=True, exist_ok=True)
        stdout_path = worker_root / "stdout.txt"
        stderr_path = worker_root / "stderr.txt"
        timing_path = worker_root / "time.txt"
        telemetry_path = worker_root / "telemetry.json"
        arguments = worker_args(
            index=index,
            guidance=guidance,
            input_path=input_path,
            telemetry_path=telemetry_path,
            soft_cpu=soft_cpu,
            processed_limit=processed_limit,
            print_saturated=print_saturated,
        )
        command = ["taskset", "--cpu-list", str(cpu)]
        if not cancel_on_proof:
            command.extend(
                ["/usr/bin/time", "-v", "-o", str(timing_path)]
            )
        command.extend([str(binary), *arguments])
        stdout_handle = stdout_path.open("wb")
        stderr_handle = stderr_path.open("wb")
        try:
            worker_environment = dict(environment)
            worker_environment["TMPDIR"] = str(temporary_root)
            popen = subprocess.Popen(
                command,
                stdout=stdout_handle,
                stderr=stderr_handle,
                env=worker_environment,
                start_new_session=True,
            )
        except BaseException:
            stdout_handle.close()
            stderr_handle.close()
            for worker in live:
                kill_group(worker, signal.SIGKILL)
                worker.popen.wait()
                close_worker(worker)
            raise
        live.append(
            LiveWorker(
                command=command,
                cpu=cpu,
                index=index,
                input_path=input_path,
                popen=popen,
                stderr_handle=stderr_handle,
                stderr_path=stderr_path,
                stdout_handle=stdout_handle,
                stdout_path=stdout_path,
                telemetry_path=telemetry_path,
                timing_path=timing_path,
            )
        )

    peak_rss_kib = 0
    peak_processes = 0
    winner: int | None = None
    cancelled: set[int] = set()
    wall_deadline = started + max(soft_budgets) + 15.0
    while any(worker.popen.poll() is None for worker in live):
        rss, processes = process_group_rss_kib(
            {worker.popen.pid for worker in live}
        )
        peak_rss_kib = max(peak_rss_kib, rss)
        peak_processes = max(peak_processes, processes)
        if cancel_on_proof and winner is None:
            completed_proofs: list[int] = []
            for worker in live:
                if worker.popen.poll() is None:
                    continue
                worker.stdout_handle.flush()
                text = worker.stdout_path.read_text(
                    encoding="utf-8", errors="replace"
                )
                if final_status(text) in PROOF_STATUSES:
                    completed_proofs.append(worker.index)
            if completed_proofs:
                winner = min(completed_proofs)
                for worker in live:
                    if worker.popen.poll() is None:
                        cancelled.add(worker.index)
                        kill_group(worker, signal.SIGTERM)
        if time.monotonic() >= wall_deadline:
            for worker in live:
                if worker.popen.poll() is None:
                    cancelled.add(worker.index)
                    kill_group(worker, signal.SIGTERM)
            break
        time.sleep(0.01)

    grace_deadline = time.monotonic() + 1.0
    while any(worker.popen.poll() is None for worker in live):
        if time.monotonic() >= grace_deadline:
            for worker in live:
                if worker.popen.poll() is None:
                    kill_group(worker, signal.SIGKILL)
            break
        time.sleep(0.01)
    for worker in live:
        worker.popen.wait()
        close_worker(worker)

    survivors = process_group_rss_kib({worker.popen.pid for worker in live})[1]
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    workers: list[dict[str, Any]] = []
    for worker in live:
        stdout = worker.stdout_path.read_text(encoding="utf-8", errors="replace")
        stderr = worker.stderr_path.read_text(encoding="utf-8", errors="replace")
        status = final_status(stdout)
        workers.append(
            {
                "cancelled": worker.index in cancelled,
                "command": worker.command,
                "cpu": worker.cpu,
                "index": worker.index,
                "input_path": str(worker.input_path),
                "input_sha256": sha256_file(worker.input_path),
                "return_code": worker.popen.returncode,
                "status": status,
                "stderr_path": str(worker.stderr_path),
                "stderr_sha256": sha256_file(worker.stderr_path),
                "stderr_size_bytes": len(stderr.encode("utf-8")),
                "stdout_path": str(worker.stdout_path),
                "stdout_sha256": sha256_file(worker.stdout_path),
                "stdout_size_bytes": len(stdout.encode("utf-8")),
                "telemetry_path": (
                    str(worker.telemetry_path)
                    if worker.telemetry_path.is_file()
                    else None
                ),
                "telemetry_sha256": (
                    sha256_file(worker.telemetry_path)
                    if worker.telemetry_path.is_file()
                    else None
                ),
                "temp_residue": sorted(
                    str(path.relative_to(temporary_root))
                    for path in temporary_root.rglob("*")
                    if path.is_file() or path.is_symlink()
                ),
                "timing": parse_time_report(worker.timing_path),
                "timing_path": (
                    str(worker.timing_path)
                    if worker.timing_path.is_file()
                    else None
                ),
                "timing_sha256": (
                    sha256_file(worker.timing_path)
                    if worker.timing_path.is_file()
                    else None
                ),
            }
        )
    proof_workers = [
        worker["index"] for worker in workers if worker["status"] in PROOF_STATUSES
    ]
    if winner is None and proof_workers:
        winner = min(proof_workers)
    return {
        "aggregate_resources": {
            "max_sampled_processes": peak_processes,
            "peak_sampled_rss_kib": peak_rss_kib,
            "system_cpu_seconds": after.ru_stime - before.ru_stime,
            "total_cpu_seconds": (
                after.ru_utime
                - before.ru_utime
                + after.ru_stime
                - before.ru_stime
            ),
            "user_cpu_seconds": after.ru_utime - before.ru_utime,
            "wall_seconds": time.monotonic() - started,
        },
        "cancelled_workers": sorted(cancelled),
        "surviving_processes": survivors,
        "winner": winner,
        "workers": workers,
    }


def cnf_bodies(text: str) -> set[str]:
    bodies: set[str] = set()
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("cnf(") or not line.endswith(")."):
            continue
        inner = line[4:-2]
        depth = 0
        commas: list[int] = []
        quote: str | None = None
        escaped = False
        for index, char in enumerate(inner):
            if quote is not None:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    quote = None
                continue
            if char in {"'", '"'}:
                quote = char
            elif char in "([{":
                depth += 1
            elif char in ")]}":
                depth -= 1
            elif char == "," and depth == 0:
                commas.append(index)
                if len(commas) == 2:
                    break
        if len(commas) != 2:
            continue
        bodies.add(normalize_tptp(inner[commas[1] + 1 :]))
    return bodies


def run_cnf(
    *,
    binary: Path,
    cpu: int,
    problem: Path,
    root: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    stdout = root / "stdout.p"
    stderr = root / "stderr.txt"
    timing = root / "time.txt"
    command = [
        "taskset",
        "--cpu-list",
        str(cpu),
        "/usr/bin/time",
        "-v",
        "-o",
        str(timing),
        str(binary),
        "--cnf",
        "--tstp-out",
        "--output-level=1",
        f"--memory-limit={MEMORY_MIB}",
        "--tstp-in",
        str(problem),
    ]
    started = time.monotonic()
    with stdout.open("wb") as stdout_handle, stderr.open("wb") as stderr_handle:
        completed = subprocess.run(
            command,
            check=False,
            stdout=stdout_handle,
            stderr=stderr_handle,
            env=environment,
            timeout=30,
        )
    text = stdout.read_text(encoding="utf-8", errors="replace")
    bodies = cnf_bodies(text)
    if completed.returncode != 0 or not bodies:
        raise ExperimentError(
            f"canonical CNF failed for {problem}: {completed.returncode}"
        )
    return {
        "body_count": len(bodies),
        "command": command,
        "return_code": completed.returncode,
        "stderr_path": str(stderr),
        "stderr_sha256": sha256_file(stderr),
        "stdout_path": str(stdout),
        "stdout_sha256": sha256_file(stdout),
        "timing": parse_time_report(timing),
        "timing_path": str(timing),
        "timing_sha256": sha256_file(timing),
        "wall_seconds": time.monotonic() - started,
    }


def preprocess_audit(
    *,
    binary: Path,
    cpu_list: Sequence[int],
    problem: Path,
    root: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    report_path = root / "audit.json"
    if report_path.is_file():
        report = json.loads(report_path.read_text(encoding="utf-8"))
        runs = [report["single"], *report["parallel"]]
        valid = True
        for run in runs:
            for field in ("stdout", "stderr", "timing"):
                path = Path(run[f"{field}_path"])
                if (
                    not path.is_file()
                    or sha256_file(path) != run[f"{field}_sha256"]
                ):
                    valid = False
        if valid:
            hashes = {run["stdout_sha256"] for run in runs}
            if len(hashes) == 1 and hashes == {report["cnf_sha256"]}:
                return report
    single = run_cnf(
        binary=binary,
        cpu=cpu_list[0],
        problem=problem,
        root=root / "single",
        environment=environment,
    )
    started = time.monotonic()
    handles: list[tuple[subprocess.Popen[bytes], Any, Any, Path, Path, Path, list[str]]] = []
    for index, cpu in enumerate(cpu_list):
        run_root = root / "parallel" / f"worker-{index}"
        run_root.mkdir(parents=True, exist_ok=True)
        stdout = run_root / "stdout.p"
        stderr = run_root / "stderr.txt"
        timing = run_root / "time.txt"
        command = [
            "taskset",
            "--cpu-list",
            str(cpu),
            "/usr/bin/time",
            "-v",
            "-o",
            str(timing),
            str(binary),
            "--cnf",
            "--tstp-out",
            "--output-level=1",
            f"--memory-limit={MEMORY_MIB}",
            "--tstp-in",
            str(problem),
        ]
        stdout_handle = stdout.open("wb")
        stderr_handle = stderr.open("wb")
        process = subprocess.Popen(
            command,
            stdout=stdout_handle,
            stderr=stderr_handle,
            env=environment,
            start_new_session=True,
        )
        handles.append(
            (process, stdout_handle, stderr_handle, stdout, stderr, timing, command)
        )
    parallel: list[dict[str, Any]] = []
    for process, stdout_handle, stderr_handle, stdout, stderr, timing, command in handles:
        return_code = process.wait(timeout=30)
        stdout_handle.close()
        stderr_handle.close()
        text = stdout.read_text(encoding="utf-8", errors="replace")
        if return_code != 0 or not cnf_bodies(text):
            raise ExperimentError(f"parallel CNF conversion failed: {return_code}")
        parallel.append(
            {
                "body_count": len(cnf_bodies(text)),
                "command": command,
                "return_code": return_code,
                "stderr_path": str(stderr),
                "stderr_sha256": sha256_file(stderr),
                "stdout_path": str(stdout),
                "stdout_sha256": sha256_file(stdout),
                "timing": parse_time_report(timing),
                "timing_path": str(timing),
                "timing_sha256": sha256_file(timing),
            }
        )
    hashes = {single["stdout_sha256"], *(run["stdout_sha256"] for run in parallel)}
    if len(hashes) != 1:
        raise ExperimentError(f"CNF output is not reproducible for {problem}")
    report = {
        "cnf_sha256": single["stdout_sha256"],
        "kind": "cooperative-multicore-preprocess-audit",
        "parallel": parallel,
        "parallel_wall_seconds": time.monotonic() - started,
        "problem": str(problem),
        "problem_sha256": sha256_file(problem),
        "schema_version": 1,
        "single": single,
    }
    atomic_json(report_path, report)
    return report


def wrapper_inputs(
    *,
    coordinate_root: Path,
    original: Path,
    clauses_by_worker: Sequence[Sequence[dict[str, Any]]],
    cap: int,
    wave: int,
) -> tuple[list[Path], list[dict[str, Any]]]:
    inputs: list[Path] = []
    records: list[dict[str, Any]] = []
    for recipient in range(4):
        selected = rank_peer_clauses(
            clauses_by_worker, recipient=recipient, cap=cap
        )
        wrapper = coordinate_root / f"wrapper-wave-{wave}-worker-{recipient}.p"
        wrapper.write_text(
            render_wrapper(
                original, selected, wave=wave, recipient=recipient
            ),
            encoding="utf-8",
        )
        inputs.append(wrapper)
        records.append(
            {
                "clause_count": len(selected),
                "clauses": selected,
                "recipient": recipient,
                "sha256": sha256_file(wrapper),
                "size_bytes": wrapper.stat().st_size,
                "wave": wave,
                "wrapper_path": str(wrapper),
            }
        )
    return inputs, records


def run_validation_gate(
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
    stdout = report.with_suffix(".stdout.txt")
    stderr = report.with_suffix(".stderr.txt")
    stdout.write_text(completed.stdout, encoding="utf-8")
    stderr.write_text(completed.stderr, encoding="utf-8")
    return {
        "command": command,
        "report_path": str(report),
        "report_sha256": sha256_file(report) if report.is_file() else None,
        "return_code": completed.returncode,
        "stderr_path": str(stderr),
        "stderr_sha256": sha256_file(stderr),
        "stdout_path": str(stdout),
        "stdout_sha256": sha256_file(stdout),
        "verified": completed.returncode == 0,
    }


def replay_winner(
    *,
    winner: dict[str, Any],
    original: Path,
    proofcheck: Path,
    validation_gate: Path,
    root: Path,
    environment: dict[str, str],
    expected_status: str,
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    proof = root / "proof.tstp"
    stderr = root / "stderr.txt"
    telemetry = root / "telemetry.json"
    command = list(winner["command"])
    input_path = Path(winner["input_path"])
    if command[-1] != str(input_path):
        raise ExperimentError("winner command/input mismatch")
    command = [
        (
            f"--search-telemetry={telemetry}"
            if argument.startswith("--search-telemetry=")
            else argument
        )
        for argument in command
    ]
    command[-1:-1] = ["--proof-object=1", "--force-deriv=2"]
    started = time.monotonic()
    temporary_root = root / "tmp"
    temporary_root.mkdir(parents=True, exist_ok=True)
    replay_environment = dict(environment)
    replay_environment["TMPDIR"] = str(temporary_root)
    with proof.open("wb") as stdout_handle, stderr.open("wb") as stderr_handle:
        completed = subprocess.run(
            command,
            check=False,
            stdout=stdout_handle,
            stderr=stderr_handle,
            env=replay_environment,
            timeout=45,
        )
    text = proof.read_text(encoding="utf-8", errors="replace")
    status = final_status(text)
    steps = proof_step_count(text)
    original_gate = run_validation_gate(
        validation_gate=validation_gate,
        proofcheck=proofcheck,
        problem=original,
        proof=proof,
        report=root / "original-validation.json",
    )
    wrapper_gate = None
    if input_path.resolve() != original.resolve():
        wrapper_gate = run_validation_gate(
            validation_gate=validation_gate,
            proofcheck=proofcheck,
            problem=input_path,
            proof=proof,
            report=root / "wrapper-validation.json",
        )
    return {
        "command": command,
        "expected_status": expected_status,
        "logical_watchlist_reference": "coop_w" in text,
        "original_gate": original_gate,
        "proof_path": str(proof),
        "proof_sha256": sha256_file(proof),
        "proof_size_bytes": proof.stat().st_size,
        "proof_steps": steps,
        "reproduced": (
            completed.returncode == 0
            and status == expected_status
            and steps > 0
            and original_gate["verified"]
            and (wrapper_gate is None or wrapper_gate["verified"])
            and "coop_w" not in text
        ),
        "return_code": completed.returncode,
        "status": status,
        "stderr_path": str(stderr),
        "stderr_sha256": sha256_file(stderr),
        "wall_seconds": time.monotonic() - started,
        "wrapper_gate": wrapper_gate,
        "telemetry_path": str(telemetry) if telemetry.is_file() else None,
        "telemetry_sha256": (
            sha256_file(telemetry) if telemetry.is_file() else None
        ),
        "temp_residue": sorted(
            str(path.relative_to(temporary_root))
            for path in temporary_root.rglob("*")
            if path.is_file() or path.is_symlink()
        ),
    }


def parse_wave_clauses(
    *,
    wave_result: dict[str, Any],
    wave: int,
    original_bodies: set[str],
) -> tuple[list[list[dict[str, Any]]], list[str]]:
    pools: list[list[dict[str, Any]]] = [[], [], [], []]
    errors: list[str] = []
    for worker in wave_result["workers"]:
        if worker["cancelled"]:
            continue
        text = Path(worker["stdout_path"]).read_text(
            encoding="utf-8", errors="replace"
        )
        clauses, worker_errors = parse_saturated_clauses(
            text,
            producer=int(worker["index"]),
            wave=wave,
            original_bodies=original_bodies,
        )
        pools[int(worker["index"])] = clauses
        errors.extend(
            f"worker {worker['index']}: {error}" for error in worker_errors
        )
    return pools, errors


def merge_pools(
    *pool_sets: Sequence[Sequence[dict[str, Any]]],
) -> list[list[dict[str, Any]]]:
    merged: list[list[dict[str, Any]]] = [[], [], [], []]
    for pools in pool_sets:
        for index, clauses in enumerate(pools):
            merged[index].extend(clauses)
    return merged


def coordinate_failure_reasons(
    *,
    result: dict[str, Any],
    record: dict[str, Any],
) -> list[str]:
    failures: list[str] = []
    for wave in result["waves"]:
        if wave["surviving_processes"] != 0:
            failures.append("surviving_process")
        for worker in wave["workers"]:
            status = worker["status"]
            if status in BAD_STATUSES:
                failures.append(
                    f"bad_status:worker-{worker['index']}:{status}"
                )
            if worker["temp_residue"]:
                failures.append(
                    f"temp_residue:worker-{worker['index']}"
                )
    failures.extend(f"exchange:{error}" for error in result["exchange_errors"])
    replay = result.get("proof_replay")
    if result["status"] in PROOF_STATUSES and (
        replay is None or not replay["reproduced"]
    ):
        failures.append("proof_replay_failed")
    if replay is not None and replay["temp_residue"]:
        failures.append("proof_replay_temp_residue")
    return sorted(set(failures))


def existing_result_valid(result: dict[str, Any]) -> bool:
    try:
        problem = Path(result["problem_path"])
        if (
            not problem.is_file()
            or sha256_file(problem) != result["problem_sha256"]
        ):
            return False
        for wave in result["waves"]:
            for worker in wave["workers"]:
                for field in ("stdout", "stderr"):
                    path = Path(worker[f"{field}_path"])
                    if (
                        not path.is_file()
                        or sha256_file(path) != worker[f"{field}_sha256"]
                    ):
                        return False
                for field in ("telemetry", "timing"):
                    path_value = worker[f"{field}_path"]
                    hash_value = worker[f"{field}_sha256"]
                    if (path_value is None) != (hash_value is None):
                        return False
                    if path_value is not None and (
                        not Path(path_value).is_file()
                        or sha256_file(Path(path_value)) != hash_value
                    ):
                        return False
                input_path = Path(worker["input_path"])
                if (
                    not input_path.is_file()
                    or sha256_file(input_path) != worker["input_sha256"]
                ):
                    return False
        for wrapper in result["exchange"]["wrappers"]:
            path = Path(wrapper["wrapper_path"])
            if (
                not path.is_file()
                or sha256_file(path) != wrapper["sha256"]
                or path.stat().st_size != wrapper["size_bytes"]
            ):
                return False
        proof = result.get("proof_replay")
        if proof is not None:
            path = Path(proof["proof_path"])
            if (
                not path.is_file()
                or sha256_file(path) != proof["proof_sha256"]
            ):
                return False
            telemetry_path = proof["telemetry_path"]
            telemetry_sha256 = proof["telemetry_sha256"]
            if (telemetry_path is None) != (telemetry_sha256 is None):
                return False
            if telemetry_path is not None and (
                not Path(telemetry_path).is_file()
                or sha256_file(Path(telemetry_path)) != telemetry_sha256
            ):
                return False
    except (KeyError, TypeError, ValueError):
        return False
    return True


def run_coordinate(
    *,
    arm: str,
    binary: Path,
    contract_id: str,
    cpu_list: Sequence[int],
    original: Path,
    original_bodies: set[str],
    output_root: Path,
    proofcheck: Path,
    record: dict[str, Any],
    repetition: int,
    selection: dict[str, Any],
    validation_gate: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    coordinate_root = output_root / str(record["problem_id"]) / f"{arm}-r{repetition}"
    coordinate_root.mkdir(parents=True, exist_ok=True)
    result_path = coordinate_root / "result.json"
    if result_path.is_file():
        existing = json.loads(result_path.read_text(encoding="utf-8"))
        if (
            existing.get("contract_id") == contract_id
            and existing_result_valid(existing)
        ):
            return existing

    waves: list[dict[str, Any]] = []
    wrappers: list[dict[str, Any]] = []
    exchange_errors: list[str] = []
    winner: dict[str, Any] | None = None
    winning_wave: int | None = None
    if arm in {"independent_equal", "independent_unequal"}:
        budgets = (
            [4, 4, 4, 4]
            if arm == "independent_equal"
            else [int(value) for value in selection["worker_budgets"]]
        )
        wave = run_wave(
            binary=binary,
            cpu_list=cpu_list,
            root=coordinate_root / "wave-final",
            inputs=[original] * 4,
            soft_budgets=budgets,
            guidance=False,
            processed_limit=None,
            print_saturated=False,
            cancel_on_proof=True,
            environment=environment,
        )
        wave["wave"] = 0
        waves.append(wave)
        if wave["winner"] is not None:
            winner = wave["workers"][int(wave["winner"])]
            winning_wave = 0
    else:
        first = run_wave(
            binary=binary,
            cpu_list=cpu_list,
            root=coordinate_root / "wave-1",
            inputs=[original] * 4,
            soft_budgets=[1, 1, 1, 1],
            guidance=False,
            processed_limit=CHECKPOINTS[0],
            print_saturated=True,
            cancel_on_proof=True,
            environment=environment,
        )
        first["wave"] = 1
        waves.append(first)
        first_pools, errors = parse_wave_clauses(
            wave_result=first, wave=1, original_bodies=original_bodies
        )
        exchange_errors.extend(errors)
        if first["winner"] is not None:
            winner = first["workers"][int(first["winner"])]
            winning_wave = 1
        else:
            if arm in SHARING_CAPS:
                second_inputs, records = wrapper_inputs(
                    coordinate_root=coordinate_root,
                    original=original,
                    clauses_by_worker=first_pools,
                    cap=SHARING_CAPS[arm],
                    wave=2,
                )
                wrappers.extend(records)
                second_guidance = True
            else:
                second_inputs = [original] * 4
                second_guidance = False
            second = run_wave(
                binary=binary,
                cpu_list=cpu_list,
                root=coordinate_root / "wave-2",
                inputs=second_inputs,
                soft_budgets=[1, 1, 1, 1],
                guidance=second_guidance,
                processed_limit=CHECKPOINTS[1],
                print_saturated=True,
                cancel_on_proof=True,
                environment=environment,
            )
            second["wave"] = 2
            waves.append(second)
            second_pools, errors = parse_wave_clauses(
                wave_result=second, wave=2, original_bodies=original_bodies
            )
            exchange_errors.extend(errors)
            if second["winner"] is not None:
                winner = second["workers"][int(second["winner"])]
                winning_wave = 2
            else:
                if arm in SHARING_CAPS:
                    final_inputs, records = wrapper_inputs(
                        coordinate_root=coordinate_root,
                        original=original,
                        clauses_by_worker=merge_pools(first_pools, second_pools),
                        cap=SHARING_CAPS[arm],
                        wave=3,
                    )
                    wrappers.extend(records)
                    final_guidance = True
                else:
                    final_inputs = [original] * 4
                    final_guidance = False
                final = run_wave(
                    binary=binary,
                    cpu_list=cpu_list,
                    root=coordinate_root / "wave-final",
                    inputs=final_inputs,
                    soft_budgets=[2, 2, 2, 2],
                    guidance=final_guidance,
                    processed_limit=None,
                    print_saturated=False,
                    cancel_on_proof=True,
                    environment=environment,
                )
                final["wave"] = 3
                waves.append(final)
                if final["winner"] is not None:
                    winner = final["workers"][int(final["winner"])]
                    winning_wave = 3

    status = winner["status"] if winner is not None else "GaveUp"
    proof_replay = None
    if winner is not None:
        proof_replay = replay_winner(
            winner=winner,
            original=original,
            proofcheck=proofcheck,
            validation_gate=validation_gate,
            root=coordinate_root / "proof-replay",
            environment=environment,
            expected_status=str(winner["status"]),
        )
    aggregate = {
        "peak_rss_kib": max(
            wave["aggregate_resources"]["peak_sampled_rss_kib"]
            for wave in waves
        ),
        "system_cpu_seconds": sum(
            wave["aggregate_resources"]["system_cpu_seconds"] for wave in waves
        ),
        "total_cpu_seconds": sum(
            wave["aggregate_resources"]["total_cpu_seconds"] for wave in waves
        ),
        "user_cpu_seconds": sum(
            wave["aggregate_resources"]["user_cpu_seconds"] for wave in waves
        ),
        "wall_seconds": sum(
            wave["aggregate_resources"]["wall_seconds"] for wave in waves
        ),
    }
    result: dict[str, Any] = {
        "aggregate_resources": aggregate,
        "arm": arm,
        "contract_id": contract_id,
        "exchange": {
            "clause_count": sum(wrapper["clause_count"] for wrapper in wrappers),
            "unique_clause_hash": sha256_bytes(
                canonical_json(
                    sorted(
                        {
                            clause["body_sha256"]
                            for wrapper in wrappers
                            for clause in wrapper["clauses"]
                        }
                    )
                )
            ),
            "wrapper_bytes": sum(wrapper["size_bytes"] for wrapper in wrappers),
            "wrappers": wrappers,
        },
        "exchange_errors": exchange_errors,
        "expected_class": record["expected_class"],
        "family": record["family"],
        "kind": "cooperative-multicore-coordinate",
        "problem_id": record["problem_id"],
        "problem_path": str(original),
        "problem_sha256": sha256_file(original),
        "proof_replay": proof_replay,
        "repetition": repetition,
        "schema_version": 1,
        "status": status,
        "waves": waves,
        "winner": None if winner is None else int(winner["index"]),
        "winning_wave": winning_wave,
    }
    result["correctness_failures"] = coordinate_failure_reasons(
        result=result, record=record
    )
    atomic_json(result_path, result)
    return result


def calibration_selection(
    *,
    binary: Path,
    contract_id: str,
    cpu_list: Sequence[int],
    environment: dict[str, str],
    output_root: Path,
    problem_root: Path,
    proofcheck: Path,
    records: Sequence[dict[str, Any]],
    selection_output: Path,
    validation_gate: Path,
) -> dict[str, Any]:
    if selection_output.is_file():
        selected = json.loads(selection_output.read_text(encoding="utf-8"))
        unsigned = dict(selected)
        selection_id = unsigned.pop("selection_id", None)
        proof_artifacts = [
            proof
            for score in selected.get("scores", [])
            for proof in score.get("proofs", [])
        ]
        if (
            selected.get("contract_id") == contract_id
            and selection_id == sha256_bytes(canonical_json(unsigned))
            and all(
                Path(proof["proof_path"]).is_file()
                and sha256_file(Path(proof["proof_path"]))
                == proof["proof_sha256"]
                for proof in proof_artifacts
            )
        ):
            return selected
    scores = [
        {"cpu": [], "proofs": [], "solve_count": 0, "worker": index}
        for index in range(4)
    ]
    for record in records:
        original = problem_root / str(record["path"])
        root = output_root / "_solo" / str(record["problem_id"])
        wave = run_wave(
            binary=binary,
            cpu_list=cpu_list,
            root=root,
            inputs=[original] * 4,
            soft_budgets=[4, 4, 4, 4],
            guidance=False,
            processed_limit=None,
            print_saturated=False,
            cancel_on_proof=False,
            environment=environment,
        )
        for worker in wave["workers"]:
            index = int(worker["index"])
            if worker["status"] not in PROOF_STATUSES:
                continue
            replay = replay_winner(
                winner=worker,
                original=original,
                proofcheck=proofcheck,
                validation_gate=validation_gate,
                root=root / f"proof-replay-{index}",
                environment=environment,
                expected_status=str(worker["status"]),
            )
            if not replay["reproduced"]:
                raise ExperimentError(
                    f"solo proof replay failed: {record['problem_id']} worker {index}"
                )
            scores[index]["solve_count"] += 1
            scores[index]["proofs"].append(
                {
                    "problem_id": record["problem_id"],
                    "proof_path": replay["proof_path"],
                    "proof_sha256": replay["proof_sha256"],
                }
            )
            timing = worker["timing"]
            if timing is not None:
                scores[index]["cpu"].append(timing["total_cpu_seconds"])
    for score in scores:
        score["median_solve_cpu_seconds"] = (
            statistics.median(score["cpu"]) if score["cpu"] else None
        )
    ranking = sorted(
        range(4),
        key=lambda index: (
            -int(scores[index]["solve_count"]),
            (
                float(scores[index]["median_solve_cpu_seconds"])
                if scores[index]["median_solve_cpu_seconds"] is not None
                else math.inf
            ),
            index,
        ),
    )
    budget_by_rank = [7, 4, 3, 2]
    budgets = [0, 0, 0, 0]
    for rank, worker in enumerate(ranking):
        budgets[worker] = budget_by_rank[rank]
    selection = {
        "contract_id": contract_id,
        "kind": "cooperative-multicore-selection",
        "ranking": ranking,
        "schema_version": 1,
        "scores": scores,
        "worker_budgets": budgets,
    }
    selection["selection_id"] = sha256_bytes(canonical_json(selection))
    atomic_json(selection_output, selection)
    return selection


def script_hashes() -> dict[str, str]:
    paths = sorted(EXPERIMENT_ROOT.glob("*.py")) + [PREREGISTRATION]
    return {path.name: sha256_file(path) for path in paths}


def make_contract(
    *,
    binary: Path,
    cpu_list: Sequence[int],
    manifest: Path,
    phase: str,
    proofcheck: Path,
    records: Sequence[dict[str, Any]],
    selection: dict[str, Any] | None,
    validation_gate: Path,
    validation_report: Path | None,
) -> dict[str, Any]:
    contract: dict[str, Any] = {
        "arms": list(ARMS),
        "binary": {"path": str(binary), "sha256": sha256_file(binary)},
        "budgets": {
            "independent_equal": [4, 4, 4, 4],
            "periodic": [1, 1, 2],
            "total_soft_cpu_seconds": 16,
        },
        "checkpoints": list(CHECKPOINTS),
        "corpus": {"path": str(manifest), "sha256": CORPUS_SHA256},
        "cpu_list": list(cpu_list),
        "kind": "cooperative-multicore-contract",
        "memory_mib_per_process": MEMORY_MIB,
        "phase": phase,
        "problem_ids": [record["problem_id"] for record in records],
        "proofcheck": {
            "path": str(proofcheck),
            "sha256": sha256_file(proofcheck),
        },
        "schema_version": 1,
        "script_hashes": script_hashes(),
        "seeds": [list(seed) for seed in SEEDS],
        "selection_id": None if selection is None else selection["selection_id"],
        "sharing_caps": SHARING_CAPS,
        "source_revision": SOURCE_REVISION,
        "validation_gate": {
            "path": str(validation_gate),
            "sha256": sha256_file(validation_gate),
        },
        "validation_report": (
            None
            if validation_report is None
            else {
                "path": str(validation_report),
                "sha256": sha256_file(validation_report),
            }
        ),
        "worker_evaluators": list(BASE_EVALUATORS),
    }
    contract["contract_id"] = sha256_bytes(canonical_json(contract))
    return contract


def validate_cpu_list(cpu_list: Sequence[int]) -> None:
    if len(cpu_list) != 4 or len(set(cpu_list)) != 4:
        raise ExperimentError("--cpu-list must contain four distinct CPUs")
    available = os.sched_getaffinity(0)
    missing = set(cpu_list) - available
    if missing:
        raise ExperimentError(f"requested CPUs are unavailable: {sorted(missing)}")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("train", "validation", "test"), required=True
    )
    parser.add_argument("--cpu-list", default="0,1,2,3")
    parser.add_argument("--selection", type=Path)
    parser.add_argument("--selection-output", type=Path)
    parser.add_argument("--validation-report", type=Path)
    parser.add_argument("--repetitions", type=int)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("the experiment controller requires Linux")
    cpu_list = [int(value) for value in arguments.cpu_list.split(",")]
    validate_cpu_list(cpu_list)
    repo_root = arguments.repo_root.resolve()
    manifest = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    output_root = arguments.output_root.resolve()
    binary = arguments.binary.resolve()
    proofcheck = arguments.proofcheck.resolve()
    validation_gate = arguments.validation_gate.resolve()
    for path, label in (
        (binary, "Umlaut binary"),
        (proofcheck, "ProofCheck binary"),
        (validation_gate, "validation gate"),
    ):
        if not path.is_file():
            raise ExperimentError(f"{label} is missing: {path}")
    if arguments.source_revision != SOURCE_REVISION:
        raise ExperimentError(
            "--source-revision does not match preregistration"
        )
    _, all_records = load_corpus(manifest)
    records = [
        record
        for record in all_records
        if record["experiment_split"] == arguments.phase
    ]
    repetitions = arguments.repetitions or (1 if arguments.phase == "train" else 2)
    if repetitions != (1 if arguments.phase == "train" else 2):
        raise ExperimentError("repetition count differs from preregistration")
    environment = dict(os.environ)
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    selection: dict[str, Any] | None = None
    if arguments.phase != "train":
        if arguments.selection is None or not arguments.selection.is_file():
            raise ExperimentError("held-out phases require --selection")
        selection = json.loads(arguments.selection.read_text(encoding="utf-8"))
        if selection.get("kind") != "cooperative-multicore-selection":
            raise ExperimentError("invalid selection file")
        unsigned_selection = dict(selection)
        selection_id = unsigned_selection.pop("selection_id", None)
        if selection_id != sha256_bytes(canonical_json(unsigned_selection)):
            raise ExperimentError("selection hash mismatch")
        budgets = selection.get("worker_budgets")
        if (
            not isinstance(budgets, list)
            or sorted(budgets) != [2, 3, 4, 7]
        ):
            raise ExperimentError("selection worker budgets are invalid")
    if arguments.phase == "test":
        if (
            arguments.validation_report is None
            or not arguments.validation_report.is_file()
        ):
            raise ExperimentError("test requires --validation-report")
        validation_analysis = json.loads(
            arguments.validation_report.read_text(encoding="utf-8")
        )
        if (
            validation_analysis.get("kind")
            != "cooperative-multicore-phase-analysis"
            or validation_analysis.get("phase") != "validation"
            or validation_analysis.get("correctness_failures")
        ):
            raise ExperimentError("validation report is not an accepted analysis")
    elif arguments.validation_report is not None:
        raise ExperimentError("--validation-report is accepted only for test")

    preliminary = make_contract(
        binary=binary,
        cpu_list=cpu_list,
        manifest=manifest,
        phase=arguments.phase,
        proofcheck=proofcheck,
        records=records,
        selection=selection,
        validation_gate=validation_gate,
        validation_report=arguments.validation_report,
    )
    if arguments.phase == "train":
        if arguments.selection_output is None:
            raise ExperimentError("train requires --selection-output")
        selection = calibration_selection(
            binary=binary,
            contract_id=preliminary["contract_id"],
            cpu_list=cpu_list,
            environment=environment,
            output_root=output_root,
            problem_root=problem_root,
            proofcheck=proofcheck,
            records=records,
            selection_output=arguments.selection_output.resolve(),
            validation_gate=validation_gate,
        )
    contract = make_contract(
        binary=binary,
        cpu_list=cpu_list,
        manifest=manifest,
        phase=arguments.phase,
        proofcheck=proofcheck,
        records=records,
        selection=selection,
        validation_gate=validation_gate,
        validation_report=arguments.validation_report,
    )
    output_root.mkdir(parents=True, exist_ok=True)
    atomic_json(output_root / "contract.json", contract)

    completed = 0
    for record in records:
        original = problem_root / str(record["path"])
        if not original.is_file() or sha256_file(original) != record["sha256"]:
            raise ExperimentError(f"problem hash mismatch: {original}")
        audit = preprocess_audit(
            binary=binary,
            cpu_list=cpu_list,
            problem=original,
            root=output_root / "_preprocess" / str(record["problem_id"]),
            environment=environment,
        )
        canonical = Path(audit["single"]["stdout_path"])
        original_bodies = cnf_bodies(
            canonical.read_text(encoding="utf-8", errors="replace")
        )
        for repetition in range(1, repetitions + 1):
            for arm in ARMS:
                result = run_coordinate(
                    arm=arm,
                    binary=binary,
                    contract_id=contract["contract_id"],
                    cpu_list=cpu_list,
                    original=original,
                    original_bodies=original_bodies,
                    output_root=output_root,
                    proofcheck=proofcheck,
                    record=record,
                    repetition=repetition,
                    selection=selection,
                    validation_gate=validation_gate,
                    environment=environment,
                )
                if result["correctness_failures"]:
                    raise ExperimentError(
                        f"{record['problem_id']} {arm} r{repetition}: "
                        + ", ".join(result["correctness_failures"])
                    )
                completed += 1
                print(
                    f"{arguments.phase}: {record['problem_id']} {arm} "
                    f"r{repetition} -> {result['status']}",
                    flush=True,
                )
    print(
        f"completed {completed} coordinates under contract "
        f"{contract['contract_id']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
