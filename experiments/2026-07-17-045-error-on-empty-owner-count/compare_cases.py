#!/usr/bin/env python3
"""Compare focused C/Rust --error-on-empty owner-count outcomes."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


CASES = {
    "comment_only": (False, ["comment_only"]),
    "type_only": (True, ["type_only"]),
    "true_only": (True, ["true_only"]),
    "watchlist_only": (False, ["watchlist_only"]),
    "ordinary_clause": (True, ["ordinary_clause"]),
    "comment_then_ordinary": (True, ["comment_only", "ordinary_clause"]),
}

MODES = {
    "syntax_only": ["--syntax-only"],
    "cnf_only": ["--cnf", "--silent"],
}


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "accepted": completed.returncode == 0,
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent
    results: list[dict[str, object]] = []
    for mode, mode_args in MODES.items():
        for name, (expected_accepted, input_names) in CASES.items():
            input_paths = [
                root / "inputs" / f"{input_name}.p" for input_name in input_names
            ]
            common = ["--tstp-in", "--error-on-empty", *mode_args]
            rust = run(
                [
                    str(args.rust_exe.resolve()),
                    *common,
                    *(str(input_path) for input_path in input_paths),
                ]
            )
            c = run(
                [
                    "wsl",
                    "-d",
                    args.distro,
                    "--",
                    args.c_exe,
                    *common,
                    *(wsl_path(input_path) for input_path in input_paths),
                ]
            )
            results.append(
                {
                    "name": name,
                    "mode": mode,
                    "expected_accepted": expected_accepted,
                    "outcomes_match": c["accepted"]
                    == rust["accepted"]
                    == expected_accepted,
                    "exact_match": c == rust,
                    "c": c,
                    "rust": rust,
                }
            )

    print(
        json.dumps(
            {
                "all_outcomes_match": all(
                    bool(result["outcomes_match"]) for result in results
                ),
                "all_exact_match": all(
                    bool(result["exact_match"]) for result in results
                ),
                "results": results,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
