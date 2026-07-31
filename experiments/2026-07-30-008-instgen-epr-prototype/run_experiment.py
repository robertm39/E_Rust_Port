#!/usr/bin/env python3
"""Run the preregistered standalone, portfolio, and cooperative comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import subprocess
import time
from pathlib import Path
from typing import Any, Sequence


LONG_SECONDS = 4.0
SHORT_SECONDS = 2.0
MEMORY_MIB = 1_536
SZS_STATUS = re.compile(r"SZS status\s+([A-Za-z_-]+)", re.IGNORECASE)
TIME_FIELDS = {
    "User time (seconds)": ("user_seconds", float),
    "System time (seconds)": ("system_seconds", float),
    "Elapsed (wall clock) time": ("elapsed_text", str),
    "Maximum resident set size (kbytes)": ("max_rss_kib", int),
}


class ExperimentError(RuntimeError):
    """A run or artifact violates the frozen experiment contract."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    os.replace(temporary, path)


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


def normalized_status(raw_status: str) -> str:
    value = raw_status.lower().replace("-", "").replace("_", "")
    if value in {"theorem", "unsatisfiable", "contradictoryaxioms"}:
        return "unsat"
    if value in {"satisfiable", "countersatisfiable"}:
        return "sat"
    return "unknown"


def expected_status(record: dict[str, Any]) -> str:
    return "sat" if record["expected_class"] == "satisfiable" else "unsat"


def proof_gate(
    *,
    problem: Path,
    solution: Path,
    report: Path,
    proofcheck: Path,
    validation_gate: Path,
) -> dict[str, Any]:
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
        str(report),
    ]
    started = time.monotonic()
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    stdout = report.with_suffix(".stdout.txt")
    stderr = report.with_suffix(".stderr.txt")
    stdout.write_text(completed.stdout, encoding="utf-8")
    stderr.write_text(completed.stderr, encoding="utf-8")
    return {
        "command": command,
        "return_code": completed.returncode,
        "verified": completed.returncode == 0,
        "wall_seconds": time.monotonic() - started,
        "report_sha256": sha256_file(report) if report.is_file() else None,
        "stdout_sha256": sha256_file(stdout),
        "stderr_sha256": sha256_file(stderr),
    }


def run_umlaut(
    *,
    binary: Path,
    problem: Path,
    output_root: Path,
    budget_seconds: float,
    proofcheck: Path,
    validation_gate: Path,
) -> dict[str, Any]:
    output_root.mkdir(parents=True, exist_ok=True)
    solution = output_root / "solution.txt"
    stderr = output_root / "stderr.txt"
    timing = output_root / "time.txt"
    hard_cpu = max(1, math.ceil(budget_seconds))
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(timing),
        "timeout",
        "--signal=KILL",
        f"{budget_seconds:.6f}s",
        str(binary),
        "--auto",
        "--tstp-out",
        "--proof-object=1",
        f"--cpu-limit={hard_cpu}",
        f"--memory-limit={MEMORY_MIB}",
        str(problem),
    ]
    started = time.monotonic()
    with solution.open("wb") as stdout_handle, stderr.open("wb") as stderr_handle:
        try:
            completed = subprocess.run(
                command,
                check=False,
                stdout=stdout_handle,
                stderr=stderr_handle,
                timeout=budget_seconds + 10.0,
            )
            return_code = completed.returncode
            outer_timeout = False
        except subprocess.TimeoutExpired:
            return_code = 124
            outer_timeout = True
    text = solution.read_text(encoding="utf-8", errors="replace")
    statuses = SZS_STATUS.findall(text)
    raw_status = statuses[-1] if statuses else "unknown"
    status = normalized_status(raw_status)
    validation = None
    if status == "unsat":
        validation = proof_gate(
            problem=problem,
            solution=solution,
            report=output_root / "validation.json",
            proofcheck=proofcheck,
            validation_gate=validation_gate,
        )
    result = {
        "kind": "umlaut",
        "budget_seconds": budget_seconds,
        "command": command,
        "return_code": return_code,
        "outer_timeout": outer_timeout,
        "measured_wall_seconds": time.monotonic() - started,
        "raw_status": raw_status,
        "status": status,
        "proof_verified": bool(
            validation is not None and validation["verified"]
        ),
        "validation": validation,
        "solution_path": solution.name,
        "solution_sha256": sha256_file(solution),
        "solution_bytes": solution.stat().st_size,
        "stderr_sha256": sha256_file(stderr),
        "time_sha256": sha256_file(timing) if timing.is_file() else None,
        **parse_time_file(timing),
    }
    atomic_json(output_root / "result.json", result)
    return result


