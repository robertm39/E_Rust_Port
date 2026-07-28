#!/usr/bin/env python3
"""Run the fresh-family goal-hard-priority escalation experiment on Linux."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
import platform
import socket
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_HARNESS_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-005-layered-clause-selection"
    / "run.py"
)
CATEGORIES = ("FEQ", "ICU", "SLH", "TEQ", "TFE", "UEQ")
COMMON_ARGS = ("--term-ordering=KBO6",)
BUDGETS: dict[str, dict[str, int]] = {
    "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
    "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


BASE = load_module("layered_clause_selection_base_run", BASE_HARNESS_PATH)
STRATEGIES: dict[str, dict[str, Any]] = {
    name: dict(BASE.STRATEGIES[name])
    for name in ("global_aw", "goal_hard_priority", "goal_relevance_scalar")
}


class ExperimentError(RuntimeError):
    """A contract, corpus, or result-validation failure."""


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def sha256_file(path: Path) -> str:
    return BASE.sha256_file(path)


def canonical_json(value: Any) -> bytes:
    return BASE.canonical_json(value)


def atomic_json(path: Path, value: Any) -> None:
    BASE.atomic_json(path, value)


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    return BASE.load_manifest(path)


def select_fresh_records(
    records: list[dict[str, Any]],
    prior_problem_ids: set[str],
    per_category: int,
) -> tuple[list[str], list[dict[str, Any]]]:
    excluded_families = sorted(
        {
            record["family"]
            for record in records
            if record["problem_id"] in prior_problem_ids
        }
    )
    excluded = set(excluded_families)
    selected: list[dict[str, Any]] = []
    for category in CATEGORIES:
        candidates = sorted(
            (
                record
                for record in records
                if record["holdout_split"] == "test"
                and record["category"] == category
                and record["family"] not in excluded
            ),
            key=lambda record: (
                record["official_category_order"],
                record["problem_id"],
            ),
        )
        selected.extend(BASE.evenly_spaced(candidates, per_category))
    return excluded_families, selected


def result_is_resumable(
    result_path: Path,
    *,
    contract_id: str,
    problem_sha256: str,
    binary_sha256: str,
) -> bool:
    return BASE.result_is_resumable(
        result_path,
        contract_id=contract_id,
        problem_sha256=problem_sha256,
        binary_sha256=binary_sha256,
    )


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
    budget_name: str,
    budget: dict[str, int],
    repetition: int,
    memory_mib: int,
) -> dict[str, Any]:
    run_dir = (
        output_root
        / "runs"
        / budget_name
        / strategy_name
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
    command = [
        str(binary),
        f"--expert-heuristic={strategy['heuristic']}",
        *COMMON_ARGS,
        f"--soft-cpu-limit={budget['soft_cpu_seconds']}",
        f"--cpu-limit={budget['hard_cpu_seconds']}",
        f"--memory-limit={memory_mib}",
        f"--search-telemetry={telemetry_path}",
        str(problem_root / record["path"]),
    ]
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    started_at = utc_now()
    started = time.monotonic()
    external_timeout = False
    try:
        completed = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=budget["hard_cpu_seconds"] + 10,
            check=False,
        )
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        external_timeout = True
        return_code = None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    wall_seconds = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    status = BASE.final_status(stdout.decode("utf-8", errors="replace"))
    telemetry, telemetry_sha256, telemetry_error = BASE.load_optional_telemetry(
        telemetry_path
    )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "family": record["family"],
        "category": record["category"],
        "division": record["division"],
        "holdout_split": record["holdout_split"],
        "difficulty_band": record["difficulty_band"],
        "expected_class": record["expected_class"],
        "strategy": strategy_name,
        "strategy_kind": strategy["kind"],
        "budget": budget_name,
        "soft_cpu_seconds": budget["soft_cpu_seconds"],
        "hard_cpu_seconds": budget["hard_cpu_seconds"],
        "repetition": repetition,
        "binary_sha256": binary_sha256,
        "command": command,
        "started_at": started_at,
        "completed_at": utc_now(),
        "return_code": return_code,
        "external_timeout": external_timeout,
        "wall_seconds": wall_seconds,
        "szs_status": status,
        "expected_status_match": BASE.expected_status_match(
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
    parser.add_argument("--prior-selection", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--per-category", type=int, default=4)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--memory-mib", type=int, default=1536)
    parser.add_argument("--smoke", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("prover experiments may run only on Linux")
    for name in ("per_category", "repetitions", "workers"):
        if getattr(arguments, name) < 1:
            raise ExperimentError(f"--{name.replace('_', '-')} must be positive")
    if arguments.memory_mib < 256:
        raise ExperimentError("--memory-mib must be at least 256")

    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    prior_selection_path = arguments.prior_selection.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(f"binary is missing or not executable: {binary}")
    prior_selection = json.loads(
        prior_selection_path.read_text(encoding="utf-8")
    )
    if sha256_file(manifest_path) != prior_selection["manifest_sha256"]:
        raise ExperimentError("prior selection references a different manifest")
    metadata, records = load_manifest(manifest_path)
    excluded_families, selected = select_fresh_records(
        records,
        set(prior_selection["selected_problem_ids"]),
        arguments.per_category,
    )
    strategies = STRATEGIES
    budgets = BUDGETS
    repetitions = arguments.repetitions
    workers = arguments.workers
    if arguments.smoke:
        theorem = next(
            record
            for record in selected
            if record["expected_class"] in BASE.THEOREM_EXPECTATIONS
        )
        unsatisfiable = next(
            record
            for record in selected
            if record["expected_class"] == "unsatisfiable"
        )
        selected = [theorem, unsatisfiable]
        strategies = {
            name: STRATEGIES[name]
            for name in ("global_aw", "goal_hard_priority")
        }
        budgets = {"short": BUDGETS["short"]}
        repetitions = 1
        workers = min(workers, 2)
    BASE.verify_selected_corpus(problem_root, selected)

    binary_sha256 = sha256_file(binary)
    contract_body = json.loads(
        canonical_json(
            {
                "schema_version": 1,
                "manifest_sha256": sha256_file(manifest_path),
                "manifest_problem_count": metadata["problem_count"],
                "axiom_tree_sha256": metadata["sources"]["axiom_tree_sha256"],
                "prior_contract_id": prior_selection["prior_contract_id"],
                "prior_contract_sha256": prior_selection[
                    "prior_contract_sha256"
                ],
                "prior_selection_sha256": sha256_file(prior_selection_path),
                "excluded_prior_families": excluded_families,
                "selected_problem_ids": [
                    record["problem_id"] for record in selected
                ],
                "selected_problem_sha256": {
                    record["problem_id"]: record["sha256"] for record in selected
                },
                "selected_families": sorted(
                    {record["family"] for record in selected}
                ),
                "categories": CATEGORIES,
                "split": "test",
                "target_per_category": arguments.per_category,
                "realized_category_counts": {
                    category: sum(
                        record["category"] == category for record in selected
                    )
                    for category in CATEGORIES
                },
                "strategies": strategies,
                "budgets": budgets,
                "repetitions": repetitions,
                "binary_sha256": binary_sha256,
                "harness_sha256": sha256_file(Path(__file__).resolve()),
                "base_harness_sha256": sha256_file(BASE_HARNESS_PATH),
                "common_args": COMMON_ARGS,
                "resources": {
                    "workers": workers,
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
        (record, strategy_name, strategy, budget_name, budget, repetition)
        for record in selected
        for strategy_name, strategy in strategies.items()
        for budget_name, budget in budgets.items()
        for repetition in range(1, repetitions + 1)
    ]
    jobs.sort(
        key=lambda job: hashlib.sha256(
            (
                f"{contract_id}:{job[0]['problem_id']}:{job[1]}:"
                f"{job[3]}:{job[5]}"
            ).encode()
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
                budget_name=budget_name,
                budget=budget,
                repetition=repetition,
                memory_mib=arguments.memory_mib,
            )
            for record, strategy_name, strategy, budget_name, budget, repetition
            in jobs
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
        invalid = []
        for result_path in result_paths:
            result = json.loads(result_path.read_text(encoding="utf-8"))
            if not result["telemetry_present"] or result["szs_status"] is None:
                invalid.append(str(result_path))
        if invalid:
            raise ExperimentError(
                "smoke runs must emit telemetry and an SZS status: "
                + ", ".join(sorted(invalid))
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
