#!/usr/bin/env python3
"""Run the held-out layered-clause-selection ablation on Linux."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence


CATEGORIES = ("FNE", "FEQ", "EPS", "SLH")
SPLITS = ("validation", "test")
SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
THEOREM_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NON_THEOREM_STATUSES = {"CounterSatisfiable", "Satisfiable"}
THEOREM_EXPECTATIONS = {"theorem", "unsatisfiable"}
NON_THEOREM_EXPECTATIONS = {"non_theorem", "satisfiable"}
BASE_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
GOAL_WEIGHT = "Refinedweight(PreferGoals,2,1,1.5,1.1,1.1)"
HORN_WEIGHT = "Refinedweight(PreferHorn,2,1,1.5,1.1,1.1)"
UNIT_WEIGHT = "Refinedweight(PreferUnits,2,1,1.5,1.1,1.1)"
FIFO = "FIFOWeight(ConstPrio)"
COMMON_ARGS = ("--term-ordering=KBO6",)
STRATEGIES: dict[str, dict[str, Any]] = {
    "global_aw": {
        "heuristic": f"(5*{BASE_WEIGHT},1*{FIFO})",
        "kind": "baseline",
    },
    "goal_hard_priority": {
        "heuristic": (
            "(5*Refinedweight(PreferGoals,2,1,1.5,1.1,1.1),"
            f"1*{FIFO})"
        ),
        "kind": "single_queue_control",
    },
    "goal_relevance_scalar": {
        "heuristic": (
            "(5*ConjectureRelativeSymbolWeight("
            f"ConstPrio,0.5,100,100,100,100,1.5,1.5,1),1*{FIFO})"
        ),
        "kind": "scalar_control",
    },
    "goal_layered_4_1": {
        "heuristic": f"(4*{GOAL_WEIGHT},1*{BASE_WEIGHT},1*{FIFO})",
        "kind": "layered",
        "predicate": "goal",
        "layer_ratio": "4:1",
    },
    "goal_layered_1_4": {
        "heuristic": f"(1*{GOAL_WEIGHT},4*{BASE_WEIGHT},1*{FIFO})",
        "kind": "layered",
        "predicate": "goal",
        "layer_ratio": "1:4",
    },
    "horn_layered_4_1": {
        "heuristic": f"(4*{HORN_WEIGHT},1*{BASE_WEIGHT},1*{FIFO})",
        "kind": "layered",
        "predicate": "horn",
        "layer_ratio": "4:1",
    },
    "unit_layered_4_1": {
        "heuristic": f"(4*{UNIT_WEIGHT},1*{BASE_WEIGHT},1*{FIFO})",
        "kind": "layered",
        "predicate": "unit",
        "layer_ratio": "4:1",
    },
    "global_static_prune": {
        "heuristic": f"(5*{BASE_WEIGHT},1*{FIFO})",
        "kind": "lrs_falsification_control",
        "extra_args": ["--delete-bad-limit=1000000"],
    },
}


class ExperimentError(RuntimeError):
    """A contract, corpus, or result-validation failure."""


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(value) + b"\n")
    temporary.replace(path)


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows or rows[0].get("record_type") != "manifest":
        raise ExperimentError(f"invalid manifest header: {path}")
    records = rows[1:]
    if len(records) != rows[0].get("problem_count"):
        raise ExperimentError("manifest problem count does not match its records")
    return rows[0], records


def evenly_spaced(records: list[dict[str, Any]], count: int) -> list[dict[str, Any]]:
    count = min(count, len(records))
    if count == 0:
        raise ExperimentError("requested stratum is empty")
    if count == 1:
        return [records[len(records) // 2]]
    indices = [
        round(index * (len(records) - 1) / (count - 1))
        for index in range(count)
    ]
    if len(set(indices)) != count:
        raise ExperimentError("evenly spaced selection produced duplicate indices")
    return [records[index] for index in indices]


def select_records(
    records: list[dict[str, Any]], per_stratum: int
) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for split in SPLITS:
        for category in CATEGORIES:
            stratum = sorted(
                (
                    record
                    for record in records
                    if record["holdout_split"] == split
                    and record["category"] == category
                ),
                key=lambda record: (
                    record["official_category_order"],
                    record["problem_id"],
                ),
            )
            selected.extend(evenly_spaced(stratum, per_stratum))
    return selected


def verify_selected_corpus(problem_root: Path, selected: Sequence[dict[str, Any]]) -> None:
    for record in selected:
        path = problem_root / record["path"]
        if not path.is_file():
            raise ExperimentError(f"missing selected problem: {path}")
        actual = sha256_file(path)
        if actual != record["sha256"]:
            raise ExperimentError(
                f"problem hash mismatch for {record['problem_id']}: {actual}"
            )
        for include in record["includes"]:
            include_path = (
                problem_root / "problems" / "casc_2025" / include
            )
            if not include_path.is_file():
                raise ExperimentError(f"missing selected include: {include_path}")


def expected_status_match(expected_class: str, status: str | None) -> bool:
    if status is None:
        return False
    if expected_class in THEOREM_EXPECTATIONS:
        return status in THEOREM_STATUSES
    if expected_class in NON_THEOREM_EXPECTATIONS:
        return status in NON_THEOREM_STATUSES
    raise ExperimentError(f"unknown expected class: {expected_class}")


def final_status(stdout: str) -> str | None:
    statuses = SZS_RE.findall(stdout)
    return statuses[-1] if statuses else None


def load_optional_telemetry(
    telemetry_path: Path,
) -> tuple[dict[str, Any] | None, str | None, str | None]:
    if not telemetry_path.is_file():
        return None, None, None
    telemetry_sha256 = sha256_file(telemetry_path)
    try:
        telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return None, telemetry_sha256, f"{type(error).__name__}: {error}"
    return telemetry, telemetry_sha256, None


def result_is_resumable(
    result_path: Path,
    *,
    contract_id: str,
    problem_sha256: str,
    binary_sha256: str,
) -> bool:
    if not result_path.is_file():
        return False
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
        stdout_path = result_path.parent / "stdout.txt"
        stderr_path = result_path.parent / "stderr.txt"
        telemetry_path = result_path.parent / "telemetry.json"
        return (
            result["contract_id"] == contract_id
            and result["problem_sha256"] == problem_sha256
            and result["binary_sha256"] == binary_sha256
            and sha256_file(stdout_path) == result["stdout_sha256"]
            and sha256_file(stderr_path) == result["stderr_sha256"]
            and (
                result["telemetry_sha256"] is None
                or sha256_file(telemetry_path) == result["telemetry_sha256"]
            )
        )
    except (KeyError, OSError, ValueError, json.JSONDecodeError):
        return False


def run_one(
    *,
    binary: Path,
    binary_sha256: str,
    problem_root: Path,
    output_root: Path,
    contract_id: str,
    record: dict[str, Any],
    strategy_name: str,
    strategy: dict[str, Any],
    repetition: int,
    soft_cpu_seconds: int,
    hard_cpu_seconds: int,
    memory_mib: int,
) -> dict[str, Any]:
    run_dir = (
        output_root
        / "runs"
        / strategy_name
        / record["holdout_split"]
        / record["category"]
        / record["problem_id"]
        / f"rep-{repetition}"
    )
    result_path = run_dir / "result.json"
    if result_is_resumable(
        result_path,
        contract_id=contract_id,
        problem_sha256=record["sha256"],
        binary_sha256=binary_sha256,
    ):
        return {"resumed": True, "result_path": str(result_path)}

    run_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = run_dir / "stdout.txt"
    stderr_path = run_dir / "stderr.txt"
    telemetry_path = run_dir / "telemetry.json"
    telemetry_path.unlink(missing_ok=True)
    problem_path = problem_root / record["path"]
    command = [
        str(binary),
        f"--expert-heuristic={strategy['heuristic']}",
        *COMMON_ARGS,
        f"--soft-cpu-limit={soft_cpu_seconds}",
        f"--cpu-limit={hard_cpu_seconds}",
        f"--memory-limit={memory_mib}",
        f"--search-telemetry={telemetry_path}",
        *strategy.get("extra_args", []),
        str(problem_path),
    ]
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    started_at = utc_now()
    started = time.monotonic()
    timed_out = False
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=hard_cpu_seconds + 10,
            check=False,
        )
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        return_code = None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    wall_seconds = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    stdout_text = stdout.decode("utf-8", errors="replace")
    status = final_status(stdout_text)
    telemetry, telemetry_sha256, telemetry_error = load_optional_telemetry(
        telemetry_path
    )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "category": record["category"],
        "division": record["division"],
        "holdout_split": record["holdout_split"],
        "difficulty_band": record["difficulty_band"],
        "expected_class": record["expected_class"],
        "strategy": strategy_name,
        "strategy_kind": strategy["kind"],
        "repetition": repetition,
        "binary_sha256": binary_sha256,
        "command": command,
        "started_at": started_at,
        "completed_at": utc_now(),
        "return_code": return_code,
        "external_timeout": timed_out,
        "wall_seconds": wall_seconds,
        "szs_status": status,
        "expected_status_match": expected_status_match(
            record["expected_class"], status
        ),
        "telemetry_present": telemetry is not None,
        "telemetry_sha256": telemetry_sha256,
        "telemetry_error": telemetry_error,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }
    atomic_json(result_path, result)
    return {"resumed": False, "result_path": str(result_path)}


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--per-stratum", type=int, default=6)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--soft-cpu-seconds", type=int, default=5)
    parser.add_argument("--hard-cpu-seconds", type=int, default=7)
    parser.add_argument("--memory-mib", type=int, default=1536)
    parser.add_argument("--smoke", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("prover experiments may run only on Linux")
    for name in ("per_stratum", "repetitions", "workers", "soft_cpu_seconds"):
        if getattr(arguments, name) < 1:
            raise ExperimentError(f"--{name.replace('_', '-')} must be positive")
    if arguments.hard_cpu_seconds <= arguments.soft_cpu_seconds:
        raise ExperimentError("--hard-cpu-seconds must exceed --soft-cpu-seconds")
    if arguments.memory_mib < 256:
        raise ExperimentError("--memory-mib must be at least 256")

    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(f"binary is missing or not executable: {binary}")
    metadata, records = load_manifest(manifest_path)
    selected = select_records(records, arguments.per_stratum)
    strategies = STRATEGIES
    repetitions = arguments.repetitions
    workers = arguments.workers
    if arguments.smoke:
        selected = [
            next(
                record
                for record in selected
                if record["expected_class"] in THEOREM_EXPECTATIONS
            ),
            next(
                record
                for record in selected
                if record["expected_class"] in NON_THEOREM_EXPECTATIONS
            ),
        ]
        strategies = {
            name: STRATEGIES[name]
            for name in ("global_aw", "goal_layered_4_1")
        }
        repetitions = 1
        workers = min(workers, 2)
    verify_selected_corpus(problem_root, selected)

    binary_sha256 = sha256_file(binary)
    contract_body = json.loads(
        canonical_json(
            {
                "schema_version": 1,
                "manifest_sha256": sha256_file(manifest_path),
                "manifest_problem_count": metadata["problem_count"],
                "axiom_tree_sha256": metadata["sources"]["axiom_tree_sha256"],
                "selected_problem_ids": [
                    record["problem_id"] for record in selected
                ],
                "selected_problem_sha256": {
                    record["problem_id"]: record["sha256"] for record in selected
                },
                "categories": CATEGORIES,
                "splits": SPLITS,
                "expected_class_groups": {
                    "theorem": sorted(THEOREM_EXPECTATIONS),
                    "non_theorem": sorted(NON_THEOREM_EXPECTATIONS),
                },
                "target_per_stratum": arguments.per_stratum,
                "realized_stratum_counts": {
                    f"{split}:{category}": sum(
                        record["holdout_split"] == split
                        and record["category"] == category
                        for record in selected
                    )
                    for split in SPLITS
                    for category in CATEGORIES
                },
                "strategies": strategies,
                "repetitions": repetitions,
                "binary_sha256": binary_sha256,
                "harness_sha256": sha256_file(Path(__file__).resolve()),
                "common_args": COMMON_ARGS,
                "resources": {
                    "workers": workers,
                    "soft_cpu_seconds": arguments.soft_cpu_seconds,
                    "hard_cpu_seconds": arguments.hard_cpu_seconds,
                    "memory_mib": arguments.memory_mib,
                },
            }
        )
    )
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    contract = {
        **contract_body,
        "contract_id": contract_id,
        "created_at": utc_now(),
        "host": {
            "hostname": socket.gethostname(),
            "platform": platform.platform(),
            "cpu_count": os.cpu_count(),
        },
    }
    output_root = arguments.output_root.resolve()
    contract_path = output_root / "contract.json"
    if contract_path.is_file():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        existing_body = {
            key: value
            for key, value in existing.items()
            if key not in {"created_at", "host"}
        }
        current_body = {
            key: value
            for key, value in contract.items()
            if key not in {"created_at", "host"}
        }
        if existing_body != current_body:
            raise ExperimentError("output root contains an incompatible contract")
    else:
        atomic_json(contract_path, contract)

    jobs = [
        (record, strategy_name, strategy, repetition)
        for record in selected
        for strategy_name, strategy in strategies.items()
        for repetition in range(1, repetitions + 1)
    ]
    jobs.sort(
        key=lambda job: hashlib.sha256(
            f"{contract_id}:{job[0]['problem_id']}:{job[1]}:{job[3]}".encode()
        ).digest()
    )
    completed_count = 0
    resumed_count = 0
    result_paths: list[Path] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as executor:
        pending = [
            executor.submit(
                run_one,
                binary=binary,
                binary_sha256=binary_sha256,
                problem_root=problem_root,
                output_root=output_root,
                contract_id=contract_id,
                record=record,
                strategy_name=strategy_name,
                strategy=strategy,
                repetition=repetition,
                soft_cpu_seconds=arguments.soft_cpu_seconds,
                hard_cpu_seconds=arguments.hard_cpu_seconds,
                memory_mib=arguments.memory_mib,
            )
            for record, strategy_name, strategy, repetition in jobs
        ]
        for future in concurrent.futures.as_completed(pending):
            result = future.result()
            completed_count += 1
            resumed_count += int(result["resumed"])
            result_paths.append(Path(result["result_path"]))
            if completed_count % 25 == 0 or completed_count == len(jobs):
                print(
                    f"{completed_count}/{len(jobs)} complete "
                    f"({resumed_count} resumed)",
                    flush=True,
                )
    if arguments.smoke:
        invalid_smoke_results = []
        for result_path in result_paths:
            result = json.loads(result_path.read_text(encoding="utf-8"))
            if not result["telemetry_present"] or result["szs_status"] is None:
                invalid_smoke_results.append(str(result_path))
        if invalid_smoke_results:
            raise ExperimentError(
                "smoke runs must emit telemetry and an SZS status: "
                + ", ".join(sorted(invalid_smoke_results))
            )
    print(
        f"OK: contract {contract_id}; {len(selected)} problems; "
        f"{len(jobs)} runs; {resumed_count} resumed"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
