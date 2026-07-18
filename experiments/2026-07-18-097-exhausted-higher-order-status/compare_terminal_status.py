#!/usr/bin/env python3
"""Compare exhausted-HO, satisfiable-FO, and theorem terminal statuses."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


EXPERIMENT = "experiments/2026-07-18-097-exhausted-higher-order-status"
CASES = {
    "exhausted_higher_order_axioms": {
        "fixture": f"{EXPERIMENT}/exhausted.p",
        "options": [
            "--output-level=2",
            "--print-statistics",
            "--processed-clauses-limit=2",
        ],
        "counters": (
            "Initial clauses in saturation",
            "Processed clauses",
            "Generated clauses",
            "Total rewrite steps",
        ),
    },
    "first_order_satisfiable": {
        "fixture": f"{EXPERIMENT}/satisfiable.lop",
        "options": ["--lop-in"],
        "counters": (),
    },
    "higher_order_theorem": {
        "fixture": f"{EXPERIMENT}/theorem.p",
        "options": [],
        "counters": (),
    },
}


def normalize_ids(line: str) -> str:
    return re.sub(r"c_0_-?\d+", "c_0_N", line.strip())


def counter(stdout: str, name: str) -> int:
    prefix = f"% {name}"
    lines = [line for line in stdout.splitlines() if line.startswith(prefix)]
    if len(lines) != 1:
        raise RuntimeError(f"expected one {name!r} counter, got {lines}")
    return int(lines[0].rsplit(":", 1)[1])


def terminal_lines(stdout: str) -> list[str]:
    prefixes = (
        "% Proof found!",
        "% No proof found!",
        "% Failure: Out of unprocessed clauses!",
        "% Clause set closed under restricted calculus!",
        "% SZS status ",
    )
    return [line for line in stdout.splitlines() if line.startswith(prefixes)]


def summarize(
    completed: subprocess.CompletedProcess[bytes], counter_names: tuple[str, ...]
) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    return {
        "exit": completed.returncode,
        "stderr": stderr,
        "terminal_lines": terminal_lines(stdout),
        "final_clauses": [
            normalize_ids(line) for line in stdout.splitlines() if ",[\'final\'])." in line
        ],
        "counters": {name: counter(stdout, name) for name in counter_names},
    }


def run_cases(exe: str, repo: Path) -> dict[str, Any]:
    reports: dict[str, Any] = {}
    for name, case in CASES.items():
        completed = subprocess.run(
            [exe, *case["options"], str(repo / case["fixture"])],
            check=False,
            capture_output=True,
            timeout=60,
        )
        reports[name] = summarize(completed, case["counters"])
    return reports


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--repo", type=Path)
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    if args.worker:
        if args.exe is None or args.repo is None:
            parser.error("--worker requires --exe and --repo")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.repo), sort_keys=True))
        return 0

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")
    repo = Path(__file__).resolve().parents[2]
    rust_results = run_cases(str(args.rust_exe.resolve()), repo)
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
            "--repo",
            windows_to_wsl(repo),
        ],
        check=False,
        capture_output=True,
        timeout=180,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    mismatches = [name for name in CASES if rust_results[name] != c_results[name]]
    report = {
        "case_count": len(CASES),
        "cases": rust_results,
        "mismatches": mismatches,
    }
    report["sha256"] = digest(report)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("terminal-status comparison differs from the retained reference")
            return 1
    if mismatches:
        print(f"mismatches: {mismatches}")
        for name in mismatches:
            print(f"  {name} C: {json.dumps(c_results[name], sort_keys=True)}")
            print(f"  {name} Rust: {json.dumps(rust_results[name], sort_keys=True)}")
        return 1
    print(f"validated {len(CASES)} exact C/Rust terminal-status cases")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
