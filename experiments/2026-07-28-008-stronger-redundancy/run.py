#!/usr/bin/env python3
"""Run the staged, family-held-out stronger-redundancy experiment on Linux."""

from __future__ import annotations

import hashlib
import importlib.util
import json
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
CATEGORIES = ("FEQ", "FNE", "EPS", "UEQ")
SPLIT_QUOTAS = {
    "train": {"FEQ": 6, "FNE": 6, "EPS": 6, "UEQ": 6},
    "validation": {"FEQ": 6, "FNE": 6, "EPS": 6, "UEQ": 6},
    "test": {"FEQ": 6, "FNE": 6, "EPS": 2, "UEQ": 6},
}
PROOF_EXPECTATIONS = {"theorem", "unsatisfiable"}
NON_PROOF_EXPECTATIONS = {"non_theorem", "satisfiable"}
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NON_PROOF_STATUSES = {"CounterSatisfiable", "Satisfiable"}
BASE_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
FIFO = "FIFOWeight(ConstPrio)"
COMMON_ARGS = [
    f"--expert-heuristic=(5*{BASE_WEIGHT},1*{FIFO})",
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("stronger_redundancy_base_run", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError


def strategy(
    kind: str, features: Sequence[str], *extra_args: str
) -> dict[str, Any]:
    return {
        "kind": kind,
        "features": list(features),
        "args": [*COMMON_ARGS, *extra_args],
        "base_harness_sha256": BASE.sha256_file(BASE_RUN_PATH),
    }


STRATEGIES: dict[str, dict[str, Any]] = {
    "baseline": strategy(
        "baseline",
        ("indexed_subsumption", "full_forward_demodulation"),
    ),
    "baseline_direct": strategy(
        "slow_reference",
        (
            "direct_subsumption",
            "full_forward_demodulation",
        ),
        "--conventional-subsumption",
    ),
    "strong_unit_subsumption": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "strong_unit_forward_subsumption",
            "full_forward_demodulation",
        ),
        "--strong-forward-subsumption",
    ),
    "aggressive_forward_subsumption": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "generated_clause_forward_subsumption",
            "full_forward_demodulation",
        ),
        "--fw-subsumption-aggressive",
    ),
    "contextual_sr": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "selected_clause_contextual_simplify_reflect",
            "full_forward_demodulation",
        ),
        "--forward-context-sr",
    ),
    "contextual_sr_full": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "selected_and_generated_contextual_simplify_reflect",
            "backward_contextual_simplify_reflect",
            "full_forward_demodulation",
        ),
        "--forward-context-sr",
        "--forward-context-sr-aggressive",
        "--backward-context-sr",
    ),
    "condensation": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "selected_clause_condensation",
            "full_forward_demodulation",
        ),
        "--condense",
    ),
    "condensation_full": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "selected_and_generated_clause_condensation",
            "full_forward_demodulation",
        ),
        "--condense",
        "--condense-aggressive",
    ),
    "strong_demodulation": strategy(
        "redundancy_candidate",
        (
            "indexed_subsumption",
            "full_forward_demodulation",
            "strong_rewrite_instantiation",
            "prefer_general_demodulators",
        ),
        "--strong-rw-inst",
        "--prefer-general-demodulators",
    ),
    "redundancy_bundle": strategy(
        "redundancy_candidate",
        (
            "strong_unit_forward_subsumption",
            "generated_clause_forward_subsumption",
            "bidirectional_contextual_simplify_reflect",
            "selected_and_generated_clause_condensation",
            "full_forward_demodulation",
            "strong_rewrite_instantiation",
            "prefer_general_demodulators",
        ),
        "--strong-forward-subsumption",
        "--fw-subsumption-aggressive",
        "--forward-context-sr",
        "--forward-context-sr-aggressive",
        "--backward-context-sr",
        "--condense",
        "--condense-aggressive",
        "--strong-rw-inst",
        "--prefer-general-demodulators",
    ),
}
CANDIDATE_NAMES = tuple(
    name
    for name, config in STRATEGIES.items()
    if config["kind"] == "redundancy_candidate"
)


