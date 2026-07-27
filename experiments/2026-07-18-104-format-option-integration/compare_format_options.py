#!/usr/bin/env python3
"""Compare production format-option effects in upstream C and Rust."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


CASES: dict[str, dict[str, Any]] = {
    "pcl_shell_0": {
        "mode": "docs",
        "fixture": "contradiction.lop",
        "args": ["--lop-in", "--output-level=2", "--pcl-out", "--pcl-shell-level=0"],
    },
    "pcl_shell_1": {
        "mode": "docs",
        "fixture": "contradiction.lop",
        "args": ["--lop-in", "--output-level=2", "--pcl-out", "--pcl-shell-level=1"],
    },
    "pcl_shell_2": {
        "mode": "docs",
        "fixture": "contradiction.lop",
        "args": ["--lop-in", "--output-level=2", "--pcl-out", "--pcl-shell-level=2"],
    },
    "pcl_compact": {
        "mode": "docs",
        "fixture": "contradiction.lop",
        "args": ["--lop-in", "--output-level=2", "--pcl-out", "--pcl-compact"],
    },
    "pcl_terms_compressed": {
        "mode": "docs",
        "fixture": "deep-term.lop",
        "args": [
            "--lop-in",
            "--output-level=2",
            "--no-generation",
            "--pcl-out",
            "--pcl-terms-compressed",
        ],
    },
    "pcl_preprocessing_terms_full": {
        "mode": "docs",
        "fixture": "deep-conjunction.p",
        "args": [
            "--tstp-in",
            "--output-level=2",
            "--no-generation",
            "--pcl-out",
            "--pcl-terms-compressed",
        ],
    },
    "pcl_print_types": {
        "mode": "docs",
        "fixture": "typed-equation.p",
        "args": [
            "--tstp-in",
            "--output-level=2",
            "--no-generation",
            "--pcl-out",
            "--print-types",
        ],
    },
    "pcl_watchlist_side_channel": {
        "mode": "docs",
        "fixture": "clause.lop",
        "args": [
            "--lop-in",
            "--output-level=2",
            "--no-generation",
            "--pcl-out",
            "--watchlist=watch.lop",
        ],
    },
    "tstp_docs": {
        "mode": "docs",
        "fixture": "contradiction.lop",
        "args": ["--lop-in", "--output-level=2", "--tstp-out"],
    },
    "lop_default": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": ["--lop-in", "--cnf", "--output-level=1"],
    },
    "lop_eqn_no_infix": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": ["--lop-in", "--cnf", "--output-level=1", "--eqn-no-infix"],
    },
    "lop_full_equational": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": ["--lop-in", "--cnf", "--output-level=1", "--full-equational-rep"],
    },
    "lop_oriented_rules": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": [
            "--lop-in",
            "--cnf",
            "--output-level=1",
            "--print-oriented-eqlits-as-rules",
        ],
    },
    "tptp_out": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": ["--lop-in", "--cnf", "--output-level=1", "--tptp-out"],
    },
    "tptp_format": {
        "mode": "clauses",
        "fixture": "old-tptp.p",
        "args": ["--tptp-format", "--cnf", "--output-level=1"],
    },
    "tstp_in_out": {
        "mode": "clauses",
        "fixture": "direct-cnf.p",
        "args": ["--tstp-in", "--tstp-out", "--cnf", "--output-level=1"],
    },
    "tstp_format": {
        "mode": "clauses",
        "fixture": "direct-cnf.p",
        "args": ["--tstp-format", "--cnf", "--output-level=1"],
    },
    "print_formulas_ignores_clause_format": {
        "mode": "clauses",
        "fixture": "equations.lop",
        "args": [
            "--lop-in",
            "--print-formulas",
            "--tptp-out",
            "--eqn-no-infix",
        ],
    },
}


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def normalize_line(line: str) -> str:
    line = re.sub(r"c_0_-?\d+", "c_0_N", line)
    line = re.sub(r"i_0_-?\d+", "i_0_N", line)
    return line.rstrip()


def logical_clause_lines(stdout: str) -> list[str]:
    prefixes = ("cnf(", "fof(", "tff(", "thf(", "tcf(", "input_clause(")
    result = []
    for line in stdout.splitlines():
        stripped = line.strip()
        if stripped.startswith(prefixes) or (stripped.endswith(".") and "<-" in stripped):
            result.append(normalize_line(stripped))
    return result


def project_output(mode: str, process: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = process.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = process.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    status = next(
        (line for line in stdout.splitlines() if line.startswith("% SZS status ")),
        None,
    )
    result: dict[str, Any] = {
        "exit_code": process.returncode,
        "stderr": stderr,
        "status": status,
    }
    if mode == "docs":
        before_init = stdout.split("% Initializing proof state", maxsplit=1)[0]
        result["output"] = [
            normalize_line(line)
            for line in before_init.splitlines()
            if line and not line.startswith("% (")
        ]
    elif mode == "clauses":
        result["output"] = logical_clause_lines(stdout)
    else:
        raise ValueError(f"unknown projection mode {mode!r}")
    return result


def run_cases(exe: str, fixture_dir: Path) -> dict[str, dict[str, Any]]:
    results: dict[str, dict[str, Any]] = {}
    for name, case in CASES.items():
        arguments = [*case["args"], case["fixture"]]
        process = subprocess.run(
            [exe, *arguments],
            cwd=fixture_dir,
            check=False,
            capture_output=True,
            timeout=120,
        )
        results[name] = project_output(case["mode"], process)
    return results


def option_effects(results: dict[str, dict[str, Any]]) -> dict[str, bool]:
    rendered = {name: "\n".join(result["output"]) for name, result in results.items()}
    return {
        "shell_levels_distinct": len(
            {rendered["pcl_shell_0"], rendered["pcl_shell_1"], rendered["pcl_shell_2"]}
        )
        == 3,
        "compact_steps_observed": rendered["pcl_compact"].startswith("1::"),
        "watchlist_c_side_channel_preserved": "XX\n" in rendered["pcl_watchlist_side_channel"],
        "ordinary_pcl_has_no_xx": "XX" not in rendered["pcl_shell_0"],
        "preprocessing_terms_are_full": "*" not in rendered["pcl_preprocessing_terms_full"],
        "eqn_no_infix_observed": "equal(f(a), a) <- ." in rendered["lop_eqn_no_infix"],
        "full_equational_observed": "p(a)=$true <- ." in rendered["lop_full_equational"],
        "oriented_rule_observed": "f(a)->a <- ." in rendered["lop_oriented_rules"],
        "tptp_output_observed": "input_clause(" in rendered["tptp_out"],
        "tstp_output_observed": "cnf(" in rendered["tstp_format"],
        "print_formulas_is_tstp": any(
            line.startswith(("fof(", "tff(", "thf("))
            for line in results["print_formulas_ignores_clause_format"]["output"]
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture-dir", type=Path)
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parent
    if args.worker:
        if args.exe is None or args.fixture_dir is None:
            parser.error("--worker requires --exe and --fixture-dir")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.fixture_dir), sort_keys=True))
        return 0
    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")

    rust_results = run_cases(str(args.rust_exe.resolve()), fixture_dir)
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture-dir",
            windows_to_wsl(fixture_dir),
        ],
        check=False,
        capture_output=True,
        timeout=900,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    effects = option_effects(rust_results)
    report = {
        "schema_version": 1,
        "case_count": len(CASES),
        "c": c_results,
        "rust": rust_results,
        "all_exact": c_results == rust_results,
        "option_effects": effects,
        "all_effects_observed": all(effects.values()),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")
    if args.expected is not None and encoded != args.expected.read_text(encoding="utf-8"):
        print("format-option reference changed", file=sys.stderr)
        return 1
    if not report["all_exact"] or not report["all_effects_observed"]:
        print("format-option comparison failed", file=sys.stderr)
        return 1
    print(f"validated {len(CASES)}/{len(CASES)} exact C/Rust format-option cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
