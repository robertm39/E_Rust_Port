#!/usr/bin/env python3
"""Run the staged, family-held-out UEQ completion experiment on Linux."""

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


SZS_RE = re.compile(r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
UEQ_CATEGORY = "UEQ"
FIFO = "FIFOWeight(ConstPrio)"
GENERAL_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
ORIENT_WEIGHT = "OrientLMaxWeight(ConstPrio,2,1,2,1,1)"
GOAL_WEIGHT = "Refinedweight(PreferGoals,1,1,1.5,1.1,1.1)"
GENERAL_STRATEGIES = ("auto_general", "manual_general")
SPECIALIST_STRATEGIES = (
    "completion_queue",
    "completion_presat",
    "completion_simul",
    "completion_strong_rw",
    "completion_lpo",
    "completion_ac_units",
    "completion_initial",
)
STRATEGIES: dict[str, dict[str, Any]] = {
    "auto_general": {
        "kind": "general_auto",
        "features": ["automatic_problem_classification"],
        "args": ["--auto"],
    },
    "manual_general": {
        "kind": "general_manual",
        "features": ["age_weight_given_clause", "kbo6"],
        "args": [
            f"--expert-heuristic=(5*{GENERAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
        ],
    },
    "completion_queue": {
        "kind": "completion",
        "features": [
            "orientability_queue",
            "no_literal_selection",
            "horn_factoring_elision",
            "full_forward_demodulation",
            "kbo6",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
        ],
    },
    "completion_presat": {
        "kind": "completion",
        "features": [
            "completion_queue",
            "presaturation_interreduction",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
        ],
    },
    "completion_simul": {
        "kind": "completion",
        "features": [
            "completion_presat",
            "simultaneous_paramodulation",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
            "--simul-paramod",
        ],
    },
    "completion_strong_rw": {
        "kind": "completion",
        "features": [
            "completion_presat",
            "strong_rewrite_instantiation",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
            "--strong-rw-inst",
        ],
    },
    "completion_lpo": {
        "kind": "completion",
        "features": [
            "completion_presat",
            "lpo4",
            "inverse_frequency_precedence",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=LPO4",
            "--order-precedence-generation=invfreq",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
        ],
    },
    "completion_ac_units": {
        "kind": "completion",
        "features": [
            "completion_presat",
            "retain_unit_ac_axioms",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
            "--ac-handling=KeepUnits",
        ],
    },
    "completion_initial": {
        "kind": "completion",
        "features": [
            "completion_presat",
            "initial_equations_first",
        ],
        "args": [
            f"--expert-heuristic=(5*{ORIENT_WEIGHT},2*{GOAL_WEIGHT},1*{FIFO})",
            "--term-ordering=KBO6",
            "--literal-selection-strategy=NoSelection",
            "--disable-eq-factoring",
            "--forward-demod-level=2",
            "--presat-simplify=true",
            "--prefer-initial-clauses",
        ],
    },
}
PHASE_CONFIGS: dict[str, dict[str, Any]] = {
    "calibration": {
        "split": "train",
        "target_problems": 28,
        "repetitions": 1,
        "budgets": {
            "calibration": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6}
        },
        "proof_objects": False,
    },
    "validation": {
        "split": "validation",
        "target_problems": 20,
        "repetitions": 2,
        "budgets": {
            "validation": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": False,
    },
    "test": {
        "split": "test",
        "target_problems": 20,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    },
}


class ExperimentError(RuntimeError):
    """A contract, corpus, selection, or execution failure."""


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
        return []
    if count == 1:
        return [records[len(records) // 2]]
    indices = [
        round(index * (len(records) - 1) / (count - 1))
        for index in range(count)
    ]
    if len(set(indices)) != count:
        raise ExperimentError("evenly spaced selection produced duplicate indices")
    return [records[index] for index in indices]


def select_family_balanced_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    candidates = sorted(
        (
            record
            for record in records
            if record["category"] == UEQ_CATEGORY
            and record["holdout_split"] == split
        ),
        key=lambda record: (
            record["official_category_order"],
            record["problem_id"],
        ),
    )
    if target < 1 or target > len(candidates):
        raise ExperimentError(
            f"target {target} is invalid for {len(candidates)} {split} records"
        )
    by_family: dict[str, list[dict[str, Any]]] = {}
    for record in candidates:
        by_family.setdefault(record["family"], []).append(record)
    families = sorted(by_family)
    quotas = {family: 0 for family in families}
    remaining = target
    while remaining:
        progressed = False
        for family in families:
            if quotas[family] < len(by_family[family]):
                quotas[family] += 1
                remaining -= 1
                progressed = True
                if remaining == 0:
                    break
        if not progressed:
            raise ExperimentError("family quota allocation stalled")
    selected = [
        record
        for family in families
        for record in evenly_spaced(by_family[family], quotas[family])
    ]
    selected.sort(
        key=lambda record: (
            record["official_category_order"],
            record["problem_id"],
        )
    )
    if len(selected) != target or len(
        {record["problem_id"] for record in selected}
    ) != target:
        raise ExperimentError("family-balanced selection is not unique")
    if {record["family"] for record in selected} != set(families):
        raise ExperimentError("family-balanced selection omitted a family")
    return selected


def verify_selected_corpus(
    problem_root: Path, selected: Sequence[dict[str, Any]]
) -> None:
    for record in selected:
        path = problem_root / record["path"]
        if not path.is_file():
            raise ExperimentError(f"missing selected problem: {path}")
        if sha256_file(path) != record["sha256"]:
            raise ExperimentError(f"problem hash mismatch: {record['problem_id']}")
        for include in record["includes"]:
            include_path = problem_root / "problems" / "casc_2025" / include
            if not include_path.is_file():
                raise ExperimentError(f"missing selected include: {include_path}")


def load_selection(
    path: Path, *, source_phase: str, count: int
) -> tuple[dict[str, Any], str]:
    if not path.is_file():
        raise ExperimentError(f"missing {source_phase} selection: {path}")
    selection = json.loads(path.read_text(encoding="utf-8"))
    body = {
        key: value
        for key, value in selection.items()
        if key != "selection_id"
    }
    expected_id = hashlib.sha256(canonical_json(body)).hexdigest()
    if selection.get("selection_id") != expected_id:
        raise ExperimentError(f"invalid selection ID: {path}")
    if selection.get("source_phase") != source_phase:
        raise ExperimentError(
            f"selection source phase is not {source_phase}: {path}"
        )
    chosen = selection.get("selected_strategies")
    if not isinstance(chosen, list) or len(chosen) != count:
        raise ExperimentError(
            f"{source_phase} selection must contain {count} strategies"
        )
    if any(
        name not in SPECIALIST_STRATEGIES
        or STRATEGIES[name]["kind"] != "completion"
        for name in chosen
    ):
        raise ExperimentError("selection contains a non-specialist strategy")
    return selection, sha256_file(path)


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[dict[str, dict[str, Any]], dict[str, Any] | None, str | None]:
    if phase == "calibration":
        return dict(STRATEGIES), None, None
    if selection_path is None:
        raise ExperimentError(f"--selection is required for {phase}")
    source_phase, count = (
        ("calibration", 3) if phase == "validation" else ("validation", 1)
    )
    selection, selection_sha256 = load_selection(
        selection_path, source_phase=source_phase, count=count
    )
    names = [*GENERAL_STRATEGIES, *selection["selected_strategies"]]
    return (
        {name: STRATEGIES[name] for name in names},
        selection,
        selection_sha256,
    )


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
        run_dir = result_path.parent
        telemetry_path = run_dir / "telemetry.json"
        return (
            result["contract_id"] == contract_id
            and result["problem_sha256"] == problem_sha256
            and result["binary_sha256"] == binary_sha256
            and sha256_file(run_dir / "stdout.txt") == result["stdout_sha256"]
            and sha256_file(run_dir / "stderr.txt") == result["stderr_sha256"]
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
    phase_root: Path,
    contract_id: str,
    phase: str,
    record: dict[str, Any],
    strategy_name: str,
    strategy: dict[str, Any],
    budget_name: str,
    budget: dict[str, int],
    repetition: int,
    memory_mib: int,
    proof_objects: bool,
) -> dict[str, Any]:
    run_dir = (
        phase_root
        / "runs"
        / budget_name
        / strategy_name
        / record["family"]
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
    proof_args = ["--tstp-out", "--proof-object=1"] if proof_objects else []
    command = [
        str(binary),
        *strategy["args"],
        *proof_args,
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
    status = final_status(stdout.decode("utf-8", errors="replace"))
    telemetry, telemetry_sha256, telemetry_error = load_optional_telemetry(
        telemetry_path
    )
    result = {
        "schema_version": 1,
        "contract_id": contract_id,
        "phase": phase,
        "problem_id": record["problem_id"],
        "problem_sha256": record["sha256"],
        "problem_path": record["path"],
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
        "expected_status_match": status in PROOF_STATUSES,
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
    parser.add_argument("--phase", choices=tuple(PHASE_CONFIGS), required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--selection", type=Path)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--memory-mib", type=int, default=1536)
    parser.add_argument("--smoke", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("prover experiments may run only on Linux")
    if arguments.workers < 1:
        raise ExperimentError("--workers must be positive")
    if arguments.memory_mib < 256:
        raise ExperimentError("--memory-mib must be at least 256")

    phase = arguments.phase
    phase_config = json.loads(canonical_json(PHASE_CONFIGS[phase]))
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    binary = arguments.binary.resolve()
    output_root = arguments.output_root.resolve()
    selection_path = arguments.selection.resolve() if arguments.selection else None
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(f"binary is missing or not executable: {binary}")
    metadata, records = load_manifest(manifest_path)
    selected = select_family_balanced_records(
        records, phase_config["split"], phase_config["target_problems"]
    )
    strategies, selection, selection_sha256 = phase_strategies(
        phase, selection_path
    )
    repetitions = phase_config["repetitions"]
    budgets = phase_config["budgets"]
    proof_objects = phase_config["proof_objects"]
    workers = arguments.workers
    if arguments.smoke:
        selected = selected[:1] if phase == "calibration" else selected[:2]
        strategy_names = (
            list(strategies)
            if phase == "calibration"
            else list(GENERAL_STRATEGIES)
        )
        strategies = {name: strategies[name] for name in strategy_names}
        repetitions = 1
        budgets = {"smoke": {"soft_cpu_seconds": 2, "hard_cpu_seconds": 4}}
        workers = min(workers, 2)
    verify_selected_corpus(problem_root, selected)

    binary_sha256 = sha256_file(binary)
    contract_body = json.loads(
        canonical_json(
            {
                "schema_version": 1,
                "phase": phase,
                "manifest_sha256": sha256_file(manifest_path),
                "manifest_problem_count": metadata["problem_count"],
                "axiom_tree_sha256": metadata["sources"]["axiom_tree_sha256"],
                "category": UEQ_CATEGORY,
                "holdout_split": phase_config["split"],
                "selected_problem_ids": [
                    record["problem_id"] for record in selected
                ],
                "selected_problem_sha256": {
                    record["problem_id"]: record["sha256"] for record in selected
                },
                "selected_families": sorted(
                    {record["family"] for record in selected}
                ),
                "selected_difficulty_bands": sorted(
                    {record["difficulty_band"] for record in selected}
                ),
                "target_problems": phase_config["target_problems"],
                "strategies": strategies,
                "budgets": budgets,
                "repetitions": repetitions,
                "binary_sha256": binary_sha256,
                "harness_sha256": sha256_file(Path(__file__).resolve()),
                "proof_objects": proof_objects,
                "selection": selection,
                "selection_sha256": selection_sha256,
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
    phase_root = output_root / phase
    contract_path = phase_root / "contract.json"
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
            raise ExperimentError(
                f"{phase} output contains an incompatible contract"
            )
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
                phase_root=phase_root,
                contract_id=contract_id,
                phase=phase,
                record=record,
                strategy_name=strategy_name,
                strategy=strategy,
                budget_name=budget_name,
                budget=budget,
                repetition=repetition,
                memory_mib=arguments.memory_mib,
                proof_objects=proof_objects,
            )
            for (
                record,
                strategy_name,
                strategy,
                budget_name,
                budget,
                repetition,
            ) in jobs
        ]
        for future in concurrent.futures.as_completed(pending):
            result = future.result()
            completed_count += 1
            resumed_count += int(result["resumed"])
            result_paths.append(Path(result["result_path"]))
            if completed_count % 25 == 0 or completed_count == len(jobs):
                print(
                    f"{phase}: {completed_count}/{len(jobs)} complete "
                    f"({resumed_count} resumed)",
                    flush=True,
                )
    if arguments.smoke:
        invalid = []
        for result_path in result_paths:
            result = json.loads(result_path.read_text(encoding="utf-8"))
            telemetry_is_valid_or_hard_stop = (
                result["telemetry_present"]
                or result["szs_status"] == "ResourceOut"
            )
            if (
                not telemetry_is_valid_or_hard_stop
                or result["szs_status"] is None
            ):
                invalid.append(str(result_path))
        if invalid:
            raise ExperimentError(
                "smoke runs must emit an SZS status and either valid "
                "telemetry or a hard ResourceOut: "
                + ", ".join(sorted(invalid))
            )
    print(
        f"OK: {phase} contract {contract_id}; {len(selected)} problems; "
        f"{len(jobs)} runs; {resumed_count} resumed"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
