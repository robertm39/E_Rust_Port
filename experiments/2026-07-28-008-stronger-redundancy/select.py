#!/usr/bin/env python3
"""Select stronger-redundancy candidates from calibration or validation."""

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


RUN = load_module("stronger_redundancy_run", EXPERIMENT_ROOT / "run.py")
BASE = load_module("stronger_redundancy_base_analyze", BASE_ANALYZE_PATH)


class SelectionError(RuntimeError):
    """An invalid phase or candidate-selection request."""


def selection_key(summary: dict[str, Any], name: str) -> tuple[Any, ...]:
    cpu = summary["median_solved_cpu_seconds"]
    generated = summary["median_solved_generated"]
    high_water = summary["median_solved_high_water_total"]
    return (
        -summary["reproducible_solved"],
        float("inf") if cpu is None else cpu,
        float("inf") if generated is None else generated,
        float("inf") if high_water is None else high_water,
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
    eligible = [
        name
        for name, config in contract["strategies"].items()
        if config["kind"] == "redundancy_candidate"
    ]
    if count > len(eligible):
        raise SelectionError("selection count exceeds eligible candidates")
    summaries = {
        name: BASE.aggregate_strategy(contract, results, name, budget)
        for name in eligible
    }
    ranking = sorted(
        eligible, key=lambda name: selection_key(summaries[name], name)
    )
    body = {
        "schema_version": 1,
        "source_phase": contract["phase"],
        "source_contract_id": contract["contract_id"],
        "source_binary_sha256": contract["binary_sha256"],
        "budget": budget,
        "eligible_strategies": eligible,
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
            }
            for index, name in enumerate(ranking, 1)
        ],
        "rule": (
            "Rank redundancy candidates by reproducible solve count "
            "(descending), median solved CPU, generated clauses, and "
            "high-water clauses (ascending), then strategy name."
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
    count = 3 if arguments.phase == "calibration" else 1
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
