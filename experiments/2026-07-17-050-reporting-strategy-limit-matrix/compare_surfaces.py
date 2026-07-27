#!/usr/bin/env python3
"""Compare stable reporting, strategy, filtering, and limit surfaces."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
import tempfile
from pathlib import Path


NAMED_STRATEGY = "G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN"


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
        "stderr": completed.stderr.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def digest(text: object) -> str:
    return hashlib.sha256(str(text).encode("utf-8")).hexdigest()


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = str(result["stdout"])
    stderr = str(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stdout_sha256": digest(stdout),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "stderr_sha256": digest(stderr),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    repo_root = (
        args.repo_root.resolve()
        if args.repo_root is not None
        else Path(__file__).resolve().parents[2]
    )
    experiment_dir = Path(__file__).resolve().parent
    sat = experiment_dir / "sat.lop"
    unsat = experiment_dir / "unsat.lop"
    answer = repo_root / "eprover/EXAMPLE_PROBLEMS/SMOKETEST/ans_test06.p"
    cases = [
        ("strategy_current", ["--print-strategy"]),
        ("strategy_all_names", ["--print-strategy=>all-names<"]),
        ("strategy_named", [f"--print-strategy={NAMED_STRATEGY}"]),
        (
            "strategy_selected",
            [f"--select-strategy={NAMED_STRATEGY}", "--print-strategy"],
        ),
        (
            "processed_limit",
            [
                "--lop-in",
                "--no-generation",
                "--processed-clauses-limit=0",
                str(sat),
            ],
        ),
        (
            "saturated_limit",
            [
                "--lop-in",
                "--no-generation",
                "--processed-clauses-limit=0",
                "--print-saturated=eigEIG",
                str(sat),
            ],
        ),
        (
            "saturated_filter_noop",
            [
                "--lop-in",
                "--no-generation",
                "--processed-clauses-limit=0",
                "--print-saturated=eigEIG",
                "--filter-saturated=eig",
                str(sat),
            ],
        ),
        ("proof_found_saturated", ["--lop-in", "--print-saturated=e", str(unsat)]),
        ("statistics", ["--lop-in", "--print-statistics", str(sat)]),
        ("answer_limit", ["--answers=1", "--silent", str(answer)]),
    ]

    results: list[dict[str, object]] = []

    def compare(case_name: str, rust_args: list[str], c_args: list[str]) -> None:
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(shlex.quote(arg) for arg in c_args),
            ]
        )
        exact_match = rust == c
        result: dict[str, object] = {
            "case": case_name,
            "exact_match": exact_match,
            "rust": summarize(rust),
            "c": summarize(c),
        }
        if not exact_match:
            result["mismatch"] = {"rust": rust, "c": c}
        results.append(result)

    for case_name, rust_args in cases:
        c_args = [wsl_path(Path(arg)) if arg in {str(sat), str(unsat), str(answer)} else arg for arg in rust_args]
        compare(case_name, rust_args, c_args)

    named_c = next(result for result in results if result["case"] == "strategy_named")
    named_transcript = named_c.get("mismatch", {}).get("c", {}).get("stdout")
    if named_transcript is None:
        named_result = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                f"--print-strategy={NAMED_STRATEGY}",
            ]
        )
        named_transcript = str(named_result["stdout"])
    with tempfile.TemporaryDirectory(dir=repo_root / "target") as temp_dir:
        strategy_path = Path(temp_dir) / "strategy.txt"
        strategy_path.write_text(str(named_transcript), encoding="utf-8")
        compare(
            "strategy_parsed",
            [f"--parse-strategy={strategy_path}", "--print-strategy"],
            [f"--parse-strategy={wsl_path(strategy_path)}", "--print-strategy"],
        )

    rendered = json.dumps(
        {
            "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
            "case_count": len(results),
            "exact_count": sum(bool(result["exact_match"]) for result in results),
            "results": results,
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
