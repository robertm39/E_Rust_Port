#!/usr/bin/env python3
"""Run the preregistered explicit-AC mode experiment on Linux."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "run.py"
)
SELECTION_PATH = EXPERIMENT_ROOT / "selected-problems.json"


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("ac_mode_base_runner", BASE_PATH)
BASE.__doc__ = __doc__
BASE_SHA256 = hashlib.sha256(BASE_PATH.read_bytes()).hexdigest()
FIFO = "FIFOWeight(ConstPrio)"
ORIENT = "OrientLMaxWeight(ConstPrio,2,1,2,1,1)"
GOAL = "Refinedweight(PreferGoals,1,1,1.5,1.1,1.1)"
COMMON_ARGS = [
    f"--expert-heuristic=(5*{ORIENT},2*{GOAL},1*{FIFO})",
    "--term-ordering=KBO6",
    "--literal-selection-strategy=NoSelection",
    "--disable-eq-factoring",
    "--forward-demod-level=2",
    "--presat-simplify=true",
]


def strategy(mode: str) -> dict[str, Any]:
    return {
        "kind": "existing_ac_redundancy_mode",
        "features": ["completion_presat", f"ac_handling_{mode.lower()}"],
        "base_harness_sha256": BASE_SHA256,
        "args": [*COMMON_ARGS, f"--ac-handling={mode}"],
    }


BASE.STRATEGIES = {
    "none": strategy("None"),
    "discard_all": strategy("DiscardAll"),
    "keep_units": strategy("KeepUnits"),
    "keep_orientable": strategy("KeepOrientable"),
}
BASE.GENERAL_STRATEGIES = tuple(BASE.STRATEGIES)
BASE.SPECIALIST_STRATEGIES = ()
BASE.UEQ_CATEGORY = "explicit-ac-ueq-feq"
BASE.PHASE_CONFIGS = {
    "calibration": {
        "split": "train",
        "target_problems": 21,
        "repetitions": 1,
        "budgets": {
            "calibration": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6}
        },
        "proof_objects": False,
    },
    "validation": {
        "split": "validation",
        "target_problems": 16,
        "repetitions": 2,
        "budgets": {
            "validation": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": False,
    },
    "test": {
        "split": "test",
        "target_problems": 4,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    },
}


def selected_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    selection = json.loads(SELECTION_PATH.read_text(encoding="utf-8"))
    phase = {
        "train": "calibration",
        "validation": "validation",
        "test": "test",
    }[split]
    identifiers = selection[phase]
    by_id = {record["problem_id"]: record for record in records}
    selected = [by_id[problem_id] for problem_id in identifiers]
    if len(selected) != target:
        raise BASE.ExperimentError(
            f"{phase}: expected {target} selected problems, found {len(selected)}"
        )
    if any(record["holdout_split"] != split for record in selected):
        raise BASE.ExperimentError(f"{phase}: selected problem has wrong split")
    return selected


def all_strategies(
    phase: str, selection_path: Path | None
) -> tuple[dict[str, dict[str, Any]], None, None]:
    del phase, selection_path
    return dict(BASE.STRATEGIES), None, None


BASE.select_family_balanced_records = selected_records
BASE.phase_strategies = all_strategies


if __name__ == "__main__":
    raise SystemExit(BASE.main())
