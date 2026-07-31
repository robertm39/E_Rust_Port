#!/usr/bin/env python3
"""Extract and verify the frozen 32-problem corpus from the CASC archive."""

from __future__ import annotations

import argparse
import tarfile
from pathlib import Path, PurePosixPath
from typing import Sequence

from common import (
    ExperimentError,
    atomic_json,
    load_corpus,
    sha256_bytes,
    sha256_file,
)


ARCHIVE_SHA256 = (
    "efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d"
)


def safe_member_name(name: str) -> str:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        raise ExperimentError(f"unsafe archive member: {name}")
    return str(path)


def required_paths(problems: list[dict]) -> dict[str, str | None]:
    required: dict[str, str | None] = {}
    for problem in problems:
        problem_path = safe_member_name(str(problem["path"]))
        required[problem_path] = str(problem["sha256"])
        for include in problem.get("includes", []):
            include_path = safe_member_name(
                f"problems/casc_2025/{str(include).replace(chr(92), '/')}"
            )
            required.setdefault(include_path, None)
    return required


def prepare(
    *, archive: Path, manifest: Path, output_root: Path, report: Path
) -> dict:
    if sha256_file(archive) != ARCHIVE_SHA256:
        raise ExperimentError(f"CASC archive hash mismatch: {archive}")
    _, problems = load_corpus(manifest)
    required = required_paths(problems)
    observed: dict[str, dict] = {}
    with tarfile.open(archive, "r:gz") as bundle:
        members = {safe_member_name(member.name): member for member in bundle}
        for relative, expected in sorted(required.items()):
            member = members.get(relative)
            if member is None or not member.isfile() or member.issym() or member.islnk():
                raise ExperimentError(f"missing regular archive member: {relative}")
            handle = bundle.extractfile(member)
            if handle is None:
                raise ExperimentError(f"cannot read archive member: {relative}")
            data = handle.read()
            digest = sha256_bytes(data)
            if expected is not None and digest != expected:
                raise ExperimentError(f"manifest hash mismatch: {relative}")
            destination = output_root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            observed[relative] = {
                "path": relative,
                "sha256": digest,
                "size_bytes": len(data),
            }
    result = {
        "archive": {
            "path": str(archive.resolve()),
            "sha256": ARCHIVE_SHA256,
        },
        "corpus_manifest": {
            "path": str(manifest.resolve()),
            "sha256": sha256_file(manifest),
        },
        "file_count": len(observed),
        "files": [observed[key] for key in sorted(observed)],
        "kind": "cooperative-multicore-prepared-corpus",
        "output_root": str(output_root.resolve()),
        "problem_count": len(problems),
        "schema_version": 1,
    }
    atomic_json(report, result)
    return result


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    result = prepare(
        archive=arguments.archive.resolve(),
        manifest=arguments.manifest.resolve(),
        output_root=arguments.output_root.resolve(),
        report=arguments.report.resolve(),
    )
    print(
        f"prepared {result['problem_count']} problems and "
        f"{result['file_count'] - result['problem_count']} include files"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
