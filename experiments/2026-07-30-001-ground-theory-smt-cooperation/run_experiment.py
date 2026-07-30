#!/usr/bin/env python3
"""Run the frozen no-SMT, process, and C API FFI comparison."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any, Iterable, Sequence

from ground_theory import (
    ProcessSession,
    ProtocolError,
    SolverResult,
    asserted_constraints,
    evidence_hash,
    fraction_text,
    load_corpus,
    parse_ffi_results,
    parse_fraction,
    verify_result,
    write_ffi_protocol,
)


EXPERIMENT_ROOT = Path(__file__).resolve().parent
DEFAULT_CORPUS = EXPERIMENT_ROOT / "corpus.json"
EXPECTED_Z3_COMMIT = "2d48fd119ce5074b880944c2b1c59e537c99cd46"
EXPECTED_Z3_ARCHIVE_SHA256 = (
    "9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8"
)
MEASURED_REPETITIONS = 5


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile(values: Sequence[int], percentile_value: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    position = (len(ordered) - 1) * percentile_value
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def timing_summary(values: Sequence[int]) -> dict[str, float | int]:
    return {
        "count": len(values),
        "total_ns": sum(values),
        "median_ns": statistics.median(values) if values else 0,
        "p95_ns": percentile(values, 0.95),
        "minimum_ns": min(values) if values else 0,
        "maximum_ns": max(values) if values else 0,
    }


def no_smt_results(corpus: dict[str, Any]) -> tuple[list[SolverResult], int]:
    started = time.perf_counter_ns()
    results = [
        SolverResult(
            workload["id"],
            branch["id"],
            "unknown",
            0,
            reason="no theory solver configured",
        )
        for workload in corpus["workloads"]
        for branch in workload["branches"]
    ]
    return results, time.perf_counter_ns() - started


def run_process_repetition(
    executable: Path,
    workloads: Sequence[dict[str, Any]],
) -> tuple[list[SolverResult], dict[str, int]]:
    with ProcessSession(executable) as session:
        results = [
            result
            for workload in workloads
            for result in session.run_workload(workload)
        ]
    return results, {
        "startup_ns": session.startup_ns,
        "shutdown_ns": session.shutdown_ns,
    }


def run_ffi_repetition(
    driver: Path,
    protocol_path: Path,
    result_path: Path,
    environment: dict[str, str],
) -> tuple[list[SolverResult], dict[str, Any]]:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        [str(driver), "run", str(protocol_path), str(result_path)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=120,
        env=environment,
    )
    total_ns = time.perf_counter_ns() - started
    if completed.returncode != 0:
        raise ProtocolError(
            "FFI driver failed: "
            f"returncode={completed.returncode}, "
            f"stdout={completed.stdout.strip()}, stderr={completed.stderr.strip()}"
        )
    results, metadata = parse_ffi_results(result_path)
    return results, {
        "driver_total_ns": total_ns,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "metadata": metadata,
    }


def index_corpus(
    corpus: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], dict[tuple[str, str], dict[str, Any]]]:
    workloads = {workload["id"]: workload for workload in corpus["workloads"]}
    branches = {
        (workload["id"], branch["id"]): branch
        for workload in corpus["workloads"]
        for branch in workload["branches"]
    }
    return workloads, branches


def analyze_backend(
    backend: str,
    results: Sequence[SolverResult],
    corpus: dict[str, Any],
) -> tuple[dict[str, Any], list[tuple[SolverResult, dict[str, Any], dict[str, Any]]]]:
    workloads, branches = index_corpus(corpus)
    raw = collections.Counter()
    trusted = collections.Counter()
    cohorts: dict[str, collections.Counter[str]] = collections.defaultdict(
        collections.Counter
    )
    partitions: dict[str, collections.Counter[str]] = collections.defaultdict(
        collections.Counter
    )
    evidence = collections.Counter()
    verified_records = []
    expected_failures = []
    unsupported_raw_decisions = 0
    eligible_raw_decisions = 0
    unverified_eligible_decisions = []
    solver_calls = 0
    workload_statuses: dict[str, list[str]] = collections.defaultdict(list)

    for result in results:
        workload = workloads[result.workload_id]
        branch = branches[(result.workload_id, result.branch_id)]
        raw[result.raw_status] += 1
        if result.raw_status in {"sat", "unsat"}:
            solver_calls += 1
        verification = verify_result(workload, branch, result)
        trusted[verification.trusted_status] += 1
        cohorts[workload["cohort"]][verification.trusted_status] += 1
        partitions[workload["partition"]][verification.trusted_status] += 1
        workload_statuses[workload["id"]].append(verification.trusted_status)
        if verification.verified:
            evidence[verification.evidence_kind] += 1
            verified_records.append((result, workload, branch))
        if (
            workload["eligible"]
            and result.raw_status in {"sat", "unsat"}
        ):
            eligible_raw_decisions += 1
            if result.raw_status != branch["expected"]:
                expected_failures.append(
                    {
                        "workload": workload["id"],
                        "branch": branch["id"],
                        "expected": branch["expected"],
                        "actual": result.raw_status,
                    }
                )
            if not verification.verified:
                unverified_eligible_decisions.append(
                    {
                        "workload": workload["id"],
                        "branch": branch["id"],
                        "raw_status": result.raw_status,
                        "core": list(result.core),
                        "model": list(result.model),
                        "reason": verification.reason,
                    }
                )
        if not workload["eligible"] and result.raw_status in {"sat", "unsat"}:
            unsupported_raw_decisions += 1

    closed = 0
    expected_closed = 0
    for workload in corpus["workloads"]:
        if workload["expected_closed"]:
            expected_closed += 1
            statuses = workload_statuses.get(workload["id"], [])
            if len(statuses) == len(workload["branches"]) and all(
                status == "unsat" for status in statuses
            ):
                closed += 1

    raw_decisions = raw["sat"] + raw["unsat"]
    trusted_decisions = trusted["sat"] + trusted["unsat"]
    return {
        "backend": backend,
        "raw": dict(raw),
        "trusted": dict(trusted),
        "evidence": dict(evidence),
        "cohorts": {key: dict(value) for key, value in sorted(cohorts.items())},
        "partitions": {
            key: dict(value) for key, value in sorted(partitions.items())
        },
        "solver_calls": solver_calls,
        "pruned_branches": trusted["unsat"],
        "closed_workloads": closed,
        "expected_closed_workloads": expected_closed,
        "unsupported_raw_decisions_rejected": unsupported_raw_decisions,
        "raw_decisions": raw_decisions,
        "eligible_raw_decisions": eligible_raw_decisions,
        "trusted_decisions": trusted_decisions,
        "python_verified_decisions": len(verified_records),
        "unverified_eligible_decisions": unverified_eligible_decisions,
        "python_verified_over_eligible_raw": (
            len(
                [
                    record
                    for record in verified_records
                    if record[1]["eligible"]
                ]
            )
            / eligible_raw_decisions
            if eligible_raw_decisions
            else 1.0
        ),
        "python_verified_over_raw": (
            len(verified_records) / raw_decisions if raw_decisions else 0.0
        ),
        "python_verified_over_trusted": (
            len(verified_records) / trusted_decisions if trusted_decisions else 1.0
        ),
        "expected_failures": expected_failures,
    }, verified_records


def normalized_result_map(
    results: Sequence[SolverResult],
) -> dict[tuple[str, str], dict[str, Any]]:
    return {
        (result.workload_id, result.branch_id): result.normalized_evidence()
        for result in results
    }


def compare_backends(
    process_results: Sequence[SolverResult],
    ffi_results: Sequence[SolverResult],
) -> list[dict[str, str]]:
    process = normalized_result_map(process_results)
    ffi = normalized_result_map(ffi_results)
    mismatches = []
    for key in sorted(process.keys() | ffi.keys()):
        left = process.get(key)
        right = ffi.get(key)
        if left is None or right is None or left["status"] != right["status"]:
            mismatches.append(
                {
                    "workload": key[0],
                    "branch": key[1],
                    "process": "missing" if left is None else left["status"],
                    "ffi": "missing" if right is None else right["status"],
                }
            )
    return mismatches


def write_certificates(
    path: Path,
    backend_records: dict[
        str, Sequence[tuple[SolverResult, dict[str, Any], dict[str, Any]]]
    ],
) -> int:
    lines = ["UMLAUT_GROUND_THEORY_CERT_V1"]
    count = 0
    for backend in sorted(backend_records):
        for result, workload, branch in backend_records[backend]:
            lines.append(
                "\t".join(
                    [
                        "DECISION",
                        backend,
                        workload["id"],
                        branch["id"],
                        workload["sort"],
                        result.raw_status,
                    ]
                )
            )
            for constraint in asserted_constraints(workload, branch):
                if constraint["kind"] != "difference":
                    raise ProtocolError("trusted certificate contains unsupported syntax")
                bound = parse_fraction(constraint["bound"])
                lines.append(
                    "\t".join(
                        [
                            "CONSTRAINT",
                            constraint["label"],
                            constraint["lhs"],
                            constraint["rhs"],
                            str(bound.numerator),
                            str(bound.denominator),
                        ]
                    )
                )
            if result.raw_status == "unsat":
                lines.append("CORE\t" + ",".join(result.core))
            else:
                for variable, raw_value in result.model:
                    value = parse_fraction(raw_value)
                    lines.append(
                        "\t".join(
                            [
                                "MODEL",
                                variable,
                                str(value.numerator),
                                str(value.denominator),
                            ]
                        )
                    )
            lines.append("END_DECISION")
            count += 1
    lines.append("END")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return count


def run_replay(driver: Path, certificate: Path) -> dict[str, Any]:
    completed = subprocess.run(
        [str(driver), str(certificate)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=30,
    )
    summary = {
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }
    if completed.returncode != 0:
        raise ProtocolError(f"Rust replay failed: {summary}")
    fields = completed.stdout.strip().split("\t")
    if (
        len(fields) != 7
        or fields[0] != "SUMMARY"
        or fields[1] != "total"
        or fields[3] != "verified"
        or fields[5] != "invalid"
    ):
        raise ProtocolError("malformed Rust replay summary")
    summary.update(
        {
            "total": int(fields[2]),
            "verified": int(fields[4]),
            "invalid": int(fields[6]),
        }
    )
    return summary


def run_mutations(
    checker: Path,
    certificate: Path,
    output_directory: Path,
) -> dict[str, Any]:
    script = EXPERIMENT_ROOT / "mutate_certificates.py"
    output_directory.mkdir(parents=True, exist_ok=True)
    report_path = output_directory / "mutation-report.json"
    completed = subprocess.run(
        [
            os.fspath(Path(os.sys.executable)),
            os.fspath(script),
            "--checker",
            os.fspath(checker),
            "--certificate",
            os.fspath(certificate),
            "--output-dir",
            os.fspath(output_directory),
            "--report",
            os.fspath(report_path),
        ],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=60,
    )
    if completed.returncode != 0:
        raise ProtocolError(
            f"certificate mutation gate failed: {completed.stderr.strip()}"
        )
    return json.loads(report_path.read_text(encoding="utf-8"))


def process_unknown_probe(executable: Path) -> dict[str, Any]:
    script = "\n".join(
        [
            "(set-option :rlimit 1)",
            "(set-logic QF_NIA)",
            "(declare-const x Int)",
            "(declare-const y Int)",
            "(assert (= (* x y) 1234567891011))",
            "(assert (> x 1))",
            "(assert (> y 1))",
            "(check-sat)",
            "(get-info :reason-unknown)",
            "(exit)",
            "",
        ]
    )
    completed = subprocess.run(
        [str(executable), "-in", "-smt2"],
        input=script,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=10,
    )
    output_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
    return {
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "status": output_lines[0] if output_lines else "missing",
        "passed": completed.returncode == 0
        and bool(output_lines)
        and output_lines[0] == "unknown",
    }


def pigeonhole_script(pigeons: int = 45, holes: int = 44) -> str:
    lines = []
    for pigeon in range(pigeons):
        for hole in range(holes):
            lines.append(f"(declare-const p_{pigeon}_{hole} Bool)")
    for pigeon in range(pigeons):
        variables = " ".join(f"p_{pigeon}_{hole}" for hole in range(holes))
        lines.append(f"(assert (or {variables}))")
    for hole in range(holes):
        for left in range(pigeons):
            for right in range(left + 1, pigeons):
                lines.append(
                    f"(assert (or (not p_{left}_{hole}) (not p_{right}_{hole})))"
                )
    lines.append("(check-sat)")
    return "\n".join(lines) + "\n"


def process_cancel_probe(executable: Path) -> dict[str, Any]:
    started = time.perf_counter_ns()
    process = subprocess.Popen(
        [str(executable), "-in", "-smt2"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    assert process.stdin is not None
    process.stdin.write(pigeonhole_script())
    process.stdin.flush()
    time.sleep(0.01)
    kill_started = time.perf_counter_ns()
    process.kill()
    stdout, stderr = process.communicate(timeout=1)
    kill_ns = time.perf_counter_ns() - kill_started
    total_ns = time.perf_counter_ns() - started
    return {
        "returncode": process.returncode,
        "stdout": stdout,
        "stderr": stderr,
        "kill_ns": kill_ns,
        "total_ns": total_ns,
        "counted_result": False,
        "passed": process.returncode != 0 and kill_ns < 1_000_000_000,
    }


def ffi_cancel_probe(
    driver: Path,
    output: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    completed = subprocess.run(
        [str(driver), "cancel", str(output)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=120,
        env=environment,
    )
    record = output.read_text(encoding="utf-8").strip() if output.is_file() else ""
    fields = record.split("\t")
    passed = (
        completed.returncode == 0
        and len(fields) == 4
        and fields[0] == "CANCEL"
        and fields[1] == "unknown"
        and int(fields[2]) < 1_000_000_000
    )
    return {
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "record": record,
        "passed": passed,
    }


def ldd(path: Path) -> dict[str, Any]:
    completed = subprocess.run(
        ["ldd", str(path)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=30,
    )
    return {
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def artifact_metadata(path: Path) -> dict[str, Any]:
    return {
        "path": str(path),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
        "ldd": ldd(path),
    }


def directory_size(path: Path) -> int:
    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())


def source_identity(
    root: Path,
    claimed_commit: str,
    archive: Path,
) -> dict[str, Any]:
    archive_hash = sha256(archive)
    if (root / ".git").exists():
        commit = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=30,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "-C", str(root), "status", "--short"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=30,
        ).stdout
        identity_mode = "git_checkout"
        clean = status == ""
    else:
        commit = claimed_commit
        status = "archive extraction has no Git metadata"
        identity_mode = "pinned_git_archive"
        clean = archive_hash == EXPECTED_Z3_ARCHIVE_SHA256
    return {
        "commit": commit,
        "expected_commit": EXPECTED_Z3_COMMIT,
        "commit_matches": commit == EXPECTED_Z3_COMMIT,
        "archive": str(archive),
        "archive_sha256": archive_hash,
        "expected_archive_sha256": EXPECTED_Z3_ARCHIVE_SHA256,
        "archive_matches": archive_hash == EXPECTED_Z3_ARCHIVE_SHA256,
        "identity_mode": identity_mode,
        "clean": clean,
        "status": status,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--z3", type=Path, required=True)
    parser.add_argument("--z3-library", type=Path, required=True)
    parser.add_argument("--z3-source-root", type=Path, required=True)
    parser.add_argument("--z3-source-archive", type=Path, required=True)
    parser.add_argument("--z3-source-commit", default=EXPECTED_Z3_COMMIT)
    parser.add_argument("--z3-build-root", type=Path, required=True)
    parser.add_argument("--ffi-driver", type=Path, required=True)
    parser.add_argument("--replay-driver", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--certificate-output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=MEASURED_REPETITIONS)
    args = parser.parse_args()

    if args.repetitions < 2:
        raise SystemExit("--repetitions must be at least 2")
    corpus = load_corpus(args.corpus)
    solver_workloads = [
        workload
        for workload in corpus["workloads"]
        if workload["cohort"] != "neutral"
    ]
    environment = os.environ.copy()
    library_directory = str(args.z3_library.parent)
    current_library_path = environment.get("LD_LIBRARY_PATH", "")
    environment["LD_LIBRARY_PATH"] = (
        library_directory
        if not current_library_path
        else library_directory + os.pathsep + current_library_path
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.certificate_output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="ground-theory-smt-", dir=args.output.parent
    ) as temporary:
        temporary_root = Path(temporary)
        protocol_path = temporary_root / "ffi-protocol.txt"
        write_ffi_protocol(protocol_path, solver_workloads)

        run_process_repetition(args.z3, solver_workloads)
        run_ffi_repetition(
            args.ffi_driver,
            protocol_path,
            temporary_root / "ffi-warmup.txt",
            environment,
        )

        process_runs = []
        process_metadata = []
        ffi_runs = []
        ffi_metadata = []
        no_smt_runs = []
        no_smt_timings = []
        for repetition in range(args.repetitions):
            no_smt, no_smt_elapsed = no_smt_results(corpus)
            no_smt_runs.append(no_smt)
            no_smt_timings.append(no_smt_elapsed)
            process, process_meta = run_process_repetition(
                args.z3, solver_workloads
            )
            process_runs.append(process)
            process_metadata.append(process_meta)
            ffi, ffi_meta = run_ffi_repetition(
                args.ffi_driver,
                protocol_path,
                temporary_root / f"ffi-{repetition}.txt",
                environment,
            )
            ffi_runs.append(ffi)
            ffi_metadata.append(ffi_meta)

        no_smt_analysis, _ = analyze_backend("no_smt", no_smt_runs[0], corpus)
        process_analysis, process_verified = analyze_backend(
            "process", process_runs[0], corpus
        )
        ffi_analysis, ffi_verified = analyze_backend("ffi", ffi_runs[0], corpus)

        certificate_count = write_certificates(
            args.certificate_output,
            {"ffi": ffi_verified, "process": process_verified},
        )
        replay = run_replay(args.replay_driver, args.certificate_output)
        mutations = run_mutations(
            args.replay_driver,
            args.certificate_output,
            temporary_root / "mutations",
        )
        cancel_output = temporary_root / "ffi-cancel.txt"
        cancellation = {
            "process": process_cancel_probe(args.z3),
            "ffi": ffi_cancel_probe(args.ffi_driver, cancel_output, environment),
            "process_unknown": process_unknown_probe(args.z3),
        }

        process_hashes = [evidence_hash(run) for run in process_runs]
        ffi_hashes = [evidence_hash(run) for run in ffi_runs]
        process_call_times = [
            result.elapsed_ns for run in process_runs for result in run
        ]
        ffi_call_times = [
            result.elapsed_ns for run in ffi_runs for result in run
        ]
        raw_decisions = (
            process_analysis["raw_decisions"] + ffi_analysis["raw_decisions"]
        )
        trusted_decisions = (
            process_analysis["trusted_decisions"]
            + ffi_analysis["trusted_decisions"]
        )
        report = {
            "schema": "umlaut-ground-theory-smt-report-v1",
            "corpus": {
                "path": str(args.corpus),
                "sha256": sha256(args.corpus),
                "workloads": len(corpus["workloads"]),
                "solver_workloads": len(solver_workloads),
                "branches": sum(
                    len(workload["branches"])
                    for workload in corpus["workloads"]
                ),
            },
            "configuration": {
                "warmups": 1,
                "measured_repetitions": args.repetitions,
                "solver_timeout_ms": 5_000,
                "z3_expected_commit": EXPECTED_Z3_COMMIT,
            },
            "source": source_identity(
                args.z3_source_root,
                args.z3_source_commit,
                args.z3_source_archive,
            ),
            "backends": {
                "no_smt": no_smt_analysis,
                "process": process_analysis,
                "ffi": ffi_analysis,
            },
            "agreement": {
                "process_ffi_status_mismatches": compare_backends(
                    process_runs[0], ffi_runs[0]
                ),
                "process_evidence_hashes": process_hashes,
                "ffi_evidence_hashes": ffi_hashes,
                "process_deterministic": len(set(process_hashes)) == 1,
                "ffi_deterministic": len(set(ffi_hashes)) == 1,
            },
            "timing": {
                "no_smt_dispatch": timing_summary(no_smt_timings),
                "process_calls": timing_summary(process_call_times),
                "process_startup": timing_summary(
                    [item["startup_ns"] for item in process_metadata]
                ),
                "process_shutdown": timing_summary(
                    [item["shutdown_ns"] for item in process_metadata]
                ),
                "ffi_calls": timing_summary(ffi_call_times),
                "ffi_driver_total": timing_summary(
                    [item["driver_total_ns"] for item in ffi_metadata]
                ),
            },
            "replay": {
                **replay,
                "certificate_path": str(args.certificate_output),
                "certificate_sha256": sha256(args.certificate_output),
                "certificate_count": certificate_count,
                "verified_over_raw": (
                    replay["verified"] / raw_decisions if raw_decisions else 0.0
                ),
                "verified_over_trusted": (
                    replay["verified"] / trusted_decisions
                    if trusted_decisions
                    else 1.0
                ),
                "mutations": mutations,
            },
            "cancellation_and_unknown": cancellation,
            "package": {
                "z3_executable": artifact_metadata(args.z3),
                "z3_shared_library": artifact_metadata(args.z3_library),
                "ffi_driver": artifact_metadata(args.ffi_driver),
                "replay_driver": artifact_metadata(args.replay_driver),
                "z3_build_tree_bytes": directory_size(args.z3_build_root),
                "no_smt_runtime_delta_bytes": 0,
                "process_candidate_bytes": args.z3.stat().st_size,
                "ffi_prototype_candidate_bytes": (
                    args.z3_library.stat().st_size
                    + args.ffi_driver.stat().st_size
                ),
                "combined_z3_build_outputs_bytes": (
                    args.z3.stat().st_size + args.z3_library.stat().st_size
                ),
            },
        }

        gates = {
            "source_pin": report["source"]["commit_matches"]
            and report["source"]["archive_matches"]
            and report["source"]["clean"],
            "backend_agreement": not report["agreement"][
                "process_ffi_status_mismatches"
            ],
            "process_deterministic": report["agreement"]["process_deterministic"],
            "ffi_deterministic": report["agreement"]["ffi_deterministic"],
            "process_expected": not process_analysis["expected_failures"],
            "ffi_expected": not ffi_analysis["expected_failures"],
            "python_verification": (
                not process_analysis["unverified_eligible_decisions"]
                and not ffi_analysis["unverified_eligible_decisions"]
            ),
            "rust_replay": replay["invalid"] == 0
            and replay["verified"] == certificate_count,
            "mutation_rejection": mutations["passed"],
            "unsupported_rejected": (
                process_analysis["unsupported_raw_decisions_rejected"] > 0
                and ffi_analysis["unsupported_raw_decisions_rejected"] > 0
            ),
            "process_cancel": cancellation["process"]["passed"],
            "ffi_interrupt": cancellation["ffi"]["passed"],
            "unknown": cancellation["process_unknown"]["passed"],
            "neutral_bypass": all(
                result.workload_id not in {
                    workload["id"]
                    for workload in corpus["workloads"]
                    if workload["cohort"] == "neutral"
                }
                for run in [process_runs[0], ffi_runs[0]]
                for result in run
            ),
        }
        report["gates"] = gates
        report["all_correctness_gates_passed"] = all(gates.values())
        args.output.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    print(json.dumps(report["gates"], indent=2, sort_keys=True))
    print(f"report: {args.output}")
    if not report["all_correctness_gates_passed"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
