#!/usr/bin/env python3
"""Compile and capture unchanged C old/new memory policy boundaries."""

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
OLD_REQUESTS = [7, 8, 64, 8191, 8192]
NEW_REQUESTS = [1, 255, 256, 4096, 131_056, 131_057]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-root", default=DEFAULT_C_ROOT)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    parser.add_argument(
        "--build-dir", type=Path, default=Path("target/memory-policy-reference")
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


def expected_record(mode: str, request: int) -> dict[str, int | str]:
    if mode == "old":
        bucket = request if 8 <= request < 8192 else -1
        valid = bucket >= 0
        return {
            "mode": mode,
            "request": request,
            "min": 8,
            "align": 0,
            "chunk_limit": 0,
            "multiplier": 0,
            "bucket": bucket,
            "after_alloc": 0 if valid else -1,
            "after_free": 1 if valid else -1,
            "after_flush": 0 if valid else -1,
        }

    effective = max(request, 8)
    index = (effective + 15) // 16
    bucket = index if index < 8192 else -1
    valid = bucket >= 0
    chunked = valid and effective < 256
    return {
        "mode": mode,
        "request": request,
        "min": 8,
        "align": 16,
        "chunk_limit": 256,
        "multiplier": 1024,
        "bucket": bucket,
        "after_alloc": 1023 if chunked else (0 if valid else -1),
        "after_free": 1024 if chunked else (1 if valid else -1),
        "after_flush": 1024 if chunked else (1 if valid else -1),
    }


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    c_repo = repo / "eprover"
    commit = run(["git", "rev-parse", "HEAD"], cwd=c_repo).stdout.strip()
    if commit != REFERENCE_COMMIT:
        raise RuntimeError(f"expected C commit {REFERENCE_COMMIT}, found {commit}")

    experiment = Path(__file__).resolve().parent
    probe = experiment / "probe_memory_policy.c"
    build_dir = (repo / args.build_dir).resolve()
    build_dir.mkdir(parents=True, exist_ok=True)
    basics = f"{args.c_root}/BASICS"
    archive = f"{basics}/BASICS.a"
    executables: dict[str, Path] = {}
    for mode in ("old", "new"):
        executable = build_dir / f"probe_memory_policy_{mode}"
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
        if mode == "new":
            command.append("-DUSE_NEWMEM")
        command.extend(["-I", basics, wsl_path(probe, args.distro)])
        if mode == "new":
            command.append(f"{basics}/clb_newmem.c")
        command.extend(
            [archive, "-lm", "-lpthread", "-o", wsl_path(executable, args.distro)]
        )
        run(command)
        executables[mode] = executable

    results = []
    for mode, requests in (("old", OLD_REQUESTS), ("new", NEW_REQUESTS)):
        for request in requests:
            completed = run(
                [
                    "wsl.exe",
                    "-d",
                    args.distro,
                    "--exec",
                    wsl_path(executables[mode], args.distro),
                    str(request),
                ],
                check=False,
            )
            record = parse_record(completed.stdout) if completed.stdout else {}
            results.append(
                {
                    "compile_mode": mode,
                    "request": request,
                    "exit_code": completed.returncode,
                    "stdout": completed.stdout,
                    "stderr": completed.stderr,
                    "record": record,
                }
            )

    accepted = all(
        result["exit_code"] == 0
        and result["stderr"] == ""
        and result["record"] == expected_record(result["compile_mode"], result["request"])
        for result in results
    )
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "Linux under WSL 2",
            "basics_archive_sha256": wsl_sha256(archive, args.distro),
            "clb_memory_h_sha256": sha256(c_repo / "BASICS/clb_memory.h"),
            "clb_memory_c_sha256": sha256(c_repo / "BASICS/clb_memory.c"),
            "clb_newmem_h_sha256": sha256(c_repo / "BASICS/clb_newmem.h"),
            "clb_newmem_c_sha256": sha256(c_repo / "BASICS/clb_newmem.c"),
            "probe_sha256": sha256(probe),
        },
        "results": results,
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
            print(f"memory-policy mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    print(f"unchanged C memory policies: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
