#!/usr/bin/env python3
"""Run the frozen proof-lemma/watchlist held-out comparison on Linux."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_RUN_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "run.py"
)
SOURCE_REVISION = "ce75ea3b68c34ab1640e0f362438a656626a5b0e"
CATEGORIES = ("FNE", "FEQ", "EPU", "UEQ")
PREPARED_ROOT: Path | None = None
CONTROL_HEURISTIC = (
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
    "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
    "5*OrientLMaxWeight(ConstPrio,2,1,2,1,1),"
    "1*FIFOWeight(ConstPrio))"
)
WATCH_HEURISTIC = (
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
    "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
    "5*OrientLMaxWeight(PreferWatchlist,2,1,2,1,1),"
    "1*FIFOWeight(PreferWatchlist))"
)
COMMON_ARGS = (
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
    "--pcl-out",
    "--proof-object=1",
    "--force-deriv=2",
)
STRATEGIES: dict[str, dict[str, Any]] = {
    "control": {
        "kind": "structure_matched_control",
        "mechanism": "none",
        "transfer_mode": "none",
        "args": [
            f"--expert-heuristic={CONTROL_HEURISTIC}",
            *COMMON_ARGS,
        ],
    },
    "watch_same": {
        "kind": "proof_derived_watchlist",
        "mechanism": "watchlist",
        "transfer_mode": "same_category",
        "args": [
            f"--expert-heuristic={WATCH_HEURISTIC}",
            "--static-watchlist=Use inline watchlist type",
            *COMMON_ARGS,
        ],
    },
    "lemma_same": {
        "kind": "target_entailed_explicit_lemmas",
        "mechanism": "explicit_lemma",
        "transfer_mode": "same_category",
        "args": [
            f"--expert-heuristic={CONTROL_HEURISTIC}",
            *COMMON_ARGS,
        ],
    },
    "watch_cross": {
        "kind": "proof_derived_watchlist",
        "mechanism": "watchlist",
        "transfer_mode": "cross_category",
        "args": [
            f"--expert-heuristic={WATCH_HEURISTIC}",
            "--static-watchlist=Use inline watchlist type",
            *COMMON_ARGS,
        ],
    },
    "lemma_cross": {
        "kind": "target_entailed_explicit_lemmas",
        "mechanism": "explicit_lemma",
        "transfer_mode": "cross_category",
        "args": [
            f"--expert-heuristic={CONTROL_HEURISTIC}",
            *COMMON_ARGS,
        ],
    },
}
PHASE_CONFIGS = {
    "validation": {
        "split": "validation",
        "target_problems": 8,
        "repetitions": 2,
        "budgets": {
            "heldout": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        # PCL proof arguments are part of every strategy so the base runner
        # does not append its TSTP output switch.
        "proof_objects": False,
    },
    "test": {
        "split": "test",
        "target_problems": 8,
        "repetitions": 2,
        "budgets": {
            "heldout": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": False,
    },
}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("lemma_watchlist_base_run", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError
_base_run_one = BASE.run_one


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    selected = sorted(
        (
            record
            for record in records
            if record["experiment_split"] == split
        ),
        key=lambda record: (
            CATEGORIES.index(str(record["category"])),
            str(record["selection_rank"]),
        ),
    )
    if len(selected) != target:
        raise ExperimentError(
            f"{split} contains {len(selected)} records, expected {target}"
        )
    if {record["category"] for record in selected} != set(CATEGORIES):
        raise ExperimentError(f"{split} does not cover all frozen categories")
    return selected


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[dict[str, dict[str, Any]], None, None]:
    if phase not in PHASE_CONFIGS:
        raise ExperimentError(f"unsupported phase: {phase}")
    if selection_path is not None:
        raise ExperimentError("--selection is not accepted")
    return dict(STRATEGIES), None, None


def run_one(**kwargs: Any) -> dict[str, Any]:
    if PREPARED_ROOT is None:
        raise ExperimentError("prepared root is not configured")
    strategy_name = str(kwargs["strategy_name"])
    original = kwargs["record"]
    variants = original.get("variants")
    if not isinstance(variants, dict) or strategy_name not in variants:
        raise ExperimentError(
            f"{original['problem_id']} has no {strategy_name} prepared variant"
        )
    variant = variants[strategy_name]
    record = dict(original)
    if strategy_name == "control":
        input_path = kwargs["problem_root"] / str(variant["path"])
    else:
        input_path = PREPARED_ROOT / str(variant["path"])
    if not input_path.is_file():
        raise ExperimentError(f"prepared input is missing: {input_path}")
    observed_sha256 = BASE.sha256_file(input_path)
    if observed_sha256 != variant["sha256"]:
        raise ExperimentError(f"prepared input hash mismatch: {input_path}")
    record["path"] = str(input_path)
    record["sha256"] = observed_sha256
    kwargs["record"] = record
    outcome = _base_run_one(**kwargs)
    result_path = Path(outcome["result_path"])
    result = json.loads(result_path.read_text(encoding="utf-8"))
    expected_additions = {
        "input_wrapper_sha256": observed_sha256,
        "target_problem_sha256": original["target_sha256"],
        "guidance_clause_count": int(variant["guidance_clause_count"]),
        "added_clause_count": int(variant["added_clause_count"]),
        "candidate_ids": list(variant.get("candidate_ids", [])),
        "admissibility_cpu_seconds": float(
            variant.get("admissibility_cpu_seconds", 0.0)
        ),
        "admissibility_wall_seconds": float(
            variant.get("admissibility_wall_seconds", 0.0)
        ),
    }
    if any(result.get(key) != value for key, value in expected_additions.items()):
        result.update(expected_additions)
        BASE.atomic_json(result_path, result)
    return outcome


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.UEQ_CATEGORY = "+".join(CATEGORIES)
    BASE.GENERAL_STRATEGIES = tuple(STRATEGIES)
    BASE.SPECIALIST_STRATEGIES = ()
    BASE.STRATEGIES = STRATEGIES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.select_family_balanced_records = select_records
    BASE.phase_strategies = phase_strategies
    BASE.run_one = run_one


def parse_experiment_args(
    argv: Sequence[str],
) -> tuple[Path, list[str]]:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--prepared-root", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    arguments, remaining = parser.parse_known_args(argv)
    if arguments.source_revision != SOURCE_REVISION:
        raise ExperimentError("source revision differs from preregistration")
    prepared_root = arguments.prepared_root.resolve()
    manifest = prepared_root / "prepared-manifest.jsonl"
    if not manifest.is_file():
        raise ExperimentError(f"prepared manifest is missing: {manifest}")
    rows = [
        json.loads(line)
        for line in manifest.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if (
        not rows
        or rows[0].get("source_revision") != SOURCE_REVISION
        or rows[0].get("problem_count") != 16
    ):
        raise ExperimentError("prepared manifest does not match the frozen contract")
    return prepared_root, remaining


def contract_preview(prepared_root: Path) -> dict[str, Any]:
    manifest = prepared_root / "prepared-manifest.jsonl"
    body = {
        "schema_version": 1,
        "source_revision": SOURCE_REVISION,
        "phase_configs": PHASE_CONFIGS,
        "strategies": STRATEGIES,
        "prepared_manifest_sha256": BASE.sha256_file(manifest),
        "harness_sha256": BASE.sha256_file(Path(__file__).resolve()),
    }
    return {
        **body,
        "preview_id": hashlib.sha256(BASE.canonical_json(body)).hexdigest(),
    }


def main(argv: Sequence[str] | None = None) -> int:
    global PREPARED_ROOT

    actual_argv = list(sys.argv[1:] if argv is None else argv)
    prepared_root, remaining = parse_experiment_args(actual_argv)
    PREPARED_ROOT = prepared_root
    if remaining == ["--contract-preview"]:
        print(
            json.dumps(
                contract_preview(prepared_root), indent=2, sort_keys=True
            )
        )
        return 0
    manifest = prepared_root / "prepared-manifest.jsonl"
    if "--manifest" not in remaining:
        remaining.extend(["--manifest", str(manifest)])
    configure_base()
    return BASE.main(remaining)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error

