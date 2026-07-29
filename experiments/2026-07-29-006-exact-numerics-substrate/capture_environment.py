#!/usr/bin/env python3
"""Capture build and linkage context for the retained experiment evidence."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import pathlib
import platform
import subprocess
from typing import Any


def command_output(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument(
        "--binary",
        type=pathlib.Path,
        action="append",
        default=[],
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    commands = [
        ["uname", "-a"],
        ["lscpu", "--json"],
        ["rustc", "-Vv"],
        ["cargo", "-V"],
        ["gcc", "--version"],
        ["python3", "--version"],
        ["/usr/bin/time", "--version"],
    ]
    payload = {
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "platform": platform.platform(),
        "commands": [command_output(command) for command in commands],
        "binaries": [],
    }
    for binary in args.binary:
        payload["binaries"].append(
            {
                "path": str(binary),
                "bytes": binary.stat().st_size,
                "sha256": sha256_file(binary),
                "file": command_output(["file", str(binary)]),
                "ldd": command_output(["ldd", str(binary)]),
            }
        )
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
