#!/usr/bin/env python3
"""Run the new learning-tool interop cases against optimized Windows binaries."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


EXPECTED_EXIT_CODES = {
    "direct_examples/branching-protocol": 0,
    "direct_examples/missing-input": 6,
    "ekb_delete/drop-middle-example": 0,
    "tsm_classify/recursive-mixed": 0,
    "tsm_classify/empty-test-set": 0,
}
EXPECTED_NORMALIZED_OUTPUT = {
    "direct_examples/branching-protocol": ("% Axioms:", "% Examples:"),
    "direct_examples/missing-input": ("<OS ERROR: NOT FOUND>",),
    "tsm_classify/recursive-mixed": ("12 terms,",),
    "tsm_classify/empty-test-set": ("0 terms, 0 successes, <NAN> percent",),
}


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main() -> int:
    cases = {
        case["name"]: case
        for case in e_interop.tool_comparison_cases(
            ["direct_examples", "ekb_delete", "tsm_classify"]
        )
        if case["name"] in EXPECTED_EXIT_CODES
    }
    if set(cases) != set(EXPECTED_EXIT_CODES):
        missing = sorted(set(EXPECTED_EXIT_CODES) - set(cases))
        raise RuntimeError(f"missing configured candidate case(s): {', '.join(missing)}")

    records = []
    with tempfile.TemporaryDirectory(prefix="learning-tool-candidates-") as temporary:
        temporary_root = Path(temporary)
        for serial, name in enumerate(EXPECTED_EXIT_CODES, 1):
            case = cases[name]
            case_root = temporary_root / f"{serial:02d}"
            workdir = case_root / "workdir"
            fixture_dir = case_root / "fixtures"
            workdir.mkdir(parents=True)
            e_interop.materialize_tool_workdir_directories(case, workdir)
            e_interop.materialize_tool_workdir_files(case, workdir)
            fixtures = e_interop.materialize_tool_fixture_files(case, fixture_dir)
            arguments = e_interop.substitute_tool_fixture_arguments(
                case["arguments"], fixtures
            )
            binary = REPO_ROOT / "target" / "release" / f"{case['tool']}.exe"
            if not binary.is_file():
                raise RuntimeError(f"missing optimized candidate binary: {binary}")

            result = e_interop.execute(
                binary,
                arguments,
                timeout=30,
                env=os.environ.copy(),
                stdin_text=case["stdin"],
                cwd=workdir,
            )
            expected_exit = EXPECTED_EXIT_CODES[name]
            if result["timed_out"] or result["exit_code"] != expected_exit:
                raise RuntimeError(
                    f"{name}: exit={result['exit_code']} timed_out={result['timed_out']}\n"
                    f"stdout:\n{result['stdout']}\nstderr:\n{result['stderr']}"
                )
            normalized_output = e_interop.normalize_output(
                result["stdout"] + result["stderr"]
            )
            missing_snippets = [
                snippet
                for snippet in EXPECTED_NORMALIZED_OUTPUT.get(name, ())
                if snippet not in normalized_output
            ]
            if missing_snippets:
                raise RuntimeError(
                    f"{name}: normalized output is missing {missing_snippets}\n"
                    f"{normalized_output}"
                )

            missing_outputs = [
                output
                for output in case.get("output_files", ())
                if not (workdir / output).is_file()
            ]
            present_forbidden = [
                output
                for output in case.get("output_absent_files", ())
                if (workdir / output).exists()
            ]
            missing_directories = [
                output
                for output in case.get("output_directories", ())
                if not (workdir / output).is_dir()
            ]
            if missing_outputs or present_forbidden or missing_directories:
                raise RuntimeError(
                    f"{name}: missing_outputs={missing_outputs}, "
                    f"present_forbidden={present_forbidden}, "
                    f"missing_directories={missing_directories}"
                )

            records.append(
                {
                    "name": name,
                    "exit_code": result["exit_code"],
                    "stdout_bytes": len(result["stdout"].encode("utf-8")),
                    "stderr_bytes": len(result["stderr"].encode("utf-8")),
                    "stdout_sha256": digest(result["stdout"]),
                    "stderr_sha256": digest(result["stderr"]),
                    "wall_seconds": result["wall_seconds"],
                }
            )

    print(json.dumps({"case_count": len(records), "cases": records}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
