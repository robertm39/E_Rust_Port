#!/usr/bin/env python3
"""Combine prepared workload directories with capture-hash deduplication."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def combine(
    inputs: list[Path], output: Path, prefixes: tuple[str, ...] = ()
) -> dict[str, object]:
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise FileExistsError(f"output directory is not empty: {output}")
    seen: set[str] = set()
    sessions: list[dict[str, object]] = []
    source_sessions = 0
    for root in inputs:
        manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
        for record in manifest["sessions"]:
            if prefixes and not any(
                str(record["session"]).startswith(prefix) for prefix in prefixes
            ):
                continue
            source_sessions += 1
            digest = str(record["capture_sha256"])
            if digest in seen:
                continue
            seen.add(digest)
            source = root / str(record["session"])
            if sha256(source) != record["session_sha256"]:
                raise ValueError(f"{source}: session hash mismatch")
            destination = output / source.name
            if destination.exists():
                destination = output / f"{source.stem}-{digest[:8]}.isat"
            shutil.copyfile(source, destination)
            copied = dict(record)
            copied["session"] = destination.name
            copied["session_sha256"] = sha256(destination)
            copied["source_workload_root"] = str(root)
            sessions.append(copied)
    sessions.sort(key=lambda record: str(record["session"]))
    manifest = {
        "schema": 1,
        "source_roots": [str(root) for root in inputs],
        "source_sessions": source_sessions,
        "unique": len(sessions),
        "sessions": sessions,
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return {
        "source_sessions": source_sessions,
        "unique": len(sessions),
        "ge128": sum(int(record["clauses"]) >= 128 for record in sessions),
        "ge128_problems": sorted(
            {
                str(record["problem_id"])
                for record in sessions
                if int(record["clauses"]) >= 128
            }
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("inputs", type=Path, nargs="+")
    parser.add_argument("--prefix", action="append", default=[])
    arguments = parser.parse_args()
    print(
        json.dumps(
            combine(arguments.inputs, arguments.output, tuple(arguments.prefix)),
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
