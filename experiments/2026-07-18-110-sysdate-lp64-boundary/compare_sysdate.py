#!/usr/bin/env python3
"""Compile unchanged C SysDate code under WSL and compare its LP64 output."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
MASK_64 = (1 << 64) - 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    parser.add_argument(
        "--build-dir", type=Path, default=Path("target/sysdate-lp64-reference")
    )
    return parser.parse_args()


def run(command: list[str], *, cwd: Path | None = None) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout


def wsl_path(path: Path, distro: str) -> str:
    return run(
        ["wsl.exe", "-d", distro, "--exec", "wslpath", "-a", str(path.resolve())]
    ).strip()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_probe(output: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in output.splitlines():
        key, separator, value = line.partition("=")
        if not separator or key in values:
            raise ValueError(f"invalid probe line: {line!r}")
        values[key] = value
    return values


def render_rust_model(raw: int) -> str:
    return f"{raw & MASK_64:>5}"


def collect(repo: Path, args: argparse.Namespace) -> dict[str, Any]:
    c_repo = repo / "eprover"
    commit = run(["git", "rev-parse", "HEAD"], cwd=c_repo).strip()
    if commit != REFERENCE_COMMIT:
        raise RuntimeError(
            f"expected unchanged C commit {REFERENCE_COMMIT}, found {commit}"
        )

    experiment = Path(__file__).resolve().parent
    build_dir = (repo / args.build_dir).resolve()
    build_dir.mkdir(parents=True, exist_ok=True)
    probe_exe = build_dir / "probe_sysdate"

    basics = c_repo / "BASICS"
    compile_command = [
        "wsl.exe",
        "-d",
        args.distro,
        "--exec",
        "cc",
        "-std=gnu99",
        "-Wall",
        "-Wextra",
        "-I",
        wsl_path(basics, args.distro),
        wsl_path(experiment / "probe_sysdate.c", args.distro),
        wsl_path(basics / "clb_sysdate.c", args.distro),
        "-o",
        wsl_path(probe_exe, args.distro),
    ]
    run(compile_command)
    values = parse_probe(
        run(
            [
                "wsl.exe",
                "-d",
                args.distro,
                "--exec",
                wsl_path(probe_exe, args.distro),
            ]
        )
    )

    expected_abi = {
        "abi.long_bytes": "8",
        "abi.long_bits": "64",
        "abi.long_max": str((1 << 63) - 1),
        "abi.ulong_max": str(MASK_64),
    }
    cases = [
        ("creation", 0),
        ("ordinary", 42),
        ("invalid", -1),
        ("maximum", (1 << 63) - 1),
    ]
    case_results = []
    for name, raw in cases:
        c_output = values.get(f"date.{name}")
        rust_output = render_rust_model(raw)
        case_results.append(
            {
                "name": name,
                "raw": raw,
                "c_output": c_output,
                "rust_model_output": rust_output,
                "exact": c_output == rust_output,
            }
        )

    observed_abi = {key: values.get(key) for key in expected_abi}
    extra_keys = sorted(set(values) - set(expected_abi) - {f"date.{n}" for n, _ in cases})
    abi_exact = observed_abi == expected_abi
    all_exact = abi_exact and all(case["exact"] for case in case_results) and not extra_keys
    return {
        "schema_version": 1,
        "reference": {
            "commit": commit,
            "platform": "Linux under WSL 2",
            "data_model": "LP64",
            "clb_sysdate_h_sha256": sha256(basics / "clb_sysdate.h"),
            "clb_sysdate_c_sha256": sha256(basics / "clb_sysdate.c"),
        },
        "rust_contract": {
            "raw_type": "i64",
            "raw_bytes": 8,
            "host_independent": True,
        },
        "abi": {
            "expected": expected_abi,
            "observed": observed_abi,
            "exact": abi_exact,
        },
        "cases": case_results,
        "unexpected_probe_keys": extra_keys,
        "all_exact": all_exact,
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
            print(f"reference mismatch: {args.output} != {args.expected}", file=sys.stderr)
            return 1
    if not result["all_exact"]:
        print("SysDate LP64 comparison failed", file=sys.stderr)
        return 1
    print("SysDate LP64 comparison: 4/4 exact; ABI exact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
