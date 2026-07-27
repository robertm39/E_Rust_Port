#!/usr/bin/env python3
"""Compare C/Rust formula-owner executable modes on canonical repo fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


FIXTURES = {
    "nested_quantifier": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/CNFTest.p",
    "nested_existential": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/GROUP1st.p",
    "conjecture": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p",
    "question": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/ans_test06.p",
    "mixed_clause_formula": "eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p",
    "old_tptp_set": "eprover/PROVER/SET366+4+rm_eq_rstfp.tptp",
    "old_tptp_ring": "eprover/PROVER/RNG019-6+rm_eq_rstfp.tptp",
}

MODES = {
    "syntax_only": ["--syntax-only", "--silent"],
    "print_formulas": ["--print-formulas", "--silent"],
    "prune": ["--prune", "--silent"],
    "cnf": ["--cnf", "--silent"],
}


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


def summarize_run(result: dict[str, object]) -> dict[str, object]:
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
    parser.add_argument("--output", type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    repo_root = (
        args.repo_root.resolve()
        if args.repo_root is not None
        else Path(__file__).resolve().parents[2]
    )
    results: list[dict[str, object]] = []
    for fixture_name, relative_path in FIXTURES.items():
        input_path = repo_root / relative_path
        for mode_name, mode_args in MODES.items():
            rust = run([str(args.rust_exe.resolve()), *mode_args, str(input_path)])
            c = run(
                [
                    "wsl",
                    "-d",
                    args.distro,
                    "--",
                    args.c_exe,
                    *mode_args,
                    wsl_path(input_path),
                ]
            )
            exact_match = c == rust
            result: dict[str, object] = {
                "fixture": fixture_name,
                "path": relative_path,
                "mode": mode_name,
                "exact_match": exact_match,
                "c": summarize_run(c),
                "rust": summarize_run(rust),
            }
            if not exact_match:
                result["mismatch"] = {"c": c, "rust": rust}
            results.append(result)

    rendered = json.dumps(
        {
            "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
            "fixture_count": len(FIXTURES),
            "mode_count": len(MODES),
            "case_count": len(results),
            "all_exact_match": all(bool(result["exact_match"]) for result in results),
            "results": results,
        },
        indent=2,
    )
    if args.output is not None:
        args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
