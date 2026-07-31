#!/usr/bin/env python3
"""Emit a compact machine-readable summary from the audited phase result."""

from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path


def macro(record: dict[str, object], metric: str) -> float:
    return float(record["metrics"]["macro"][metric])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    result = json.loads(arguments.result.read_text(encoding="utf-8"))
    linear = result["linear"]
    recursive = result["recursive"]["seeds"]
    selected_seed = result["recursive"]["selected_seed"]
    selected = next(row for row in recursive if row["seed"] == selected_seed)
    summary = {
        "verdict": result["verdict"],
        "phase": result["phase"],
        "selected_seed": selected_seed,
        "validation_problem_count": len(selected["metrics"]["problems"]),
        "validation_clause_count": result["split_counts"]["validation"]["rows"],
        "validation_positive_count": result["split_counts"]["validation"][
            "positives"
        ],
        "macro": {
            "chronological": result["chronological"]["metrics"]["macro"],
            "linear": linear["metrics"]["macro"],
            "recursive_selected": selected["metrics"]["macro"],
        },
        "recursive_seed_average_precision": {
            str(row["seed"]): macro(row, "average_precision") for row in recursive
        },
        "recursive_ap_median": statistics.median(
            macro(row, "average_precision") for row in recursive
        ),
        "recursive_ap_range": result["recursive"]["ap_range"],
        "gate_checks": result["gate_checks"],
        "resources": {
            "phase_wall_seconds": result["resources"].get("phase_wall_seconds"),
            "process_user_cpu_seconds": result["resources"].get(
                "process_user_cpu_seconds"
            ),
            "process_system_cpu_seconds": result["resources"].get(
                "process_system_cpu_seconds"
            ),
            "peak_process_rss_bytes": result["resources"]["peak_process_rss_bytes"],
            "external_worker_peak_rss_bytes": result["recursive"][
                "external_process"
            ].get("worker_peak_rss_bytes"),
            "selected_model_bytes": selected["model_bytes"],
            "in_process_microseconds_per_clause": result["recursive"]["in_process"][
                "microseconds_per_clause"
            ],
            "external_microseconds_per_clause": result["recursive"][
                "external_process"
            ]["microseconds_per_clause"],
            "linear_training_seconds": linear["training_seconds"],
            "recursive_training_seconds_by_seed": {
                str(row["seed"]): row["training_seconds"] for row in recursive
            },
        },
        "test_status": result["test"]["status"],
        "end_to_end_status": result["end_to_end"]["status"],
    }
    text = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if arguments.output is None:
        print(text, end="")
    else:
        arguments.output.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
