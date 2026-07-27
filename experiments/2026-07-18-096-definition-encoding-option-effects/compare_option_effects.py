#!/usr/bin/env python3
"""Compare definition, CNF, symbol, and encoding option effects with C."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


EXPERIMENT = "experiments/2026-07-18-096-definition-encoding-option-effects"
STRATEGY_OPTIONS = [
    "--print-strategy",
    "--definitional-cnf=31",
    "--fool-unroll=false",
    "--miniscope-limit=17",
    "--arg-cong=max",
    "--neg-ext=all",
    "--pos-ext=off",
]


def cases() -> dict[str, dict[str, Any]]:
    return {
        "strategy_overrides": {"kind": "raw", "options": STRATEGY_OPTIONS},
        "invalid_definitional_cnf": {
            "kind": "raw",
            "options": ["--definitional-cnf=-1"],
        },
        "invalid_miniscope_limit": {
            "kind": "raw",
            "options": ["--miniscope-limit=-1"],
        },
        "invalid_fool_unroll": {
            "kind": "raw",
            "options": ["--fool-unroll=maybe"],
        },
        "invalid_arg_cong": {"kind": "raw", "options": ["--arg-cong=bad"]},
        "invalid_neg_ext": {"kind": "raw", "options": ["--neg-ext=bad"]},
        "invalid_pos_ext": {"kind": "raw", "options": ["--pos-ext=bad"]},
        "custom_heuristic": {
            "kind": "normalized",
            "fixture": f"{EXPERIMENT}/proof.lop",
            "options": [
                "--lop-in",
                "--define-weight-function=custom_fifo=FIFOWeight(ConstPrio)",
                "--define-heuristic=CustomSearch=(1*custom_fifo)",
                "--expert-heuristic=CustomSearch",
            ],
        },
        "free_number_rejected": {
            "kind": "diagnostic",
            "fixture": f"{EXPERIMENT}/free-number.lop",
            "options": ["--syntax-only", "--lop-in"],
        },
        "free_number_allowed": {
            "kind": "normalized",
            "fixture": f"{EXPERIMENT}/free-number.lop",
            "options": ["--syntax-only", "--lop-in", "--free-numbers"],
        },
        "free_object_rejected": {
            "kind": "diagnostic",
            "fixture": f"{EXPERIMENT}/free-object.lop",
            "options": ["--syntax-only", "--lop-in"],
        },
        "free_object_allowed": {
            "kind": "normalized",
            "fixture": f"{EXPERIMENT}/free-object.lop",
            "options": ["--syntax-only", "--lop-in", "--free-objects"],
        },
        "fool_unroll_enabled": {
            "kind": "normalized",
            "fixture": f"{EXPERIMENT}/fool.p",
            "options": ["--tstp-in", "--cnf", "--fool-unroll=true"],
        },
        "fool_unroll_disabled": {
            "kind": "normalized",
            "fixture": f"{EXPERIMENT}/fool.p",
            "options": ["--tstp-in", "--cnf", "--fool-unroll=false"],
        },
        "typed_clause_output": {
            "kind": "normalized",
            "fixture": "experiments/2026-07-17-056-weight-parser-context-matrix/problem.lop",
            "options": ["--cnf", "--lop-in", "--print-types"],
        },
        "app_encode": {
            "kind": "app_encode",
            "fixture": "experiments/2026-07-17-046-app-encode-typed-application-types/input.p",
            "options": ["--tstp-in", "--app-encode"],
        },
    }


def normalize_text(text: str, repo: Path) -> str:
    normalized = text.replace("\r\n", "\n")
    resolved_repo = repo.resolve()
    repo_variants = {
        str(resolved_repo).replace("\\", "/"),
        str(resolved_repo),
    }
    if resolved_repo.drive:
        repo_variants.add(windows_to_wsl(resolved_repo))
    for variant in sorted(repo_variants, key=len, reverse=True):
        normalized = normalized.replace(variant, "<repo>")
    normalized = normalized.replace("\\", "/")
    normalized = re.sub(r"c_0_-?\d+", "c_0_N", normalized)
    return normalized


def normalize_app_encode(stdout: str) -> str:
    lines = stdout.splitlines()
    prelude: list[str] = []
    declarations: list[tuple[str, str]] = []
    tail: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("%-- "):
            if index + 1 >= len(lines) or not lines[index + 1].startswith(
                "tff(typedecl"
            ):
                raise ValueError(f"type comment lacks declaration: {line!r}")
            declaration = re.sub(r"typedecl\d+", "typedeclN", lines[index + 1])
            declarations.append((line, declaration))
            index += 2
        elif declarations:
            tail.append(line)
            index += 1
        else:
            prelude.append(line)
            index += 1
    canonical_declarations = [
        line for pair in sorted(declarations) for line in pair
    ]
    return "\n".join([*prelude, *canonical_declarations, *tail]) + "\n"


def summarize(
    kind: str, completed: subprocess.CompletedProcess[bytes], repo: Path
) -> dict[str, Any]:
    stdout = normalize_text(completed.stdout.decode("utf-8", errors="replace"), repo)
    stderr = normalize_text(completed.stderr.decode("utf-8", errors="replace"), repo)
    if kind == "diagnostic":
        return {
            "exit": completed.returncode,
            "stdout": stdout,
            "stderr": stderr,
        }
    if kind == "app_encode":
        stdout = normalize_app_encode(stdout)
    return {"exit": completed.returncode, "stdout": stdout, "stderr": stderr}


def run_cases(exe: str, repo: Path, selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    reports: dict[str, Any] = {}
    for name, case in selected.items():
        command = [exe, *case["options"]]
        if "fixture" in case:
            command.append(str(repo / case["fixture"]))
        completed = subprocess.run(command, check=False, capture_output=True, timeout=60)
        reports[name] = summarize(case["kind"], completed, repo)
    return reports


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    tail = resolved.as_posix().split(":", maxsplit=1)[1]
    return f"/mnt/{drive}{tail}"


def digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


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
    selected = cases()

    if args.worker:
        if args.exe is None or args.repo is None:
            parser.error("--worker requires --exe and --repo")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.repo, selected), sort_keys=True))
        return 0

    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")
    repo = Path(__file__).resolve().parents[2]
    rust_results = run_cases(str(args.rust_exe.resolve()), repo, selected)
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
        timeout=600,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_results = json.loads(worker.stdout.decode("utf-8"))
    mismatches = [name for name in selected if rust_results[name] != c_results[name]]
    effects = {
        "free_number_gate": (
            rust_results["free_number_rejected"]["exit"] != 0
            and rust_results["free_number_allowed"]["exit"] == 0
        ),
        "free_object_gate": (
            rust_results["free_object_rejected"]["exit"] != 0
            and rust_results["free_object_allowed"]["exit"] == 0
        ),
        "fool_unroll_changes_cnf": (
            rust_results["fool_unroll_enabled"] != rust_results["fool_unroll_disabled"]
        ),
        "typed_clause_annotations": ":$i" in rust_results["typed_clause_output"]["stdout"],
        "app_encode_symbols": "app_" in rust_results["app_encode"]["stdout"],
    }
    report = {
        "case_count": len(selected),
        "cases": rust_results,
        "effects": effects,
        "mismatches": mismatches,
    }
    report["sha256"] = digest(report)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("option-effect comparison differs from the retained reference")
            return 1
    if mismatches or not all(effects.values()):
        print(f"mismatches: {mismatches}")
        for name in mismatches:
            print(f"  {name} C: {json.dumps(c_results[name], sort_keys=True)}")
            print(f"  {name} Rust: {json.dumps(rust_results[name], sort_keys=True)}")
        print(f"effects: {effects}")
        return 1
    print(f"validated {len(selected)} exact C/Rust option-effect cases")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
