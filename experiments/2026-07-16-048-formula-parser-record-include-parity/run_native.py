"""Exercise the permanent eground parser cases against the optimized Rust binary."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


EXPECTED_EXIT_CODES = {
    "eground/help": 0,
    "eground/version": 0,
    "eground/lop-basic": 0,
    "eground/tstp-formula-ground": 0,
    "eground/selected-include": 0,
    "eground/nested-selected-include": 0,
    "eground/verbose-conjecture-progress": 0,
    "eground/dimacs-output-stream-split": 0,
    "eground/malformed-term": 3,
    "eground/trailing-token": 3,
    "eground/non-ground-infinite-universe": 12,
    "eground/give-up-estimate": 0,
    "eground/resource-options-success": 0,
    "eground/invalid-hard-after-soft": 5,
    "eground/invalid-soft-after-hard": 5,
    "eground/missing-input": 6,
    "eground/missing-output-parent": 6,
}


def main() -> None:
    binary = REPO_ROOT / "target" / "release" / "eground.exe"
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}")

    cases = e_interop.tool_comparison_cases(["eground"])
    if [case["name"] for case in cases] != list(EXPECTED_EXIT_CODES):
        raise AssertionError("permanent eground case order changed; update this experiment")

    with tempfile.TemporaryDirectory(prefix="eground-parser-native-") as temporary:
        temporary_root = Path(temporary)
        for index, case in enumerate(cases, 1):
            uses_workdir = bool(
                case.get("isolated_workdir")
                or case.get("workdir_files")
                or case.get("workdir_directories")
                or case.get("output_files")
                or case.get("output_absent_files")
                or case.get("output_directories")
            )
            cwd = temporary_root / f"case-{index:02d}" if uses_workdir else REPO_ROOT
            cwd.mkdir(parents=True, exist_ok=True)
            e_interop.materialize_tool_workdir_directories(case, cwd)
            e_interop.materialize_tool_workdir_files(case, cwd)
            fixture_paths = e_interop.materialize_tool_fixture_files(
                case, temporary_root / "fixtures" / f"case-{index:02d}"
            )
            arguments = e_interop.substitute_tool_fixture_arguments(
                case["arguments"], fixture_paths, windows_paths=True
            )
            result = e_interop.execute(
                binary,
                arguments,
                timeout=30,
                env=os.environ.copy(),
                stdin_text=case["stdin"],
                cwd=cwd,
            )

            expected = EXPECTED_EXIT_CODES[case["name"]]
            if result["exit_code"] != expected:
                raise AssertionError(
                    f"{case['name']}: exit {result['exit_code']} != {expected}\n"
                    f"stdout={result['stdout']!r}\nstderr={result['stderr']!r}"
                )
            for name in case.get("output_files", ()):
                if not (cwd / name).is_file():
                    raise AssertionError(f"{case['name']}: missing output file {name}")
            for name in case.get("output_absent_files", ()):
                if (cwd / name).exists():
                    raise AssertionError(f"{case['name']}: unexpected output path {name}")

            if case["name"] == "eground/nested-selected-include":
                if "p(a)" not in result["stdout"] or "q(a)" in result["stdout"]:
                    raise AssertionError(
                        f"nested selector output changed: {result['stdout']!r}"
                    )

            print(f"PASS {case['name']} exit={result['exit_code']}")

    print(f"All {len(cases)} native eground cases passed")


if __name__ == "__main__":
    main()
