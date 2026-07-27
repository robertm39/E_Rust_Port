#!/usr/bin/env python3
"""Compile and capture unchanged C PQueue growth layouts."""

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
        "--build-dir", type=Path, default=Path("target/pqueue-grow-reference")
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


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    c_repo = repo / "eprover"
    commit = run(["git", "rev-parse", "HEAD"], cwd=c_repo).stdout.strip()
    if commit != REFERENCE_COMMIT:
        raise RuntimeError(f"expected C commit {REFERENCE_COMMIT}, found {commit}")

    experiment = Path(__file__).resolve().parent
    probe = experiment / "probe_pqueue.c"
    build_dir = (repo / args.build_dir).resolve()
    build_dir.mkdir(parents=True, exist_ok=True)
    executable = build_dir / "probe_pqueue"
    basics = f"{args.c_root}/BASICS"
    archive = f"{basics}/BASICS.a"
    run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "cc",
            "-std=gnu99",
            "-O2",
            "-DNDEBUG",
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
    expected_stdout = (
        "store=size:8,head:0,tail:4,card:4,slots:10,20,30,40,drain:10,20,30,40\n"
        "bury=size:8,head:0,tail:4,card:4,slots:40,30,20,10,drain:40,30,20,10\n"
        "wrapped=size:8,head:1,tail:5,card:4,copied:5,2,3,4,drain:2,3,4,5\n"
        "direct_full=size:8,head:1,tail:5,card:4,copied:4,1,2,3,drain:1,2,3,4\n"
        "direct_nonfull=size:8,head:2,tail:4,card:6,copied:10,20,30,40,"
        "indices:4,5,6,7,0,1\n"
    )
    accepted = (
        completed.returncode == 0
        and completed.stdout == expected_stdout
        and completed.stderr == ""
    )
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "Linux under WSL 2",
            "basics_archive_sha256": wsl_sha256(archive, args.distro),
            "clb_pqueue_h_sha256": sha256(c_repo / "BASICS/clb_pqueue.h"),
            "clb_pqueue_c_sha256": sha256(c_repo / "BASICS/clb_pqueue.c"),
            "probe_sha256": sha256(probe),
        },
        "result": {
            "exit_code": completed.returncode,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        },
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
            print(f"PQueue layout mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"unchanged C PQueue layouts: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
