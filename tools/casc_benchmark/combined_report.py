#!/usr/bin/env python3
"""Combine independently contracted complete or partial CASC reports."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Sequence

from batch import BatchError, canonical_json, sha256_file
from manifest import ManifestError, load_manifest
from report import build_report, load_results, overlap_summary, solver_summary

COMBINED_REPORT_SCHEMA_VERSION = 1


def build_combined_report(
    inputs: Sequence[tuple[str, Path, Path]],
    *,
    require_complete: bool = True,
) -> dict[str, Any]:
    if len(inputs) < 2:
        raise BatchError("a combined report requires at least two releases")

    release_reports: dict[str, dict[str, Any]] = {}
    combined_records: list[dict[str, Any]] = []
    combined_results: dict[tuple[str, str], dict[str, Any]] = {}
    expected_solvers: list[str] | None = None
    official_csv_count = 0

    for release, manifest_path, run_root in inputs:
        if release in release_reports:
            raise BatchError(f"duplicate combined-report release {release!r}")
        metadata, records = load_manifest(manifest_path)
        try:
            contract = json.loads(
                (run_root / "contract.json").read_text(encoding="utf-8")
            )
        except (OSError, json.JSONDecodeError) as error:
            raise BatchError(
                f"cannot read run contract for {release}: {error}"
            ) from error
        if sha256_file(manifest_path) != contract.get("manifest_sha256"):
            raise BatchError(f"{release} manifest does not match its run contract")

        per_release = build_report(
            manifest_path, run_root, require_complete=require_complete
        )
        solvers = sorted(contract["solvers"])
        if expected_solvers is None:
            expected_solvers = solvers
        elif solvers != expected_solvers:
            raise BatchError(
                f"{release} solvers {solvers} do not match {expected_solvers}"
            )

        selected_by_id = {record["problem_id"]: record for record in records}
        selected = [
            selected_by_id[problem_id]
            for problem_id in contract["selected_problem_ids"]
        ]
        results = load_results(run_root, contract)
        for record in selected:
            prefixed_id = f"{release}:{record['problem_id']}"
            combined_record = dict(record)
            combined_record["problem_id"] = prefixed_id
            combined_record["release"] = release
            combined_records.append(combined_record)
            for solver in solvers:
                key = (solver, record["problem_id"])
                if key not in results:
                    continue
                result = dict(results[key])
                result["problem_id"] = prefixed_id
                result["release"] = release
                combined_results[(solver, prefixed_id)] = result

        official_files = metadata["sources"]["official_result_file_sha256"]
        official_csv_count += len(official_files)
        release_reports[release] = {
            "corpus": metadata["corpus"],
            "manifest_sha256": contract["manifest_sha256"],
            "contract_id": contract["contract_id"],
            "official_csv_count": len(official_files),
            "summary": per_release,
        }

    if expected_solvers is None:  # pragma: no cover - input length is checked.
        raise AssertionError("combined-report solver set was not initialized")
    expected_results = len(combined_records) * len(expected_solvers)
    missing_results = expected_results - len(combined_results)
    if require_complete and missing_results:
        raise BatchError(
            f"combined results are incomplete: {len(combined_results)}/"
            f"{expected_results}"
        )

    value: dict[str, Any] = {
        "schema_version": COMBINED_REPORT_SCHEMA_VERSION,
        "kind": "umlaut-casc-combined-benchmark-report",
        "complete": missing_results == 0,
        "targeted_problems": len(combined_records),
        "expected_results": expected_results,
        "completed_results": len(combined_results),
        "missing_results": missing_results,
        "official_context": {
            "csv_count": official_csv_count,
            "warning": (
                "Official result CSVs are contextual. Local Umlaut and pinned "
                "Vampire runs are not claimed to reproduce official entries or "
                "the StarExec environment."
            ),
        },
        "releases": dict(sorted(release_reports.items())),
        "solvers": {
            solver: solver_summary(solver, combined_records, combined_results)
            for solver in expected_solvers
        },
    }
    if len(expected_solvers) == 2:
        value["overlap"] = overlap_summary(
            expected_solvers[0],
            expected_solvers[1],
            combined_records,
            combined_results,
        )
    return value


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input",
        action="append",
        nargs=3,
        metavar=("RELEASE", "MANIFEST", "RUN_ROOT"),
        required=True,
        help="add one release as label, manifest path, and run root",
    )
    parser.add_argument(
        "--allow-partial",
        action="store_true",
        help="write a combined report with explicit missing-result counts",
    )
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        inputs = [
            (release, Path(manifest).resolve(), Path(run_root).resolve())
            for release, manifest, run_root in arguments.input
        ]
        value = build_combined_report(
            inputs, require_complete=not arguments.allow_partial
        )
        output = arguments.output.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(canonical_json(value))
        print(
            f"OK: {value['completed_results']}/{value['expected_results']} "
            f"combined results; summary {output}"
        )
        return 0
    except (BatchError, ManifestError, OSError, ValueError, KeyError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
