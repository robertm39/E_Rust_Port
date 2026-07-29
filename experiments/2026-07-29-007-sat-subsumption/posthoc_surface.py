#!/usr/bin/env python3
"""Export the post-hoc SAT-subsumption threshold crossover surface."""

from __future__ import annotations

import argparse
import csv
import importlib.util
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent


def load_analyzer() -> ModuleType:
    spec = importlib.util.spec_from_file_location(
        "sat_subsumption_frozen_analyzer",
        EXPERIMENT_ROOT / "analyze.py",
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--captures-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def surface_rows(
    analyzer: ModuleType, captures_root: Path
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for phase in ("calibration", "validation", "test"):
        phase_root = captures_root / phase
        contract, results = analyzer.load_results(phase_root)
        records = analyzer.load_records(phase_root, results)
        for min_side in range(2, 9):
            for min_main in range(2, 13):
                for min_choices in (0, 4, 8, 16, 32, 64):
                    policy = {
                        "min_side_literals": min_side,
                        "min_main_literals": min_main,
                        "min_positive_choices": min_choices,
                    }
                    selected = analyzer.policy_records(records, policy)
                    if not selected:
                        continue
                    metrics = analyzer.policy_metrics(selected)
                    sample_eligible = (
                        metrics["records"] >= 200
                        and metrics["problems"] >= 6
                    )
                    rows.append(
                        {
                            "phase": phase,
                            "contract_id": contract["contract_id"],
                            **policy,
                            **metrics,
                            "sample_eligible": sample_eligible,
                            "calibration_gate": (
                                sample_eligible
                                and metrics["aggregate_ratio"] <= 0.80
                                and metrics["p95_ratio"] <= 0.90
                                and metrics["maximum_estimated_bytes"]
                                < 256 * 1024
                            ),
                        }
                    )
    return rows


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    analyzer = load_analyzer()
    rows = surface_rows(analyzer, arguments.captures_root.resolve())
    if not rows:
        raise RuntimeError("post-hoc surface has no populated regimes")
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(
            stream, fieldnames=list(rows[0]), lineterminator="\n"
        )
        writer.writeheader()
        writer.writerows(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
