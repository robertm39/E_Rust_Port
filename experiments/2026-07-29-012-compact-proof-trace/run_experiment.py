#!/usr/bin/env python3
"""Run the compact proof-trace storage study on native Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import struct
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Sequence

import proof_trace


PROOFCHECK_SHA256 = (
    "92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e"
)
SZS_STATUS_RE = re.compile(
    rb"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
CHECKER_STATUS_RE = re.compile(
    rb"SZS\s+status\s+(VerifiedGood|VerifiedBad|Unknown)\b",
    re.IGNORECASE,
)
FORMULA_RE = re.compile(rb"^\s*(?:cnf|fof)\s*\(", re.IGNORECASE)
INFERENCE_RE = re.compile(rb",\s*inference\s*\(", re.IGNORECASE)
MAX_RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
ELAPSED_RE = re.compile(r"Elapsed \(wall clock\) time.*:\s*(\S+)")


class ExperimentError(RuntimeError):
    """A frozen experiment gate failed."""


@dataclass(frozen=True)
class Case:
    name: str
    problem: Path
    cpu_seconds: int
    wall_seconds: int
    expected_status: str


@dataclass(frozen=True)
class ProcessResult:
    command: list[str]
    returncode: int
    wall_seconds: float
    stdout: bytes
    stderr: bytes


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run_capture(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: int,
) -> ProcessResult:
    started = time.perf_counter()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        capture_output=True,
        timeout=timeout,
    )
    return ProcessResult(
        command=list(command),
        returncode=completed.returncode,
        wall_seconds=time.perf_counter() - started,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def write_process(root: Path, name: str, result: ProcessResult) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / f"{name}.stdout").write_bytes(result.stdout)
    (root / f"{name}.stderr").write_bytes(result.stderr)
    write_json(
        root / f"{name}.json",
        {
            "command": result.command,
            "returncode": result.returncode,
            "wall_seconds": result.wall_seconds,
        },
    )


def final_szs_status(output: bytes) -> str:
    matches = SZS_STATUS_RE.findall(output)
    return matches[-1].decode("ascii").lower() if matches else "missing"


def proofcheck(
    checker: Path,
    problem: Path,
    solution: Path,
    artifact_root: Path,
    name: str,
) -> dict[str, Any]:
    result = run_capture(
        [str(checker), "-p", str(problem), str(solution)],
        cwd=checker.parent,
        timeout=300,
    )
    write_process(artifact_root / "checker", name, result)
    statuses = CHECKER_STATUS_RE.findall(result.stdout + b"\n" + result.stderr)
    status = statuses[-1].decode("ascii").lower() if statuses else "missing"
    if result.returncode != 0 or status != "verifiedgood":
        raise ExperimentError(
            f"{name}: ProofCheck returned {result.returncode}/{status}"
        )
    return {
        "returncode": result.returncode,
        "status": status,
        "wall_seconds": result.wall_seconds,
        "stdout_sha256": sha256_bytes(result.stdout),
        "stderr_sha256": sha256_bytes(result.stderr),
    }


def count_tstp_records(payload: bytes) -> tuple[int, int]:
    formula_count = 0
    inferred_count = 0
    for line in payload.splitlines():
        if not FORMULA_RE.match(line):
            continue
        formula_count += 1
        if INFERENCE_RE.search(line):
            inferred_count += 1
    return formula_count, inferred_count


def time_codec(
    proof: Path,
    spool: Path,
    compact_output: Path,
    spool_output: Path,
    repetitions: int,
) -> tuple[bytes, dict[str, Any]]:
    samples: dict[str, list[float]] = {
        "compact_encode": [],
        "compact_replay": [],
        "spool_encode": [],
        "spool_replay": [],
    }
    compact_trace = b""
    expected = proof.read_bytes()

    for repetition in range(repetitions):
        operations = (
            ("compact_encode", "spool_encode", "compact_replay", "spool_replay")
            if repetition % 2 == 0
            else ("spool_replay", "compact_replay", "spool_encode", "compact_encode")
        )
        for operation in operations:
            started = time.perf_counter_ns()
            if operation == "compact_encode":
                candidate, _ = proof_trace.encode_path_to_bytes(proof)
                if compact_trace and candidate != compact_trace:
                    raise ExperimentError("compact encoding is nondeterministic")
                compact_trace = candidate
            elif operation == "spool_encode":
                proof_trace.encode_path_to_path(proof, spool)
            elif operation == "compact_replay":
                if not compact_trace:
                    compact_trace, _ = proof_trace.encode_path_to_bytes(proof)
                reconstructed, _ = proof_trace.decode_bytes(compact_trace)
                if reconstructed != expected:
                    raise ExperimentError("compact replay changed proof bytes")
                compact_output.write_bytes(reconstructed)
            else:
                if not spool.is_file():
                    proof_trace.encode_path_to_path(proof, spool)
                proof_trace.atomic_replay(spool, spool_output)
                if spool_output.read_bytes() != expected:
                    raise ExperimentError("spooled replay changed proof bytes")
            samples[operation].append((time.perf_counter_ns() - started) / 1e9)

    return compact_trace, {
        name: {
            "median_seconds": statistics.median(values),
            "minimum_seconds": min(values),
            "maximum_seconds": max(values),
            "repetitions": len(values),
        }
        for name, values in samples.items()
    }


def frame_payload_spans(trace: bytes) -> list[tuple[int, int]]:
    if not trace.startswith(proof_trace.MAGIC):
        raise ExperimentError("compact trace has no format header")
    spans: list[tuple[int, int]] = []
    offset = len(proof_trace.MAGIC)
    while offset < len(trace):
        tag = trace[offset]
        if tag == proof_trace.TRAILER:
            break
        if tag not in {proof_trace.FRAME_RAW, proof_trace.FRAME_ZLIB}:
            raise ExperimentError(f"unexpected trace tag {tag}")
        header_end = offset + proof_trace.FRAME_HEADER.size
        if header_end > len(trace):
            raise ExperimentError("trace header is truncated")
        _, _, stored_length, _ = proof_trace.FRAME_HEADER.unpack(
            trace[offset:header_end]
        )
        payload_end = header_end + stored_length
        if payload_end > len(trace):
            raise ExperimentError("trace payload is truncated")
        spans.append((header_end, payload_end))
        offset = payload_end
    if not spans:
        raise ExperimentError("proof trace contains no frames")
    return spans


def failure_recovery(
    compact_trace: bytes,
    case_root: Path,
    script: Path,
    python: Path,
) -> dict[str, Any]:
    mutations: dict[str, bytes] = {}
    spans = frame_payload_spans(compact_trace)
    last_start, last_end = spans[-1]
    mutations["truncated-final-frame"] = compact_trace[
        : last_start + max(1, (last_end - last_start) // 2)
    ]

    flipped = bytearray(compact_trace)
    flipped[last_start + (last_end - last_start) // 2] ^= 1
    mutations["payload-bit-flip"] = bytes(flipped)

    invalid_length = bytearray(compact_trace)
    raw_length_offset = len(proof_trace.MAGIC) + 1
    invalid_length[raw_length_offset : raw_length_offset + 4] = struct.pack(
        ">I", proof_trace.MAX_FRAME_BYTES + 1
    )
    mutations["invalid-length"] = bytes(invalid_length)

    mutation_results: dict[str, Any] = {}
    for name, payload in mutations.items():
        path = case_root / f"{name}.uptl"
        output = case_root / f"{name}.published"
        path.write_bytes(payload)
        error_text = ""
        try:
            proof_trace.atomic_replay(path, output)
        except proof_trace.TraceFormatError as error:
            error_text = str(error)
        else:
            raise ExperimentError(f"{name}: malformed log was accepted")
        temporary_files = list(case_root.glob(f".{output.name}.*.tmp"))
        if output.exists() or temporary_files:
            raise ExperimentError(f"{name}: partial output was published or retained")
        mutation_results[name] = {
            "error": error_text,
            "published": output.exists(),
            "temporary_file_count": len(temporary_files),
            "sha256": sha256_bytes(payload),
        }

    spool = case_root / "proof.uptl"
    interrupted_output = case_root / "interrupted.published"
    ready = case_root / "interrupt.ready"
    process = subprocess.Popen(
        [
            str(python),
            str(script),
            "replay",
            "--input",
            str(spool),
            "--output",
            str(interrupted_output),
            "--pause-after-frame",
            "1",
            "--ready-file",
            str(ready),
        ],
        cwd=script.parent,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    deadline = time.monotonic() + 15
    while not ready.exists() and process.poll() is None and time.monotonic() < deadline:
        time.sleep(0.01)
    if not ready.exists():
        process.kill()
        stdout, stderr = process.communicate(timeout=10)
        raise ExperimentError(
            "interrupted replay did not reach the first-frame barrier: "
            + (stdout + stderr).decode("utf-8", errors="replace")
        )
    process.terminate()
    interrupted_stdout, interrupted_stderr = process.communicate(timeout=10)
    if interrupted_output.exists():
        raise ExperimentError("interrupted replay published a final output")
    orphaned_temporaries = list(
        case_root.glob(f".{interrupted_output.name}.*.tmp")
    )
    replay_stats = proof_trace.atomic_replay(spool, interrupted_output)
    for temporary in orphaned_temporaries:
        temporary.unlink(missing_ok=True)
    return {
        "mutations": mutation_results,
        "interruption": {
            "returncode": process.returncode,
            "stdout_sha256": sha256_bytes(interrupted_stdout),
            "stderr_sha256": sha256_bytes(interrupted_stderr),
            "published_before_complete_replay": False,
            "orphaned_temporary_count": len(orphaned_temporaries),
            "complete_replay_sha256": replay_stats.sha256,
        },
    }


def parse_elapsed(value: str) -> float:
    parts = value.split(":")
    try:
        if len(parts) == 2:
            return float(parts[0]) * 60 + float(parts[1])
        if len(parts) == 3:
            return float(parts[0]) * 3600 + float(parts[1]) * 60 + float(parts[2])
        return float(value)
    except ValueError as error:
        raise ExperimentError(f"unparseable GNU time elapsed value: {value}") from error


def measure_worker_rss(
    *,
    mode: str,
    input_path: Path | None,
    output_path: Path | None,
    script: Path,
    python: Path,
    artifact_root: Path,
    name: str,
) -> dict[str, Any]:
    time_report = artifact_root / f"{name}.time.txt"
    command = [
        "/usr/bin/time",
        "-v",
        "-o",
        str(time_report),
        str(python),
        str(script),
        "worker",
        "--mode",
        mode,
    ]
    if input_path is not None:
        command.extend(["--input", str(input_path)])
    if output_path is not None:
        command.extend(["--output", str(output_path)])
    result = run_capture(command, cwd=script.parent, timeout=120)
    write_process(artifact_root, name, result)
    if result.returncode != 0:
        raise ExperimentError(f"{name}: RSS worker failed")
    report = time_report.read_text(encoding="utf-8", errors="replace")
    rss_match = MAX_RSS_RE.search(report)
    elapsed_match = ELAPSED_RE.search(report)
    if rss_match is None or elapsed_match is None:
        raise ExperimentError(f"{name}: GNU time report is incomplete")
    return {
        "maximum_rss_kib": int(rss_match.group(1)),
        "elapsed_seconds": parse_elapsed(elapsed_match.group(1)),
        "stdout_sha256": sha256_bytes(result.stdout),
        "stderr_sha256": sha256_bytes(result.stderr),
    }


def run_case(
    *,
    case: Case,
    repo: Path,
    binary: Path,
    checker: Path,
    artifact_root: Path,
    repetitions: int,
    noop_rss_kib: int,
) -> dict[str, Any]:
    case_root = artifact_root / "cases" / case.name
    case_root.mkdir(parents=True)
    proof = case_root / "original.solution"
    prover = run_capture(
        [
            str(binary),
            "--auto",
            "--tstp-out",
            "--proof-object=1",
            f"--cpu-limit={case.cpu_seconds}",
            str(case.problem),
        ],
        cwd=repo,
        timeout=case.wall_seconds,
    )
    proof.write_bytes(prover.stdout)
    (case_root / "prover.stderr").write_bytes(prover.stderr)
    write_json(
        case_root / "prover.json",
        {
            "command": prover.command,
            "returncode": prover.returncode,
            "wall_seconds": prover.wall_seconds,
        },
    )
    status = final_szs_status(prover.stdout)
    if prover.returncode != 0 or status != case.expected_status:
        raise ExperimentError(
            f"{case.name}: prover returned {prover.returncode}/{status}"
        )
    original_check = proofcheck(
        checker, case.problem, proof, artifact_root, f"{case.name}-original"
    )

    spool = case_root / "proof.uptl"
    compact_output = case_root / "compact.solution"
    spool_output = case_root / "spooled.solution"
    compact_trace, latency = time_codec(
        proof,
        spool,
        compact_output,
        spool_output,
        repetitions,
    )
    spool.write_bytes(compact_trace)
    compact_stats = proof_trace.encode_path_to_path(proof, spool)
    if spool.read_bytes() != compact_trace:
        raise ExperimentError(f"{case.name}: memory and spool encoders disagree")
    expected_hash = sha256_file(proof)
    compact_hash = sha256_file(compact_output)
    spool_hash = sha256_file(spool_output)
    second_compact, _ = proof_trace.decode_bytes(compact_trace)
    second_spool = case_root / "spooled-second.solution"
    proof_trace.atomic_replay(spool, second_spool)
    hashes = {
        "original": expected_hash,
        "compact": compact_hash,
        "compact_second": sha256_bytes(second_compact),
        "spooled": spool_hash,
        "spooled_second": sha256_file(second_spool),
    }
    if len(set(hashes.values())) != 1:
        raise ExperimentError(f"{case.name}: reconstructed SHA-256 values differ")

    compact_check = proofcheck(
        checker,
        case.problem,
        compact_output,
        artifact_root,
        f"{case.name}-compact",
    )
    spool_check = proofcheck(
        checker,
        case.problem,
        spool_output,
        artifact_root,
        f"{case.name}-spooled",
    )
    formula_count, inferred_count = count_tstp_records(prover.stdout)
    if inferred_count == 0:
        raise ExperimentError(f"{case.name}: proof has no inferred TSTP records")

    python = Path(sys.executable)
    script = Path(proof_trace.__file__).resolve()
    rss_root = case_root / "rss"
    rss_root.mkdir()
    rss = {
        "eager_retain": measure_worker_rss(
            mode="eager-retain",
            input_path=proof,
            output_path=None,
            script=script,
            python=python,
            artifact_root=rss_root,
            name="eager-retain",
        ),
        "compact_retain": measure_worker_rss(
            mode="compact-retain",
            input_path=proof,
            output_path=None,
            script=script,
            python=python,
            artifact_root=rss_root,
            name="compact-retain",
        ),
        "compact_replay": measure_worker_rss(
            mode="compact-replay",
            input_path=spool,
            output_path=None,
            script=script,
            python=python,
            artifact_root=rss_root,
            name="compact-replay",
        ),
        "spooled_replay": measure_worker_rss(
            mode="spooled-replay",
            input_path=spool,
            output_path=rss_root / "spooled-worker.solution",
            script=script,
            python=python,
            artifact_root=rss_root,
            name="spooled-replay",
        ),
    }
    for measurement in rss.values():
        measurement["payload_adjusted_rss_kib"] = (
            measurement["maximum_rss_kib"] - noop_rss_kib
        )

    recovery = failure_recovery(compact_trace, case_root, script, python)
    if recovery["interruption"]["complete_replay_sha256"] != expected_hash:
        raise ExperimentError(f"{case.name}: recovery replay changed proof bytes")

    eager_bytes = proof.stat().st_size
    compact_bytes = spool.stat().st_size
    latency_limit = max(0.1, prover.wall_seconds * 0.25)
    maximum_replay = max(
        latency["compact_replay"]["median_seconds"],
        latency["spool_replay"]["median_seconds"],
    )
    return {
        "name": case.name,
        "problem": str(case.problem),
        "problem_sha256": sha256_file(case.problem),
        "prover": {
            "returncode": prover.returncode,
            "status": status,
            "wall_seconds": prover.wall_seconds,
            "stderr_sha256": sha256_bytes(prover.stderr),
        },
        "records": {
            "tstp_formula_count": formula_count,
            "inferred_record_count": inferred_count,
        },
        "storage": {
            "eager_bytes": eager_bytes,
            "compact_bytes": compact_bytes,
            "spool_bytes": compact_bytes,
            "compact_ratio": compact_bytes / eager_bytes,
            "eager_bytes_per_inferred_record": eager_bytes / inferred_count,
            "compact_bytes_per_inferred_record": compact_bytes / inferred_count,
            "spool_bytes_per_inferred_record": compact_bytes / inferred_count,
            "frame_count": compact_stats.frame_count,
        },
        "hashes": hashes,
        "latency": latency,
        "latency_gate": {
            "limit_seconds": latency_limit,
            "maximum_replay_median_seconds": maximum_replay,
            "passed": maximum_replay <= latency_limit,
        },
        "proofcheck": {
            "original": original_check,
            "compact": compact_check,
            "spooled": spool_check,
        },
        "rss": rss,
        "failure_recovery": recovery,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--held-out-root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-snapshot-sha256", required=True)
    parser.add_argument("--repetitions", type=int, default=25)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    if sys.platform != "linux":
        raise ExperimentError("the experiment must run on native Linux")
    if arguments.repetitions != 25:
        raise ExperimentError("the frozen experiment requires 25 repetitions")
    repo = arguments.repo_root.resolve()
    artifact_root = arguments.artifact_root.resolve()
    binary = arguments.umlaut.resolve()
    checker = arguments.proofcheck.resolve()
    held_out = arguments.held_out_root.resolve()
    fixture_root = (
        repo
        / "experiments/2026-07-27-004-soundness-validation-gates/fixtures"
    )
    if not (repo / "Cargo.toml").is_file() or not binary.is_file():
        raise ExperimentError("repository or release Umlaut binary is missing")
    if not checker.is_file() or sha256_file(checker) != PROOFCHECK_SHA256:
        raise ExperimentError("pinned ProofCheck binary hash mismatch")
    if artifact_root.exists():
        raise ExperimentError(f"artifact root already exists: {artifact_root}")
    artifact_root.mkdir(parents=True)

    controller = run_capture(
        [
            sys.executable,
            str(Path(proof_trace.__file__).with_name("test_proof_trace.py")),
        ],
        cwd=repo,
        timeout=120,
    )
    write_process(artifact_root, "controller-tests", controller)
    if controller.returncode != 0:
        raise ExperimentError("Ubuntu controller tests failed")

    self_certification = run_capture(
        [str(checker), "-self-certify"],
        cwd=checker.parent,
        timeout=300,
    )
    write_process(artifact_root, "proofcheck-self-certify", self_certification)
    certification_text = self_certification.stdout + self_certification.stderr
    if (
        self_certification.returncode != 0
        or b"Tests: 117 run, 117 passed, 0 failed" not in certification_text
    ):
        raise ExperimentError("ProofCheck self-certification failed")

    rss_root = artifact_root / "rss-control"
    rss_root.mkdir()
    noop = measure_worker_rss(
        mode="noop",
        input_path=None,
        output_path=None,
        script=Path(proof_trace.__file__).resolve(),
        python=Path(sys.executable),
        artifact_root=rss_root,
        name="noop",
    )

    cases = (
        Case(
            "fof-theorem",
            fixture_root / "fof_theorem.p",
            30,
            120,
            "theorem",
        ),
        Case(
            "cnf-unsatisfiable",
            fixture_root / "cnf_unsatisfiable.p",
            30,
            120,
            "unsatisfiable",
        ),
        Case(
            "held-out-col003-19",
            held_out / "UEQ/COL003-19.p",
            20,
            120,
            "unsatisfiable",
        ),
        Case(
            "held-out-syn846-1",
            held_out / "EPU/SYN846-1.p",
            20,
            120,
            "unsatisfiable",
        ),
    )
    for case in cases:
        if not case.problem.is_file():
            raise ExperimentError(f"missing frozen problem: {case.problem}")

    case_results = [
        run_case(
            case=case,
            repo=repo,
            binary=binary,
            checker=checker,
            artifact_root=artifact_root,
            repetitions=arguments.repetitions,
            noop_rss_kib=noop["maximum_rss_kib"],
        )
        for case in cases
    ]
    eager_total = sum(case["storage"]["eager_bytes"] for case in case_results)
    compact_total = sum(case["storage"]["compact_bytes"] for case in case_results)
    largest = max(case_results, key=lambda case: case["storage"]["eager_bytes"])
    size_gate = compact_total <= eager_total * 0.70
    latency_gate = all(case["latency_gate"]["passed"] for case in case_results)
    spool_rss_gate = (
        largest["rss"]["spooled_replay"]["payload_adjusted_rss_kib"]
        <= largest["rss"]["eager_retain"]["payload_adjusted_rss_kib"]
    )
    output_viable = size_gate and latency_gate and spool_rss_gate

    report = {
        "schema": "umlaut.compact-proof-trace.v1",
        "source_commit": arguments.source_commit,
        "source_snapshot_sha256": arguments.source_snapshot_sha256,
        "umlaut_sha256": sha256_file(binary),
        "proofcheck": {
            "version": "1.0",
            "executable_sha256": sha256_file(checker),
            "self_certification_returncode": self_certification.returncode,
            "self_certification_sha256": sha256_bytes(certification_text),
        },
        "host": {
            "platform": platform.platform(),
            "python": sys.version,
            "uname": list(platform.uname()),
        },
        "codec": {
            "magic_hex": proof_trace.MAGIC.hex(),
            "target_frame_bytes": proof_trace.TARGET_FRAME_BYTES,
            "maximum_frame_bytes": proof_trace.MAX_FRAME_BYTES,
            "repetitions": arguments.repetitions,
            "controller_tests": 7,
        },
        "rss_no_payload_control": noop,
        "cases": case_results,
        "aggregate": {
            "eager_bytes": eager_total,
            "compact_bytes": compact_total,
            "spool_bytes": compact_total,
            "compact_ratio": compact_total / eager_total,
            "size_gate": size_gate,
            "latency_gate": latency_gate,
            "largest_case": largest["name"],
            "spool_rss_gate": spool_rss_gate,
            "output_log_technically_viable": output_viable,
        },
        "production_search_baseline": {
            "workload": "LUSK6",
            "massif_total_peak_bytes": 197_700_288,
            "massif_useful_heap_bytes": 186_313_522,
            "rewrite_derivation_useful_heap_bytes": 35_000_832,
            "rewrite_derivation_useful_heap_fraction": 0.1879,
            "derivation_entry_bytes": 32,
            "integrated_candidate": False,
            "reason": (
                "the output codec retains exact rendered bytes but cannot release "
                "live semantic derivation parents or archived clause/formula bodies"
            ),
        },
        "decision": {
            "output_log": "viable" if output_viable else "rejected",
            "production_integration": "rejected",
            "production_source_changed": False,
        },
    }
    write_json(artifact_root / "report.json", report)
    print(
        json.dumps(
            {
                "aggregate": report["aggregate"],
                "decision": report["decision"],
                "report": str(artifact_root / "report.json"),
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    try:
        main()
    except (
        ExperimentError,
        OSError,
        proof_trace.TraceFormatError,
        subprocess.TimeoutExpired,
    ) as error:
        print(f"run_experiment.py: {error}", file=sys.stderr)
        raise SystemExit(2) from error
