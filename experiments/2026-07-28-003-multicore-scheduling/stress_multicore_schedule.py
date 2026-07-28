#!/usr/bin/env python3
"""Exercise Umlaut's process-portfolio lifecycle on a Linux runner."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

ADDRESS_LIMIT_LABEL = "Max address space"
RESOURCE_LINE = re.compile(
    r"^% (?:(?:User|System|Total|Preprocessing) time\s+:"
    r"|Maximum resident set size:|Page faults).*?$",
    re.MULTILINE,
)
PID_LINE = re.compile(r"( with pid )\d+( completed)")
PROOF_BLOCK = re.compile(
    r"% SZS output start CNFRefutation.*?% SZS output end CNFRefutation",
    re.DOTALL,
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def process_ids_referencing(marker: str) -> list[int]:
    matches: list[int] = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            command_line = (entry / "cmdline").read_bytes()
        except OSError:
            continue
        if marker.encode() in command_line:
            matches.append(int(entry.name))
    return sorted(matches)


def wait_for_processes(marker: str, parent: subprocess.Popen[bytes], timeout: float) -> list[int]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        process_ids = process_ids_referencing(marker)
        children = [process_id for process_id in process_ids if process_id != parent.pid]
        if children:
            return process_ids
        if parent.poll() is not None:
            raise RuntimeError(
                f"scheduler exited with {parent.returncode} before workers became observable"
            )
        time.sleep(0.002)
    raise RuntimeError(f"scheduler workers were not observable within {timeout:.1f}s")


def wait_for_no_processes(marker: str, timeout: float) -> list[int]:
    deadline = time.monotonic() + timeout
    survivors = process_ids_referencing(marker)
    while survivors and time.monotonic() < deadline:
        time.sleep(0.01)
        survivors = process_ids_referencing(marker)
    return survivors


def process_snapshot(process_id: int) -> dict[str, Any]:
    proc = Path("/proc") / str(process_id)
    status = read_text(proc / "status")
    limits = read_text(proc / "limits")
    try:
        command_line = (proc / "cmdline").read_bytes().replace(b"\0", b" ").decode(
            "utf-8", errors="replace"
        )
    except OSError:
        command_line = ""
    status_values: dict[str, str] = {}
    for line in status.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            status_values[key] = value.strip()
    address_limit = None
    for line in limits.splitlines():
        if line.startswith(ADDRESS_LIMIT_LABEL):
            columns = line.split()
            if len(columns) >= 4:
                address_limit = columns[-3]
            break
    return {
        "pid": process_id,
        "ppid": status_values.get("PPid"),
        "state": status_values.get("State"),
        "vm_rss": status_values.get("VmRSS"),
        "vm_peak": status_values.get("VmPeak"),
        "cpus_allowed_list": status_values.get("Cpus_allowed_list"),
        "max_address_space_bytes": address_limit,
        "command_line": command_line,
    }


def rss_kib(snapshot: dict[str, Any]) -> int:
    value = snapshot.get("vm_rss")
    if not isinstance(value, str):
        return 0
    match = re.fullmatch(r"(\d+)\s+kB", value)
    return int(match.group(1)) if match else 0


def normalize_stdout(stdout: str) -> str:
    normalized = PID_LINE.sub(r"\1<PID>\2", stdout)
    normalized = RESOURCE_LINE.sub("% <RESOURCE>", normalized)
    return normalized


def resource_totals(stdout: str) -> list[float]:
    values: list[float] = []
    for line in stdout.splitlines():
        if not line.startswith("% Total time               : "):
            continue
        value = line.removeprefix("% Total time               : ").removesuffix(" s")
        try:
            values.append(float(value))
        except ValueError:
            continue
    return values


def write_interrupt_problem(path: Path, comment_mebibytes: int) -> None:
    with path.open("wb") as problem:
        problem.write(b"%")
        chunk = b"x" * (64 * 1024)
        for _ in range(comment_mebibytes * 16):
            problem.write(chunk)
        problem.write(
            b"\nfof(interrupt_axiom, axiom, p(a)).\n"
            b"fof(interrupt_goal, conjecture, p(a)).\n"
        )


def write_seeded_problem(path: Path) -> None:
    lines = ["fof(seed, axiom, p0(a))."]
    for index in range(96):
        lines.append(
            f"fof(chain_{index}, axiom, "
            f"(![X]:(p{index}(X)=>p{index + 1}(X))))."
        )
    lines.append("fof(goal, conjecture, p96(a)).")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def base_command(
    executable: Path,
    problem: Path,
    cores: int,
    memory_limit_mb: int,
) -> list[str]:
    return [
        str(executable),
        f"--auto-schedule={cores}",
        f"--memory-limit={memory_limit_mb}",
        "--cpu-limit=30",
        "--resources-info",
        "--tstp-out",
        "--proof-object=1",
        "--output-level=0",
        str(problem),
    ]


def run_lifecycle_case(
    *,
    name: str,
    executable: Path,
    problem: Path,
    cores: int,
    memory_limit_mb: int,
    temp_root: Path,
) -> dict[str, Any]:
    case_temp = temp_root / name
    case_temp.mkdir()
    environment = os.environ.copy()
    environment["TMPDIR"] = str(case_temp)
    before_files = sorted(path.name for path in case_temp.iterdir())
    started = time.monotonic()
    process = subprocess.Popen(
        base_command(executable, problem, cores, memory_limit_mb),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
    )
    observed = wait_for_processes(str(problem), process, 30.0)
    snapshots = [
        process_snapshot(process_id)
        for process_id in observed
        if Path(f"/proc/{process_id}").exists()
    ]

    action_pid = process.pid
    if name == "timeout":
        process.send_signal(signal.SIGALRM)
    elif name == "cancellation":
        process.send_signal(signal.SIGTERM)
    elif name == "worker_crash":
        worker_ids = [process_id for process_id in observed if process_id != process.pid]
        if not worker_ids:
            raise RuntimeError("worker-crash case found no worker to terminate")
        action_pid = worker_ids[0]
        os.kill(action_pid, signal.SIGKILL)
    else:
        raise ValueError(f"unknown lifecycle case {name}")

    try:
        stdout_bytes, stderr_bytes = process.communicate(timeout=45)
    except subprocess.TimeoutExpired:
        process.kill()
        stdout_bytes, stderr_bytes = process.communicate(timeout=10)
        raise RuntimeError(f"{name} case did not terminate within 45 seconds")
    survivors = wait_for_no_processes(str(problem), 5.0)
    after_files = sorted(path.name for path in case_temp.iterdir())
    stdout = stdout_bytes.decode("utf-8", errors="replace")
    stderr = stderr_bytes.decode("utf-8", errors="replace")
    expected_limit = str(memory_limit_mb * 1024 * 1024)
    observed_limits = [
        snapshot["max_address_space_bytes"]
        for snapshot in snapshots
        if snapshot["max_address_space_bytes"] is not None
    ]
    return {
        "name": name,
        "command": base_command(executable, problem, cores, memory_limit_mb),
        "action_pid": action_pid,
        "exit_code": process.returncode,
        "elapsed_seconds": time.monotonic() - started,
        "observed_process_count": len(snapshots),
        "aggregate_observed_rss_kib": sum(rss_kib(snapshot) for snapshot in snapshots),
        "processes": snapshots,
        "expected_address_limit_bytes": expected_limit,
        "all_observed_address_limits_match": bool(observed_limits)
        and all(value == expected_limit for value in observed_limits),
        "surviving_process_ids": survivors,
        "temp_files_before": before_files,
        "temp_files_after": after_files,
        "stdout_sha256": sha256_bytes(stdout_bytes),
        "stderr_sha256": sha256_bytes(stderr_bytes),
        "stdout": stdout,
        "stderr": stderr,
    }


def run_seeded_reproducibility(
    *,
    executable: Path,
    problem: Path,
    cores: int,
    memory_limit_mb: int,
    iterations: int,
    temp_root: Path,
) -> dict[str, Any]:
    command = base_command(executable, problem, cores, memory_limit_mb)
    command[1:1] = [
        "--define-weight-function=seeded_random=RandomWeight(ConstPrio,1000,0,0,11,13,17)",
        "--define-heuristic=SeededRandom=(1*seeded_random)",
        "--expert-heuristic=SeededRandom",
    ]
    runs: list[dict[str, Any]] = []
    for iteration in range(iterations):
        iteration_temp = temp_root / f"seeded-{iteration}"
        iteration_temp.mkdir()
        environment = os.environ.copy()
        environment["TMPDIR"] = str(iteration_temp)
        completed = subprocess.run(
            command,
            capture_output=True,
            check=False,
            timeout=60,
            env=environment,
        )
        stdout = completed.stdout.decode("utf-8", errors="replace")
        proof_match = PROOF_BLOCK.search(stdout)
        normalized = normalize_stdout(stdout)
        totals = resource_totals(stdout)
        runs.append(
            {
                "iteration": iteration,
                "exit_code": completed.returncode,
                "stdout_sha256": sha256_bytes(completed.stdout),
                "stderr_sha256": sha256_bytes(completed.stderr),
                "normalized_stdout_sha256": sha256_bytes(normalized.encode()),
                "proof_sha256": (
                    sha256_bytes(proof_match.group(0).encode()) if proof_match else None
                ),
                "winning_strategies": re.findall(
                    r"^% Result found by (.+)$", stdout, re.MULTILINE
                ),
                "resource_total_seconds": totals,
                "temp_files_after": sorted(path.name for path in iteration_temp.iterdir()),
            }
        )
    normalized_hashes = {run["normalized_stdout_sha256"] for run in runs}
    proof_hashes = {run["proof_sha256"] for run in runs}
    winning_strategies = {
        tuple(run["winning_strategies"])
        for run in runs
    }
    return {
        "command": command,
        "explicit_random_weight_seeds": [11, 13, 17],
        "runs": runs,
        "all_exit_zero": all(run["exit_code"] == 0 for run in runs),
        "normalized_stdout_reproducible": len(normalized_hashes) == 1,
        "proof_reproducible": None not in proof_hashes and len(proof_hashes) == 1,
        "winning_strategies_reproducible": len(winning_strategies) == 1,
        "resource_accounting_present": all(
            len(run["resource_total_seconds"]) >= 2 for run in runs
        ),
        "resource_totals_monotonic": all(
            all(left <= right for left, right in zip(totals, totals[1:]))
            for totals in (run["resource_total_seconds"] for run in runs)
        ),
        "no_temp_files": all(not run["temp_files_after"] for run in runs),
    }


def host_metadata(expected_host_cpus: int) -> dict[str, Any]:
    os_release: dict[str, str] = {}
    for line in read_text(Path("/etc/os-release")).splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            os_release[key] = value.strip('"')
    meminfo = read_text(Path("/proc/meminfo"))
    mem_total_match = re.search(r"^MemTotal:\s+(\d+)\s+kB$", meminfo, re.MULTILINE)
    cpu_count = os.cpu_count()
    return {
        "hostname": platform.node(),
        "platform": platform.platform(),
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "os_release": os_release,
        "logical_cpu_count": cpu_count,
        "expected_logical_cpu_count": expected_host_cpus,
        "cpu_count_matches": cpu_count == expected_host_cpus,
        "mem_total_kib": int(mem_total_match.group(1)) if mem_total_match else None,
        "lscpu": subprocess.run(
            ["lscpu"], capture_output=True, text=True, check=False
        ).stdout,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cores", type=int, default=8)
    parser.add_argument("--expected-host-cpus", type=int, required=True)
    parser.add_argument("--memory-limit-mb", type=int, default=131_072)
    parser.add_argument("--iterations", type=int, default=4)
    parser.add_argument("--comment-mebibytes", type=int, default=2)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    executable = args.umlaut.resolve()
    if not executable.is_file():
        raise SystemExit(f"Umlaut executable does not exist: {executable}")
    if args.cores <= 0 or args.expected_host_cpus <= 0 or args.iterations <= 1:
        raise SystemExit("cores/expected-host-cpus must be positive and iterations must exceed one")

    with tempfile.TemporaryDirectory(prefix="umlaut-multicore-stress-") as directory:
        root = Path(directory)
        interrupt_problem = root / "interrupt-workload.p"
        seeded_problem = root / "seeded-workload.p"
        temp_root = root / "tmp"
        temp_root.mkdir()
        write_interrupt_problem(interrupt_problem, args.comment_mebibytes)
        write_seeded_problem(seeded_problem)
        lifecycle = {
            name: run_lifecycle_case(
                name=name,
                executable=executable,
                problem=interrupt_problem,
                cores=args.cores,
                memory_limit_mb=args.memory_limit_mb,
                temp_root=temp_root,
            )
            for name in ("timeout", "cancellation", "worker_crash")
        }
        reproducibility = run_seeded_reproducibility(
            executable=executable,
            problem=seeded_problem,
            cores=args.cores,
            memory_limit_mb=args.memory_limit_mb,
            iterations=args.iterations,
            temp_root=temp_root,
        )
        metadata = host_metadata(args.expected_host_cpus)
        checks = {
            "host_cpu_count": metadata["cpu_count_matches"],
            "timeout_exit_8": lifecycle["timeout"]["exit_code"] == 8,
            "timeout_no_orphans": not lifecycle["timeout"]["surviving_process_ids"],
            "cancellation_completed": lifecycle["cancellation"]["exit_code"] is not None,
            "cancellation_no_orphans": not lifecycle["cancellation"][
                "surviving_process_ids"
            ],
            "worker_crash_recovered": lifecycle["worker_crash"]["exit_code"] == 0,
            "worker_crash_no_orphans": not lifecycle["worker_crash"][
                "surviving_process_ids"
            ],
            "address_limits": all(
                case["all_observed_address_limits_match"]
                for case in lifecycle.values()
            ),
            "lifecycle_temp_cleanup": all(
                case["temp_files_before"] == case["temp_files_after"]
                for case in lifecycle.values()
            ),
            "seeded_proof_and_exit": reproducibility["all_exit_zero"]
            and reproducibility["proof_reproducible"],
            "aggregate_resource_footer": reproducibility[
                "resource_accounting_present"
            ]
            and reproducibility["resource_totals_monotonic"],
            "seeded_temp_cleanup": reproducibility["no_temp_files"],
        }
        result = {
            "schema_version": 1,
            "created_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "executable": str(executable),
            "executable_sha256": sha256_bytes(executable.read_bytes()),
            "requested_schedule_cores": args.cores,
            "memory_limit_mb": args.memory_limit_mb,
            "host": metadata,
            "lifecycle": lifecycle,
            "seeded_reproducibility": reproducibility,
            "checks": checks,
            "all_checks_passed": all(checks.values()),
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"checks": checks, "all_checks_passed": all(checks.values())}))
        return 0 if result["all_checks_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
