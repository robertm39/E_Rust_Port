#!/usr/bin/env python3
"""Select bounded higher-order candidates from calibration or validation."""

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
BASE_ANALYZE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "analyze.py"
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("higher_order_gap_base_analyze", BASE_ANALYZE_PATH)


class SelectionError(RuntimeError):
    """An invalid phase or candidate-selection request."""


def ratio(left: float | None, right: float | None) -> float | None:
    if left is None or right in (None, 0):
        return None
    return left / right


def selection_key(summary: dict[str, Any], name: str) -> tuple[Any, ...]:
    cpu = summary["median_solved_cpu_seconds"]
    generated = summary["median_solved_generated"]
    high_water = summary["median_solved_high_water_total"]
    term_storage = summary.get("median_solved_term_storage")
    return (
        -summary["reproducible_solved"],
        float("inf") if cpu is None else cpu,
        float("inf") if generated is None else generated,
        float("inf") if high_water is None else high_water,
        float("inf") if term_storage is None else term_storage,
        name,
    )


def select_candidates(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    count: int,
) -> dict[str, Any]:
    if count < 1:
        raise SelectionError("selection count must be positive")
    if len(contract["budgets"]) != 1:
        raise SelectionError("selection phases must have exactly one budget")
    budget = next(iter(contract["budgets"]))
    baseline = BASE.aggregate_strategy(
        contract, results, "baseline_auto", budget
    )
    candidates = [
        name
        for name, config in contract["strategies"].items()
        if config["kind"] == "ho_candidate"
    ]
    summaries = {
        name: BASE.aggregate_strategy(contract, results, name, budget)
        for name in candidates
    }
    eligibility: dict[str, dict[str, Any]] = {}
    for name in candidates:
        summary = summaries[name]
        high_water_ratio = ratio(
            summary["median_solved_high_water_total"],
            baseline["median_solved_high_water_total"],
        )
        reasons = []
        if summary["contradictory_statuses"]:
            reasons.append("contradictory_status")
        if high_water_ratio is not None and high_water_ratio > 1.25:
            reasons.append("high_water_ratio_above_1.25")
        eligibility[name] = {
            "eligible": not reasons,
            "exclusion_reasons": reasons,
            "median_solved_high_water_ratio": high_water_ratio,
        }
    eligible = [
        name for name in candidates if eligibility[name]["eligible"]
    ]
    if count > len(eligible):
        raise SelectionError(
            f"selection needs {count} eligible candidates, found "
            f"{len(eligible)}"
        )
    ranking = sorted(
        eligible, key=lambda name: selection_key(summaries[name], name)
    )
    body = {
        "schema_version": 1,
        "source_phase": contract["phase"],
        "source_contract_id": contract["contract_id"],
        "source_binary_sha256": contract["binary_sha256"],
        "budget": budget,
        "candidate_strategies": candidates,
        "eligibility": eligibility,
        "selected_strategies": ranking[:count],
        "ranking": [
            {
                "rank": index,
                "strategy": name,
                "reproducible_solved": summaries[name][
                    "reproducible_solved"
                ],
                "median_solved_cpu_seconds": summaries[name][
                    "median_solved_cpu_seconds"
                ],
                "median_solved_generated": summaries[name][
                    "median_solved_generated"
                ],
                "median_solved_high_water_total": summaries[name][
                    "median_solved_high_water_total"
                ],
                "median_solved_term_storage": summaries[name].get(
                    "median_solved_term_storage"
                ),
            }
            for index, name in enumerate(ranking, 1)
        ],
        "rule": (
            "Exclude contradictory-status candidates and candidates above "
            "1.25 baseline median solved high-water clauses. Rank remaining "
            "candidates by reproducible solve count (descending), then "
            "median solved CPU, generated clauses, high-water clauses, term "
            "storage, and strategy name (ascending)."
        ),
    }
    return {
        **body,
        "selection_id": hashlib.sha256(
            BASE.canonical_json(body)
        ).hexdigest(),
    }


def write_selection(path: Path, selection: dict[str, Any]) -> None:
    if path.is_file():
        existing = json.loads(path.read_text(encoding="utf-8"))
        if existing != selection:
            raise SelectionError(
                f"refusing to replace another selection: {path}"
            )
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(BASE.canonical_json(selection) + b"\n")
    temporary.replace(path)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--phase", choices=("calibration", "validation"), required=True
    )
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    contract, results = BASE.load_phase(
        arguments.experiment_root.resolve(), arguments.phase
    )
    count = 2 if arguments.phase == "calibration" else 1
    selection = select_candidates(contract, results, count)
    write_selection(arguments.output.resolve(), selection)
    print(
        f"OK: {arguments.phase} selected "
        f"{', '.join(selection['selected_strategies'])}; "
        f"selection {selection['selection_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        SelectionError,
        BASE.AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
