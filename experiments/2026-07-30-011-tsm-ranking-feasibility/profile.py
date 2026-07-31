#!/usr/bin/env python3
"""Run phase-isolated native and Callgrind profiles of proof-derived TSM ranking."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


PRIOR_ARCHIVE_SHA256 = (
    "8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156"
)
KNOWLEDGE_BASE_SHA256 = (
    "838a4f14137344c8d1c0c17a0503fb8fc0a136dbcb206b35f6927c898fe7d13f"
)
PROBLEM_SHA256 = (
    "b0e8c769ae659ad7d89f632be19849c9bcdb0c9a34e72380466a8c7eaa556111"
)
WEIGHTED_VALIDATION_OCCURRENCES = 150
NATIVE_REPETITIONS = 11
SEARCH_PROCESSED_LIMIT = 128


class ExperimentError(RuntimeError):
    """Raised when frozen input or measurement invariants fail."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_tree(path: Path) -> str:
    digest = hashlib.sha256()
    for entry in sorted(candidate for candidate in path.rglob("*") if candidate.is_file()):
        digest.update(entry.relative_to(path).as_posix().encode())
        digest.update(b"\0")
        digest.update(sha256_file(entry).encode())
        digest.update(b"\n")
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def child_cpu_seconds() -> float:
    import resource

    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def measured_run(
    command: list[str], timeout: int
) -> tuple[subprocess.CompletedProcess[bytes], float, float]:
    cpu_before = child_cpu_seconds()
    started = time.monotonic()
    completed = subprocess.run(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    return completed, time.monotonic() - started, child_cpu_seconds() - cpu_before


def classifier_command(binary: Path, input_path: Path) -> list[str]:
    return [
        str(binary),
        "-l",
        "1",
        "-i",
        "IndexIdentity",
        "-d",
        "100000",
        "-t",
        "Flat",
        str(input_path),
    ]


def control_search_args() -> list[str]:
    return [
        "--define-weight-function=TSMControl=Clauseweight(ConstPrio,1,1,2)",
        (
            "--define-heuristic=TSMControlSearch="
            "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
            "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
            "5*TSMControl,1*FIFOWeight(PreferWatchlist))"
        ),
        "--expert-heuristic=TSMControlSearch",
    ]


def learned_search_args(knowledge_base: Path) -> list[str]:
    return [
        (
            "--define-weight-function=TSMLearned="
            "TSMWeight(ConstPrio,1,1,2,flat,"
            f"{knowledge_base.as_posix()},100000,1.0,1.0,Flat,"
            "IndexIdentity,100000,-20,20,-2,-1,0,2)"
        ),
        (
            "--define-heuristic=TSMLearnedSearch="
            "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
            "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
            "5*TSMLearned,1*FIFOWeight(PreferWatchlist))"
        ),
        "--expert-heuristic=TSMLearnedSearch",
    ]


def search_command(
    binary: Path,
    strategy: str,
    knowledge_base: Path,
    problem: Path,
    telemetry: Path,
) -> list[str]:
    treatment = (
        control_search_args()
        if strategy == "control"
        else learned_search_args(knowledge_base)
    )
    return [
        str(binary),
        *treatment,
        "--term-ordering=KBO6",
        "--forward-demod-level=2",
        f"--processed-clauses-limit={SEARCH_PROCESSED_LIMIT}",
        "--memory-limit=1536",
        f"--search-telemetry={telemetry}",
        str(problem),
    ]


def make_empty_test(full_input: Path, empty_input: Path) -> None:
    contents = full_input.read_text(encoding="utf-8")
    marker = "\nTest:\n"
    training, separator, _test = contents.partition(marker)
    if not separator or not training.startswith("Training:\n"):
        raise ExperimentError("validation classifier input has an unexpected shape")
    empty_input.write_text(
        training + marker + ".\n",
        encoding="utf-8",
        newline="\n",
    )
    rebuilt = empty_input.read_text(encoding="utf-8")
    if rebuilt.partition(marker)[0] != training:
        raise ExperimentError("empty-test input changed the training prefix")


def require_classifier_success(
    label: str, completed: subprocess.CompletedProcess[bytes]
) -> None:
    if completed.returncode != 0:
        raise ExperimentError(
            f"{label} returned {completed.returncode}: "
            f"{completed.stderr.decode(errors='replace')}"
        )
    if completed.stderr:
        raise ExperimentError(f"{label} emitted stderr")


def telemetry_signature(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "outcome": record["outcome"],
        "input_funnel": record["input_funnel"],
        "search_funnel": record["search_funnel"],
        "inferences": record["inferences"],
        "simplification": record["simplification"],
        "proof": record["proof"],
    }


def require_search_result(
    label: str,
    completed: subprocess.CompletedProcess[bytes],
    telemetry_path: Path,
) -> dict[str, Any]:
    stdout = completed.stdout.decode(errors="replace")
    if "SZS status" not in stdout:
        raise ExperimentError(f"{label} did not emit an SZS status")
    if completed.stderr:
        raise ExperimentError(f"{label} emitted stderr")
    if not telemetry_path.is_file():
        raise ExperimentError(f"{label} did not emit telemetry")
    record = json.loads(telemetry_path.read_text(encoding="utf-8"))
    if record.get("schema") != "umlaut.search-telemetry":
        raise ExperimentError(f"{label} telemetry has the wrong schema")
    if record.get("record_kind", "final") != "final":
        raise ExperimentError(f"{label} telemetry is not final")
    if record["outcome"]["reason"] not in {"processed_limit", "step_limit"}:
        raise ExperimentError(f"{label} did not reach the processed-clause stop")
    return record


def run_native_classifier(
    binary: Path,
    empty_input: Path,
    full_input: Path,
    output_root: Path,
) -> dict[str, Any]:
    for name, input_path in (("empty", empty_input), ("full", full_input)):
        completed, _wall, _cpu = measured_run(
            classifier_command(binary, input_path), timeout=120
        )
        require_classifier_success(f"classifier {name} warm-up", completed)
        (output_root / f"classifier-{name}.stdout").write_bytes(completed.stdout)

    timings: dict[str, list[dict[str, Any]]] = {"empty": [], "full": []}
    expected_hashes: dict[str, str] = {}
    for repetition in range(1, NATIVE_REPETITIONS + 1):
        order = ("full", "empty") if repetition % 2 else ("empty", "full")
        for name in order:
            input_path = full_input if name == "full" else empty_input
            completed, wall, cpu = measured_run(
                classifier_command(binary, input_path), timeout=120
            )
            require_classifier_success(
                f"classifier {name} repetition {repetition}", completed
            )
            output_hash = sha256_bytes(completed.stdout)
            expected = expected_hashes.setdefault(name, output_hash)
            if output_hash != expected:
                raise ExperimentError(f"classifier {name} output is not stable")
            timings[name].append(
                {
                    "repetition": repetition,
                    "cpu_seconds": cpu,
                    "wall_seconds": wall,
                    "stdout_sha256": output_hash,
                }
            )
    paired_cpu = [
        timings["full"][index]["cpu_seconds"] - timings["empty"][index]["cpu_seconds"]
        for index in range(NATIVE_REPETITIONS)
    ]
    return {
        "repetitions": NATIVE_REPETITIONS,
        "timings": timings,
        "paired_full_minus_empty_cpu_seconds": paired_cpu,
        "median_full_minus_empty_cpu_seconds": statistics.median(paired_cpu),
        "median_microseconds_per_weighted_occurrence": (
            statistics.median(paired_cpu)
            * 1_000_000
            / WEIGHTED_VALIDATION_OCCURRENCES
        ),
    }


def run_native_search(
    binary: Path,
    knowledge_base: Path,
    problem: Path,
    output_root: Path,
) -> dict[str, Any]:
    timings: dict[str, list[dict[str, Any]]] = {"control": [], "learned": []}
    signatures: dict[str, dict[str, Any]] = {}
    repetitions = 7
    for repetition in range(1, repetitions + 1):
        order = ("learned", "control") if repetition % 2 else ("control", "learned")
        for strategy in order:
            telemetry = output_root / f"native-{strategy}-{repetition}.telemetry.json"
            command = search_command(
                binary, strategy, knowledge_base, problem, telemetry
            )
            completed, wall, cpu = measured_run(command, timeout=180)
            record = require_search_result(
                f"native {strategy} repetition {repetition}",
                completed,
                telemetry,
            )
            signature = telemetry_signature(record)
            expected = signatures.setdefault(strategy, signature)
            if signature != expected:
                raise ExperimentError(f"{strategy} search work changed across repetitions")
            timings[strategy].append(
                {
                    "repetition": repetition,
                    "child_cpu_seconds": cpu,
                    "telemetry_cpu_seconds": record["resources"][
                        "total_cpu_seconds"
                    ],
                    "wall_seconds": wall,
                    "stdout_sha256": sha256_bytes(completed.stdout),
                    "return_code": completed.returncode,
                }
            )
    if signatures["control"] != signatures["learned"]:
        raise ExperimentError("control and learned diagnostic search work differ")
    control_cpu = statistics.median(
        item["telemetry_cpu_seconds"] for item in timings["control"]
    )
    learned_cpu = statistics.median(
        item["telemetry_cpu_seconds"] for item in timings["learned"]
    )
    return {
        "repetitions": repetitions,
        "timings": timings,
        "work_signature": signatures["control"],
        "median_control_cpu_seconds": control_cpu,
        "median_learned_cpu_seconds": learned_cpu,
        "learned_control_cpu_ratio": learned_cpu / control_cpu,
    }


def run_callgrind(
    label: str,
    command: list[str],
    output_root: Path,
    timeout: int,
) -> dict[str, Any]:
    profile = output_root / f"callgrind-{label}.out"
    log = output_root / f"callgrind-{label}.log"
    completed = subprocess.run(
        [
            "valgrind",
            "--tool=callgrind",
            "--dump-instr=yes",
            "--collect-jumps=yes",
            f"--callgrind-out-file={profile}",
            f"--log-file={log}",
            *command,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    (output_root / f"callgrind-{label}.stdout").write_bytes(completed.stdout)
    (output_root / f"callgrind-{label}.stderr").write_bytes(completed.stderr)
    if not profile.is_file():
        raise ExperimentError(f"Callgrind {label} did not emit a profile")
    annotation = subprocess.run(
        [
            "callgrind_annotate",
            "--inclusive=yes",
            "--tree=both",
            "--threshold=0.01",
            str(profile),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=180,
        check=False,
    )
    if annotation.returncode != 0:
        raise ExperimentError(f"cannot annotate Callgrind profile {label}")
    (output_root / f"callgrind-{label}.annotated.txt").write_bytes(
        annotation.stdout
    )
    summary = None
    for line in profile.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("summary:"):
            summary = int(line.removeprefix("summary:").strip().split()[0])
    if summary is None:
        raise ExperimentError(f"Callgrind {label} has no instruction summary")
    return {
        "command": command,
        "return_code": completed.returncode,
        "instructions": summary,
        "profile": profile.name,
        "profile_sha256": sha256_file(profile),
        "annotation": f"callgrind-{label}.annotated.txt",
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
        "log_sha256": sha256_file(log),
    }


def run_callgrind_profiles(
    classifier: Path,
    prover: Path,
    empty_input: Path,
    full_input: Path,
    knowledge_base: Path,
    problem: Path,
    output_root: Path,
) -> dict[str, Any]:
    profiles = {
        "startup": run_callgrind(
            "startup", [str(classifier), "--help"], output_root, timeout=300
        ),
        "classifier_empty": run_callgrind(
            "classifier-empty",
            classifier_command(classifier, empty_input),
            output_root,
            timeout=600,
        ),
        "classifier_full": run_callgrind(
            "classifier-full",
            classifier_command(classifier, full_input),
            output_root,
            timeout=600,
        ),
    }
    search_records: dict[str, dict[str, Any]] = {}
    for strategy in ("control", "learned"):
        telemetry = output_root / f"callgrind-{strategy}.telemetry.json"
        command = search_command(
            prover, strategy, knowledge_base, problem, telemetry
        )
        result = run_callgrind(
            f"search-{strategy}", command, output_root, timeout=1800
        )
        stdout_path = output_root / f"callgrind-search-{strategy}.stdout"
        stderr_path = output_root / f"callgrind-search-{strategy}.stderr"
        completed = subprocess.CompletedProcess(
            command,
            result["return_code"],
            stdout_path.read_bytes(),
            stderr_path.read_bytes(),
        )
        search_records[strategy] = require_search_result(
            f"Callgrind {strategy}", completed, telemetry
        )
        profiles[f"search_{strategy}"] = result
    control_signature = telemetry_signature(search_records["control"])
    learned_signature = telemetry_signature(search_records["learned"])
    if control_signature != learned_signature:
        raise ExperimentError("Callgrind control and learned search work differ")
    profiles["search_work_signature"] = control_signature
    return profiles


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--prior-root", type=Path, required=True)
    parser.add_argument("--prior-archive", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument(
        "--mode",
        choices=("native", "callgrind", "all"),
        default="all",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("profiles may run only on Linux")
    source_root = arguments.source_root.resolve()
    prior_root = arguments.prior_root.resolve()
    prior_archive = arguments.prior_archive.resolve()
    output_root = arguments.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=False)

    if sha256_file(prior_archive) != PRIOR_ARCHIVE_SHA256:
        raise ExperimentError("prior archive hash differs from preregistration")
    knowledge_base = prior_root / "E_KNOWLEDGE"
    if sha256_tree(knowledge_base) != KNOWLEDGE_BASE_SHA256:
        raise ExperimentError("knowledge-base tree hash differs from preregistration")
    full_input = prior_root / "classifier-inputs-v4" / "validation.tsm"
    problem = source_root / "problems" / "casc_2025" / "UEQ" / "LCL026-10.p"
    if sha256_file(problem) != PROBLEM_SHA256:
        raise ExperimentError("profile problem hash differs from preregistration")
    classifier = source_root / "target" / "release" / "umlaut-tsm-classify"
    prover = source_root / "target" / "release" / "umlaut"
    for executable in (classifier, prover):
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ExperimentError(f"missing executable: {executable}")

    empty_input = output_root / "validation-empty.tsm"
    make_empty_test(full_input, empty_input)
    metadata: dict[str, Any] = {
        "schema_version": 1,
        "host": {
            "node": platform.node(),
            "platform": platform.platform(),
            "python": platform.python_version(),
        },
        "inputs": {
            "prior_archive_sha256": sha256_file(prior_archive),
            "knowledge_base_sha256": sha256_tree(knowledge_base),
            "full_classifier_input_sha256": sha256_file(full_input),
            "empty_classifier_input_sha256": sha256_file(empty_input),
            "problem_sha256": sha256_file(problem),
            "weighted_validation_occurrences": WEIGHTED_VALIDATION_OCCURRENCES,
        },
        "executables": {
            "classifier_sha256": sha256_file(classifier),
            "prover_sha256": sha256_file(prover),
        },
        "mode": arguments.mode,
    }
    if arguments.mode in {"native", "all"}:
        metadata["native_classifier"] = run_native_classifier(
            classifier, empty_input, full_input, output_root
        )
        metadata["native_search"] = run_native_search(
            prover, knowledge_base, problem, output_root
        )
    if arguments.mode in {"callgrind", "all"}:
        metadata["callgrind"] = run_callgrind_profiles(
            classifier,
            prover,
            empty_input,
            full_input,
            knowledge_base,
            problem,
            output_root,
        )
    write_json(output_root / "summary.json", metadata)
    print(json.dumps(metadata, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
