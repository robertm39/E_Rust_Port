#!/usr/bin/env python3
"""Forecast guarded CASC slices from a fully validated checkpoint."""

from __future__ import annotations

import argparse
import collections
import json
import math
import sys
import tempfile
from pathlib import Path
from typing import Any, Sequence

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from validate_casc_checkpoint import (
    SHA256_PATTERN,
    ValidationError,
    canonical_json,
    checkpoint_root_name,
    copy_validated_inner_archive,
    parse_outer_result_count,
    read_inner_archive,
    sha256_file,
    validate_outer_lifecycle_evidence,
    validate_outer_result_inventory,
    validate_run,
)


def distribution_counts(values: Sequence[dict[str, Any]]) -> dict[str, int]:
    counts = collections.Counter(str(value["classification"]) for value in values)
    return dict(sorted(counts.items()))


def build_forecast(
    run: dict[str, Any],
    *,
    session_seconds: int,
    recent_window: int,
) -> dict[str, Any]:
    records = run["_records"]
    results = run["_results"]
    solvers = sorted(run["_contract"]["solvers"])
    coordinates = [
        (index, solver, record)
        for index, record in enumerate(records, start=1)
        for solver in solvers
    ]
    missing = [
        value
        for value in coordinates
        if (value[1], value[2]["problem_id"]) not in results
    ]
    completed = sorted(
        results.values(), key=lambda value: str(value["completed_at"])
    )
    recent = completed[-recent_window:]
    recent_wall_seconds = sum(float(value["wall_seconds"]) for value in recent)
    mean_recent_wall = recent_wall_seconds / len(recent) if recent else None
    projected_new = (
        min(len(missing), math.floor(session_seconds / mean_recent_wall))
        if mean_recent_wall
        else 0
    )
    projected_slices = (
        math.ceil(len(missing) / projected_new) if projected_new else None
    )

    per_solver: dict[str, dict[str, Any]] = {}
    for solver in solvers:
        solver_recent = [value for value in recent if value["solver"] == solver]
        wall = sum(float(value["wall_seconds"]) for value in solver_recent)
        cpu = sum(float(value["cpu_seconds"]) for value in solver_recent)
        per_solver[solver] = {
            "completed": len(solver_recent),
            "classification_counts": distribution_counts(solver_recent),
            "wall_seconds": wall,
            "cpu_seconds": cpu,
            "mean_cpu_cores": cpu / wall if wall else None,
        }

    remaining_limits = collections.Counter(
        (
            str(record["category"]),
            str(record["limit_kind"]),
            int(record["limit_seconds"]),
        )
        for _index, _solver, record in missing
    )
    first_missing = None
    if missing:
        index, solver, record = missing[0]
        first_missing = {
            "manifest_index": index,
            "solver": solver,
            "problem_id": record["problem_id"],
            "category": record["category"],
            "limit_kind": record["limit_kind"],
            "limit_seconds": record["limit_seconds"],
        }
    return {
        "schema_version": 1,
        "kind": "umlaut-casc-checkpoint-forecast",
        "contract_id": run["contract_id"],
        "completed_results": len(results),
        "remaining_results": len(missing),
        "first_missing": first_missing,
        "remaining_timeout_upper_bound_seconds": sum(
            int(record["limit_seconds"]) for _index, _solver, record in missing
        ),
        "remaining_limit_counts": {
            f"{category}|{kind}|{seconds}": count
            for (category, kind, seconds), count in sorted(remaining_limits.items())
        },
        "recent_window": {
            "requested": recent_window,
            "completed": len(recent),
            "wall_seconds": recent_wall_seconds,
            "mean_wall_seconds": mean_recent_wall,
            "classification_counts": distribution_counts(recent),
            "solvers": per_solver,
        },
        "stationary_projection": {
            "warning": (
                "The projection assumes the recent completion-time distribution "
                "continues; later categories and difficulty may differ."
            ),
            "session_seconds": session_seconds,
            "projected_new_results": projected_new,
            "projected_remaining_slices": projected_slices,
        },
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--run-name", required=True)
    parser.add_argument("--contract-id", required=True)
    parser.add_argument("--session-seconds", type=int, default=14400)
    parser.add_argument("--recent-window", type=int, default=100)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        archive = arguments.archive.resolve()
        manifest = arguments.manifest.resolve()
        expected_archive_hash = arguments.archive_sha256.lower()
        contract_id = arguments.contract_id.lower()
        if arguments.session_seconds <= 0 or arguments.recent_window <= 0:
            raise ValidationError("forecast windows must be positive")
        if not archive.is_file() or not manifest.is_file():
            raise ValidationError("forecast archive or manifest is missing")
        if not SHA256_PATTERN.fullmatch(expected_archive_hash):
            raise ValidationError("forecast archive SHA-256 is invalid")
        if not SHA256_PATTERN.fullmatch(contract_id):
            raise ValidationError("forecast contract ID is invalid")
        if sha256_file(archive) != expected_archive_hash:
            raise ValidationError("forecast checkpoint SHA-256 mismatch")

        root = checkpoint_root_name(archive)
        with tempfile.TemporaryDirectory(prefix="umlaut-casc-forecast-") as temporary:
            inner_path = Path(temporary) / "casc-runs.tar.gz"
            outer = copy_validated_inner_archive(archive, root, inner_path)
            hashes, structured, _member_count = read_inner_archive(
                inner_path, [arguments.run_name]
            )
        result_count = parse_outer_result_count(outer["captured"])
        run = validate_run(
            hashes=hashes,
            structured=structured,
            run_name=arguments.run_name,
            manifest_path=manifest,
            contract_id=contract_id,
            expected_results=result_count,
        )
        validate_outer_result_inventory(
            captured=outer["captured"],
            hashes=hashes,
            run_name=arguments.run_name,
            expected_results=result_count,
        )
        validate_outer_lifecycle_evidence(outer["captured"])
        forecast = build_forecast(
            run,
            session_seconds=arguments.session_seconds,
            recent_window=arguments.recent_window,
        )
        output = canonical_json(forecast)
        if arguments.output is not None:
            destination = arguments.output.resolve()
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(output)
        sys.stdout.buffer.write(output)
        return 0
    except (OSError, ValidationError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
