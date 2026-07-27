#!/usr/bin/env python3
"""Compile and capture both unchanged C tagged-local-stack layouts."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
DEFAULT_C_ROOT = (
    "/home/rober/.cache/e-rust-port/sources/"
    f"{REFERENCE_COMMIT}/fol"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-root", default=DEFAULT_C_ROOT)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    parser.add_argument(
        "--build-dir", type=Path, default=Path("target/plocal-tagged-reference")
    )
    return parser.parse_args()


def run(command: list[str], *, cwd: Path | None = None, check: bool = True) -> Any:
    return subprocess.run(
        command,
        cwd=cwd,
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def wsl_path(path: Path, distro: str) -> str:
    return run(
        ["wsl.exe", "-d", distro, "--exec", "wslpath", "-a", str(path.resolve())]
    ).stdout.strip()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wsl_sha256(path: str, distro: str) -> str:
    return run(
        ["wsl.exe", "-d", distro, "--exec", "sha256sum", path]
    ).stdout.split()[0]


def parse_record(stdout: str) -> dict[str, int | str]:
    result: dict[str, int | str] = {}
    for field in stdout.strip().split(","):
        key, value = field.split("=", maxsplit=1)
        result[key] = value if key == "mode" else int(value)
    return result


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    c_repo = repo / "eprover"
    commit = run(["git", "rev-parse", "HEAD"], cwd=c_repo).stdout.strip()
    if commit != REFERENCE_COMMIT:
        raise RuntimeError(f"expected C commit {REFERENCE_COMMIT}, found {commit}")

    experiment = Path(__file__).resolve().parent
    probe = experiment / "probe_tagged_stack.c"
    build_dir = (repo / args.build_dir).resolve()
    build_dir.mkdir(parents=True, exist_ok=True)
    basics = f"{args.c_root}/BASICS"
    archive = f"{basics}/BASICS.a"
    records: list[dict[str, Any]] = []
    for mode, define in (("tagged", "-DTAGGED_POINTERS"), ("portable", None)):
        executable = build_dir / f"probe_tagged_stack_{mode}"
        command = [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "cc",
            "-std=gnu99",
            "-O2",
            "-DNDEBUG",
        ]
        if define is not None:
            command.append(define)
        command.extend(
            [
                "-I",
                basics,
                wsl_path(probe, args.distro),
                archive,
                "-lm",
                "-lpthread",
                "-o",
                wsl_path(executable, args.distro),
            ]
        )
        run(command)
        completed = run(
            [
                "wsl.exe",
                "-d",
                args.distro,
                "--exec",
                wsl_path(executable, args.distro),
            ],
            check=False,
        )
        record = parse_record(completed.stdout) if completed.stdout else {}
        records.append(
            {
                "compile_mode": mode,
                "exit_code": completed.returncode,
                "stdout": completed.stdout,
                "stderr": completed.stderr,
                "record": record,
            }
        )

    checksum = sum((index + 1) * 5 + (index & 3) for index in range(40))
    expected_records = [
        {
            "mode": "tagged",
            "pointer_bytes": 8,
            "tag_bits": 2,
            "tag_mask": 3,
            "size": 64,
            "current": 40,
            "entry_slots": 1,
            "allocated_bytes": 512,
            "checksum": checksum,
        },
        {
            "mode": "portable",
            "pointer_bytes": 8,
            "tag_bits": 2,
            "tag_mask": 3,
            "size": 256,
            "current": 80,
            "entry_slots": 2,
            "allocated_bytes": 2048,
            "checksum": checksum,
        },
    ]
    accepted = all(
        record["exit_code"] == 0
        and record["stderr"] == ""
        and record["record"] == expected
        for record, expected in zip(records, expected_records, strict=True)
    )
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "Linux under WSL 2",
            "basics_archive_sha256": wsl_sha256(archive, args.distro),
            "clb_plocalstacks_h_sha256": sha256(
                c_repo / "BASICS/clb_plocalstacks.h"
            ),
            "clb_plocalstacks_c_sha256": sha256(
                c_repo / "BASICS/clb_plocalstacks.c"
            ),
            "makefile_vars_sha256": sha256(c_repo / "Makefile.vars"),
            "probe_sha256": sha256(probe),
        },
        "results": records,
        "accepted": accepted,
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo, args)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if rendered != expected:
            print(f"tagged-stack mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"unchanged C tagged-stack layouts: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