PHASE_CONFIGS: dict[str, dict[str, Any]] = {
    "calibration": {
        "split": "train",
        "target_problems": 24,
        "repetitions": 1,
        "budgets": {
            "calibration": {"soft_cpu_seconds": 4, "hard_cpu_seconds": 6}
        },
        "proof_objects": False,
    },
    "validation": {
        "split": "validation",
        "target_problems": 24,
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


def family_balanced(
    records: list[dict[str, Any]], count: int
) -> list[dict[str, Any]]:
    by_family: dict[str, list[dict[str, Any]]] = {}
    for record in records:
        by_family.setdefault(record["family"], []).append(record)
    quotas = {family: 0 for family in sorted(by_family)}
    remaining = count
    while remaining:
        progressed = False
        for family in sorted(by_family):
            if quotas[family] < len(by_family[family]):
                quotas[family] += 1
                remaining -= 1
                progressed = True
                if remaining == 0:
                    break
        if not progressed:
            raise ExperimentError("family quota allocation stalled")
    return [
        record
        for family in sorted(by_family)
        for record in BASE.evenly_spaced(by_family[family], quotas[family])
    ]


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    for category in CATEGORIES:
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
        quota = SPLIT_QUOTAS[split][category]
        if quota > len(candidates):
            raise ExperimentError(
                f"{category}/{split} has {len(candidates)} records, "
                f"fewer than quota {quota}"
            )
        selected.extend(family_balanced(candidates, quota))
    selected.sort(
        key=lambda record: (
            CATEGORIES.index(record["category"]),
            record["official_category_order"],
            record["problem_id"],
        )
    )
    if len(selected) != target:
        raise ExperimentError(
            f"selected {len(selected)} problems, expected {target}"
        )
    if len({record["problem_id"] for record in selected}) != target:
        raise ExperimentError("selected problem IDs are not unique")
    return selected


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
    expected_id = hashlib.sha256(BASE.canonical_json(body)).hexdigest()
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
    if any(name not in CANDIDATE_NAMES for name in chosen):
        raise ExperimentError("selection contains a non-candidate strategy")
    return selection, BASE.sha256_file(path)


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
    chosen = selection["selected_strategies"]
    names = ["baseline", *chosen]
    strategies = {name: STRATEGIES[name] for name in names}
    if phase == "test":
        chosen_name = chosen[0]
        chosen_config = STRATEGIES[chosen_name]
        strategies["baseline_direct"] = STRATEGIES["baseline_direct"]
        strategies["selected_direct"] = {
            **chosen_config,
            "kind": "slow_reference",
            "features": [
                *chosen_config["features"],
                "direct_subsumption",
            ],
            "args": [
                *chosen_config["args"],
                "--conventional-subsumption",
            ],
            "reference_for": chosen_name,
        }
    return strategies, selection, selection_sha256


def expected_status_match(record: dict[str, Any], status: str | None) -> bool:
    expected = record["expected_class"]
    if expected in PROOF_EXPECTATIONS:
        return status in PROOF_STATUSES
    if expected in NON_PROOF_EXPECTATIONS:
        return status in NON_PROOF_STATUSES
    raise ExperimentError(f"unsupported expected class: {expected}")


_base_run_one = BASE.run_one


def run_one(**kwargs: Any) -> dict[str, Any]:
    outcome = _base_run_one(**kwargs)
    result_path = Path(outcome["result_path"])
    result = json.loads(result_path.read_text(encoding="utf-8"))
    record = kwargs["record"]
    corrected = expected_status_match(record, result["szs_status"])
    if (
        result.get("expected_status_match") != corrected
        or result.get("expected_class") != record["expected_class"]
    ):
        result["expected_class"] = record["expected_class"]
        result["expected_status_match"] = corrected
        BASE.atomic_json(result_path, result)
    return outcome


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.UEQ_CATEGORY = "+".join(CATEGORIES)
    BASE.GENERAL_STRATEGIES = ("baseline",)
    BASE.SPECIALIST_STRATEGIES = CANDIDATE_NAMES
    BASE.STRATEGIES = STRATEGIES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.select_family_balanced_records = select_records
    BASE.phase_strategies = phase_strategies
    BASE.run_one = run_one


def main(argv: Sequence[str] | None = None) -> int:
    configure_base()
    return BASE.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
