#!/usr/bin/env python3
"""Run fixed baseline, static-split, and bounded AVATAR comparisons."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any, Sequence

import tptp_split


TOTAL_WALL_SECONDS = 20.0
MEMORY_LIMIT_MIB = 2048
MAX_SPLIT_CLAUSES = 6
MAX_MODELS = 32
SUCCESS_STATUSES = {"unsatisfiable", "contradictoryaxioms"}
SZS_STATUS = re.compile(
    r"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
TIME_FIELDS = {
    "User time (seconds)": ("user_seconds", float),
    "System time (seconds)": ("system_seconds", float),
    "Elapsed (wall clock) time (h:mm:ss or m:ss)": (
        "gnu_wall_seconds",
        str,
    ),
    "Maximum resident set size (kbytes)": ("max_rss_kib", int),
}
WRITE_LOCK = threading.Lock()


class ExperimentError(RuntimeError):
    """The experiment cannot safely continue."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def slug(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value)


def read_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not rows or rows[0].get("record_type") != "manifest":
        raise ExperimentError("invalid experiment corpus manifest")
    if rows[0].get("problem_count") != len(rows) - 1:
        raise ExperimentError("experiment corpus count mismatch")
    return rows[0], rows[1:]


def parse_time_file(path: Path) -> dict[str, Any]:
    metrics: dict[str, Any] = {}
    if not path.is_file():
        return metrics
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        for field, (name, conversion) in TIME_FIELDS.items():
            prefix = field + ":"
            if stripped.startswith(prefix):
                raw = stripped[len(prefix) :].strip()
                try:
                    metrics[name] = conversion(raw)
                except ValueError:
                    pass
    return metrics


def proof_gate(
    problem: Path,
    solution: Path,
    report_path: Path,
    proofcheck: Path,
    validation_gate: Path,
) -> dict[str, Any]:
    stdout_path = report_path.with_suffix(".stdout.txt")
    stderr_path = report_path.with_suffix(".stderr.txt")
    command = [
        "python3",
        str(validation_gate),
        str(problem),
        str(solution),
        "--proof-command-json",
        json.dumps(
            [str(proofcheck), "-p", "{problem}", "{artifact}"]
        ),
        "--report",
        str(report_path),
    ]
    started = time.monotonic()
    completed = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=180,
    )
    duration = time.monotonic() - started
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    return {
        "return_code": completed.returncode,
        "verified": completed.returncode == 0,
        "wall_seconds": duration,
        "report_path": report_path.name,
        "report_sha256": (
            sha256_file(report_path) if report_path.is_file() else None
        ),
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }


def run_prover(
    *,
    binary: Path,
    problem: Path,
    output_root: Path,
    label: str,
    wall_seconds: float,
    extra_options: Sequence[str],
    proofcheck: Path,
    validation_gate: Path,
) -> dict[str, Any]:
    output_root.mkdir(parents=True, exist_ok=True)
    solution_path = output_root / f"{label}.solution.txt"
    stderr_path = output_root / f"{label}.stderr.txt"
    time_path = output_root / f"{label}.time.txt"
    validation_path = output_root / f"{label}.validation.json"
    hard_seconds = max(1, math.ceil(wall_seconds))
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(time_path),
        "timeout",
        "--signal=KILL",
        f"{wall_seconds:.6f}s",
        str(binary),
        "--auto",
        "--tstp-out",
        "--proof-object=1",
        f"--cpu-limit={hard_seconds}",
        f"--memory-limit={MEMORY_LIMIT_MIB}",
        *extra_options,
        str(problem),
    ]
    started = time.monotonic()
    with solution_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        try:
            completed = subprocess.run(
                command,
                check=False,
                stdout=stdout,
                stderr=stderr,
                timeout=wall_seconds + 10.0,
            )
            return_code = completed.returncode
            outer_timeout = False
        except subprocess.TimeoutExpired:
            return_code = 124
            outer_timeout = True
    measured_wall = time.monotonic() - started
    solution_text = solution_path.read_text(
        encoding="utf-8", errors="replace"
    )
    statuses = [status.lower() for status in SZS_STATUS.findall(solution_text)]
    claimed_status = statuses[-1] if statuses else "unknown"
    validation = None
    if claimed_status.replace("-", "").replace("_", "") in SUCCESS_STATUSES:
        validation = proof_gate(
            problem,
            solution_path,
            validation_path,
            proofcheck,
            validation_gate,
        )
    metrics = parse_time_file(time_path)
    metrics.update(
        {
            "command": command,
            "return_code": return_code,
            "outer_timeout": outer_timeout,
            "measured_wall_seconds": measured_wall,
            "claimed_status": claimed_status,
            "proof_verified": bool(
                validation and validation["verified"] is True
            ),
            "validation": validation,
            "solution_path": solution_path.name,
            "solution_sha256": sha256_file(solution_path),
            "solution_bytes": solution_path.stat().st_size,
            "stderr_path": stderr_path.name,
            "stderr_sha256": sha256_file(stderr_path),
            "time_path": time_path.name,
            "time_sha256": (
                sha256_file(time_path) if time_path.is_file() else None
            ),
        }
    )
    return metrics


