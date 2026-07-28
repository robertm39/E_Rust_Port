#!/usr/bin/env python3
"""Run the staged, family-held-out fingerprint-index bake-off on Linux."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_RUN_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-008-stronger-redundancy"
    / "run.py"
)
CATEGORIES = ("FEQ", "FNE", "UEQ")
SPLIT_QUOTAS = {
    split: {category: 6 for category in CATEGORIES}
    for split in ("train", "validation", "test")
}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("index_retrieval_base_run", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError


def index_strategy(
    variant: str, *, kind: str = "index_candidate"
) -> dict[str, Any]:
    return {
        "kind": kind,
        "features": [
            "fingerprint_index",
            "backward_rewrite",
            "paramodulation_from",
            "paramodulation_into",
            variant,
        ],
        "args": [
            *BASE.COMMON_ARGS,
            f"--fp-index={variant}",
        ],
        "index_variant": variant,
        "base_harness_sha256": BASE.BASE.sha256_file(BASE_RUN_PATH),
    }


STRATEGIES: dict[str, dict[str, Any]] = {
    "baseline_fp7": index_strategy("FP7", kind="baseline"),
    "fp0": index_strategy("FP0"),
    "fp1": index_strategy("FP1"),
    "fp2": index_strategy("FP2"),
    "fp3d": index_strategy("FP3D"),
    "fp3w": index_strategy("FP3W"),
    "fp4m": index_strategy("FP4M"),
    "fp7m": index_strategy("FP7M"),
    "fp4x2_2": index_strategy("FP4X2_2"),
    "npdt": index_strategy("NPDT"),
}
CANDIDATE_NAMES = tuple(
    name
    for name, config in STRATEGIES.items()
    if config["kind"] == "index_candidate"
)

PHASE_CONFIGS: dict[str, dict[str, Any]] = {
    "calibration": {
        "split": "train",
        "target_problems": 18,
        "repetitions": 1,
        "budgets": {
            "calibration": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6}
        },
        "proof_objects": False,
    },
    "validation": {
        "split": "validation",
        "target_problems": 18,
        "repetitions": 2,
        "budgets": {
            "validation": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": False,
    },
    "test": {
        "split": "test",
        "target_problems": 18,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    },
}


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[
    dict[str, dict[str, Any]],
    dict[str, Any] | None,
    str | None,
]:
    if phase == "calibration":
        return dict(STRATEGIES), None, None
    if selection_path is None:
        raise ExperimentError(f"--selection is required for {phase}")
    source_phase, count = (
        ("calibration", 3) if phase == "validation" else ("validation", 1)
    )
    selection, selection_sha256 = BASE.load_selection(
        selection_path, source_phase=source_phase, count=count
    )
    names = ["baseline_fp7", *selection["selected_strategies"]]
    return (
        {name: STRATEGIES[name] for name in names},
        selection,
        selection_sha256,
    )


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.CATEGORIES = CATEGORIES
    BASE.SPLIT_QUOTAS = SPLIT_QUOTAS
    BASE.STRATEGIES = STRATEGIES
    BASE.CANDIDATE_NAMES = CANDIDATE_NAMES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.UEQ_CATEGORY = "+".join(CATEGORIES)
    BASE.GENERAL_STRATEGIES = ("baseline_fp7",)
    BASE.phase_strategies = phase_strategies


def main(argv: Sequence[str] | None = None) -> int:
    configure_base()
    BASE.configure_base()
    BASE.BASE.__doc__ = __doc__
    BASE.BASE.GENERAL_STRATEGIES = ("baseline_fp7",)
    BASE.BASE.SPECIALIST_STRATEGIES = CANDIDATE_NAMES
    return BASE.BASE.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
