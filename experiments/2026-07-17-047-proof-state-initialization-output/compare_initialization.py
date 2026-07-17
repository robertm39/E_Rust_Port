#!/usr/bin/env python3
"""Compare C/Rust proof-state initialization output for file and stdin input."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


MODES = {
    "proof_search": [],
    "cnf_only": ["--cnf"],
}


def run(command: list[str], stdin: bytes | None = None) -> dict[str, object]:
    completed = subprocess.run(
        command,
        input=stdin,
        check=False,
        capture_output=True,
    )
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


def ordering_checks(stdout: str, mode: str) -> dict[str, bool]:
    init = stdout.find("% Initializing proof state\n")
    ac_scan = stdout.find("% Scanning for AC axioms\n")
    checks = {
        "has_initialization_line": init >= 0,
        "initialization_precedes_ac_scan": 0 <= init < ac_scan,
    }
    if mode == "cnf_only":
        success = stdout.find("% CNFization successful!\n")
        checks.update(
            {
                "ac_scan_precedes_cnf_success": 0 <= ac_scan < success,
                "auto_detected_tstp_sections": all(
                    fragment in stdout
                    for fragment in [
                        "cnf(i_0_1, plain, (human(socrates))).",
                        "cnf(i_0_3, negated_conjecture, (~mortal(socrates))).",
                        "cnf(i_0_2, plain, (mortal(X1)|~human(X1))).",
                    ]
                ),
                "does_not_force_lop_sections": "human(socrates) <- ."
                not in stdout,
            }
        )
    else:
        proof = stdout.find("% Proof found!\n")
        checks.update(
            {
                "ac_scan_precedes_proof_result": 0 <= ac_scan < proof,
                "auto_detected_tstp_selected_clause_progress": all(
                    fragment in stdout
                    for fragment in [
                        "%cnf(i_0_1, plain, (human(socrates))).",
                        "%cnf(i_0_2, plain, (mortal(X1)|~human(X1))).",
                        "%cnf(i_0_4, plain, (mortal(socrates))).",
                    ]
                ),
                "does_not_force_lop_selected_clause_progress": "%human(socrates) <- ."
                not in stdout,
            }
        )
    return checks


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    input_path = Path(__file__).resolve().with_name("socrates.p")
    stdin_bytes = input_path.read_bytes()
    results: list[dict[str, object]] = []
    for mode, mode_args in MODES.items():
        for source in ["file", "stdin"]:
            rust_source = str(input_path) if source == "file" else "-"
            c_source = wsl_path(input_path) if source == "file" else "-"
            stdin = stdin_bytes if source == "stdin" else None
            rust = run(
                [str(args.rust_exe.resolve()), *mode_args, rust_source],
                stdin,
            )
            c = run(
                [
                    "wsl",
                    "-d",
                    args.distro,
                    "--",
                    args.c_exe,
                    *mode_args,
                    c_source,
                ],
                stdin,
            )
            checks = ordering_checks(str(rust["stdout"]), mode)
            results.append(
                {
                    "mode": mode,
                    "source": source,
                    "exact_match": c == rust,
                    "ordering_checks": checks,
                    "all_ordering_checks_pass": all(checks.values()),
                    "c": c,
                    "rust": rust,
                }
            )

    rendered = json.dumps(
        {
            "all_exact_match": all(bool(result["exact_match"]) for result in results),
            "all_ordering_checks_pass": all(
                bool(result["all_ordering_checks_pass"]) for result in results
            ),
            "results": results,
        },
        indent=2,
    )
    if args.output is not None:
        args.output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()
