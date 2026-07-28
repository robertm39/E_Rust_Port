#!/usr/bin/env python3
"""Profile current term, clause, and index memory behavior on Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import statistics
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


@dataclass(frozen=True)
class Workload:
    name: str
    relative_problem: str
    options: tuple[str, ...]


NATIVE_WORKLOADS = (
    Workload(
        "socrates_solve",
        "eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p",
        ("--auto", "--silent", "--cpu-limit=60"),
    ),
    Workload(
        "syn190_solve",
        "eprover/EXAMPLE_PROBLEMS/TPTP/SYN190-1.p",
        ("--output-level=1", "--processed-clauses-limit=10000"),
    ),
    Workload(
        "lcl365_limit",
        "eprover/EXAMPLE_PROBLEMS/TPTP/LCL365-1.p",
        ("--output-level=1", "--processed-clauses-limit=20000"),
    ),
    Workload(
        "swv851_limit",
        "eprover/EXAMPLE_PROBLEMS/TPTP/SWV851-1.p",
        ("--output-level=1", "--processed-clauses-limit=20000"),
    ),
    Workload(
        "lusk6_solve",
        "eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop",
        (
            "--auto",
            "--silent",
            "--cpu-limit=600",
            "--memory-limit=2048",
            "--detsort-rw",
            "--detsort-new",
        ),
    ),
)

MASSIF_WORKLOADS = (
    next(workload for workload in NATIVE_WORKLOADS if workload.name == "lcl365_limit"),
    next(workload for workload in NATIVE_WORKLOADS if workload.name == "lusk6_solve"),
)

CACHEGRIND_WORKLOADS = (
    Workload(
        "syn190_first_1000",
        "eprover/EXAMPLE_PROBLEMS/TPTP/SYN190-1.p",
        ("--output-level=1", "--processed-clauses-limit=1000"),
    ),
    Workload(
        "lcl365_first_2000",
        "eprover/EXAMPLE_PROBLEMS/TPTP/LCL365-1.p",
        ("--output-level=1", "--processed-clauses-limit=2000"),
    ),
)

GC_WORKLOAD = next(
    workload for workload in NATIVE_WORKLOADS if workload.name == "socrates_solve"
)

LAYOUT_TESTS = (
    "term_links_retain_compact_single_threaded_mutation_boundary",
    "clause_keeps_nullable_derivation_owner_out_of_line",
    "object_and_tree_link_slots_preserve_c_cell_shape",
    "traversal_frame_keeps_two_state_cursor_compact",
    "eval_index_tree_preserves_set_order_removal_and_slot_reuse",
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_captured(
    command: Sequence[str],
    *,
    cwd: Path,
    stdout_path: Path,
    stderr_path: Path,
    timeout: int,
) -> subprocess.CompletedProcess[bytes]:
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        capture_output=True,
        timeout=timeout,
    )
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    return completed


def prover_command(
    binary: Path,
    repo: Path,
    workload: Workload,
    *,
    telemetry_path: Path | None = None,
) -> list[str]:
    command = [str(binary), *workload.options]
    if telemetry_path is not None:
        command.append(f"--search-telemetry={telemetry_path}")
    command.append(str(repo / workload.relative_problem))
    return command


def parse_time_verbose(path: Path) -> dict[str, float | int | str]:
    fields: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if ":" not in line:
            continue
        key, value = line.strip().split(":", 1)
        fields[key] = value.strip()
    required = (
        "User time (seconds)",
        "System time (seconds)",
        "Maximum resident set size (kbytes)",
    )
    missing = [name for name in required if name not in fields]
    if missing:
        raise RuntimeError(f"GNU time output lacks {missing}: {path}")
    user = float(fields["User time (seconds)"])
    system = float(fields["System time (seconds)"])
    return {
        "user_seconds": user,
        "system_seconds": system,
        "cpu_seconds": user + system,
        "max_rss_kib": int(fields["Maximum resident set size (kbytes)"]),
        "elapsed_text": fields.get("Elapsed (wall clock) time (h:mm:ss or m:ss)", ""),
    }


def run_native(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
    repetitions: int,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    raw_runs: list[dict[str, Any]] = []
    summaries: dict[str, Any] = {}
    native_dir = artifact_dir / "native"
    native_dir.mkdir()

    for workload in NATIVE_WORKLOADS:
        print(f"[native] {workload.name}", flush=True)
        warmup = subprocess.run(
            prover_command(binary, repo, workload),
            cwd=repo,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=180,
        )
        runs: list[dict[str, Any]] = []
        for repetition in range(repetitions):
            run_id = f"{workload.name}-{repetition}"
            telemetry_path = native_dir / f"{run_id}.telemetry.json"
            time_path = native_dir / f"{run_id}.time.txt"
            stdout_path = native_dir / f"{run_id}.stdout"
            stderr_path = native_dir / f"{run_id}.stderr"
            command = [
                "/usr/bin/time",
                "-v",
                "-o",
                str(time_path),
                *prover_command(
                    binary,
                    repo,
                    workload,
                    telemetry_path=telemetry_path,
                ),
            ]
            completed = run_captured(
                command,
                cwd=repo,
                stdout_path=stdout_path,
                stderr_path=stderr_path,
                timeout=180,
            )
            if not telemetry_path.is_file():
                raise RuntimeError(f"{run_id} did not write search telemetry")
            telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
            if telemetry["outcome"]["exit_status"] != completed.returncode:
                raise RuntimeError(f"{run_id} telemetry/process status mismatch")
            timing = parse_time_verbose(time_path)
            run = {
                "run_id": run_id,
                "workload": workload.name,
                "returncode": completed.returncode,
                "timing": timing,
                "stdout_sha256": sha256_file(stdout_path),
                "stderr_sha256": sha256_file(stderr_path),
                "telemetry": telemetry,
            }
            runs.append(run)
            raw_runs.append(run)

        statuses = {run["returncode"] for run in runs}
        stdout_hashes = {run["stdout_sha256"] for run in runs}
        stderr_hashes = {run["stderr_sha256"] for run in runs}
        if len(statuses) != 1 or len(stdout_hashes) != 1 or len(stderr_hashes) != 1:
            raise RuntimeError(f"{workload.name} native behavior varied across repetitions")
        summaries[workload.name] = {
            "warmup_returncode": warmup.returncode,
            "returncode": runs[0]["returncode"],
            "stdout_sha256": runs[0]["stdout_sha256"],
            "stderr_sha256": runs[0]["stderr_sha256"],
            "median_cpu_seconds": statistics.median(
                run["timing"]["cpu_seconds"] for run in runs
            ),
            "median_max_rss_kib": statistics.median(
                run["timing"]["max_rss_kib"] for run in runs
            ),
            "min_max_rss_kib": min(run["timing"]["max_rss_kib"] for run in runs),
            "max_max_rss_kib": max(run["timing"]["max_rss_kib"] for run in runs),
            "telemetry": runs[-1]["telemetry"],
        }
    return raw_runs, summaries


def parse_massif(path: Path) -> dict[str, Any]:
    lines = path.read_text(encoding="utf-8").splitlines()
    snapshots: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    for index, line in enumerate(lines):
        if line.startswith("snapshot="):
            if current is not None:
                snapshots.append(current)
            current = {"snapshot": int(line.split("=", 1)[1]), "line_index": index}
        elif current is not None and "=" in line:
            key, value = line.split("=", 1)
            if key in {"time", "mem_heap_B", "mem_heap_extra_B", "mem_stacks_B"}:
                current[key] = int(value)
            elif key == "heap_tree":
                current[key] = value
    if current is not None:
        snapshots.append(current)
    if not snapshots:
        raise RuntimeError(f"no Massif snapshots found in {path}")
    peak = max(
        snapshots,
        key=lambda snapshot: snapshot.get("mem_heap_B", 0)
        + snapshot.get("mem_heap_extra_B", 0)
        + snapshot.get("mem_stacks_B", 0),
    )
    start = int(peak["line_index"])
    end = next(
        (
            index
            for index in range(start + 1, len(lines))
            if lines[index].startswith("snapshot=")
        ),
        len(lines),
    )
    tree_lines = lines[start:end]
    node_pattern = re.compile(r"^(?P<indent> *)n\d+: (?P<bytes>\d+) (?P<label>.*)$")
    root_children: list[dict[str, Any]] = []
    significant_tree: list[str] = []
    heap_bytes = int(peak.get("mem_heap_B", 0))
    for line in tree_lines:
        match = node_pattern.match(line)
        if match is None:
            continue
        allocation_bytes = int(match.group("bytes"))
        depth = len(match.group("indent"))
        percent = 100.0 * allocation_bytes / heap_bytes if heap_bytes else 0.0
        if depth == 1:
            root_children.append(
                {
                    "bytes": allocation_bytes,
                    "percent_of_useful_heap": percent,
                    "label": match.group("label"),
                }
            )
        if percent >= 1.0:
            significant_tree.append(line)
    return {
        "peak_snapshot": peak["snapshot"],
        "peak_useful_heap_bytes": peak.get("mem_heap_B", 0),
        "peak_allocator_extra_bytes": peak.get("mem_heap_extra_B", 0),
        "peak_stack_bytes": peak.get("mem_stacks_B", 0),
        "peak_total_bytes": peak.get("mem_heap_B", 0)
        + peak.get("mem_heap_extra_B", 0)
        + peak.get("mem_stacks_B", 0),
        "root_allocation_children": root_children,
        "significant_peak_tree": significant_tree,
    }


def run_massif(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
) -> dict[str, Any]:
    results: dict[str, Any] = {}
    massif_dir = artifact_dir / "massif"
    massif_dir.mkdir()
    for workload in MASSIF_WORKLOADS:
        print(f"[massif] {workload.name}", flush=True)
        output_path = massif_dir / f"{workload.name}.massif"
        log_path = massif_dir / f"{workload.name}.log"
        completed = run_captured(
            [
                "valgrind",
                "--tool=massif",
                "--time-unit=B",
                "--stacks=yes",
                "--detailed-freq=1",
                "--max-snapshots=100",
                "--threshold=0.1",
                f"--log-file={log_path}",
                f"--massif-out-file={output_path}",
                *prover_command(binary, repo, workload),
            ],
            cwd=repo,
            stdout_path=massif_dir / f"{workload.name}.stdout",
            stderr_path=massif_dir / f"{workload.name}.stderr",
            timeout=1200,
        )
        if not output_path.is_file():
            raise RuntimeError(f"Massif did not write {output_path}")
        printed = subprocess.run(
            ["ms_print", "--threshold=0.1", str(output_path)],
            cwd=repo,
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
        (massif_dir / f"{workload.name}.ms-print.txt").write_text(
            printed.stdout, encoding="utf-8"
        )
        results[workload.name] = {
            "returncode": completed.returncode,
            "stdout_sha256": sha256_file(massif_dir / f"{workload.name}.stdout"),
            "stderr_sha256": sha256_file(massif_dir / f"{workload.name}.stderr"),
            **parse_massif(output_path),
        }
    return results


def parse_valgrind_summary(path: Path) -> dict[str, int]:
    events: list[str] | None = None
    summary: list[int] | None = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("events: "):
            events = line.split()[1:]
        elif line.startswith("summary: "):
            summary = [int(value) for value in line.split()[1:]]
    if events is None or summary is None or len(events) != len(summary):
        raise RuntimeError(f"invalid Valgrind event summary in {path}")
    return dict(zip(events, summary, strict=True))


def cache_rates(events: dict[str, int]) -> dict[str, float]:
    instruction_refs = events["Ir"]
    data_refs = events["Dr"] + events["Dw"]
    return {
        "l1_instruction_miss_percent": 100.0 * events["I1mr"] / instruction_refs,
        "last_level_instruction_miss_percent": 100.0 * events["ILmr"] / instruction_refs,
        "l1_data_miss_percent": 100.0
        * (events["D1mr"] + events["D1mw"])
        / data_refs,
        "last_level_data_miss_percent": 100.0
        * (events["DLmr"] + events["DLmw"])
        / data_refs,
    }


def run_cachegrind(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
) -> dict[str, Any]:
    results: dict[str, Any] = {}
    cache_dir = artifact_dir / "cachegrind"
    cache_dir.mkdir()
    for workload in CACHEGRIND_WORKLOADS:
        print(f"[cachegrind] {workload.name}", flush=True)
        output_path = cache_dir / f"{workload.name}.out"
        log_path = cache_dir / f"{workload.name}.log"
        completed = run_captured(
            [
                "valgrind",
                "--tool=cachegrind",
                "--cache-sim=yes",
                "--branch-sim=yes",
                f"--log-file={log_path}",
                f"--cachegrind-out-file={output_path}",
                *prover_command(binary, repo, workload),
            ],
            cwd=repo,
            stdout_path=cache_dir / f"{workload.name}.stdout",
            stderr_path=cache_dir / f"{workload.name}.stderr",
            timeout=1200,
        )
        events = parse_valgrind_summary(output_path)
        annotated = subprocess.run(
            [
                "cg_annotate",
                "--auto=yes",
                "--show=Ir,I1mr,ILmr,Dr,D1mr,DLmr,Dw,D1mw,DLmw",
                "--threshold=0.5",
                str(output_path),
            ],
            cwd=repo,
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
        (cache_dir / f"{workload.name}.annotated.txt").write_text(
            annotated.stdout, encoding="utf-8"
        )
        results[workload.name] = {
            "returncode": completed.returncode,
            "events": events,
            "rates": cache_rates(events),
        }
    return results


def run_gc_callgrind(
    *,
    repo: Path,
    binary: Path,
    artifact_dir: Path,
) -> dict[str, Any]:
    callgrind_dir = artifact_dir / "callgrind"
    callgrind_dir.mkdir()
    print(f"[callgrind] {GC_WORKLOAD.name} GC slice", flush=True)
    output_path = callgrind_dir / "socrates-gc.out"
    completed = run_captured(
        [
            "valgrind",
            "--tool=callgrind",
            f"--log-file={callgrind_dir / 'socrates-gc.log'}",
            f"--callgrind-out-file={output_path}",
            *prover_command(binary, repo, GC_WORKLOAD),
        ],
        cwd=repo,
        stdout_path=callgrind_dir / "socrates-gc.stdout",
        stderr_path=callgrind_dir / "socrates-gc.stderr",
        timeout=600,
    )
    events = parse_valgrind_summary(output_path)
    annotated = subprocess.run(
        ["callgrind_annotate", "--inclusive=yes", "--threshold=0", str(output_path)],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
        timeout=120,
    )
    annotation_path = callgrind_dir / "socrates-gc.annotated.txt"
    annotation_path.write_text(annotated.stdout, encoding="utf-8")
    gc_fragments = (
        "gc_sweep",
        "collect_term_garbage",
        "gc_mark",
        "garbage_coll",
        "collect_matching",
    )
    gc_lines = [
        line
        for line in annotated.stdout.splitlines()
        if any(fragment in line for fragment in gc_fragments)
    ]
    return {
        "returncode": completed.returncode,
        "events": events,
        "gc_annotation_lines": gc_lines,
    }


def run_layout_tests(repo: Path, artifact_dir: Path) -> dict[str, Any]:
    layout_dir = artifact_dir / "layout-tests"
    layout_dir.mkdir()
    results: dict[str, Any] = {}
    for test_name in LAYOUT_TESTS:
        print(f"[layout] {test_name}", flush=True)
        completed = run_captured(
            [
                str(Path.home() / ".cargo" / "bin" / "cargo"),
                "test",
                "--locked",
                "--lib",
                test_name,
            ],
            cwd=repo,
            stdout_path=layout_dir / f"{test_name}.stdout",
            stderr_path=layout_dir / f"{test_name}.stderr",
            timeout=600,
        )
        if completed.returncode != 0:
            raise RuntimeError(f"layout test failed: {test_name}")
        results[test_name] = {"returncode": completed.returncode}
    return results


def command_text(command: Sequence[str], cwd: Path) -> str:
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    return completed.stdout.strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--binary", type=Path, default=Path("target/release/umlaut"))
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-snapshot-sha256", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if sys.platform != "linux":
        raise RuntimeError("this profile must run on the Linux authority")
    if args.repetitions < 3:
        raise RuntimeError("at least three native repetitions are required")
    for command in (
        "valgrind",
        "ms_print",
        "cg_annotate",
        "callgrind_annotate",
        "/usr/bin/time",
    ):
        if shutil.which(command) is None:
            raise RuntimeError(f"required profiling command is missing: {command}")

    repo = args.repo.resolve()
    binary = args.binary if args.binary.is_absolute() else repo / args.binary
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    for workload in {*NATIVE_WORKLOADS, *CACHEGRIND_WORKLOADS}:
        problem = repo / workload.relative_problem
        if not problem.is_file():
            raise RuntimeError(f"missing profiling workload: {problem}")

    build_dir = artifact_dir / "build"
    build_dir.mkdir()
    print("[build] release umlaut", flush=True)
    build = run_captured(
        [
            str(Path.home() / ".cargo" / "bin" / "cargo"),
            "build",
            "--locked",
            "--release",
            "--bin",
            "umlaut",
        ],
        cwd=repo,
        stdout_path=build_dir / "cargo-build.stdout",
        stderr_path=build_dir / "cargo-build.stderr",
        timeout=1200,
    )
    if build.returncode != 0 or not binary.is_file():
        raise RuntimeError("release build failed")

    layout_tests = run_layout_tests(repo, artifact_dir)
    native_runs, native_summary = run_native(
        repo=repo,
        binary=binary,
        artifact_dir=artifact_dir,
        repetitions=args.repetitions,
    )
    massif = run_massif(repo=repo, binary=binary, artifact_dir=artifact_dir)
    cachegrind = run_cachegrind(repo=repo, binary=binary, artifact_dir=artifact_dir)
    collection = run_gc_callgrind(repo=repo, binary=binary, artifact_dir=artifact_dir)
    raw_runs_path = artifact_dir / "native-runs.json"
    write_json(raw_runs_path, native_runs)

    all_workloads = {*NATIVE_WORKLOADS, *CACHEGRIND_WORKLOADS}
    summary = {
        "schema": "umlaut.memory-representation-profile",
        "schema_version": 1,
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "rustc": command_text(["rustc", "--version"], repo),
            "cargo": command_text(["cargo", "--version"], repo),
            "valgrind": command_text(["valgrind", "--version"], repo),
        },
        "source": {
            "git_commit": args.source_commit,
            "uploaded_snapshot_sha256": args.source_snapshot_sha256,
            "binary_path": str(binary.relative_to(repo)),
            "binary_sha256": sha256_file(binary),
            "profile_script_sha256": sha256_file(Path(__file__).resolve()),
            "problem_sha256": {
                workload.relative_problem: sha256_file(repo / workload.relative_problem)
                for workload in sorted(
                    all_workloads, key=lambda workload: workload.relative_problem
                )
            },
        },
        "layout_tests": layout_tests,
        "native_repetitions": args.repetitions,
        "native": native_summary,
        "massif": massif,
        "cachegrind": cachegrind,
        "collection_callgrind": collection,
    }
    write_json(artifact_dir / "summary.json", summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
