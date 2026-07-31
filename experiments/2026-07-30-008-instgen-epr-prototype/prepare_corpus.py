#!/usr/bin/env python3
"""Extract and verify only the frozen source files from the CASC archive."""

from __future__ import annotations

import argparse
import hashlib
import json
import tarfile
from pathlib import Path, PurePosixPath


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    arguments = parser.parse_args()
    records = [
        json.loads(line)
        for line in arguments.manifest.read_text(encoding="utf-8").splitlines()
        if line
    ]
    selected = {
        str(PurePosixPath(record["path"])): record
        for record in records
        if record.get("record_type") != "manifest"
    }
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    found: set[str] = set()
    with tarfile.open(arguments.archive, "r:*") as archive:
        for member in archive:
            name = str(PurePosixPath(member.name))
            if name.startswith("./"):
                name = name[2:]
            record = selected.get(name)
            if record is None:
                continue
            if not member.isfile():
                raise ValueError(f"selected archive member is not a file: {name}")
            target = arguments.output_root / Path(*PurePosixPath(name).parts)
            resolved = target.resolve()
            if arguments.output_root.resolve() not in resolved.parents:
                raise ValueError(f"unsafe archive member: {name}")
            target.parent.mkdir(parents=True, exist_ok=True)
            source = archive.extractfile(member)
            if source is None:
                raise ValueError(f"cannot read archive member: {name}")
            target.write_bytes(source.read())
            if sha256_file(target) != record["sha256"]:
                raise ValueError(f"source hash mismatch: {record['problem_id']}")
            found.add(name)
    missing = sorted(set(selected) - found)
    if missing:
        raise ValueError(f"archive lacks {len(missing)} selected files: {missing}")
    print(
        json.dumps(
            {
                "files": len(found),
                "manifest_sha256": sha256_file(arguments.manifest),
                "archive_sha256": sha256_file(arguments.archive),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
