#!/usr/bin/env python3
"""Measure incremental packaging cost of the experiment-only native checker."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


LIMIT_BYTES = 256 * 1024


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command(*arguments: str) -> str:
    return subprocess.run(
        list(arguments),
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=30,
    ).stdout.strip()


def size_sections(path: Path) -> dict[str, int]:
    lines = command("size", str(path)).splitlines()
    if len(lines) != 2:
        raise RuntimeError(f"unexpected size output for {path}")
    headings = lines[0].split()
    values = lines[1].split()
    return {
        heading: int(value, 16) if heading == "hex" else int(value)
        for heading, value in zip(headings, values, strict=True)
        if heading != "filename"
    }


def dependencies(path: Path) -> list[str]:
    lines = command("ldd", str(path)).splitlines()
    return sorted(
        line.strip().split()[0]
        for line in lines
        if line.strip() and "=>" in line
    )


def artifact(path: Path) -> dict[str, Any]:
    return {
        "path": str(path),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
        "sections": size_sections(path),
        "dynamic_dependencies": dependencies(path),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--candidate-source", required=True, type=Path)
    parser.add_argument("--cargo-patch", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    baseline = artifact(arguments.baseline)
    candidate = artifact(arguments.candidate)
    added_dependencies = sorted(
        set(candidate["dynamic_dependencies"])
        - set(baseline["dynamic_dependencies"])
    )
    file_delta = candidate["bytes"] - baseline["bytes"]
    section_delta = (
        candidate["sections"]["dec"] - baseline["sections"]["dec"]
    )
    patch_additions = [
        line[1:]
        for line in arguments.cargo_patch.read_text(encoding="utf-8").splitlines()
        if line.startswith("+") and not line.startswith("+++")
    ]
    only_bin_registration = all(
        not line.strip()
        or line.strip() == "[[bin]]"
        or line.startswith("name = ")
        or line.startswith("path = ")
        for line in patch_additions
    )
    report = {
        "schema": "umlaut-real-ground-native-package-v1",
        "toolchain": {
            "rustc": command("rustc", "-Vv"),
            "cargo": command("cargo", "-V"),
        },
        "baseline": baseline,
        "candidate": candidate,
        "candidate_source_sha256": sha256(arguments.candidate_source),
        "cargo_patch_sha256": sha256(arguments.cargo_patch),
        "file_delta_bytes": file_delta,
        "loaded_section_delta_bytes": section_delta,
        "limit_bytes": LIMIT_BYTES,
        "added_dynamic_dependencies": added_dependencies,
        "cargo_patch_only_registers_bins": only_bin_registration,
        "default_package_delta_bytes": 0,
        "passed": (
            file_delta <= LIMIT_BYTES
            and section_delta <= LIMIT_BYTES
            and not added_dependencies
            and only_bin_registration
        ),
    }
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
