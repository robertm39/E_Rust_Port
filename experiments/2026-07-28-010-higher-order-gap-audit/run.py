#!/usr/bin/env python3
"""Run the staged, family-held-out higher-order gap experiment on Linux."""

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
THF_CATEGORIES = ("TEQ", "TNE")
THF_SPLIT_QUOTAS = {
    "train": {"TEQ": 30, "TNE": 15},
    "validation": {"TEQ": 18, "TNE": 9},
    "test": {"TEQ": 18, "TNE": 12},
}
FOF_CONTROL_QUOTAS = {"FEQ": 6, "FNE": 6, "UEQ": 6}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("higher_order_gap_base_run", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError


def strategy(
    name: str,
    *extra_args: str,
    kind: str = "ho_candidate",
    features: Sequence[str] = (),
) -> dict[str, Any]:
    return {
        "kind": kind,
        "features": [
            "automatic_problem_classification",
            "higher_order",
            name,
            *features,
        ],
        "args": ["--auto", *extra_args],
        "base_harness_sha256": BASE.BASE.sha256_file(BASE_RUN_PATH),
    }


STRATEGIES: dict[str, dict[str, Any]] = {
    "baseline_auto": strategy(
        "baseline_auto",
        kind="baseline",
        features=("established_ho_schedule",),
    ),
    "pos_ext_all": strategy(
        "positive_extensionality_all",
        "--pos-ext=all",
        "--neg-ext=off",
    ),
    "pos_ext_max": strategy(
        "positive_extensionality_maximal",
        "--pos-ext=max",
        "--neg-ext=off",
    ),
    "pos_neg_ext_all": strategy(
        "positive_and_negative_extensionality",
        "--pos-ext=all",
        "--neg-ext=all",
    ),
    "primitive_depth1": strategy(
        "bounded_primitive_enumeration",
        "--prim-enum-mode=pragmatic",
        "--prim-enum-max-depth=1",
    ),
    "choice_depth1": strategy(
        "bounded_choice_instantiation",
        "--inst-choice-max-depth=1",
    ),
    "multi_unif8": strategy(
        "bounded_multi_unification",
        "--unif-mode=multi",
        "--max-unifiers=8",
    ),
}
CANDIDATE_NAMES = tuple(
    name
    for name, config in STRATEGIES.items()
    if config["kind"] == "ho_candidate"
)


PHASE_CONFIGS: dict[str, dict[str, Any]] = {
    "calibration": {
        "split": "train",
        "target_problems": 45,
        "repetitions": 1,
        "budgets": {
            "calibration": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6}
        },
        "proof_objects": False,
    },
    "validation": {
        "split": "validation",
        "target_problems": 27,
        "repetitions": 2,
        "budgets": {
            "validation": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": False,
    },
    "test": {
        "split": "test",
        "target_problems": 30,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    },
    "fof": {
        "split": "test",
        "target_problems": 18,
        "repetitions": 2,
        "budgets": {
            "fof": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7}
        },
        "proof_objects": False,
    },
}


def select_by_quotas(
    records: list[dict[str, Any]],
    split: str,
    quotas: dict[str, int],
) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for category, quota in quotas.items():
        candidates = sorted(
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
        if quota > len(candidates):
            raise ExperimentError(
                f"{category}/{split} has {len(candidates)} records, "
                f"fewer than quota {quota}"
            )
        selected.extend(BASE.family_balanced(candidates, quota))
    selected.sort(
        key=lambda record: (
            tuple(quotas).index(record["category"]),
            record["official_category_order"],
            record["problem_id"],
        )
    )
    return selected


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    if split == "test" and target == sum(FOF_CONTROL_QUOTAS.values()):
        selected = select_by_quotas(records, split, FOF_CONTROL_QUOTAS)
    else:
        quotas = THF_SPLIT_QUOTAS.get(split)
        if quotas is None:
            raise ExperimentError(f"unsupported split: {split}")
        selected = select_by_quotas(records, split, quotas)
    if len(selected) != target:
        raise ExperimentError(
            f"selected {len(selected)} problems, expected {target}"
        )
    if len({record["problem_id"] for record in selected}) != target:
        raise ExperimentError("selected problem IDs are not unique")
    return selected


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
        ("calibration", 2) if phase == "validation" else ("validation", 1)
    )
    selection, selection_sha256 = BASE.load_selection(
        selection_path, source_phase=source_phase, count=count
    )
    names = ["baseline_auto", *selection["selected_strategies"]]
    return (
        {name: STRATEGIES[name] for name in names},
        selection,
        selection_sha256,
    )


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.CATEGORIES = THF_CATEGORIES
    BASE.SPLIT_QUOTAS = THF_SPLIT_QUOTAS
    BASE.STRATEGIES = STRATEGIES
    BASE.CANDIDATE_NAMES = CANDIDATE_NAMES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.UEQ_CATEGORY = "+".join(THF_CATEGORIES)
    BASE.GENERAL_STRATEGIES = ("baseline_auto",)
    BASE.select_records = select_records
    BASE.phase_strategies = phase_strategies


def main(argv: Sequence[str] | None = None) -> int:
    configure_base()
    BASE.configure_base()
    BASE.BASE.__doc__ = __doc__
    BASE.BASE.GENERAL_STRATEGIES = ("baseline_auto",)
    BASE.BASE.SPECIALIST_STRATEGIES = CANDIDATE_NAMES
    return BASE.BASE.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
