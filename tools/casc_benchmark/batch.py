#!/usr/bin/env python3
"""Run a resumable, resource-isolated CASC prover matrix on Linux."""

from __future__ import annotations

import argparse
import contextlib
import errno
import hashlib
import json
import os
import platform
import re
import signal
import socket
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Callable, Sequence

from manifest import (
    ManifestError,
    load_manifest,
    sha256_bytes,
    sha256_file,
    verify_corpus,
)

try:
    import resource
except ModuleNotFoundError:  # pragma: no cover - Linux execution is mandatory.
    resource = None  # type: ignore[assignment]

RUN_SCHEMA_VERSION = 1
RESULT_SCHEMA_VERSION = 1
VAMPIRE_REVISION = "3677326861181f990ce3ef461e90471ba9749225"
VAMPIRE_SHA256 = (
    "3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665"
)
MIB = 1024 * 1024
SZS_RE = re.compile(
    r"(?im)^[%#]+[ \t]*(?:\(\d+\)[ \t]*)?"
    r"SZS[ \t]+status[ \t]+(?P<status>[A-Za-z][A-Za-z0-9_]*)"
)
PROOF_STATUSES = frozenset(
    {"Theorem", "Unsatisfiable", "ContradictoryAxioms", "TautologousConclusion"}
)
MODEL_STATUSES = frozenset({"CounterSatisfiable", "Satisfiable"})
TIMEOUT_STATUSES = frozenset({"Timeout"})
RESOURCE_STATUSES = frozenset({"ResourceOut", "MemoryOut"})
GAVE_UP_STATUSES = frozenset({"GaveUp", "Unknown", "Inappropriate"})


class BatchError(RuntimeError):
    """Raised when a run cannot preserve its experiment contract."""


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def atomic_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(data)
    os.replace(temporary, path)


def atomic_write_json(path: Path, value: Any) -> None:
    atomic_write(path, canonical_json(value))


