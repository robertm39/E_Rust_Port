#!/usr/bin/env python3
"""Compare focused CNF projections for the vendored LFHOL example corpus."""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


CASES = (
    "lists.p",
    "permute_func_axioms.p",
    "permute_func_no_axioms.p",
    "SEV286^5.p",
    "sledgehammer.p",
)
COUNTERS = (
    "Parsed axioms",
    "Initial clauses",
    "Removed in clause preprocessing",
    "Initial clauses in saturation",
)
SECTION_HEADINGS = (
    "% Unprocessed positive unit clauses:",
    "% Unprocessed negative unit clauses:",
    "% Unprocessed non-unit clauses:",
)


def normalize_line(line: str) -> str:
    line = re.sub(r"c_0_-?\d+", "c_0_N", line.strip())
    return re.sub(r"decl_\d+", "decl_N", line)


def extract_counter(stdout: str, name: str) -> int:
    prefix = f"% {name}"
    lines = [
        line
        for line in stdout.splitlines()
        if line.split(":", maxsplit=1)[0].rstrip() == prefix
    ]
    if len(lines) != 1:
        raise RuntimeError(f"expected one {name!r} counter, got {lines}")
    return int(lines[0].rsplit(":", 1)[1])


def extract_type_declarations(lines: list[str]) -> list[str]:
    try:
        start = lines.index("% Type declarations:") + 1
        end = lines.index("% Processed positive unit clauses:", start)
    except ValueError as error:
        raise RuntimeError("missing CNF type-declaration section") from error
    return sorted(normalize_line(line) for line in lines[start:end] if line.startswith("thf("))


def extract_clause_sections(lines: list[str]) -> dict[str, list[str]]:
    result: dict[str, list[str]] = {}
    for index, heading in enumerate(SECTION_HEADINGS):
        try:
            start = lines.index(heading) + 1
        except ValueError as error:
            raise RuntimeError(f"missing CNF clause section {heading!r}") from error
        if index + 1 < len(SECTION_HEADINGS):
            end = lines.index(SECTION_HEADINGS[index + 1], start)
        else:
            end = next(
                (offset for offset in range(start, len(lines)) if lines[offset].startswith("% Parsed axioms")),
                len(lines),
            )
        result[heading] = sorted(
            normalize_line(line) for line in lines[start:end] if line.startswith("thf(")
        )
    return result


def clause_shape(line: str) -> str:
    variables = re.findall(r"\b(?:X|Z)\d+\b", line)
    variable_occurrences = sorted(Counter(variables).values())
    without_variables = re.sub(r"\b(?:X|Z)\d+\b", "VAR", line)
    tokens = re.findall(
        r"[A-Za-z_$][A-Za-z0-9_$]*|!=|<=>|<~>|=>|<=|~&|~\||[=|~@]",
        without_variables,
    )
    punctuation = {token: line.count(token) for token in ("(", ")", "[", "]", ",", ":")}
    return json.dumps(
        {
            "tokens": sorted(Counter(tokens).items()),
            "punctuation": punctuation,
            "variable_occurrences": variable_occurrences,
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def summarize(
    completed: subprocess.CompletedProcess[bytes], *, compare_exact_clauses: bool
) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    lines = stdout.splitlines()
    clause_sections = extract_clause_sections(lines)
    result = {
        "exit": completed.returncode,
        "stderr": stderr,
        "type_declarations": extract_type_declarations(lines),
        "counters": {name: extract_counter(stdout, name) for name in COUNTERS},
    }
    if compare_exact_clauses:
        result["clause_sections"] = clause_sections
    else:
        result["clause_shape_sections"] = {
            heading: sorted(clause_shape(line) for line in section)
            for heading, section in clause_sections.items()
        }
    return result


def run_cases(exe: str, repo: Path) -> dict[str, Any]:
    base = repo / "eprover" / "EXAMPLE_PROBLEMS" / "LFHOL"
    result: dict[str, Any] = {}
    for name in CASES:
        completed = subprocess.run(
            [exe, "--cnf", "--tstp-out", "--output-level=4", str(base / name)],
            check=False,
            capture_output=True,
            timeout=120,
        )
        result[name] = summarize(completed, compare_exact_clauses=name != "sledgehammer.p")
    return result


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def report_case_difference(name: str, c_case: dict[str, Any], rust_case: dict[str, Any]) -> None:
    print(f"  {name} scalar C/Rust:")
    for key in ("exit", "stderr", "counters"):
        if c_case[key] != rust_case[key]:
            print(f"    {key}: {c_case[key]!r} / {rust_case[key]!r}")
    for key in ("type_declarations",):
        c_items = c_case[key]
        rust_items = rust_case[key]
        if c_items != rust_items:
            print(
                f"    {key}: {len(c_items)} / {len(rust_items)}; "
                f"hashes {digest(c_items)} / {digest(rust_items)}"
            )
            print(f"      only C: {sorted(set(c_items) - set(rust_items))[:2]}")
            print(f"      only Rust: {sorted(set(rust_items) - set(c_items))[:2]}")
    section_key = (
        "clause_sections" if "clause_sections" in c_case else "clause_shape_sections"
    )
    for heading in SECTION_HEADINGS:
        c_items = c_case[section_key][heading]
        rust_items = rust_case[section_key][heading]
        if c_items != rust_items:
            print(
                f"    {heading}: {len(c_items)} / {len(rust_items)}; "
                f"hashes {digest(c_items)} / {digest(rust_items)}"
            )
            print(f"      only C: {sorted(set(c_items) - set(rust_items))[:2]}")
            print(f"      only Rust: {sorted(set(rust_items) - set(c_items))[:2]}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--repo", type=Path)
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    if args.worker:
        if args.exe is None or args.repo is None:
            parser.error("--worker requires --exe and --repo")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.repo), sort_keys=True))
        return 0

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")
    repo = Path(__file__).resolve().parents[2]
    rust_results = run_cases(str(args.rust_exe.resolve()), repo)
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
            "--repo",
            windows_to_wsl(repo),
        ],
        check=False,
        capture_output=True,
        timeout=900,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    mismatches = [name for name in CASES if rust_results[name] != c_results[name]]
    report = {"case_count": len(CASES), "cases": rust_results, "mismatches": mismatches}
    report["sha256"] = digest(report)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("LFHOL CNF comparison differs from the retained reference")
            return 1
    if mismatches:
        print(f"mismatches: {mismatches}")
        for name in mismatches:
            report_case_difference(name, c_results[name], rust_results[name])
        return 1
    print(f"validated {len(CASES)} LFHOL CNF projections")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
