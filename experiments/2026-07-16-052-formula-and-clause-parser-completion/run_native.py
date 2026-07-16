"""Exercise the FormulaAndClauseSetParse termination/caller boundary."""

from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))

import e_interop  # noqa: E402


UNIT_PATTERN = "$or1($eq(f1_1(f0_1),$true))\n"
TAIL_CASES = (
    "epatternize/lop-unrecognized-tail",
    "epatternize/tptp-unrecognized-tail",
    "epatternize/tstp-unrecognized-tail",
)
EXPECTED_CALLER_CASES = {
    "eground/trailing-token": (
        "eground.exe",
        3,
        "",
        "eground: -:1:(Column 7):(just read ','): No token (probably EOF) "
        "expected, but Comma (',') read \n",
    ),
}


def run_expanded_epatternize_matrix() -> None:
    previous_path = (
        REPO_ROOT
        / "experiments"
        / "2026-07-16-051-epatternize-expanded-comparison"
        / "run_native.py"
    )
    spec = importlib.util.spec_from_file_location("epatternize_exp051", previous_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load prior experiment runner: {previous_path}")
    previous = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(previous)

    expanded_exit_codes = {}
    for name, exit_code in previous.EXPECTED_EXIT_CODES.items():
        expanded_exit_codes[name] = exit_code
        if name == "epatternize/lop-basic":
            expanded_exit_codes.update({tail_name: 0 for tail_name in TAIL_CASES})
    previous.EXPECTED_EXIT_CODES = expanded_exit_codes
    previous.EXPECTED_STDOUT.update({name: UNIT_PATTERN for name in TAIL_CASES})
    previous.main()


def main() -> None:
    run_expanded_epatternize_matrix()
    cases = {
        case["name"]: case
        for case in e_interop.tool_comparison_cases(["eground"])
    }

    for name, (binary_name, exit_code, stdout, stderr) in EXPECTED_CALLER_CASES.items():
        case = cases[name]
        binary = REPO_ROOT / "target" / "release" / binary_name
        if not binary.is_file():
            raise SystemExit(f"missing release binary: {binary}")
        result = e_interop.execute(
            binary,
            case["arguments"],
            timeout=30,
            env=os.environ.copy(),
            stdin_text=case["stdin"],
            cwd=REPO_ROOT,
        )
        actual = (result["exit_code"], result["stdout"], result["stderr"])
        expected = (exit_code, stdout, stderr)
        if actual != expected:
            raise AssertionError(
                f"{name}: expected {expected!r}, got {actual!r}"
            )

    print("validated eground's caller-owned EOF boundary")


if __name__ == "__main__":
    main()