def read_text_if_present(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def szs_statuses(stdout: str, stderr: str = "") -> list[str]:
    """Return all statuses in output order, preferring the stdout stream."""

    return [
        match.group("status")
        for match in SZS_RE.finditer(f"{stdout}\n{stderr}")
    ]


def expected_status_match(expected_class: str, status: str | None) -> bool:
    """Check a positive terminal status against the category's semantic class."""

    if status is None:
        return False
    if expected_class == "theorem":
        return status in {
            "Theorem",
            "ContradictoryAxioms",
            "TautologousConclusion",
        }
    if expected_class == "unsatisfiable":
        return status in {"Unsatisfiable", "ContradictoryAxioms"}
    if expected_class == "satisfiable":
        return status == "Satisfiable"
    if expected_class == "non_theorem":
        return status in MODEL_STATUSES
    raise BatchError(f"unknown expected class {expected_class!r}")


def classify_result(
    *,
    status: str | None,
    return_code: int,
    termination_reason: str | None,
    oom_kills: int,
) -> str:
    """Classify one terminal process result without treating a peer as an oracle."""

    if oom_kills or termination_reason == "memory":
        return "resource_out"
    if termination_reason in {"wall", "cpu"}:
        return "timeout"
    if status in PROOF_STATUSES or status in MODEL_STATUSES:
        return "solved"
    if status in TIMEOUT_STATUSES:
        return "timeout"
    if status in RESOURCE_STATUSES:
        return "resource_out"
    if status in GAVE_UP_STATUSES:
        return "gave_up"
    if return_code < 0:
        return "crash"
    if return_code != 0:
        return "error"
    return "unknown"


def parse_key_values(path: Path) -> dict[str, int]:
    values: dict[str, int] = {}
    for line in read_text_if_present(path).splitlines():
        fields = line.split()
        if len(fields) >= 2:
            try:
                values[fields[0].rstrip(":")] = int(fields[1])
            except ValueError:
                continue
    return values


def mem_total_mib() -> int:
    values = parse_key_values(Path("/proc/meminfo"))
    kib = values.get("MemTotal")
    if kib is None:
        raise BatchError("cannot read MemTotal from /proc/meminfo")
    return kib // 1024


def cpu_model() -> str:
    for line in read_text_if_present(Path("/proc/cpuinfo")).splitlines():
        if line.lower().startswith("model name") and ":" in line:
            return line.split(":", maxsplit=1)[1].strip()
    return "unknown"


def os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    for line in read_text_if_present(Path("/etc/os-release")).splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", maxsplit=1)
        values[key] = value.strip().strip('"')
    return values


def host_metadata() -> dict[str, Any]:
    return {
        "captured_at": utc_now(),
        "hostname": socket.gethostname(),
        "platform": platform.platform(),
        "kernel": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "logical_cpus": os.cpu_count(),
        "cpu_model": cpu_model(),
        "mem_total_mib": mem_total_mib(),
        "os_release": os_release(),
        "effective_cpuset": read_text_if_present(
            Path("/sys/fs/cgroup/cpuset.cpus.effective")
        ).strip(),
    }


@dataclass(frozen=True)
class Limits:
    memory_mib: int
    pids: int
    wall_grace_seconds: float
    terminate_grace_seconds: float


class Cgroup:
    """One cgroup-v2 job boundary for aggregate accounting and cleanup."""

    def __init__(self, root: Path, name: str, limits: Limits):
        self.path = root / name
        self.limits = limits
        try:
            self.path.mkdir()
            self._write("memory.max", str(limits.memory_mib * MIB))
            swap_max = self.path / "memory.swap.max"
            if swap_max.exists():
                self._write("memory.swap.max", "0")
            self._write("pids.max", str(limits.pids))
        except OSError as error:
            with contextlib.suppress(OSError):
                self.path.rmdir()
            raise BatchError(f"cannot create strict cgroup {self.path}: {error}") from error

    def _write(self, name: str, value: str) -> None:
        (self.path / name).write_text(value, encoding="ascii")

    def attach_self(self) -> None:
        """Attach the post-fork child before it executes the prover."""

        self._write("cgroup.procs", str(os.getpid()))

    def pids(self) -> list[int]:
        result: list[int] = []
        for line in read_text_if_present(self.path / "cgroup.procs").splitlines():
            with contextlib.suppress(ValueError):
                result.append(int(line))
        return result

    def cpu_usage_usec(self) -> int:
        return parse_key_values(self.path / "cpu.stat").get("usage_usec", 0)

    def memory_peak_bytes(self) -> int:
        value = read_text_if_present(self.path / "memory.peak").strip()
        with contextlib.suppress(ValueError):
            return int(value)
        return 0

    def memory_events(self) -> dict[str, int]:
        return parse_key_values(self.path / "memory.events")

    def signal_all(self, signum: int) -> None:
        for pid in self.pids():
            with contextlib.suppress(ProcessLookupError, PermissionError):
                os.kill(pid, signum)

    def kill(self) -> None:
        kill_path = self.path / "cgroup.kill"
        if kill_path.exists():
            with contextlib.suppress(OSError):
                kill_path.write_text("1", encoding="ascii")
        else:
            self.signal_all(signal.SIGKILL)

    def close(self) -> None:
        self.kill()
        deadline = time.monotonic() + 2.0
        last_error: OSError | None = None
        while True:
            pids = self.pids()
            populated = parse_key_values(self.path / "cgroup.events").get(
                "populated"
            )
            if not pids and populated == 0:
                try:
                    self.path.rmdir()
                    return
                except OSError as error:
                    if error.errno not in {errno.EBUSY, errno.ENOTEMPTY}:
                        raise BatchError(
                            "cgroup cleanup could not remove empty boundary "
                            f"{self.path}: {error}"
                        ) from error
                    last_error = error
            if time.monotonic() >= deadline:
                detail = (
                    f"pids={pids}, populated={populated}, "
                    f"last_remove_error={last_error}"
                )
                raise BatchError(
                    "cgroup cleanup left live processes or state in "
                    f"{self.path}: {detail}"
                ) from last_error
            time.sleep(0.01)

    def __enter__(self) -> Cgroup:
        return self

    def __exit__(self, _type: Any, _value: Any, _traceback: Any) -> None:
        self.close()


def require_cgroup_v2(root: Path) -> None:
    controllers = root / "cgroup.controllers"
    if not controllers.is_file():
        raise BatchError(f"cgroup v2 is required: {controllers} is missing")
    available = set(controllers.read_text(encoding="ascii").split())
    missing = {"memory", "pids"} - available
    if missing:
        raise BatchError(f"required cgroup controllers unavailable: {sorted(missing)}")
    required_files = ["cgroup.procs", "cpu.stat"]
    for name in required_files:
        if not (root / name).exists():
            raise BatchError(f"required cgroup file is missing: {root / name}")


def validate_host(
    metadata: dict[str, Any],
    *,
    cores: int,
    memory_mib: int,
    allow_noncanonical: bool,
) -> None:
    problems: list[str] = []
    if metadata["machine"] != "x86_64":
        problems.append(f"machine is {metadata['machine']}, not x86_64")
    if (metadata["logical_cpus"] or 0) < cores:
        problems.append(
            f"host exposes {metadata['logical_cpus']} CPUs, fewer than requested {cores}"
        )
    required_memory = memory_mib + 4096
    if metadata["mem_total_mib"] < required_memory:
        problems.append(
            f"host has {metadata['mem_total_mib']} MiB, below strict "
            f"{memory_mib} MiB job limit plus 4096 MiB overhead"
        )
    if problems and not allow_noncanonical:
        raise BatchError("noncanonical host: " + "; ".join(problems))


def selected_records(
    records: Sequence[dict[str, Any]],
    *,
    categories: set[str],
    divisions: set[str],
    splits: set[str],
    problems: set[str],
    max_problems: int | None,
) -> list[dict[str, Any]]:
    selected = [
        record
        for record in records
        if (not categories or record["category"] in categories)
        and (not divisions or record["division"] in divisions)
        and (not splits or record["holdout_split"] in splits)
        and (not problems or record["problem_id"] in problems)
    ]
    if problems:
        found = {record["problem_id"] for record in selected}
        missing = sorted(problems - found)
        if missing:
            raise BatchError(f"selected problem IDs not in manifest/filters: {missing}")
    if max_problems is not None:
        selected = selected[:max_problems]
    if not selected:
        raise BatchError("selection contains no problems")
    return selected


def solver_command(
    solver: str,
    binary: Path,
    record: dict[str, Any],
    problem: Path,
    *,
    cores: int,
    memory_mib: int,
    seed: int,
) -> list[str]:
    """Build the exact pinned adapter command for one problem."""

    job_cores = 1 if record["limit_kind"] == "cpu" else cores
    seconds = int(record["limit_seconds"])
    wants_model = record["expected_class"] in {"satisfiable", "non_theorem"}
    if solver == "umlaut":
        schedule = "satauto-schedule" if wants_model else "auto-schedule"
        return [
            str(binary),
            f"--{schedule}={job_cores}",
            "--silent",
            "--resources-info",
            "--proof-object",
            "--tstp-format",
            f"--cpu-limit={seconds}",
            f"--memory-limit={memory_mib}",
            "--",
            str(problem),
        ]
    if solver == "vampire":
        schedule = "casc_sat_2025" if wants_model else "casc_2025"
        command = [
            str(binary),
            "--mode",
            "casc",
            "--schedule",
            schedule,
            "--cores",
            str(job_cores),
            "--time_limit",
            "0" if record["limit_kind"] == "cpu" else str(seconds),
            "--memory_limit",
            str(memory_mib),
            "--random_seed",
            str(seed),
            "--randomize_seed_for_portfolio_workers",
            "off",
        ]
        if wants_model:
            command.extend(["--intent", "sat"])
        command.append(str(problem))
        return command
    raise BatchError(f"unknown solver adapter {solver!r}")


def contract_value(
    *,
    manifest_sha256: str,
    selected: Sequence[dict[str, Any]],
    solvers: dict[str, dict[str, Any]],
    cores: int,
    limits: Limits,
    seed: int,
    presentation_id: str,
    source_snapshot_sha256: str | None,
    canonical_selection: bool,
) -> dict[str, Any]:
    selected_ids = [record["problem_id"] for record in selected]
    value = {
        "schema_version": RUN_SCHEMA_VERSION,
        "kind": "umlaut-casc-benchmark-run",
        "manifest_sha256": manifest_sha256,
        "selected_problem_count": len(selected),
        "selected_problem_ids": selected_ids,
        "selected_problem_ids_sha256": sha256_bytes(
            ("\n".join(selected_ids) + "\n").encode()
        ),
        "canonical_full_selection": canonical_selection,
        "presentation_id": presentation_id,
        "solvers": solvers,
        "cores": cores,
        "memory_limit_mib": limits.memory_mib,
        "pids_limit": limits.pids,
        "wall_grace_seconds": limits.wall_grace_seconds,
        "terminate_grace_seconds": limits.terminate_grace_seconds,
        "vampire_seed": seed,
        "source_snapshot_sha256": source_snapshot_sha256,
        "adapters": {
            "umlaut": "auto/satauto schedule; one core for SLH; TSTP proof output",
            "vampire": (
                "Pinned Vampire 5.0.1 casc_2025/casc_sat_2025 built-in "
                "schedules; fixed seed; one core for CPU-limited categories; "
                "TPTP input"
            ),
        },
        "accounting": {
            "memory": "cgroup-v2 aggregate memory.max and memory.peak",
            "cpu": "cgroup-v2 aggregate cpu.stat usage_usec",
            "wall": "monotonic process lifetime",
            "cleanup": "process session plus cgroup.kill",
        },
    }
    value["contract_id"] = sha256_bytes(canonical_json(value))
    return value


def ensure_contract(
    output_root: Path,
    contract: dict[str, Any],
    *,
    expected_contract_id: str | None = None,
) -> dict[str, Any]:
    path = output_root / "contract.json"
    if path.exists():
        try:
            existing = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise BatchError(f"cannot read existing run contract {path}: {error}") from error
        if existing == contract:
            if (
                expected_contract_id is not None
                and existing.get("contract_id") != expected_contract_id
            ):
                raise BatchError(
                    "existing run contract does not match the explicitly "
                    "expected contract ID"
                )
            return existing
        existing_without_id = {
            key: value for key, value in existing.items() if key != "contract_id"
        }
        contract_without_id = {
            key: value for key, value in contract.items() if key != "contract_id"
        }
        if (
            expected_contract_id is not None
            and existing.get("contract_id") == expected_contract_id
            and existing_without_id == contract_without_id
        ):
            return existing
        raise BatchError(
            "existing output uses an incompatible run contract; choose a "
            "different output directory"
        )
    else:
        if (
            expected_contract_id is not None
            and contract.get("contract_id") != expected_contract_id
        ):
            raise BatchError(
                "explicit historical contract ID requires an existing "
                "content-matching run contract"
            )
        atomic_write_json(path, contract)
        return contract


def safe_result_key(index: int, record: dict[str, Any]) -> str:
    suffix = hashlib.sha256(record["problem_id"].encode()).hexdigest()[:12]
    return f"{index:04d}-{record['category'].lower()}-{suffix}"


def result_paths(
    output_root: Path, solver: str, index: int, record: dict[str, Any]
) -> tuple[Path, Path, Path]:
    key = safe_result_key(index, record)
    base = output_root / "results" / solver / record["category"].lower()
    return base / f"{key}.json", base / f"{key}.stdout", base / f"{key}.stderr"


def validate_completed_result(
    result_path: Path,
    stdout_path: Path,
    stderr_path: Path,
    *,
    contract_id: str,
    record: dict[str, Any],
    solver: str,
) -> None:
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BatchError(f"corrupt completed result {result_path}: {error}") from error
    checks = {
        "contract_id": contract_id,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "solver": solver,
    }
    for name, expected in checks.items():
        if result.get(name) != expected:
            raise BatchError(
                f"completed result {result_path} has incompatible {name}: "
                f"{result.get(name)!r} != {expected!r}"
            )
    for artifact_path, digest_name in [
        (stdout_path, "stdout_sha256"),
        (stderr_path, "stderr_sha256"),
    ]:
        if not artifact_path.is_file():
            raise BatchError(f"completed result artifact is missing: {artifact_path}")
        if sha256_file(artifact_path) != result.get(digest_name):
            raise BatchError(f"completed result artifact hash mismatch: {artifact_path}")


def _child_setup(cgroup: Cgroup, memory_bytes: int) -> Callable[[], None]:
    def setup() -> None:
        if resource is None:
            raise BatchError("POSIX resource limits are unavailable")
        os.setsid()
        resource.setrlimit(resource.RLIMIT_AS, (memory_bytes, memory_bytes))
        cgroup.attach_self()

    return setup


def _terminate_job(process: subprocess.Popen[bytes], cgroup: Cgroup, grace: float) -> None:
    with contextlib.suppress(ProcessLookupError):
        os.killpg(process.pid, signal.SIGTERM)
    cgroup.signal_all(signal.SIGTERM)
    deadline = time.monotonic() + grace
    while cgroup.pids() and time.monotonic() < deadline:
        time.sleep(0.01)
    if cgroup.pids():
        cgroup.kill()


def run_one(
    *,
    solver: str,
    binary: Path,
    record: dict[str, Any],
    problem: Path,
    command: list[str],
    output_root: Path,
    index: int,
    contract_id: str,
    session_id: str,
    cgroup_root: Path,
    limits: Limits,
    environment: dict[str, str],
) -> dict[str, Any]:
    """Execute one solver/problem pair and atomically publish its result."""

    result_path, stdout_path, stderr_path = result_paths(
        output_root, solver, index, record
    )
    if result_path.exists():
        validate_completed_result(
            result_path,
            stdout_path,
            stderr_path,
            contract_id=contract_id,
            record=record,
            solver=solver,
        )
        return {"resumed": True, "result_path": str(result_path)}
    for incomplete in (stdout_path, stderr_path):
        with contextlib.suppress(FileNotFoundError):
            incomplete.unlink()
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    cgroup_name = (
        f"umlaut-casc-{os.getpid()}-{index}-{solver}-"
        f"{hashlib.sha256(record['problem_id'].encode()).hexdigest()[:8]}"
    )
    started_at = utc_now()
    start = time.monotonic()
    termination_reason: str | None = None
    orphan_cleanup = False
    return_code = 127
    cgroup = Cgroup(cgroup_root, cgroup_name, limits)
    try:
        with (
            stdout_path.open("wb") as stdout_file,
            stderr_path.open("wb") as stderr_file,
        ):
            try:
                process = subprocess.Popen(
                    command,
                    stdout=stdout_file,
                    stderr=stderr_file,
                    env=environment,
                    preexec_fn=_child_setup(cgroup, limits.memory_mib * MIB),
                )
            except (OSError, subprocess.SubprocessError) as error:
                stderr_file.write(f"launch error: {error}\n".encode())
                process = None
                termination_reason = "launch"
            if process is not None:
                wall_limit = float(record["limit_seconds"])
                cpu_limit_usec = int(record["limit_seconds"] * 1_000_000)
                while process.poll() is None:
                    elapsed = time.monotonic() - start
                    events = cgroup.memory_events()
                    if events.get("oom_kill", 0) > 0:
                        termination_reason = "memory"
                    elif (
                        record["limit_kind"] == "wall"
                        and elapsed
                        > wall_limit + limits.wall_grace_seconds
                    ):
                        termination_reason = "wall"
                    elif (
                        record["limit_kind"] == "cpu"
                        and cgroup.cpu_usage_usec() > cpu_limit_usec
                    ):
                        termination_reason = "cpu"
                    if termination_reason is not None:
                        _terminate_job(
                            process, cgroup, limits.terminate_grace_seconds
                        )
                        break
                    time.sleep(0.05)
                with contextlib.suppress(subprocess.TimeoutExpired):
                    return_code = process.wait(timeout=2)
                if process.poll() is None:
                    cgroup.kill()
                    return_code = process.wait(timeout=2)
                else:
                    return_code = int(process.returncode)
                if cgroup.pids():
                    orphan_cleanup = True
                    _terminate_job(process, cgroup, limits.terminate_grace_seconds)

        elapsed = time.monotonic() - start
        cpu_seconds = cgroup.cpu_usage_usec() / 1_000_000
        peak_bytes = cgroup.memory_peak_bytes()
        events = cgroup.memory_events()
    finally:
        cgroup.close()

    stdout = read_text_if_present(stdout_path)
    stderr = read_text_if_present(stderr_path)
    statuses = szs_statuses(stdout, stderr)
    final_status = statuses[-1] if statuses else None
    classification = classify_result(
        status=final_status,
        return_code=return_code,
        termination_reason=termination_reason,
        oom_kills=events.get("oom_kill", 0),
    )
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "contract_id": contract_id,
        "session_id": session_id,
        "solver": solver,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "category": record["category"],
        "division": record["division"],
        "family": record["family"],
        "holdout_split": record["holdout_split"],
        "difficulty_band": record["difficulty_band"],
        "expected_class": record["expected_class"],
        "command": command,
        "started_at": started_at,
        "completed_at": utc_now(),
        "return_code": return_code,
        "termination_reason": termination_reason,
        "classification": classification,
        "szs_statuses": statuses,
        "final_szs_status": final_status,
        "expected_status_match": expected_status_match(
            record["expected_class"], final_status
        ),
        "wall_seconds": elapsed,
        "cpu_seconds": cpu_seconds,
        "peak_memory_bytes": peak_bytes,
        "peak_memory_mib": peak_bytes / MIB,
        "memory_events": events,
        "orphan_cleanup_required": orphan_cleanup,
        "stdout_path": stdout_path.relative_to(output_root).as_posix(),
        "stderr_path": stderr_path.relative_to(output_root).as_posix(),
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }
    atomic_write_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path), "result": result}


