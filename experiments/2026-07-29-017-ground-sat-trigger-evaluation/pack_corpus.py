#!/usr/bin/env python3
"""Create a deterministic archive of the frozen corpus and its includes."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import re
import tarfile
from pathlib import Path, PurePosixPath


INCLUDE = re.compile(
    r"(?im)^\s*include\s*\(\s*(['\"])(?P<path>.+?)\1(?:\s*,|\s*\))"
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_relative(value: str) -> PurePosixPath:
    relative = PurePosixPath(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError(f"unsafe corpus path: {value}")
    return relative


def collect(selection: Path, repository: Path) -> list[Path]:
    pending: list[PurePosixPath] = []
    for line in selection.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        record = json.loads(line)
        if record.get("record_type") == "problem":
            pending.append(safe_relative(str(record["path"])))

    selected: set[PurePosixPath] = set()
    while pending:
        relative = pending.pop()
        if relative in selected:
            continue
        if relative.parts[:2] != ("problems", "casc_2025"):
            raise ValueError(f"path leaves CASC corpus: {relative}")
        source = repository.joinpath(*relative.parts)
        if not source.is_file():
            raise FileNotFoundError(source)
        selected.add(relative)
        text = source.read_text(encoding="utf-8", errors="strict")
        for match in INCLUDE.finditer(text):
            include = safe_relative(match.group("path"))
            pending.append(
                PurePosixPath("problems", "casc_2025", *include.parts)
            )
    return [repository.joinpath(*path.parts) for path in sorted(selected)]


def write_archive(
    output: Path, repository: Path, sources: list[Path]
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw:
        with gzip.GzipFile(
            filename="", mode="wb", fileobj=raw, mtime=0
        ) as compressed:
            with tarfile.open(
                fileobj=compressed,
                mode="w",
                format=tarfile.PAX_FORMAT,
            ) as archive:
                for source in sources:
                    payload = source.read_bytes()
                    info = tarfile.TarInfo(
                        source.relative_to(repository).as_posix()
                    )
                    info.size = len(payload)
                    info.mode = 0o644
                    info.mtime = 0
                    info.uid = 0
                    info.gid = 0
                    info.uname = "root"
                    info.gname = "root"
                    archive.addfile(info, io.BytesIO(payload))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("selection", type=Path)
    parser.add_argument("repository", type=Path)
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    repository = arguments.repository.resolve()
    sources = collect(arguments.selection.resolve(), repository)
    write_archive(arguments.output.resolve(), repository, sources)
    print(
        json.dumps(
            {
                "bytes": arguments.output.stat().st_size,
                "files": len(sources),
                "sha256": sha256_file(arguments.output),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
