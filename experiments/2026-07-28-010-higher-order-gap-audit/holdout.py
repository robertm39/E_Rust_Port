#!/usr/bin/env python3
"""Run the fixed positive-extensionality THF/FOF held-out audit."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_RUN_PATH = EXPERIMENT_ROOT / "run.py"


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("higher_order_gap_holdout_base", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError
STRATEGIES = {
    name: BASE.STRATEGIES[name]
    for name in ("baseline_auto", "pos_ext_all")
}
PHASE_CONFIGS: dict[str, dict[str, Any]] = {
    "pos_ext_holdout": {
        "split": "test",
        "target_problems": 30,
        "repetitions": 2,
        "budgets": {
            "holdout": {"soft_cpu_seconds": 10, "hard_cpu_seconds": 12}
        },
        "proof_objects": True,
    },
    "pos_ext_fof": {
        "split": "test",
        "target_problems": 18,
        "repetitions": 2,
        "budgets": {
            "fof": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7}
        },
        "proof_objects": False,
    },
}
_select_records = BASE.select_records


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    return _select_records(records, split, target)


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[
    dict[str, dict[str, Any]],
    dict[str, Any] | None,
    str | None,
]:
    del selection_path
    if phase not in PHASE_CONFIGS:
        raise ExperimentError(f"unsupported phase: {phase}")
    return dict(STRATEGIES), None, None


def configure_base() -> None:
    runner = BASE.BASE
    runner.__file__ = __file__
    runner.CATEGORIES = BASE.THF_CATEGORIES
    runner.SPLIT_QUOTAS = BASE.THF_SPLIT_QUOTAS
    runner.STRATEGIES = STRATEGIES
    runner.CANDIDATE_NAMES = ("pos_ext_all",)
    runner.PHASE_CONFIGS = PHASE_CONFIGS
    runner.UEQ_CATEGORY = "+".join(BASE.THF_CATEGORIES)
    runner.GENERAL_STRATEGIES = ("baseline_auto",)
    runner.select_records = select_records
    runner.phase_strategies = phase_strategies


def main(argv: Sequence[str] | None = None) -> int:
    BASE.configure_base = configure_base
    return BASE.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