def session_value(
    *,
    session_id: str,
    contract_id: str,
    host: dict[str, Any],
    cgroup_root: Path,
    runner: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "session_id": session_id,
        "contract_id": contract_id,
        "started_at": utc_now(),
        "host": host,
        "runner": runner,
        "cgroup_root": str(cgroup_root),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--umlaut-binary", type=Path)
    parser.add_argument("--vampire-binary", type=Path)
    parser.add_argument(
        "--solvers",
        choices=("both", "umlaut", "vampire"),
        default="both",
    )
    parser.add_argument("--cores", type=int, default=8)
    parser.add_argument("--memory-limit-mib", type=int, default=131072)
    parser.add_argument("--pids-limit", type=int, default=512)
    parser.add_argument("--vampire-seed", type=int, default=1)
    parser.add_argument("--wall-grace-seconds", type=float, default=0.25)
    parser.add_argument("--terminate-grace-seconds", type=float, default=1.0)
    parser.add_argument("--cgroup-root", type=Path, default=Path("/sys/fs/cgroup"))
    parser.add_argument("--category", action="append", default=[])
    parser.add_argument("--division", action="append", default=[])
    parser.add_argument(
        "--split",
        action="append",
        choices=("train", "validation", "test"),
        default=[],
    )
    parser.add_argument("--problem", action="append", default=[])
    parser.add_argument("--max-problems", type=int)
    parser.add_argument(
        "--max-new-results",
        type=int,
        help="stop this resumable session after publishing this many new results",
    )
    parser.add_argument(
        "--max-session-wall-seconds",
        type=float,
        help=(
            "stop before starting another result after this session wall time; "
            "one in-flight result may finish after the boundary"
        ),
    )
    parser.add_argument("--session-id")
    parser.add_argument("--runner-label")
    parser.add_argument("--runner-run-id")
    parser.add_argument("--linode-id", type=int)
    parser.add_argument("--source-snapshot-sha256")
    parser.add_argument(
        "--expected-contract-id",
        help=(
            "require this existing contract identity when resuming a run; "
            "all non-ID contract fields must still match"
        ),
    )
    parser.add_argument(
        "--allow-noncanonical-host",
        action="store_true",
        help="permit a smoke run on a smaller host; recorded as noncanonical",
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="validate manifest, binaries, host, and cgroups without running solvers",
    )
    return parser.parse_args(argv)


def solver_paths(arguments: argparse.Namespace) -> dict[str, Path]:
    names = (
        ["umlaut", "vampire"]
        if arguments.solvers == "both"
        else [arguments.solvers]
    )
    result: dict[str, Path] = {}
    for name in names:
        value = getattr(arguments, f"{name}_binary")
        if value is None:
            raise BatchError(f"--{name}-binary is required for {arguments.solvers}")
        path = value.resolve()
        if not path.is_file():
            raise BatchError(f"{name} binary does not exist: {path}")
        if not os.access(path, os.X_OK):
            raise BatchError(f"{name} binary is not executable: {path}")
        result[name] = path
    if "vampire" in result:
        actual = sha256_file(result["vampire"])
        if actual != VAMPIRE_SHA256:
            raise BatchError(
                f"pinned Vampire hash mismatch: {actual}; expected {VAMPIRE_SHA256}"
            )
    return result


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        if sys.platform != "linux":
            raise BatchError("the batch harness may execute provers only on Linux")
        if arguments.cores < 1:
            raise BatchError("--cores must be positive")
        if arguments.memory_limit_mib < 20:
            raise BatchError("--memory-limit-mib must be at least 20")
        if arguments.max_problems is not None and arguments.max_problems < 1:
            raise BatchError("--max-problems must be positive")
        if arguments.max_new_results is not None and arguments.max_new_results < 1:
            raise BatchError("--max-new-results must be positive")
        if (
            arguments.max_session_wall_seconds is not None
            and arguments.max_session_wall_seconds <= 0
        ):
            raise BatchError("--max-session-wall-seconds must be positive")

        manifest_path = arguments.manifest.resolve()
        metadata, records = load_manifest(manifest_path)
        manifest_sha256 = sha256_file(manifest_path)
        selected = selected_records(
            records,
            categories={value.upper() for value in arguments.category},
            divisions={value.upper() for value in arguments.division},
            splits=set(arguments.split),
            problems=set(arguments.problem),
            max_problems=arguments.max_problems,
        )
        problem_root = arguments.problem_root.resolve()
        verify_corpus(problem_root, metadata, records)

        paths = solver_paths(arguments)
        solver_contract = {
            name: {
                "sha256": sha256_file(path),
                "revision": VAMPIRE_REVISION if name == "vampire" else None,
            }
            for name, path in sorted(paths.items())
        }
        limits = Limits(
            memory_mib=arguments.memory_limit_mib,
            pids=arguments.pids_limit,
            wall_grace_seconds=arguments.wall_grace_seconds,
            terminate_grace_seconds=arguments.terminate_grace_seconds,
        )
        host = host_metadata()
        validate_host(
            host,
            cores=arguments.cores,
            memory_mib=arguments.memory_limit_mib,
            allow_noncanonical=arguments.allow_noncanonical_host,
        )
        cgroup_root = arguments.cgroup_root.resolve()
        require_cgroup_v2(cgroup_root)
        canonical_selection = (
            len(selected) == metadata["problem_count"]
            and arguments.solvers == "both"
            and not arguments.category
            and not arguments.division
            and not arguments.split
            and not arguments.problem
            and arguments.max_problems is None
            and arguments.cores == 8
            and arguments.memory_limit_mib == 131072
            and not arguments.allow_noncanonical_host
        )
        if arguments.source_snapshot_sha256 is not None and not re.fullmatch(
            r"[0-9a-fA-F]{64}", arguments.source_snapshot_sha256
        ):
            raise BatchError("--source-snapshot-sha256 must be 64 hexadecimal digits")
        if arguments.source_snapshot_sha256 is not None:
            arguments.source_snapshot_sha256 = (
                arguments.source_snapshot_sha256.lower()
            )
        if arguments.expected_contract_id is not None:
            if not re.fullmatch(
                r"[0-9a-fA-F]{64}", arguments.expected_contract_id
            ):
                raise BatchError(
                    "--expected-contract-id must be 64 hexadecimal digits"
                )
            arguments.expected_contract_id = (
                arguments.expected_contract_id.lower()
            )
        if canonical_selection and arguments.source_snapshot_sha256 is None:
            raise BatchError(
                "canonical full runs require --source-snapshot-sha256"
            )
        runner = {
            "label": arguments.runner_label,
            "run_id": arguments.runner_run_id,
            "linode_id": arguments.linode_id,
        }
        if canonical_selection and any(value is None for value in runner.values()):
            raise BatchError(
                "canonical full runs require --runner-label, --runner-run-id, "
                "and --linode-id"
            )
        contract = contract_value(
            manifest_sha256=manifest_sha256,
            selected=selected,
            solvers=solver_contract,
            cores=arguments.cores,
            limits=limits,
            seed=arguments.vampire_seed,
            presentation_id=metadata["presentation"]["id"],
            source_snapshot_sha256=arguments.source_snapshot_sha256,
            canonical_selection=canonical_selection,
        )
        output_root = arguments.output_root.resolve()
        contract = ensure_contract(
            output_root,
            contract,
            expected_contract_id=arguments.expected_contract_id,
        )
        session_id = arguments.session_id or (
            f"{datetime.now(UTC).strftime('%Y%m%dT%H%M%SZ')}-"
            f"{socket.gethostname()}-{os.getpid()}"
        )
        session = session_value(
            session_id=session_id,
            contract_id=contract["contract_id"],
            host=host,
            cgroup_root=cgroup_root,
            runner=runner,
        )
        atomic_write_json(output_root / "sessions" / f"{session_id}.json", session)
        if arguments.verify_only:
            print(
                f"OK: contract {contract['contract_id']}, "
                f"{len(selected)} selected problems, strict cgroup v2 available"
            )
            return 0

        environment = os.environ.copy()
        corpus_root = metadata["sources"].get(
            "corpus_root", "problems/casc_2025"
        )
        environment["TPTP"] = str(problem_root / corpus_root)
        completed = 0
        resumed = 0
        session_started = time.monotonic()
        stopped_after_session_limit = False
        selected_ids = {record["problem_id"] for record in selected}
        for index, record in enumerate(records, start=1):
            if record["problem_id"] not in selected_ids:
                continue
            problem = problem_root / record["path"]
            for solver, binary in paths.items():
                if (
                    arguments.max_new_results is not None
                    and completed >= arguments.max_new_results
                ) or (
                    arguments.max_session_wall_seconds is not None
                    and time.monotonic() - session_started
                    >= arguments.max_session_wall_seconds
                ):
                    stopped_after_session_limit = True
                    break
                command = solver_command(
                    solver,
                    binary,
                    record,
                    problem,
                    cores=arguments.cores,
                    memory_mib=arguments.memory_limit_mib,
                    seed=arguments.vampire_seed,
                )
                outcome = run_one(
                    solver=solver,
                    binary=binary,
                    record=record,
                    problem=problem,
                    command=command,
                    output_root=output_root,
                    index=index,
                    contract_id=contract["contract_id"],
                    session_id=session_id,
                    cgroup_root=cgroup_root,
                    limits=limits,
                    environment=environment,
                )
                if outcome["resumed"]:
                    resumed += 1
                else:
                    completed += 1
                    result = outcome["result"]
                    print(
                        f"{solver} {record['problem_id']}: "
                        f"{result['classification']} "
                        f"{result['final_szs_status'] or '-'} "
                        f"{result['wall_seconds']:.3f}s",
                        flush=True,
                    )
            if stopped_after_session_limit:
                break
        session["completed_at"] = utc_now()
        session["new_results"] = completed
        session["resumed_results"] = resumed
        session["max_new_results"] = arguments.max_new_results
        session["max_session_wall_seconds"] = arguments.max_session_wall_seconds
        session["stopped_after_session_limit"] = stopped_after_session_limit
        atomic_write_json(output_root / "sessions" / f"{session_id}.json", session)
        print(
            f"OK: contract {contract['contract_id']}; "
            f"new={completed}, resumed={resumed}"
        )
        return 0
    except (BatchError, ManifestError, OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
