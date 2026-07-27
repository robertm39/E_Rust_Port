#!/usr/bin/env python3
"""Compare learned proof search and benchmark shared-bank TSM evaluation."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import statistics
import subprocess
import time
from pathlib import Path


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
BASELINE_COMMIT = "1c637bc4"
STAT_LABELS = (
    "Parsed axioms",
    "Initial clauses",
    "Initial clauses in saturation",
    "Processed clauses",
    "Current number of processed clauses",
    "Current number of unprocessed clauses",
)


def run(command: list[str]) -> dict[str, object]:
    started = time.perf_counter()
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "elapsed_seconds": time.perf_counter() - started,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def quote_wsl_shell_metacharacters(argument: str) -> str:
    if any(character.isspace() for character in argument) or any(
        character in argument for character in "<>|&;()$`"
    ):
        return shlex.quote(argument)
    return argument


def c_command(distro: str, executable: str, arguments: list[str]) -> list[str]:
    return [
        "wsl",
        "-d",
        distro,
        "--",
        executable,
        *(quote_wsl_shell_metacharacters(argument) for argument in arguments),
    ]


def definition(kind: str, kb: str) -> str:
    if kind == "TSMWeight":
        params = f"ConstPrio,2,3,0.5,flat,{kb}"
    elif kind == "TSMRWeight":
        params = f"ConstPrio,2,3,4.0,5.0,6.0,0.5,flat,{kb}"
    else:
        raise ValueError(f"unknown learned weight kind: {kind}")
    return (
        f"learned={kind}({params},1,1.0,1.0,Flat,IndexArity,0,"
        "1,0,0,0,0,0)"
    )


def arguments(kind: str, kb: str, problem: str, benchmark: bool) -> list[str]:
    result = [
        "--lop-in",
        "--output-level=0" if benchmark else "--output-level=1",
        f"--define-weight-function={definition(kind, kb)}",
        "--define-heuristic=LearnedSearch=(1*learned)",
        "--expert-heuristic=LearnedSearch",
    ]
    if benchmark:
        result.extend(("--no-preprocessing", "--processed-clauses-limit=0"))
    else:
        result.append("--print-statistics")
    result.append(problem)
    return result


def parse_statistics(stdout: bytes) -> dict[str, int]:
    text = stdout.decode("utf-8", errors="replace")
    parsed: dict[str, int] = {}
    for label in STAT_LABELS:
        match = re.search(rf"^% {re.escape(label)}\s*: (\d+)$", text, re.MULTILINE)
        if match is not None:
            parsed[label] = int(match.group(1))
    return parsed


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = bytes(result["stdout"])
    stderr = bytes(result["stderr"])
    status = re.search(rb"^% SZS status (\S+)$", stdout, re.MULTILINE)
    return {
        "exit_code": result["exit_code"],
        "szs_status": status.group(1).decode("ascii") if status else None,
        "statistics": parse_statistics(stdout),
        "stdout_bytes": len(stdout),
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_bytes": len(stderr),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
    }


def readable(result: dict[str, object]) -> dict[str, object]:
    return {
        "stdout": bytes(result["stdout"]).decode(
            "utf-8", errors="backslashreplace"
        ),
        "stderr": bytes(result["stderr"]).decode(
            "utf-8", errors="backslashreplace"
        ),
    }


def generate_benchmark_problem(path: Path, clause_count: int) -> None:
    lines = [
        f"f(g(h(a{index % 64})))=a{index % 64}."
        for index in range(clause_count)
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def benchmark(
    name: str,
    command: list[str],
    runs: int,
) -> dict[str, object]:
    warmup = run(command)
    measured = [run(command) for _ in range(runs)]
    elapsed = [float(result["elapsed_seconds"]) for result in measured]
    return {
        "name": name,
        "warmup_exit_code": warmup["exit_code"],
        "exit_codes": [result["exit_code"] for result in measured],
        "stderr_bytes": [len(bytes(result["stderr"])) for result in measured],
        "runs_seconds": elapsed,
        "median_seconds": statistics.median(elapsed),
        "minimum_seconds": min(elapsed),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--rust-baseline-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--runs", default=5, type=int)
    parser.add_argument("--benchmark-clauses", default=4000, type=int)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    problem = experiment_dir / "problem.lop"
    kb = experiment_dir / "kb"
    rust_kb = kb.resolve().relative_to(Path.cwd().resolve()).as_posix()
    c_problem = wsl_path(problem)
    c_kb = wsl_path(kb)

    comparisons: list[dict[str, object]] = []
    for kind in ("TSMWeight", "TSMRWeight"):
        rust = run(
            [
                str(args.rust_exe.resolve()),
                *arguments(kind, rust_kb, str(problem.resolve()), False),
            ]
        )
        c = run(
            c_command(
                args.distro,
                args.c_exe,
                arguments(kind, c_kb, c_problem, False),
            )
        )
        rust_summary = summarize(rust)
        c_summary = summarize(c)
        comparison: dict[str, object] = {
            "case": kind,
            "status_match": (
                rust_summary["exit_code"] == c_summary["exit_code"] == 0
                and rust_summary["szs_status"] == c_summary["szs_status"]
                and rust_summary["szs_status"] == "Unsatisfiable"
            ),
            "statistics_match": (
                rust_summary["statistics"] == c_summary["statistics"]
            ),
            "stderr_match": (
                rust_summary["stderr_bytes"] == c_summary["stderr_bytes"] == 0
            ),
            "rust": rust_summary,
            "c": c_summary,
        }
        if not all(
            bool(comparison[key])
            for key in ("status_match", "statistics_match", "stderr_match")
        ):
            comparison["mismatch"] = {"rust": readable(rust), "c": readable(c)}
        comparisons.append(comparison)

    benchmark_problem = args.output.resolve().parent / "benchmark-problem.generated.lop"
    generate_benchmark_problem(benchmark_problem, args.benchmark_clauses)
    rust_benchmark_args = arguments(
        "TSMWeight",
        rust_kb,
        str(benchmark_problem),
        True,
    )
    c_benchmark_args = arguments(
        "TSMWeight",
        c_kb,
        wsl_path(benchmark_problem),
        True,
    )
    benchmarks = [
        benchmark(
            "rust_shared_bank",
            [str(args.rust_exe.resolve()), *rust_benchmark_args],
            args.runs,
        ),
        benchmark(
            "rust_private_bank_baseline",
            [str(args.rust_baseline_exe.resolve()), *rust_benchmark_args],
            args.runs,
        ),
        benchmark(
            "c_reference",
            c_command(args.distro, args.c_exe, c_benchmark_args),
            args.runs,
        ),
    ]
    medians = {item["name"]: float(item["median_seconds"]) for item in benchmarks}
    rendered = json.dumps(
        {
            "reference_commit": REFERENCE_COMMIT,
            "rust_baseline_commit": BASELINE_COMMIT,
            "comparison_count": len(comparisons),
            "matching_comparisons": sum(
                all(
                    bool(comparison[key])
                    for key in ("status_match", "statistics_match", "stderr_match")
                )
                for comparison in comparisons
            ),
            "comparisons": comparisons,
            "benchmark_clause_count": args.benchmark_clauses,
            "benchmark_runs": args.runs,
            "benchmarks": benchmarks,
            "median_ratios": {
                "shared_over_private_baseline": (
                    medians["rust_shared_bank"]
                    / medians["rust_private_bank_baseline"]
                ),
                "shared_over_c_reference": (
                    medians["rust_shared_bank"] / medians["c_reference"]
                ),
            },
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)
    comparisons_ok = all(
        all(
            bool(comparison[key])
            for key in ("status_match", "statistics_match", "stderr_match")
        )
        for comparison in comparisons
    )
    benchmark_exit_codes = {
        int(code)
        for item in benchmarks
        for code in [item["warmup_exit_code"], *item["exit_codes"]]
    }
    benchmark_stderr_empty = all(
        all(int(size) == 0 for size in item["stderr_bytes"])
        for item in benchmarks
    )
    if not comparisons_ok or len(benchmark_exit_codes) != 1 or not benchmark_stderr_empty:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
