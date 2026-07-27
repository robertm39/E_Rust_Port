"""Exercise every permanent epatternize case against the optimized Rust binary."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


EXPECTED_EXIT_CODES = {
    "epatternize/help": 0,
    "epatternize/version": 0,
    "epatternize/lop-basic": 0,
    "epatternize/old-tptp-record-mix": 0,
    "epatternize/tstp-mixed-corpus": 0,
    "epatternize/nested-selected-include": 0,
    "epatternize/multi-file-output": 0,
    "epatternize/malformed-lop": 3,
    "epatternize/malformed-tstp": 3,
    "epatternize/missing-include": 6,
    "epatternize/missing-input": 6,
    "epatternize/missing-output-parent": 6,
    "epatternize/invalid-class-mask": 5,
}

UNIT_PATTERN = "$or1($eq(f1_1(f0_1),$true))\n"

EXPECTED_STDOUT = {
    "epatternize/version": "epatternize 3.3.5\n",
    "epatternize/lop-basic": UNIT_PATTERN,
    "epatternize/old-tptp-record-mix": (
        UNIT_PATTERN
        + "$or2($eq(f1_1(f0_1),$true),$neq(f1_2(Xn1),$true))\n"
    ),
    "epatternize/tstp-mixed-corpus": (
        "$or1($eq($true,$true))\n"
        "$or1($eq($true,$true))\n"
        "$or1($eq($true,$true))\n"
        "$or1($eq($true,$true))\n"
        "$or1($eq($true,$true))\n"
        "$or1($eq(f1_1(f0_1),$true))\n"
        "$or1($eq(f1_1(f0_1),$true))\n"
        "$or1($eq(f1_1(f1_2(f0_1)),$true))\n"
        "$or1($eq(f1_1(f0_1),$true))\n"
        "$or1($eq(f1_1(f0_1),$true))\n"
        "$or2($neq(f1_1(f0_1),$true),$eq(f1_2(f0_1),$true))\n"
        "$or1($eq(f1_1(f1_2(f0_1)),$true))\n"
        "$or1($eq(f1_1(f0_1),$true))\n"
        "$or2($eq(f1_1(f1_2(Xn1)),$true),$eq(f1_3(Xn1),$true))\n"
        "$or2($neq(f1_1(Xn1),$true),$eq(f1_2(Xn1),$true))\n"
        "$or2($neq(f1_1(f0_1),$true),$eq(f1_2(f0_2),$true))\n"
        "$or2($eq(f1_1(f0_1),f0_2),$eq(f1_2(f0_2),$true))\n"
    ),
    "epatternize/nested-selected-include": (
        UNIT_PATTERN
        + UNIT_PATTERN
        + UNIT_PATTERN
        + UNIT_PATTERN
        + UNIT_PATTERN
        + "$or1($eq(f1_1(Xn1),$true))\n"
        + UNIT_PATTERN
    ),
    "epatternize/multi-file-output": "",
}

EXPECTED_EXACT_STDERR = {
    "epatternize/malformed-lop": (
        "epatternize: -:1:(Column 7):(just read '.'): Closing bracket (')') "
        "expected, but Fullstop ('.') read \n"
    ),
    "epatternize/malformed-tstp": (
        "epatternize: -:1:(Column 19):(just read '.'): Closing bracket (')') "
        "expected, but Fullstop ('.') read \n"
    ),
    "epatternize/invalid-class-mask": (
        "epatternize: Option -c (--class-mask) requires 13-letter string as an argument\n"
    ),
}

EXPECTED_ERROR_FIRST_LINES = {
    "epatternize/missing-include": (
        "epatternize: Cannot open file missing-include.p for reading"
    ),
    "epatternize/missing-input": (
        "epatternize: Cannot open file missing-epatternize-input.p for reading"
    ),
    "epatternize/missing-output-parent": (
        "epatternize: Cannot open file missing/patterns.out"
    ),
}

EXPECTED_OUTPUT_FILES = {
    "epatternize/multi-file-output": {
        "patterns.out": UNIT_PATTERN + UNIT_PATTERN,
    },
}


def main() -> None:
    binary = REPO_ROOT / "target" / "release" / "epatternize.exe"
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}")

    cases = e_interop.tool_comparison_cases(["epatternize"])
    if [case["name"] for case in cases] != list(EXPECTED_EXIT_CODES):
        raise AssertionError("permanent epatternize case order changed; update this experiment")

    with tempfile.TemporaryDirectory(prefix="epatternize-native-") as temporary:
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
            expected_stdout = EXPECTED_STDOUT.get(case["name"])
            if expected_stdout is not None and result["stdout"] != expected_stdout:
                raise AssertionError(
                    f"{case['name']}: unexpected stdout\n"
                    f"expected={expected_stdout!r}\nactual={result['stdout']!r}"
                )
            if case["name"] == "epatternize/help" and not result["stdout"].startswith(
                "\n\nepatternize 3.3.5\n\nUsage: classify_problem [options] [files]\n"
            ):
                raise AssertionError("help banner changed")
            if expected == 0 and result["stderr"]:
                raise AssertionError(
                    f"{case['name']}: unexpected stderr {result['stderr']!r}"
                )
            if expected != 0 and result["stdout"]:
                raise AssertionError(
                    f"{case['name']}: error path wrote stdout {result['stdout']!r}"
                )
            expected_stderr = EXPECTED_EXACT_STDERR.get(case["name"])
            if expected_stderr is not None and result["stderr"] != expected_stderr:
                raise AssertionError(
                    f"{case['name']}: unexpected stderr\n"
                    f"expected={expected_stderr!r}\nactual={result['stderr']!r}"
                )
            expected_first_line = EXPECTED_ERROR_FIRST_LINES.get(case["name"])
            if expected_first_line is not None:
                actual_lines = result["stderr"].splitlines()
                if not actual_lines or actual_lines[0] != expected_first_line:
                    raise AssertionError(
                        f"{case['name']}: unexpected first diagnostic line\n"
                        f"expected={expected_first_line!r}\nactual={actual_lines!r}"
                    )
                if len(actual_lines) < 2 or not actual_lines[1].startswith(
                    "epatternize: "
                ):
                    raise AssertionError(
                        f"{case['name']}: missing platform system-error suffix: "
                        f"{result['stderr']!r}"
                    )
            for name in case.get("output_files", ()):
                output_path = cwd / name
                if not output_path.is_file():
                    raise AssertionError(f"{case['name']}: missing output file {name}")
                expected_contents = EXPECTED_OUTPUT_FILES.get(case["name"], {}).get(
                    name
                )
                if expected_contents is not None:
                    actual_contents = output_path.read_text(encoding="utf-8")
                    if actual_contents != expected_contents:
                        raise AssertionError(
                            f"{case['name']}: unexpected output file {name}\n"
                            f"expected={expected_contents!r}\nactual={actual_contents!r}"
                        )
            for name in case.get("output_absent_files", ()):
                if (cwd / name).exists():
                    raise AssertionError(f"{case['name']}: unexpected output path {name}")

            print(f"PASS {case['name']} exit={result['exit_code']}")

    print(f"All {len(cases)} native epatternize cases passed")


if __name__ == "__main__":
    main()
