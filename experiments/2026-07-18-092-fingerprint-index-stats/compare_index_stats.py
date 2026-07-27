#!/usr/bin/env python3
"""Compare optional C and Rust fingerprint-index statistics blocks."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


COMMON_OPTIONS = [
    "--output-level=2",
    "--no-preprocessing",
    "--no-generation",
    "--processed-clauses-limit=3",
    "--expert-heuristic=(1*FIFOWeight(ConstPrio))",
    "--detsort-new",
]

CASES = {
    "fp1": ["--fp-index=FP1"],
    "fp7": ["--fp-index=FP7"],
    "null_paramod_into": [
        "--rw-bw-index=FP1",
        "--pm-from-index=FP1",
        "--pm-into-index=NoIndex",
    ],
}


def normalize_pointers(text: str) -> str:
    pointers: dict[str, str] = {}

    def replace(match: re.Match[str]) -> str:
        value = match.group(0)
        if value not in pointers:
            pointers[value] = f"PTR{len(pointers)}"
        return pointers[value]

    return re.sub(r"0x[0-9a-fA-F]+", replace, text)


def extract_block(stdout: str) -> str:
    lines = stdout.replace("\r\n", "\n").splitlines()
    start = next(
        index
        for index, line in enumerate(lines)
        if line.startswith("% Backwards rewriting index :")
    )
    end = next(
        index
        for index in range(start, len(lines))
        if lines[index].startswith("% Paramod-neg-atom index    :")
    )
    return normalize_pointers("\n".join(lines[start : end + 1]) + "\n")


def run_case(exe: str, fixture: str, options: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        [exe, *COMMON_OPTIONS, *options, fixture],
        check=False,
        capture_output=True,
        timeout=60,
    )
    stdout = completed.stdout.decode("utf-8")
    statuses = re.findall(r"^% SZS status (\S+)", stdout, re.MULTILINE)
    return {
        "exit": completed.returncode,
        "statuses": statuses,
        "stderr": completed.stderr.decode("utf-8").replace("\r\n", "\n"),
        "block": extract_block(stdout),
    }


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def run_worker(args: argparse.Namespace) -> int:
    cases = json.loads(sys.stdin.read())
    results = {
        name: run_case(args.exe, args.fixture, options)
        for name, options in cases.items()
    }
    sys.stdout.write(json.dumps(results, sort_keys=True))
    return 0


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture")
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    if args.worker:
        if args.exe is None or args.fixture is None:
            parser.error("--worker requires --exe and --fixture")
        return run_worker(args)

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")

    fixture = Path(__file__).resolve().parent / "index-stats.p"
    rust_results = {
        name: run_case(str(args.rust_exe.resolve()), str(fixture), options)
        for name, options in CASES.items()
    }
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture",
            windows_to_wsl(fixture),
        ],
        input=json.dumps(CASES).encode("utf-8"),
        check=False,
        capture_output=True,
        timeout=180,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))

    cases = []
    for name in CASES:
        c_result = c_results[name]
        rust_result = rust_results[name]
        cases.append(
            {
                "name": name,
                "exact": c_result == rust_result,
                "c_sha256": digest(c_result),
                "rust_sha256": digest(rust_result),
                "block_sha256": digest(rust_result["block"]),
                "block_lines": len(rust_result["block"].splitlines()),
                "graph_nodes": len(
                    re.findall(r'^\s+lPTR\d+ \[label="', rust_result["block"], re.MULTILINE)
                ),
                "payload_records": rust_result["block"].count("shape=record"),
            }
        )

    report = {
        "schema_version": 1,
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "cases": cases,
        "exact_count": sum(case["exact"] for case in cases),
        "total": len(cases),
        "all_exact": all(case["exact"] for case in cases),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("index-stat report does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_exact"]:
        failed = [case["name"] for case in cases if not case["exact"]]
        print(f"fingerprint-index statistics differ: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