class SatDriver:
    def __init__(self, binary: Path):
        self.process = subprocess.Popen(
            [str(binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        ready = self._read()
        if ready[:2] != ["ready", "1"]:
            raise ExperimentError(f"SAT driver did not become ready: {ready}")

    def _read(self) -> list[str]:
        assert self.process.stdout is not None
        line = self.process.stdout.readline()
        if not line:
            stderr = (
                self.process.stderr.read()
                if self.process.stderr is not None
                else ""
            )
            raise ExperimentError(f"SAT driver stopped unexpectedly: {stderr}")
        fields = line.split()
        if fields and fields[0] == "error":
            raise ExperimentError(f"SAT driver error: {' '.join(fields)}")
        return fields

    def _write(self, command: str) -> list[str]:
        assert self.process.stdin is not None
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()
        return self._read()

    def add(self, clause: list[int]) -> None:
        fields = self._write(
            "a " + " ".join(map(str, clause)) + (" " if clause else "") + "0"
        )
        if not fields or fields[0] != "ok":
            raise ExperimentError(f"unexpected SAT add response: {fields}")

    def solve(self) -> dict[str, Any]:
        fields = self._write("s")
        if not fields:
            raise ExperimentError("empty SAT solve response")
        if fields[0] == "unsat" and len(fields) == 2:
            return {"status": "unsat", "elapsed_ns": int(fields[1])}
        if fields[0] == "sat" and len(fields) >= 3 and fields[-1] == "0":
            return {
                "status": "sat",
                "elapsed_ns": int(fields[1]),
                "model": [int(field) for field in fields[2:-1]],
            }
        if fields[0] == "unknown":
            return {
                "status": "unknown",
                "elapsed_ns": int(fields[1]),
                "reason": fields[2] if len(fields) > 2 else "unknown",
            }
        raise ExperimentError(f"unexpected SAT solve response: {fields}")

    def close(self) -> None:
        if self.process.poll() is None:
            try:
                self._write("q")
            finally:
                self.process.wait(timeout=5)


def abstraction_summary(abstraction: dict[str, Any]) -> dict[str, Any]:
    split_records = []
    for record in abstraction["split_records"]:
        components = [
            {
                "selector": component["selector"],
                "canonical": component["canonical"],
                "literals": component["literals"],
            }
            for component in record["components"]
        ]
        split_records.append(
            {
                "statement_index": record["statement_index"],
                "name": record["name"],
                "role": record["role"],
                "components": components,
                "selectors": [
                    component["selector"] for component in components
                ],
            }
        )
    return {
        "cnf_count": abstraction["cnf_count"],
        "selected_split_count": abstraction["selected_split_count"],
        "selector_count": abstraction["selector_count"],
        "split_records": split_records,
        "split_clauses": abstraction["split_clauses"],
    }


def run_avatar(
    *,
    binary: Path,
    sat_driver_binary: Path,
    problem: Path,
    output_root: Path,
    proofcheck: Path,
    validation_gate: Path,
    verifier: Path,
) -> dict[str, Any]:
    output_root.mkdir(parents=True, exist_ok=True)
    source_hash = sha256_file(problem)
    abstraction = tptp_split.analyze_file(problem, MAX_SPLIT_CLAUSES)
    summary = abstraction_summary(abstraction)
    certificate: dict[str, Any] = {
        "schema_version": 1,
        "source_sha256": source_hash,
        "max_split_clauses": MAX_SPLIT_CLAUSES,
        "abstraction": summary,
        "branches": [],
        "final_status": "unknown",
        "termination_reason": "not_started",
    }
    driver = SatDriver(sat_driver_binary)
    prover_wall = 0.0
    sat_elapsed_ns = 0
    sat_calls = 0
    peak_rss_kib = 0
    active_counts: list[int] = []
    try:
        for clause in abstraction["split_clauses"]:
            driver.add(clause)
        for model_index in range(1, MAX_MODELS + 1):
            sat_result = driver.solve()
            sat_calls += 1
            sat_elapsed_ns += sat_result["elapsed_ns"]
            if sat_result["status"] == "unsat":
                certificate["final_status"] = "unsatisfiable"
                certificate["termination_reason"] = "sat_unsat"
                break
            if sat_result["status"] != "sat":
                certificate["termination_reason"] = (
                    "sat_" + sat_result.get("reason", "unknown")
                )
                break
            remaining = TOTAL_WALL_SECONDS - prover_wall
            if remaining <= 0.001:
                certificate["termination_reason"] = "prover_budget"
                break
            model = sat_result["model"]
            active = sorted(literal for literal in model if literal > 0)
            active_counts.append(len(active))
            branch_path = output_root / f"branch-{model_index:02}.p"
            branch_path.write_text(
                tptp_split.render_branch(
                    abstraction,
                    active,
                    source_sha256=source_hash,
                    model_index=model_index,
                ),
                encoding="utf-8",
            )
            run = run_prover(
                binary=binary,
                problem=branch_path,
                output_root=output_root,
                label=f"branch-{model_index:02}",
                wall_seconds=remaining,
                extra_options=(),
                proofcheck=proofcheck,
                validation_gate=validation_gate,
            )
            prover_wall += run["measured_wall_seconds"]
            peak_rss_kib = max(peak_rss_kib, run.get("max_rss_kib", 0))
            branch_record: dict[str, Any] = {
                "model_index": model_index,
                "sat_model": model,
                "active_selectors": active,
                "branch_path": branch_path.name,
                "branch_sha256": sha256_file(branch_path),
                "proof_verified": run["proof_verified"],
                "prover": run,
                "learned_conflict": None,
            }
            if run["proof_verified"]:
                solution_path = output_root / run["solution_path"]
                conflict = [-selector for selector in active]
                branch_record.update(
                    {
                        "proof_path": solution_path.name,
                        "proof_sha256": sha256_file(solution_path),
                        "learned_conflict": conflict,
                    }
                )
                driver.add(conflict)
            else:
                certificate["termination_reason"] = "unrefuted_branch"
            certificate["branches"].append(branch_record)
            if not run["proof_verified"]:
                break
        else:
            final_sat_result = driver.solve()
            sat_calls += 1
            sat_elapsed_ns += final_sat_result["elapsed_ns"]
            if final_sat_result["status"] == "unsat":
                certificate["final_status"] = "unsatisfiable"
                certificate["termination_reason"] = "sat_unsat"
            else:
                certificate["termination_reason"] = "model_limit"
    finally:
        driver.close()

    certificate_path = output_root / "certificate.json"
    certificate_path.write_text(
        json.dumps(certificate, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    verifier_stdout = output_root / "certificate-verifier.stdout.txt"
    verifier_stderr = output_root / "certificate-verifier.stderr.txt"
    verify_started = time.monotonic()
    completed = subprocess.run(
        [
            "python3",
            str(verifier),
            str(certificate_path),
            str(problem),
            "--proofcheck",
            str(proofcheck),
            "--validation-gate",
            str(validation_gate),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=300,
    )
    verify_seconds = time.monotonic() - verify_started
    verifier_stdout.write_bytes(completed.stdout)
    verifier_stderr.write_bytes(completed.stderr)
    selector_count = abstraction["selector_count"]
    inactive_counts = [selector_count - count for count in active_counts]
    certificate_verified = completed.returncode == 0
    return {
        "method": "avatar",
        "claimed_status": certificate["final_status"],
        "proof_verified": (
            certificate["final_status"] == "unsatisfiable"
            and certificate_verified
        ),
        "termination_reason": certificate["termination_reason"],
        "measured_wall_seconds": prover_wall,
        "max_rss_kib": peak_rss_kib,
        "selected_split_count": abstraction["selected_split_count"],
        "selector_count": selector_count,
        "sat_calls": sat_calls,
        "sat_elapsed_ns": sat_elapsed_ns,
        "branch_count": len(certificate["branches"]),
        "verified_conflicts": sum(
            branch["proof_verified"] for branch in certificate["branches"]
        ),
        "active_selector_counts": active_counts,
        "inactive_component_counts": inactive_counts,
        "inactive_component_fraction": (
            sum(inactive_counts) / (len(inactive_counts) * selector_count)
            if inactive_counts and selector_count
            else 0.0
        ),
        "certificate_path": certificate_path.name,
        "certificate_sha256": sha256_file(certificate_path),
        "certificate_verified": certificate_verified,
        "certificate_verifier_return_code": completed.returncode,
        "certificate_verifier_seconds": verify_seconds,
        "certificate_verifier_stdout_sha256": sha256_file(verifier_stdout),
        "certificate_verifier_stderr_sha256": sha256_file(verifier_stderr),
    }


def run_one_problem(
    *,
    record: dict[str, Any],
    repo_root: Path,
    artifact_root: Path,
    binary: Path,
    sat_driver_binary: Path,
    proofcheck: Path,
    validation_gate: Path,
    verifier: Path,
) -> list[dict[str, Any]]:
    problem = repo_root / record["path"]
    if not problem.is_file() or sha256_file(problem) != record["sha256"]:
        raise ExperimentError(f"problem hash mismatch: {record['problem_id']}")
    problem_root = artifact_root / slug(record["problem_id"])
    problem_root.mkdir(parents=True, exist_ok=True)
    common = {
        "problem_id": record["problem_id"],
        "family": record["family"],
        "partition": record["holdout_split"],
        "cohort": record["cohort"],
        "source_sha256": record["sha256"],
    }
    results = []
    for method, options in (
        ("baseline", ()),
        (
            "static_split",
            (
                "--split-clauses=7",
                "--split-method=2",
                "--split-aggressive",
                "--split-reuse-defs",
            ),
        ),
    ):
        method_root = problem_root / method
        run = run_prover(
            binary=binary,
            problem=problem,
            output_root=method_root,
            label=method,
            wall_seconds=TOTAL_WALL_SECONDS,
            extra_options=options,
            proofcheck=proofcheck,
            validation_gate=validation_gate,
        )
        results.append({**common, "method": method, **run})
    avatar_root = problem_root / "avatar"
    avatar = run_avatar(
        binary=binary,
        sat_driver_binary=sat_driver_binary,
        problem=problem,
        output_root=avatar_root,
        proofcheck=proofcheck,
        validation_gate=validation_gate,
        verifier=verifier,
    )
    results.append({**common, **avatar})
    return results


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--sat-driver", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--phase", choices=("train", "validation", "test", "all"))
    parser.add_argument("--workers", type=int, default=4)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    repo_root = arguments.repo_root.resolve()
    artifact_root = arguments.artifact_root.resolve()
    _, records = read_manifest(arguments.corpus.resolve())
    if arguments.phase and arguments.phase != "all":
        records = [
            record
            for record in records
            if record["holdout_split"] == arguments.phase
        ]
    if arguments.workers < 1:
        raise ExperimentError("workers must be positive")
    artifact_root.mkdir(parents=True, exist_ok=True)
    validation_gate = (
        repo_root / "tools/validation/validate_tptp_solution.py"
    )
    verifier = Path(__file__).with_name("verify_certificate.py").resolve()
    results_path = artifact_root / "results.jsonl"
    completed_records: list[dict[str, Any]] = []
    with ThreadPoolExecutor(max_workers=arguments.workers) as executor:
        futures = [
            executor.submit(
                run_one_problem,
                record=record,
                repo_root=repo_root,
                artifact_root=artifact_root,
                binary=arguments.umlaut.resolve(),
                sat_driver_binary=arguments.sat_driver.resolve(),
                proofcheck=arguments.proofcheck.resolve(),
                validation_gate=validation_gate,
                verifier=verifier,
            )
            for record in records
        ]
        for future in as_completed(futures):
            result_group = future.result()
            with WRITE_LOCK:
                completed_records.extend(result_group)
                with results_path.open("a", encoding="utf-8") as stream:
                    for result in result_group:
                        stream.write(json.dumps(result, sort_keys=True) + "\n")
            print(
                f"completed {result_group[0]['problem_id']} "
                f"({len(completed_records) // 3}/{len(records)})",
                flush=True,
            )
    completed_records.sort(
        key=lambda result: (
            result["partition"],
            result["cohort"],
            result["problem_id"],
            result["method"],
        )
    )
    results_path.write_text(
        "".join(
            json.dumps(result, sort_keys=True) + "\n"
            for result in completed_records
        ),
        encoding="utf-8",
    )
    metadata = {
        "schema_version": 1,
        "total_wall_seconds": TOTAL_WALL_SECONDS,
        "memory_limit_mib": MEMORY_LIMIT_MIB,
        "max_split_clauses": MAX_SPLIT_CLAUSES,
        "max_models": MAX_MODELS,
        "workers": arguments.workers,
        "phase": arguments.phase or "all",
        "problem_count": len(records),
        "result_count": len(completed_records),
        "results_sha256": sha256_file(results_path),
    }
    (artifact_root / "run-metadata.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    try:
        main()
    except (ExperimentError, OSError, ValueError) as error:
        print(f"experiment error: {error}")
        raise SystemExit(1) from error