def run_instgen(
    *,
    script: Path,
    verifier: Path,
    repo_root: Path,
    problem: Path,
    output_root: Path,
    budget_seconds: float,
    cadical_driver: Path,
    drat_trim: Path,
) -> dict[str, Any]:
    output_root.mkdir(parents=True, exist_ok=True)
    stdout = output_root / "runner.stdout.txt"
    stderr = output_root / "runner.stderr.txt"
    timing = output_root / "runner.time.txt"
    worker = output_root / "worker"
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(timing),
        "timeout",
        "--signal=KILL",
        f"{budget_seconds + 130.0:.6f}s",
        "python3",
        str(script),
        "--problem",
        str(problem),
        "--cadical-driver",
        str(cadical_driver),
        "--drat-trim",
        str(drat_trim),
        "--output-root",
        str(worker),
        "--budget-seconds",
        str(budget_seconds),
    ]
    started = time.monotonic()
    with stdout.open("wb") as stdout_handle, stderr.open("wb") as stderr_handle:
        completed = subprocess.run(
            command,
            check=False,
            stdout=stdout_handle,
            stderr=stderr_handle,
            timeout=budget_seconds + 140.0,
        )
    if completed.returncode != 0:
        raise ExperimentError(
            "Inst-Gen worker failed: "
            + stderr.read_text(encoding="utf-8", errors="replace")[-3000:]
        )
    certificate_path = worker / "certificate.json"
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    verify_stdout = output_root / "verify.stdout.txt"
    verify_stderr = output_root / "verify.stderr.txt"
    verify_command = [
        "python3",
        str(verifier),
        "--certificate",
        str(certificate_path),
        "--problem",
        str(problem),
        "--repo-root",
        str(repo_root),
        "--drat-trim",
        str(drat_trim),
    ]
    verify_started = time.monotonic()
    verified = subprocess.run(
        verify_command,
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    verify_stdout.write_text(verified.stdout, encoding="utf-8")
    verify_stderr.write_text(verified.stderr, encoding="utf-8")
    if verified.returncode != 0:
        raise ExperimentError(
            "certificate verifier failed: "
            + (verified.stdout + verified.stderr)[-3000:]
        )
    result = {
        **certificate,
        "kind": "instgen",
        "runner_command": command,
        "runner_return_code": completed.returncode,
        "runner_wall_seconds": time.monotonic() - started,
        "runner_stdout_sha256": sha256_file(stdout),
        "runner_stderr_sha256": sha256_file(stderr),
        "runner_time_sha256": sha256_file(timing),
        "verification": {
            "command": verify_command,
            "verified": True,
            "wall_seconds": time.monotonic() - verify_started,
            "stdout_sha256": sha256_file(verify_stdout),
            "stderr_sha256": sha256_file(verify_stderr),
        },
    }
    atomic_json(output_root / "result.json", result)
    return result


def checked_terminal(
    result: dict[str, Any], expected: str
) -> tuple[str, bool]:
    status = str(result["status"])
    if status == "unknown":
        return status, False
    if status != expected:
        raise ExperimentError(
            f"terminal polarity disagrees with expected class: {status}/{expected}"
        )
    if result["kind"] == "instgen":
        verified = bool(result["verification"]["verified"])
    elif status == "unsat":
        verified = bool(result["proof_verified"])
    else:
        verified = True
    return status, verified


def compound_result(
    *,
    name: str,
    expected: str,
    first: dict[str, Any],
    second: dict[str, Any] | None,
) -> dict[str, Any]:
    first_status, first_verified = checked_terminal(first, expected)
    if first_status != "unknown" and first_verified:
        selected = "instgen_short" if first["kind"] == "instgen" else "first"
        return {
            "method": name,
            "status": first_status,
            "verified": True,
            "selected": selected,
        }
    if second is None:
        return {
            "method": name,
            "status": "unknown",
            "verified": False,
            "selected": None,
        }
    second_status, second_verified = checked_terminal(second, expected)
    return {
        "method": name,
        "status": second_status,
        "verified": second_verified,
        "selected": "second" if second_verified else None,
    }


def portfolio_result(
    *,
    expected: str,
    instgen: dict[str, Any],
    saturation: dict[str, Any],
) -> dict[str, Any]:
    instgen_status, instgen_verified = checked_terminal(instgen, expected)
    saturation_status, saturation_verified = checked_terminal(
        saturation, expected
    )
    terminal = {
        status
        for status, verified in (
            (instgen_status, instgen_verified),
            (saturation_status, saturation_verified),
        )
        if verified
    }
    if len(terminal) > 1:
        raise ExperimentError("portfolio workers disagree")
    selected = []
    if instgen_verified:
        selected.append("instgen_short")
    if saturation_verified:
        selected.append("saturation_short")
    return {
        "method": "portfolio",
        "status": next(iter(terminal), "unknown"),
        "verified": bool(terminal),
        "selected": selected,
    }


def render_augmented(
    *,
    source: Path,
    instances: Path,
    output: Path,
) -> None:
    source_bytes = source.read_bytes()
    instance_bytes = instances.read_bytes()
    output.write_bytes(
        source_bytes
        + (b"" if source_bytes.endswith(b"\n") else b"\n")
        + instance_bytes
    )


def run_coordinate(
    *,
    record: dict[str, Any],
    repetition: int,
    repo_root: Path,
    problem_root: Path,
    output_root: Path,
    binary: Path,
    cadical_driver: Path,
    drat_trim: Path,
    proofcheck: Path,
    validation_gate: Path,
    instgen_script: Path,
    verifier: Path,
) -> dict[str, Any]:
    coordinate_id = f"{record['problem_id']}-r{repetition}"
    root = output_root / coordinate_id
    root.mkdir(parents=True, exist_ok=True)
    result_path = root / "coordinate.json"
    if result_path.is_file():
        return json.loads(result_path.read_text(encoding="utf-8"))
    problem = problem_root / record["path"]
    if sha256_file(problem) != record["sha256"]:
        raise ExperimentError(f"problem hash mismatch: {record['problem_id']}")
    expected = expected_status(record)

    saturation_long = run_umlaut(
        binary=binary,
        problem=problem,
        output_root=root / "saturation-long",
        budget_seconds=LONG_SECONDS,
        proofcheck=proofcheck,
        validation_gate=validation_gate,
    )
    instgen_long = run_instgen(
        script=instgen_script,
        verifier=verifier,
        repo_root=repo_root,
        problem=problem,
        output_root=root / "instgen-long",
        budget_seconds=LONG_SECONDS,
        cadical_driver=cadical_driver,
        drat_trim=drat_trim,
    )
    saturation_short = run_umlaut(
        binary=binary,
        problem=problem,
        output_root=root / "saturation-short",
        budget_seconds=SHORT_SECONDS,
        proofcheck=proofcheck,
        validation_gate=validation_gate,
    )
    instgen_short = run_instgen(
        script=instgen_script,
        verifier=verifier,
        repo_root=repo_root,
        problem=problem,
        output_root=root / "instgen-short",
        budget_seconds=SHORT_SECONDS,
        cadical_driver=cadical_driver,
        drat_trim=drat_trim,
    )

    short_status, short_verified = checked_terminal(instgen_short, expected)
    cooperative_saturation = None
    augmented_record = None
    if short_status == "unknown" or not short_verified:
        augmented = root / "cooperative-input.p"
        render_augmented(
            source=problem,
            instances=(
                root
                / "instgen-short"
                / "worker"
                / str(instgen_short["instances_path"])
            ),
            output=augmented,
        )
        augmented_record = {
            "path": augmented.name,
            "sha256": sha256_file(augmented),
            "bytes": augmented.stat().st_size,
        }
        cooperative_saturation = run_umlaut(
            binary=binary,
            problem=augmented,
            output_root=root / "cooperative-saturation",
            budget_seconds=SHORT_SECONDS,
            proofcheck=proofcheck,
            validation_gate=validation_gate,
        )

    methods = {
        "saturation": {
            "method": "saturation",
            "status": saturation_long["status"],
            "verified": checked_terminal(saturation_long, expected)[1],
            "selected": "saturation_long",
        },
        "standalone": {
            "method": "standalone",
            "status": instgen_long["status"],
            "verified": checked_terminal(instgen_long, expected)[1],
            "selected": "instgen_long",
        },
        "portfolio": portfolio_result(
            expected=expected,
            instgen=instgen_short,
            saturation=saturation_short,
        ),
        "cooperative": compound_result(
            name="cooperative",
            expected=expected,
            first=instgen_short,
            second=cooperative_saturation,
        ),
    }
    coordinate = {
        "schema_version": 1,
        "coordinate_id": coordinate_id,
        "problem_id": record["problem_id"],
        "family": record["family"],
        "partition": record["holdout_split"],
        "expected_class": record["expected_class"],
        "expected_status": expected,
        "source_sha256": record["sha256"],
        "repetition": repetition,
        "runs": {
            "saturation_long": saturation_long,
            "instgen_long": instgen_long,
            "saturation_short": saturation_short,
            "instgen_short": instgen_short,
            "cooperative_saturation": cooperative_saturation,
        },
        "augmented": augmented_record,
        "methods": methods,
    }
    terminal_statuses = {
        method["status"]
        for method in methods.values()
        if method["verified"]
    }
    if len(terminal_statuses) > 1:
        raise ExperimentError(f"method polarity disagreement: {coordinate_id}")
    atomic_json(result_path, coordinate)
    return coordinate


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument(
        "--phase", choices=("train", "heldout", "all"), required=True
    )
    arguments = parser.parse_args()
    repo_root = arguments.repo_root.resolve()
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    output_root = arguments.output_root.resolve()
    umlaut = arguments.umlaut.resolve()
    cadical_driver = arguments.cadical_driver.resolve()
    drat_trim = arguments.drat_trim.resolve()
    proofcheck = arguments.proofcheck.resolve()
    validation_gate = arguments.validation_gate.resolve()
    umlaut_sha256 = sha256_file(umlaut)
    records = [
        json.loads(line)
        for line in manifest_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    header = records[0]
    if header.get("kind") != "umlaut-instgen-epr-corpus":
        raise ExperimentError("wrong corpus manifest")
    selected = []
    for record in records[1:]:
        partition = record["holdout_split"]
        if arguments.phase == "train" and partition != "train":
            continue
        if arguments.phase == "heldout" and partition == "train":
            continue
        repetitions = 1 if partition == "train" else 2
        for repetition in range(1, repetitions + 1):
            selected.append((record, repetition))
    output_root.mkdir(parents=True, exist_ok=True)
    experiment = Path(__file__).resolve().parent
    coordinates = []
    for index, (record, repetition) in enumerate(selected, start=1):
        print(
            f"[{index}/{len(selected)}] {record['problem_id']} r{repetition}",
            flush=True,
        )
        coordinates.append(
            run_coordinate(
                record=record,
                repetition=repetition,
                repo_root=repo_root,
                problem_root=problem_root,
                output_root=output_root,
                binary=umlaut,
                cadical_driver=cadical_driver,
                drat_trim=drat_trim,
                proofcheck=proofcheck,
                validation_gate=validation_gate,
                instgen_script=experiment / "instgen.py",
                verifier=experiment / "verify_certificate.py",
            )
        )
    summary = {
        "schema_version": 1,
        "phase": arguments.phase,
        "manifest_sha256": sha256_file(manifest_path),
        "umlaut_sha256": umlaut_sha256,
        "cadical_driver_sha256": sha256_file(cadical_driver),
        "drat_trim_sha256": sha256_file(drat_trim),
        "proofcheck_sha256": sha256_file(proofcheck),
        "coordinates": len(coordinates),
        "coordinate_ids": [
            coordinate["coordinate_id"] for coordinate in coordinates
        ],
    }
    atomic_json(output_root / f"{arguments.phase}-run.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
