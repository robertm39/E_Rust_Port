"""Exercise every permanent enormalizer case against the optimized Rust binary."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


EXPECTED_EXIT_CODES = {
    "enormalizer/help": 0,
    "enormalizer/version": 0,
    "enormalizer/term-basic": 0,
    "enormalizer/clause-basic": 0,
    "enormalizer/tstp-formula-target": 0,
    "enormalizer/old-tptp-formula-roles": 0,
    "enormalizer/stdin-include-rules": 0,
    "enormalizer/shared-stdin-consumed-by-rules": 0,
    "enormalizer/print-statistics-noop": 0,
    "enormalizer/output-file": 0,
    "enormalizer/malformed-rule": 3,
    "enormalizer/malformed-term-target": 3,
    "enormalizer/malformed-clause-target": 3,
    "enormalizer/malformed-formula-target": 3,
    "enormalizer/resource-options-success": 0,
    "enormalizer/invalid-hard-after-soft": 5,
    "enormalizer/invalid-soft-after-hard": 5,
    "enormalizer/missing-rule": 6,
    "enormalizer/missing-term-target": 6,
    "enormalizer/missing-clause-target": 6,
    "enormalizer/missing-formula-target": 6,
    "enormalizer/missing-output-parent": 6,
}

EXPECTED_STDOUT = {
    "enormalizer/term-basic": "f(b) ==> a\n",
    "enormalizer/clause-basic": "p(f(b)) <- . ==> p(a) <- .\n",
    "enormalizer/tstp-formula-target": (
        "fof(with_info, axiom, p(f(b))). ==> "
        "fof(with_info, axiom, p(a)).\n"
    ),
    "enormalizer/old-tptp-formula-roles": (
        "input_formula(lemma_form,axiom,p(f(b))). ==> "
        "input_formula(lemma_form,axiom,p(a)).\n"
        "input_formula(12,axiom,q(f(c))). ==> "
        "input_formula(12,axiom,q(a)).\n"
        "input_formula(question_form,question,r(f(d))). ==> "
        "input_formula(question_form,question,r(a)).\n"
        "input_formula(neg_form,conjecture,s(f(e))). ==> "
        "input_formula(neg_form,conjecture,s(a)).\n"
    ),
    "enormalizer/stdin-include-rules": "f(b) ==> a\n",
    "enormalizer/shared-stdin-consumed-by-rules": "",
    "enormalizer/print-statistics-noop": "f(b) ==> a\n",
    "enormalizer/output-file": "",
    "enormalizer/resource-options-success": "f(b) ==> a\n",
}

EXPECTED_EXACT_STDERR = {
    "enormalizer/invalid-hard-after-soft": (
        "enormalizer: Hard time limit has to be larger than softtime limit\n"
    ),
    "enormalizer/invalid-soft-after-hard": (
        "enormalizer: Soft time limit has to be smaller than hardtime limit\n"
    ),
}

EXPECTED_ERROR_FIRST_LINES = {
    "enormalizer/missing-rule": (
        "enormalizer: Cannot open file missing-enormalizer-rules.lop for reading"
    ),
    "enormalizer/missing-term-target": (
        "enormalizer: Cannot open file missing-terms.lop for reading"
    ),
    "enormalizer/missing-clause-target": (
        "enormalizer: Cannot open file missing-clauses.lop for reading"
    ),
    "enormalizer/missing-formula-target": (
        "enormalizer: Cannot open file missing-formulas.p for reading"
    ),
    "enormalizer/missing-output-parent": (
        "enormalizer: Cannot open file missing/normalized.out"
    ),
}

EXPECTED_OUTPUT_FILES = {
    "enormalizer/output-file": {"normalized.out": "f(b) ==> a\n"},
}


def main() -> None:
    binary = REPO_ROOT / "target" / "release" / "enormalizer.exe"
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}")

    cases = e_interop.tool_comparison_cases(["enormalizer"])
    if [case["name"] for case in cases] != list(EXPECTED_EXIT_CODES):
        raise AssertionError("permanent enormalizer case order changed; update this experiment")

    with tempfile.TemporaryDirectory(prefix="enormalizer-native-") as temporary:
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
                case["arguments"], fixture_paths
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
            if case["name"] in EXPECTED_STDOUT and result["stderr"]:
                raise AssertionError(
                    f"{case['name']}: unexpected stderr {result['stderr']!r}"
                )
            expected_stderr = EXPECTED_EXACT_STDERR.get(case["name"])
            if expected_stderr is not None and result["stderr"] != expected_stderr:
                raise AssertionError(
                    f"{case['name']}: unexpected stderr\n"
                    f"expected={expected_stderr!r}\nactual={result['stderr']!r}"
                )
            if expected != 0 and result["stdout"]:
                raise AssertionError(
                    f"{case['name']}: error path wrote stdout {result['stdout']!r}"
                )
            if expected == 3 and not result["stderr"].startswith("enormalizer: "):
                raise AssertionError(
                    f"{case['name']}: malformed-input diagnostic lacks program prefix: "
                    f"{result['stderr']!r}"
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
                    "enormalizer: "
                ):
                    raise AssertionError(
                        f"{case['name']}: missing platform system-error suffix: "
                        f"{result['stderr']!r}"
                    )
            for name in case.get("output_files", ()):
                if not (cwd / name).is_file():
                    raise AssertionError(f"{case['name']}: missing output file {name}")
                expected_contents = EXPECTED_OUTPUT_FILES.get(case["name"], {}).get(
                    name
                )
                if expected_contents is not None:
                    actual_contents = (cwd / name).read_text(encoding="utf-8")
                    if actual_contents != expected_contents:
                        raise AssertionError(
                            f"{case['name']}: unexpected output file {name}\n"
                            f"expected={expected_contents!r}\nactual={actual_contents!r}"
                        )
            for name in case.get("output_absent_files", ()):
                if (cwd / name).exists():
                    raise AssertionError(f"{case['name']}: unexpected output path {name}")

            if case["name"] == "enormalizer/shared-stdin-consumed-by-rules":
                if result["stdout"] or result["stderr"]:
                    raise AssertionError(
                        "shared stdin should be exhausted before target parsing: "
                        f"stdout={result['stdout']!r} stderr={result['stderr']!r}"
                    )

            print(f"PASS {case['name']} exit={result['exit_code']}")

    print(f"All {len(cases)} native enormalizer cases passed")


if __name__ == "__main__":
    main()
