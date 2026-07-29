#!/usr/bin/env python3
"""Build the minimal frozen CASC-30 corpus archive for this experiment."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
import tarfile
from pathlib import Path, PurePosixPath
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
SELECTION_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-008-stronger-redundancy"
    / "run.py"
)
PHASES = {
    "calibration": ("train", 24),
    "validation": ("validation", 24),
    "test": ("test", 20),
}


class CorpusError(RuntimeError):
    """A manifest, source, or archive integrity failure."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CorpusError(f"cannot load selection helper: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


SELECTION = load_module("sat_subsumption_corpus_selection", SELECTION_PATH)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    with path.open(encoding="utf-8") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    if not rows or rows[0].get("record_type") != "manifest":
        raise CorpusError(f"invalid manifest: {path}")
    if rows[0].get("problem_count") != len(rows) - 1:
        raise CorpusError("manifest problem count mismatch")
    return rows[0], rows[1:]


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    manifest = arguments.manifest.resolve()
    metadata, records = load_manifest(manifest)
    selected_by_phase = {
        phase: SELECTION.select_records(records, split, count)
        for phase, (split, count) in PHASES.items()
    }
    selected = {
        record["problem_id"]: record
        for phase_records in selected_by_phase.values()
        for record in phase_records
    }
    members: set[PurePosixPath] = set()
    for record in selected.values():
        problem_member = PurePosixPath(record["path"])
        problem_path = repo_root / Path(*problem_member.parts)
        if (
            not problem_path.is_file()
            or sha256_file(problem_path) != record["sha256"]
        ):
            raise CorpusError(f"problem hash mismatch: {record['problem_id']}")
        members.add(problem_member)
        members.update(
            PurePosixPath("problems", "casc_2025", include)
            for include in record["includes"]
        )

    member_reports = []
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(arguments.output, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        for member in sorted(members, key=str):
            source = repo_root / Path(*member.parts)
            if not source.is_file():
                raise CorpusError(f"missing corpus member: {source}")
            archive.add(source, arcname=str(member), recursive=False)
            member_reports.append(
                {
                    "path": str(member),
                    "bytes": source.stat().st_size,
                    "sha256": sha256_file(source),
                }
            )

    report = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest),
        "manifest_problem_count": metadata["problem_count"],
        "selection_helper_sha256": sha256_file(SELECTION_PATH),
        "phase_problem_ids": {
            phase: [record["problem_id"] for record in phase_records]
            for phase, phase_records in selected_by_phase.items()
        },
        "unique_problems": len(selected),
        "members": member_reports,
        "member_count": len(member_reports),
        "uncompressed_bytes": sum(
            member["bytes"] for member in member_reports
        ),
        "archive_bytes": arguments.output.stat().st_size,
        "archive_sha256": sha256_file(arguments.output),
    }
    report["report_id"] = hashlib.sha256(canonical_json(report)).hexdigest()
    arguments.report.parent.mkdir(parents=True, exist_ok=True)
    arguments.report.write_bytes(canonical_json(report) + b"\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CorpusError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
