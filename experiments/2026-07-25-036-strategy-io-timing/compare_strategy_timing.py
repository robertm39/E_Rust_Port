#!/usr/bin/env python3
"""Compare the strategy-I/O timing boundary of C and Rust eprover."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


CASES = {
    "post_cnf_print": [
        "--print-strategy",
        "--silent",
        "eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p",
    ],
    "syntax_only_precedence": [
        "--syntax-only",
        "--print-strategy",
        "--silent",
        "eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p",
    ],
    "invalid_input_precedes_strategy": [
        "--print-strategy",
        "--silent",
        "experiments/2026-07-15-005-formula-owner-boundaries/tcf-non-clause.p",
    ],
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(executable: Path, args: list[str], repo: Path) -> dict[str, object]:
    completed = subprocess.run(
        [str(executable), *args],
        cwd=repo,
        check=False,
        capture_output=True,
    )
    return {
        "exit": completed.returncode,
        "stdout_bytes": len(completed.stdout),
        "stdout_sha256": digest(completed.stdout),
        "stderr_bytes": len(completed.stderr),
        "stderr_sha256": digest(completed.stderr),
        "_stdout": completed.stdout,
        "_stderr": completed.stderr,
    }


def public_projection(result: dict[str, object]) -> dict[str, object]:
    return {key: value for key, value in result.items() if not key.startswith("_")}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--c-exe", type=Path, required=True)
    parser.add_argument("--rust-exe", type=Path, required=True)
    parser.add_argument(
        "--expected",
        type=Path,
        default=Path(__file__).with_name("reference.json"),
    )
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    repo = args.repo.resolve()
    c_exe = args.c_exe.resolve()
    rust_exe = args.rust_exe.resolve()
    expected = json.loads(args.expected.read_text(encoding="utf-8"))
    summary: dict[str, object] = {"cases": {}}
    failures: list[str] = []

    for name, case_args in CASES.items():
        c_result = run(c_exe, case_args, repo)
        rust_result = run(rust_exe, case_args, repo)
        expected_result = expected["cases"][name]
        c_public = public_projection(c_result)
        rust_public = public_projection(rust_result)
        exact = (
            c_public == rust_public == expected_result
            and c_result["_stdout"] == rust_result["_stdout"]
            and c_result["_stderr"] == rust_result["_stderr"]
        )
        summary["cases"][name] = {
            "args": case_args,
            "exact": exact,
            "result": rust_public,
        }
        if not exact:
            failures.append(name)

    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output is not None:
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    if failures:
        raise SystemExit("strategy timing mismatch: " + ", ".join(failures))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
