#!/usr/bin/env python3
"""Compare focused higher-order matching, single-MGU, and CSU behavior."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


CSU_OPTIONS = [
    "--unif-mode=multi",
    "--pattern-oracle=false",
    "--fixpoint-oracle=false",
    "--func-proj-limit=1",
    "--imit-limit=1",
    "--max-unifiers=4",
    "--max-unif-steps=32",
    "--output-level=2",
    "--processed-clauses-limit=1",
]
ORDERINGS = ("KBO", "KBO6", "LPO", "LPOCopy", "LPO4", "LPO4Copy")
DIRECT_FIXTURES = ("eta-wrapper", "flex-flex", "rigid-prefix")


def cases() -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {
        "applied_variable_match": {
            "kind": "match",
            "fixture": "experiments/2026-07-18-093-higher-order-match-csu-ownership/applied-variable-match.p",
            "options": [
                "--output-level=2",
                "--print-statistics",
                "--processed-clauses-limit=2",
            ],
        },
        "branching_eq_resolution": {
            "kind": "eq_resolution",
            "fixture": "experiments/2026-07-17-025-equality-resolution-branching-csu/input.p",
            "options": CSU_OPTIONS,
        },
        "branching_eq_factor": {
            "kind": "eq_factor",
            "fixture": "experiments/2026-07-15-001-equality-factor-multicsu/input.p",
            "options": CSU_OPTIONS,
        },
    }
    for fixture in DIRECT_FIXTURES:
        for ordering in ORDERINGS:
            result[f"direct_mgu_{fixture}_{ordering}"] = {
                "kind": "direct_mgu",
                "fixture": f"experiments/2026-07-15-003-lfho-paramod-direct-mgu/{fixture}.p",
                "options": [
                    f"--term-ordering={ordering}",
                    "--literal-selection-strategy=NoSelection",
                    "--pm-from-index=NoIndex",
                    "--pm-into-index=NoIndex",
                    "--processed-clauses-limit=2",
                    "--output-level=2",
                    "--print-statistics",
                ],
            }
    return result


def normalize_ids(line: str) -> str:
    return re.sub(r"c_0_\d+", "c_0_N", line.strip())


def counter(stdout: str, name: str) -> int:
    prefix = f"% {name}"
    lines = [line for line in stdout.splitlines() if line.startswith(prefix)]
    if len(lines) != 1:
        raise RuntimeError(f"expected one {name!r} counter, got {lines}")
    return int(lines[0].rsplit(":", 1)[1])


def summarize(kind: str, completed: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    result: dict[str, Any] = {"stderr": stderr}
    if kind == "match":
        result["final_clauses"] = [
            normalize_ids(line) for line in stdout.splitlines() if ",[\'final\'])." in line
        ]
        result["rewrite_steps"] = counter(stdout, "Total rewrite steps")
    elif kind == "eq_resolution":
        result["exit"] = completed.returncode
        result["inferences"] = [
            normalize_ids(line) for line in stdout.splitlines() if "inference(er" in line
        ]
        result["equation_resolutions"] = counter(stdout, "Equation resolutions")
    elif kind == "eq_factor":
        result["exit"] = completed.returncode
        result["inferences"] = [
            normalize_ids(line) for line in stdout.splitlines() if "inference(ef" in line
        ]
        result["factorizations"] = counter(stdout, "Factorizations")
    else:
        result["exit"] = completed.returncode
        result["inferences"] = [
            normalize_ids(line) for line in stdout.splitlines() if "inference(" in line
        ]
        result["counters"] = {
            name: counter(stdout, name)
            for name in ("Processed clauses", "Generated clauses", "Paramodulations")
        }
    return result


def run_cases(exe: str, repo: Path, selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    reports: dict[str, Any] = {}
    for name, case in selected.items():
        fixture = repo / case["fixture"]
        completed = subprocess.run(
            [exe, *case["options"], str(fixture)],
            check=False,
            capture_output=True,
            timeout=60,
        )
        reports[name] = summarize(case["kind"], completed)
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

    selected = cases()
    if args.worker:
        if args.exe is None or args.repo is None:
            parser.error("--worker requires --exe and --repo")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.repo, selected), sort_keys=True))
        return 0

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")
    repo = Path(__file__).resolve().parents[2]
    rust_results = run_cases(str(args.rust_exe.resolve()), repo, selected)
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
        timeout=600,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    mismatches = [name for name in selected if rust_results[name] != c_results[name]]
    report = {
        "case_count": len(selected),
        "cases": rust_results,
        "mismatches": mismatches,
    }
    report["sha256"] = digest(report)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("comparison differs from the retained reference")
            return 1
    if mismatches:
        print("C/Rust mismatches:")
        for name in mismatches:
            print(f"- {name}")
            print(f"  C: {json.dumps(c_results[name], sort_keys=True)}")
            print(f"  Rust: {json.dumps(rust_results[name], sort_keys=True)}")
        return 1
    print(f"validated {len(selected)} exact C/Rust higher-order unification cases")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
