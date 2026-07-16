"""Exercise every permanent edpll support-tool case against the native Rust binary."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


EXPECTED_EXIT_CODES = {
    "edpll/help": 0,
    "edpll/version": 0,
    "edpll/lop-basic": 0,
    "edpll/tptp-input-clause": 0,
    "edpll/trailing-non-clause": 0,
    "edpll/output-file": 0,
    "edpll/malformed-term-after-prefix": 3,
    "edpll/malformed-equation": 3,
    "edpll/empty-procedural-tail": 3,
    "edpll/resource-options-success": 0,
    "edpll/invalid-hard-after-soft": 5,
    "edpll/invalid-soft-after-hard": 5,
    "edpll/missing-input": 6,
    "edpll/missing-output-parent": 6,
}


def main() -> None:
    binary = REPO_ROOT / "target" / "release" / "edpll.exe"
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}")

    cases = e_interop.tool_comparison_cases(["edpll"])
    if [case["name"] for case in cases] != list(EXPECTED_EXIT_CODES):
        raise AssertionError("permanent edpll case order changed; update this experiment")

    with tempfile.TemporaryDirectory(prefix="edpll-native-") as temporary:
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

            if case["name"] == "edpll/malformed-term-after-prefix":
                expected_stdout = "New clause: p<-....accepted\n"
                if result["stdout"] != expected_stdout:
                    raise AssertionError(
                        f"partial trace changed: {result['stdout']!r}"
                    )
            if case["name"] == "edpll/output-file":
                expected_output = (
                    "New clause: p<-....accepted\n"
                    "New clause: q<-r....accepted\n"
                )
                output = (cwd / "trace.out").read_text(encoding="utf-8")
                if output != expected_output:
                    raise AssertionError(f"output trace changed: {output!r}")

            print(f"PASS {case['name']} exit={result['exit_code']}")

    print(f"All {len(cases)} native edpll cases passed")


if __name__ == "__main__":
    main()
