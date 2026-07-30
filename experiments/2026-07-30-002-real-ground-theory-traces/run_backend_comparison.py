#!/usr/bin/env python3
"""Compare native, pinned-Z3 process, and pinned-Z3 FFI decisions."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PREVIOUS_EXPERIMENT = (
    EXPERIMENT_ROOT.parent / "2026-07-30-001-ground-theory-smt-cooperation"
)
sys.path.insert(0, str(PREVIOUS_EXPERIMENT))

from ground_theory import (  # noqa: E402
    ProcessSession,
    ProtocolError,
    SolverResult,
    evidence_hash,
    load_corpus,
    parse_ffi_results,
    write_ffi_protocol,
)
from run_experiment import (  # noqa: E402
    analyze_backend,
    artifact_metadata,
    compare_backends,
    directory_size,
    ffi_cancel_probe,
    no_smt_results,
    process_cancel_probe,
    process_unknown_probe,
    run_ffi_repetition,
    run_process_repetition,
    run_replay,
    sha256,
    source_identity,
    timing_summary,
    write_certificates,
)

sys.path.pop(0)
from native_protocol import parse_results as parse_native_results  # noqa: E402


MEASURED_REPETITIONS = 5


def native_results(path: Path) -> list[SolverResult]:
    records, _ = parse_native_results(path)
    return [
        SolverResult(
            workload_id=record["id"],
            branch_id="branch",
            raw_status=record["status"],
            elapsed_ns=record["elapsed_ns"],
            core=tuple(record["core"]),
            model=tuple(sorted(record["model"].items())),
            reason=record["reason"],
        )
        for record in records
    ]


def write_process_results(path: Path, results: Sequence[SolverResult]) -> None:
    payload = [dataclasses.asdict(result) for result in results]
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run_mutations(
    checker: Path,
    certificate: Path,
    output_directory: Path,
) -> dict[str, Any]:
    report = output_directory / "mutation-report.json"
    output_directory.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(
        [
            sys.executable,
            str(EXPERIMENT_ROOT / "mutate_certificates.py"),
            "--checker",
            str(checker),
            "--certificate",
            str(certificate),
            "--output-dir",
            str(output_directory),
            "--report",
            str(report),
        ],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=180,
    )
    if completed.returncode != 0:
        raise ProtocolError(
            "certificate mutation gate failed: "
            f"stdout={completed.stdout.strip()}, stderr={completed.stderr.strip()}"
        )
    return json.loads(report.read_text(encoding="utf-8"))


def native_cancel_probe(
    driver: Path,
    protocol: Path,
    output: Path,
) -> dict[str, Any]:
    started = time.perf_counter_ns()
    process = subprocess.Popen(
        [str(driver), "run", str(protocol), str(output)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    time.sleep(0.001)
    kill_started = time.perf_counter_ns()
    process.kill()
    stdout, stderr = process.communicate(timeout=1)
    kill_ns = time.perf_counter_ns() - kill_started
    return {
        "returncode": process.returncode,
        "stdout": stdout,
        "stderr": stderr,
        "kill_ns": kill_ns,
        "total_ns": time.perf_counter_ns() - started,
        "counted_result": False,
        "passed": process.returncode != 0 and kill_ns < 1_000_000_000,
    }


def result_statuses(
    results: Sequence[SolverResult],
) -> dict[tuple[str, str], str]:
    return {
        (result.workload_id, result.branch_id): result.raw_status
        for result in results
    }


def status_mismatches(
    left_name: str,
    left: Sequence[SolverResult],
    right_name: str,
    right: Sequence[SolverResult],
) -> list[dict[str, str]]:
    left_statuses = result_statuses(left)
    right_statuses = result_statuses(right)
    mismatches = []
    for key in sorted(left_statuses.keys() | right_statuses.keys()):
        left_status = left_statuses.get(key, "missing")
        right_status = right_statuses.get(key, "missing")
        if left_status != right_status:
            mismatches.append(
                {
                    "workload": key[0],
                    "branch": key[1],
                    left_name: left_status,
                    right_name: right_status,
                }
            )
    return mismatches


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--native-protocol", required=True, type=Path)
    parser.add_argument("--cancellation-protocol", type=Path)
    parser.add_argument(
        "--native-result",
        required=True,
        action="append",
        type=Path,
    )
    parser.add_argument("--native-driver", required=True, type=Path)
    parser.add_argument("--z3", required=True, type=Path)
    parser.add_argument("--z3-library", required=True, type=Path)
    parser.add_argument("--z3-source-root", required=True, type=Path)
    parser.add_argument("--z3-source-archive", required=True, type=Path)
    parser.add_argument("--z3-source-commit", required=True)
    parser.add_argument("--z3-build-root", required=True, type=Path)
    parser.add_argument("--ffi-driver", required=True, type=Path)
    parser.add_argument("--replay-driver", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--repetitions", type=int, default=MEASURED_REPETITIONS)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    cancellation_protocol = (
        arguments.cancellation_protocol or arguments.native_protocol
    )
    if arguments.repetitions != len(arguments.native_result):
        raise SystemExit("native result count must equal --repetitions")
    if arguments.repetitions < 2:
        raise SystemExit("--repetitions must be at least 2")
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    corpus = load_corpus(arguments.corpus)
    workloads = corpus["workloads"]
    native_runs = [native_results(path) for path in arguments.native_result]
    protocol = arguments.output_root / "ffi-protocol.txt"
    write_ffi_protocol(protocol, workloads)

    environment = os.environ.copy()
    library_directory = str(arguments.z3_library.parent)
    current_library_path = environment.get("LD_LIBRARY_PATH", "")
    environment["LD_LIBRARY_PATH"] = (
        library_directory
        if not current_library_path
        else library_directory + os.pathsep + current_library_path
    )

    run_process_repetition(arguments.z3, workloads)
    run_ffi_repetition(
        arguments.ffi_driver,
        protocol,
        arguments.output_root / "ffi-warmup.txt",
        environment,
    )
    process_runs = []
    process_metadata = []
    ffi_runs = []
    ffi_metadata = []
    no_smt_timings = []
    for repetition in range(arguments.repetitions):
        _, no_smt_elapsed = no_smt_results(corpus)
        no_smt_timings.append(no_smt_elapsed)
        process, process_meta = run_process_repetition(arguments.z3, workloads)
        write_process_results(
            arguments.output_root / f"process-results-{repetition + 1}.json",
            process,
        )
        process_runs.append(process)
        process_metadata.append(process_meta)
        ffi, ffi_meta = run_ffi_repetition(
            arguments.ffi_driver,
            protocol,
            arguments.output_root / f"ffi-results-{repetition + 1}.txt",
            environment,
        )
        ffi_runs.append(ffi)
        ffi_metadata.append(ffi_meta)

    no_smt, _ = no_smt_results(corpus)
    no_smt_analysis, _ = analyze_backend("no_checker", no_smt, corpus)
    native_analysis, native_verified = analyze_backend(
        "native", native_runs[0], corpus
    )
    process_analysis, process_verified = analyze_backend(
        "process", process_runs[0], corpus
    )
    ffi_analysis, ffi_verified = analyze_backend("ffi", ffi_runs[0], corpus)
    certificate = arguments.output_root / "combined-certificates.txt"
    certificate_count = write_certificates(
        certificate,
        {
            "ffi": ffi_verified,
            "native": native_verified,
            "process": process_verified,
        },
    )
    replay = run_replay(arguments.replay_driver, certificate)
    mutations = run_mutations(
        arguments.replay_driver,
        certificate,
        arguments.output_root / "mutations",
    )
    cancellation = {
        "native": native_cancel_probe(
            arguments.native_driver,
            cancellation_protocol,
            arguments.output_root / "native-cancel-results.txt",
        ),
        "process": process_cancel_probe(arguments.z3),
        "ffi": ffi_cancel_probe(
            arguments.ffi_driver,
            arguments.output_root / "ffi-cancel.txt",
            environment,
        ),
        "process_unknown": process_unknown_probe(arguments.z3),
    }

    analyses = {
        "no_checker": no_smt_analysis,
        "native": native_analysis,
        "process": process_analysis,
        "ffi": ffi_analysis,
    }
    evidence_hashes = {
        "native": [evidence_hash(run) for run in native_runs],
        "process": [evidence_hash(run) for run in process_runs],
        "ffi": [evidence_hash(run) for run in ffi_runs],
    }
    comparisons = {
        "native_process": status_mismatches(
            "native", native_runs[0], "process", process_runs[0]
        ),
        "native_ffi": status_mismatches(
            "native", native_runs[0], "ffi", ffi_runs[0]
        ),
        "process_ffi": compare_backends(process_runs[0], ffi_runs[0]),
    }
    report = {
        "schema": "umlaut-real-ground-backend-comparison-v1",
        "corpus": {
            "path": str(arguments.corpus),
            "sha256": sha256(arguments.corpus),
            "queries": len(workloads),
        },
        "configuration": {
            "warmups": 1,
            "measured_repetitions": arguments.repetitions,
            "solver_timeout_ms": 5_000,
        },
        "source": source_identity(
            arguments.z3_source_root,
            arguments.z3_source_commit,
            arguments.z3_source_archive,
        ),
        "backends": analyses,
        "agreement": {
            "mismatches": comparisons,
            "evidence_hashes": evidence_hashes,
            "deterministic": {
                backend: len(set(hashes)) == 1
                for backend, hashes in evidence_hashes.items()
            },
        },
        "timing": {
            "no_checker_dispatch": timing_summary(no_smt_timings),
            "native_calls": timing_summary(
                [
                    result.elapsed_ns
                    for run in native_runs
                    for result in run
                ]
            ),
            "process_calls": timing_summary(
                [
                    result.elapsed_ns
                    for run in process_runs
                    for result in run
                ]
            ),
            "process_startup": timing_summary(
                [metadata["startup_ns"] for metadata in process_metadata]
            ),
            "process_shutdown": timing_summary(
                [metadata["shutdown_ns"] for metadata in process_metadata]
            ),
            "ffi_calls": timing_summary(
                [
                    result.elapsed_ns
                    for run in ffi_runs
                    for result in run
                ]
            ),
            "ffi_driver_total": timing_summary(
                [metadata["driver_total_ns"] for metadata in ffi_metadata]
            ),
        },
        "replay": {
            **replay,
            "certificate_path": str(certificate),
            "certificate_sha256": sha256(certificate),
            "certificate_count": certificate_count,
            "mutations": mutations,
        },
        "cancellation_and_unknown": cancellation,
        "package": {
            "native_driver": artifact_metadata(arguments.native_driver),
            "z3_executable": artifact_metadata(arguments.z3),
            "z3_shared_library": artifact_metadata(arguments.z3_library),
            "ffi_driver": artifact_metadata(arguments.ffi_driver),
            "replay_driver": artifact_metadata(arguments.replay_driver),
            "z3_build_tree_bytes": directory_size(arguments.z3_build_root),
            "default_runtime_delta_bytes": 0,
        },
    }
    gates = {
        "source_pin": (
            report["source"]["commit_matches"]
            and report["source"]["archive_matches"]
            and report["source"]["clean"]
        ),
        "complete_results": all(
            analysis["raw_decisions"] == len(workloads)
            for name, analysis in analyses.items()
            if name != "no_checker"
        ),
        "backend_agreement": all(not items for items in comparisons.values()),
        "deterministic": all(report["agreement"]["deterministic"].values()),
        "expected": all(
            not analysis["expected_failures"]
            for name, analysis in analyses.items()
            if name != "no_checker"
        ),
        "python_verification": all(
            not analysis["unverified_eligible_decisions"]
            for name, analysis in analyses.items()
            if name != "no_checker"
        ),
        "rust_replay": (
            replay["invalid"] == 0
            and replay["verified"] == certificate_count
        ),
        "mutation_rejection": mutations["passed"],
        "native_cancel": cancellation["native"]["passed"],
        "process_cancel": cancellation["process"]["passed"],
        "ffi_interrupt": cancellation["ffi"]["passed"],
        "unknown": cancellation["process_unknown"]["passed"],
    }
    report["gates"] = gates
    report["all_correctness_gates_passed"] = all(gates.values())
    report_path = arguments.output_root / "backend-report.json"
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(gates, indent=2, sort_keys=True))
    print(f"report: {report_path}")
    return 0 if report["all_correctness_gates_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
