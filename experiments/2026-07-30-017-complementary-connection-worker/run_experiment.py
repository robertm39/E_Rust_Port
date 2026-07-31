#!/usr/bin/env python3
"""Run one frozen phase of the complementary connection-worker experiment."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import platform
import re
import signal
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence

import connection_common as common


BASE_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
GOAL_WEIGHT = "Refinedweight(PreferGoals,2,1,1.5,1.1,1.1)"
FIFO = "FIFOWeight(ConstPrio)"
HEURISTICS = {
    "global_aw": f"(5*{BASE_WEIGHT},1*{FIFO})",
    "goal_hard_priority": f"(5*{GOAL_WEIGHT},1*{FIFO})",
}
PROCESSED_RE = re.compile(
    r"^[%#]\s*Processed clauses\s*:\s*(\d+)\s*$", re.MULTILINE
)
TIMING_PATTERNS = {
    "user_cpu_seconds": re.compile(r"^\s*User time \(seconds\):\s*(\S+)", re.MULTILINE),
    "system_cpu_seconds": re.compile(
        r"^\s*System time \(seconds\):\s*(\S+)", re.MULTILINE
    ),
    "maximum_rss_kib": re.compile(
        r"^\s*Maximum resident set size \(kbytes\):\s*(\d+)", re.MULTILINE
    ),
}


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def run_process(
    command: list[str],
    *,
    environment: dict[str, str] | None,
    timeout: float,
) -> tuple[int | None, bytes, bytes, bool, float]:
    started = time.monotonic()
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        start_new_session=True,
    )
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGTERM)
        try:
            stdout, stderr = process.communicate(timeout=2)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
    return process.returncode, stdout, stderr, timed_out, time.monotonic() - started


def parse_timing(path: Path) -> dict[str, float | int | None]:
    if not path.is_file():
        return {name: None for name in TIMING_PATTERNS}
    text = path.read_text(encoding="utf-8", errors="replace")
    values: dict[str, float | int | None] = {}
    for name, pattern in TIMING_PATTERNS.items():
        match = pattern.search(text)
        if match is None:
            values[name] = None
        elif name == "maximum_rss_kib":
            values[name] = int(match.group(1))
        else:
            values[name] = float(match.group(1))
    return values


def artifact_hashes(root: Path) -> dict[str, str]:
    return {
        str(path.relative_to(root)): common.sha256_file(path)
        for path in sorted(root.rglob("*"))
        if path.is_file() and path.name != "result.json"
    }


def resumable(result_path: Path, contract_id: str) -> dict[str, Any] | None:
    if not result_path.is_file():
        return None
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result.get("contract_id") != contract_id:
            return None
        expected = result["artifact_hashes"]
        if artifact_hashes(result_path.parent) != expected:
            return None
        return result
    except (OSError, UnicodeError, ValueError, KeyError, json.JSONDecodeError):
        return None


def write_bytes(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(value)


def run_connection(
    *,
    repo_root: Path,
    binary: Path,
    problem: Path,
    tptp_root: Path,
    run_dir: Path,
) -> dict[str, Any]:
    worker_output = run_dir / "worker"
    command = [
        sys.executable,
        str(Path(__file__).resolve().parent / "connection_worker.py"),
        "--repo-root",
        str(repo_root),
        "--binary",
        str(binary),
        "--problem",
        str(problem),
        "--tptp-root",
        str(tptp_root),
        "--output-root",
        str(worker_output),
    ]
    return_code, stdout, stderr, timed_out, controller_wall = run_process(
        command, environment=None, timeout=30
    )
    write_bytes(run_dir / "worker.stdout.txt", stdout)
    write_bytes(run_dir / "worker.stderr.txt", stderr)
    certificate_path = worker_output / "certificate.json"
    if timed_out or return_code != 0 or not certificate_path.is_file():
        raise common.ExperimentError(
            f"connection worker failed: return={return_code}, timeout={timed_out}, "
            + (stdout + stderr)[-2_000:].decode("utf-8", errors="replace")
        )
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    verification_command = [
        sys.executable,
        str(Path(__file__).resolve().parent / "verify_connection.py"),
        "--certificate",
        str(certificate_path),
        "--transcript",
        str(worker_output / "cnf.tstp"),
        "--repo-root",
        str(repo_root),
        "--binary",
        str(binary),
        "--problem",
        str(problem),
        "--tptp-root",
        str(tptp_root),
    ]
    verify_code, verify_stdout, verify_stderr, verify_timeout, verify_wall = run_process(
        verification_command, environment=None, timeout=60
    )
    write_bytes(run_dir / "verify.stdout.txt", verify_stdout)
    write_bytes(run_dir / "verify.stderr.txt", verify_stderr)
    try:
        verification = json.loads(verify_stdout)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise common.ExperimentError(f"invalid verifier output: {error}") from error
    status = certificate.get("status")
    proof_verified = bool(
        status == "Theorem"
        and verify_code == 0
        and verification.get("proof_checked")
    )
    correctness_failures: list[str] = []
    if verify_timeout or verify_code != 0 or not verification.get("valid"):
        correctness_failures.append("connection_verifier_rejected")
    if status == "Theorem" and not proof_verified:
        correctness_failures.append("unchecked_connection_theorem")
    if status not in {"Theorem", "Unknown"}:
        correctness_failures.append(f"invalid_connection_status:{status}")
    return {
        "status": status,
        "command": command,
        "return_code": return_code,
        "external_timeout": timed_out,
        "controller_wall_seconds": controller_wall,
        "solver_wall_seconds": certificate.get("wall_seconds"),
        "user_cpu_seconds": certificate.get("user_cpu_seconds"),
        "system_cpu_seconds": certificate.get("system_cpu_seconds"),
        "maximum_rss_kib": certificate.get("maximum_rss_kib"),
        "proof_verified": proof_verified,
        "proof_rule_nodes": certificate.get("proof_rule_nodes"),
        "proof_formula_count": None,
        "processed_clauses": None,
        "certificate_status": status,
        "certificate_sha256": common.sha256_file(certificate_path),
        "transcript_sha256": certificate.get("transcript_sha256"),
        "verification": verification,
        "verification_wall_seconds": verify_wall,
        "correctness_failures": correctness_failures,
    }


def run_validation_gate(
    *,
    validation_gate: Path,
    proofcheck: Path,
    problem: Path,
    proof: Path,
    run_dir: Path,
) -> dict[str, Any]:
    report = run_dir / "validation.json"
    command = [
        sys.executable,
        str(validation_gate),
        str(problem),
        str(proof),
        "--proof-command-json",
        json.dumps([str(proofcheck), "-p", "{problem}", "{artifact}"]),
        "--report",
        str(report),
    ]
    return_code, stdout, stderr, timed_out, wall = run_process(
        command, environment=None, timeout=180
    )
    write_bytes(run_dir / "validation.stdout.txt", stdout)
    write_bytes(run_dir / "validation.stderr.txt", stderr)
    return {
        "command": command,
        "return_code": return_code,
        "external_timeout": timed_out,
        "wall_seconds": wall,
        "report_sha256": common.sha256_file(report) if report.is_file() else None,
        "verified": return_code == 0 and report.is_file(),
    }


def run_saturation(
    *,
    method: str,
    binary: Path,
    problem: Path,
    tptp_root: Path,
    proofcheck: Path,
    validation_gate: Path,
    run_dir: Path,
) -> dict[str, Any]:
    proof = run_dir / "proof.tstp"
    stderr_path = run_dir / "stderr.txt"
    timing_path = run_dir / "timing.txt"
    arguments = [
        f"--expert-heuristic={HEURISTICS[method]}",
        "--term-ordering=KBO6",
        f"--soft-cpu-limit={common.SATURATION_SOFT_SECONDS}",
        f"--cpu-limit={common.SATURATION_HARD_SECONDS}",
        f"--memory-limit={common.MEMORY_MIB}",
        "--tstp-out",
        "--print-statistics",
        "--proof-object=1",
        "--force-deriv=2",
        str(problem),
    ]
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(timing_path),
        str(binary),
        *arguments,
    ]
    environment = os.environ.copy()
    environment["TPTP"] = str(tptp_root)
    return_code, stdout, stderr, timed_out, wall = run_process(
        command,
        environment=environment,
        timeout=common.SATURATION_HARD_SECONDS + 15,
    )
    write_bytes(proof, stdout)
    write_bytes(stderr_path, stderr)
    stdout_text = stdout.decode("utf-8", errors="replace")
    stderr_text = stderr.decode("utf-8", errors="replace")
    status = common.final_status(stdout_text, stderr_text)
    gate: dict[str, Any] | None = None
    if status in common.PROOF_STATUSES:
        gate = run_validation_gate(
            validation_gate=validation_gate,
            proofcheck=proofcheck,
            problem=problem,
            proof=proof,
            run_dir=run_dir,
        )
    proof_verified = bool(gate and gate["verified"])
    failures: list[str] = []
    if timed_out:
        failures.append("external_saturation_timeout")
    if status in common.PROOF_STATUSES and not proof_verified:
        failures.append("unchecked_saturation_theorem")
    if status not in common.PROOF_STATUSES | common.NO_CLAIM_STATUSES:
        failures.append(f"invalid_saturation_status:{status}")
    timing = parse_timing(timing_path)
    processed = PROCESSED_RE.findall(stdout_text)
    return {
        "status": status,
        "command": command,
        "return_code": return_code,
        "external_timeout": timed_out,
        "controller_wall_seconds": wall,
        "solver_wall_seconds": wall,
        **timing,
        "proof_verified": proof_verified,
        "proof_rule_nodes": None,
        "proof_formula_count": (
            common.count_annotated_formulas(stdout_text)
            if status in common.PROOF_STATUSES
            else None
        ),
        "processed_clauses": int(processed[-1]) if processed else None,
        "gate": gate,
        "correctness_failures": failures,
    }


def run_one(
    *,
    contract_id: str,
    phase: str,
    repetition: int,
    method: str,
    record: dict[str, Any],
    repo_root: Path,
    problem_root: Path,
    binary: Path,
    proofcheck: Path,
    validation_gate: Path,
    output_root: Path,
) -> tuple[dict[str, Any], bool]:
    run_dir = (
        output_root
        / "runs"
        / method
        / record["problem_id"]
        / f"rep-{repetition}"
    )
    run_dir.mkdir(parents=True, exist_ok=True)
    result_path = run_dir / "result.json"
    existing = resumable(result_path, contract_id)
    if existing is not None:
        return existing, True
    problem, include_hashes = common.verify_problem_record(problem_root, record)
    started_at = utc_now()
    if method == "connection":
        details = run_connection(
            repo_root=repo_root,
            binary=binary,
            problem=problem,
            tptp_root=problem_root / "problems" / "casc_2025",
            run_dir=run_dir,
        )
    else:
        details = run_saturation(
            method=method,
            binary=binary,
            problem=problem,
            tptp_root=problem_root / "problems" / "casc_2025",
            proofcheck=proofcheck,
            validation_gate=validation_gate,
            run_dir=run_dir,
        )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "phase": phase,
        "problem_id": record["problem_id"],
        "problem_path": record["path"],
        "problem_sha256": record["sha256"],
        "include_sha256": include_hashes,
        "family": record["family"],
        "difficulty_band": record["difficulty_band"],
        "expected_class": record["expected_class"],
        "method": method,
        "repetition": repetition,
        "started_at": started_at,
        "completed_at": utc_now(),
        **details,
    }
    result["artifact_hashes"] = artifact_hashes(run_dir)
    common.atomic_json(result_path, result)
    return result, False


def build_contract(
    *,
    phase: str,
    repo_root: Path,
    corpus: Path,
    binary: Path,
    proofcheck: Path,
    validation_gate: Path,
    validation_analysis: Path | None,
) -> dict[str, Any]:
    here = Path(__file__).resolve().parent
    script_hashes = {
        path.name: common.sha256_file(path)
        for path in sorted(here.glob("*.py"))
    }
    contract: dict[str, Any] = {
        "schema_version": 1,
        "source_revision": common.SOURCE_REVISION,
        "phase": phase,
        "platform": platform.platform(),
        "python": sys.version,
        "corpus_sha256": common.sha256_file(corpus),
        "preregistration_sha256": common.sha256_file(
            here / "PREREGISTRATION.md"
        ),
        "binary_path": str(binary),
        "binary_sha256": common.sha256_file(binary),
        "proofcheck_sha256": common.sha256_file(proofcheck),
        "validation_gate_sha256": common.sha256_file(validation_gate),
        "trace_parser_sha256": common.sha256_file(
            repo_root
            / "experiments"
            / "2026-07-30-002-real-ground-theory-traces"
            / "trace_model.py"
        ),
        "script_sha256": script_hashes,
        "methods": list(common.METHODS),
        "repetitions": common.REPETITIONS[phase],
        "limits": {
            "connection_budget_seconds": common.CONNECTION_BUDGET_SECONDS,
            "connection_maximum_branch_depth": common.MAX_BRANCH_DEPTH,
            "connection_maximum_search_nodes": common.MAX_SEARCH_NODES,
            "saturation_soft_cpu_seconds": common.SATURATION_SOFT_SECONDS,
            "saturation_hard_cpu_seconds": common.SATURATION_HARD_SECONDS,
            "memory_mib": common.MEMORY_MIB,
        },
        "heuristics": HEURISTICS,
        "validation_analysis_sha256": (
            common.sha256_file(validation_analysis)
            if validation_analysis is not None
            else None
        ),
    }
    contract["contract_id"] = common.sha256_bytes(common.canonical_json(contract))
    return contract


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=common.REPETITIONS, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--validation-analysis", type=Path)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    if sys.platform != "linux":
        raise common.ExperimentError("prover experiments may run only on Linux")
    arguments = parse_args(argv)
    if arguments.workers < 1 or arguments.workers > 4:
        raise common.ExperimentError("--workers must be between one and four")
    paths = {
        "repo_root": arguments.repo_root.resolve(),
        "corpus": arguments.corpus.resolve(),
        "problem_root": arguments.problem_root.resolve(),
        "binary": arguments.binary.resolve(),
        "proofcheck": arguments.proofcheck.resolve(),
        "validation_gate": arguments.validation_gate.resolve(),
        "output_root": arguments.output_root.resolve(),
    }
    for label in ("repo_root", "corpus", "problem_root", "binary", "proofcheck", "validation_gate"):
        if not paths[label].exists():
            raise common.ExperimentError(f"missing {label}: {paths[label]}")
    validation_analysis = (
        arguments.validation_analysis.resolve()
        if arguments.validation_analysis is not None
        else None
    )
    if arguments.phase == "test":
        if validation_analysis is None or not validation_analysis.is_file():
            raise common.ExperimentError(
                "test requires the completed validation analysis"
            )
        validation = json.loads(validation_analysis.read_text(encoding="utf-8"))
        if validation.get("phase") != "validation" or not validation.get(
            "correctness_gates_passed"
        ):
            raise common.ExperimentError(
                "test requires a correctness-clean validation analysis"
            )
    elif validation_analysis is not None:
        raise common.ExperimentError(
            "--validation-analysis is accepted only for the test phase"
        )
    _header, records = common.load_corpus(paths["corpus"])
    selected = [
        record for record in records
        if record["experiment_split"] == arguments.phase
    ]
    for record in selected:
        common.verify_problem_record(paths["problem_root"], record)
    contract = build_contract(
        phase=arguments.phase,
        repo_root=paths["repo_root"],
        corpus=paths["corpus"],
        binary=paths["binary"],
        proofcheck=paths["proofcheck"],
        validation_gate=paths["validation_gate"],
        validation_analysis=validation_analysis,
    )
    paths["output_root"].mkdir(parents=True, exist_ok=True)
    contract_path = paths["output_root"] / "contract.json"
    if contract_path.is_file():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        if existing != contract:
            raise common.ExperimentError("output root has a different run contract")
    else:
        common.atomic_json(contract_path, contract)

    jobs = [
        (record, method, repetition)
        for record in selected
        for method in common.METHODS
        for repetition in range(1, common.REPETITIONS[arguments.phase] + 1)
    ]
    completed: list[tuple[dict[str, Any], bool]] = []
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=arguments.workers
    ) as executor:
        futures = [
            executor.submit(
                run_one,
                contract_id=contract["contract_id"],
                phase=arguments.phase,
                repetition=repetition,
                method=method,
                record=record,
                repo_root=paths["repo_root"],
                problem_root=paths["problem_root"],
                binary=paths["binary"],
                proofcheck=paths["proofcheck"],
                validation_gate=paths["validation_gate"],
                output_root=paths["output_root"],
            )
            for record, method, repetition in jobs
        ]
        for future in concurrent.futures.as_completed(futures):
            completed.append(future.result())
    results = sorted(
        (item[0] for item in completed),
        key=lambda item: (item["problem_id"], item["method"], item["repetition"]),
    )
    common.write_jsonl(paths["output_root"] / "results.jsonl", results)
    summary = {
        "phase": arguments.phase,
        "contract_id": contract["contract_id"],
        "result_count": len(results),
        "completed": sum(not resumed for _result, resumed in completed),
        "resumed": sum(resumed for _result, resumed in completed),
        "correctness_failures": sum(
            len(result["correctness_failures"]) for result in results
        ),
    }
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())

