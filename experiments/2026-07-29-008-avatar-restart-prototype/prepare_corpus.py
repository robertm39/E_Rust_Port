#!/usr/bin/env python3
"""Build the minimal frozen corpus archive for the AVATAR experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import tarfile
from pathlib import Path, PurePosixPath
from typing import Any


class CorpusError(RuntimeError):
    """The frozen manifest or one of its source files is inconsistent."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    repo_root = arguments.repo_root.resolve()
    manifest = arguments.manifest.resolve()
    records = [
        json.loads(line)
        for line in manifest.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if (
        not records
        or records[0].get("record_type") != "manifest"
        or records[0].get("problem_count") != len(records) - 1
    ):
        raise CorpusError("invalid frozen corpus manifest")

    members: list[tuple[PurePosixPath, Path, dict[str, Any]]] = []
    for record in records[1:]:
        member = PurePosixPath(record["path"])
        if member.is_absolute() or ".." in member.parts:
            raise CorpusError(f"unsafe corpus member: {member}")
        source = repo_root.joinpath(*member.parts)
        if not source.is_file() or sha256_file(source) != record["sha256"]:
            raise CorpusError(f"problem hash mismatch: {record['problem_id']}")
        members.append((member, source, record))

    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    member_reports = []
    with tarfile.open(
        arguments.output, "w:gz", format=tarfile.PAX_FORMAT
    ) as archive:
        for member, source, record in sorted(
            members, key=lambda item: str(item[0])
        ):
            archive.add(source, arcname=str(member), recursive=False)
            member_reports.append(
                {
                    "path": str(member),
                    "problem_id": record["problem_id"],
                    "bytes": source.stat().st_size,
                    "sha256": record["sha256"],
                }
            )
    report = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest),
        "problem_count": len(members),
        "members": member_reports,
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


if __name__ == "__main__":
    try:
        main()
    except (CorpusError, OSError, ValueError) as error:
        print(f"corpus error: {error}")
        raise SystemExit(1) from error
