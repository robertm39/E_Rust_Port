#!/usr/bin/env python3
"""Compare observable relevance-pruning order with the C reference."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
LABEL_RE = re.compile(rb"^(?:cnf|fof|tff|tcf|thf)\(([^,]+),", re.MULTILINE)

CASES = (
    (
        "clauses_same_bucket",
        "problem.p",
        2,
        ("goal2", "goal1", "ax6", "ax5", "ax4", "ax3", "ax2", "ax1"),
    ),
    (
        "formulas_same_bucket",
        "formulas.p",
        2,
        ("goal2", "goal1", "ax6", "ax5", "ax4", "ax3", "ax2", "ax1"),
    ),
    (
        "overlapping_three_levels",
        "layers.p",
        3,
        (
            "goal",
            "bridge3",
            "bridge2",
            "bridge1",
            "g3",
            "g2",
            "g1",
            "h2",
            "h1",
        ),
    ),
)


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def labels(result: dict[str, object]) -> list[str]:
    stdout = bytes(result["stdout"])
    return [match.decode("utf-8") for match in LABEL_RE.findall(stdout)]


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = bytes(result["stdout"])
    stderr = bytes(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "labels": labels(result),
        "stdout_bytes": len(stdout),
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_bytes": len(stderr),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--c-runs", type=int, default=5)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    results: list[dict[str, object]] = []
    for case_name, filename, level, expected_tuple in CASES:
        fixture = experiment_dir / filename
        expected = list(expected_tuple)
        common = [
            "--prune",
            "--tstp-in",
            "--tstp-out",
            "--output-level=2",
            f"--rel-pruning-level={level}",
        ]
        rust = run([str(args.rust_exe.resolve()), *common, str(fixture)])
        c_runs = [
            run(
                [
                    "wsl",
                    "-d",
                    args.distro,
                    "--",
                    args.c_exe,
                    *common,
                    wsl_path(fixture),
                ]
            )
            for _ in range(args.c_runs)
        ]
        rust_labels = labels(rust)
        c_label_runs = [labels(result) for result in c_runs]
        order_match = rust_labels == expected and all(
            sequence == expected for sequence in c_label_runs
        )
        status_match = rust["exit_code"] == 0 and all(
            result["exit_code"] == 0 for result in c_runs
        )
        stderr_match = bytes(rust["stderr"]) == b"" and all(
            bytes(result["stderr"]) == b"" for result in c_runs
        )
        results.append(
            {
                "case": case_name,
                "expected_labels": expected,
                "order_match": order_match,
                "status_match": status_match,
                "stderr_match": stderr_match,
                "rust": summarize(rust),
                "c_runs": [summarize(result) for result in c_runs],
            }
        )

    rendered = json.dumps(
        {
            "reference_commit": REFERENCE_COMMIT,
            "case_count": len(results),
            "c_runs_per_case": args.c_runs,
            "matching_cases": sum(
                bool(result["order_match"])
                and bool(result["status_match"])
                and bool(result["stderr_match"])
                for result in results
            ),
            "results": results,
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
