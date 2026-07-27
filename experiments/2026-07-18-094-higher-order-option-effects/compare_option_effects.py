#!/usr/bin/env python3
"""Compare higher-order option materialization and FO-only preprocessing gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


FULL_OPTION_OVERRIDES = [
    "--print-strategy",
    "--ext-sup-max-depth=4",
    "--inverse-recognition",
    "--replace-inj-defs",
    "--bce=true",
    "--pred-elim=true",
    "--cnf-lambda-to-forall=false",
    "--eta-normalize=expand",
    "--ho-order-kind=lambda",
    "--eliminate-leibniz-eq=5",
    "--unroll-formulas-only=false",
    "--prim-enum-mode=full",
    "--prim-enum-max-depth=6",
    "--inst-choice-max-depth=7",
    "--local-rw=true",
    "--prune-args=true",
    "--preinstantiate-induction=true",
    "--func-proj-limit=1",
    "--unif-mode=multi",
    "--pattern-oracle=false",
    "--fixpoint-oracle=false",
    "--max-unifiers=8",
    "--max-unif-steps=9",
]


def cases() -> dict[str, dict[str, Any]]:
    proof_common = ["--output-level=2", "--print-statistics"]
    result: dict[str, dict[str, Any]] = {
        "strategy_all_overrides": {"kind": "raw", "options": FULL_OPTION_OVERRIDES},
        "strategy_bce": {
            "kind": "raw",
            "options": ["--print-strategy", "--bce=true"],
        },
        "strategy_pred_elim": {
            "kind": "raw",
            "options": ["--print-strategy", "--pred-elim=true"],
        },
        "invalid_eta": {"kind": "raw", "options": ["--eta-normalize=both"]},
        "invalid_ho_order": {"kind": "raw", "options": ["--ho-order-kind=both"]},
        "invalid_prim_enum": {"kind": "raw", "options": ["--prim-enum-mode=bad"]},
        "invalid_unif_mode": {"kind": "raw", "options": ["--unif-mode=bad"]},
        "invalid_projection_limit": {
            "kind": "raw",
            "options": ["--func-proj-limit=64"],
        },
        "invalid_ext_depth": {"kind": "raw", "options": ["--ext-sup-max-depth=-2"]},
    }
    thf_fixture = "experiments/2026-07-18-094-higher-order-option-effects/thf-first-order-shaped.p"
    thf_options = {
        "thf_baseline": [],
        "thf_bce": ["--bce=true"],
        "thf_pred_elim": ["--pred-elim=true"],
        "thf_bce_pred_elim": ["--bce=true", "--pred-elim=true"],
    }
    for name, options in thf_options.items():
        result[name] = {
            "kind": "preprocessing",
            "fixture": thf_fixture,
            "options": [*proof_common, *options],
        }
    result["fo_bce"] = {
        "kind": "preprocessing",
        "fixture": "experiments/2026-07-18-094-higher-order-option-effects/fo-bce.p",
        "options": [*proof_common, "--bce=true"],
    }
    result["fo_pred_elim"] = {
        "kind": "preprocessing",
        "fixture": "experiments/2026-07-18-094-higher-order-option-effects/fo-pred-elim.p",
        "options": [*proof_common, "--pred-elim=true"],
    }
    return result


def normalize_ids(line: str) -> str:
    return re.sub(r"c_0_-?\d+", "c_0_N", line.strip())


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


def summarize(kind: str, completed: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    if kind == "raw":
        return {"exit": completed.returncode, "stdout": stdout, "stderr": stderr}
    return {
        "stderr": stderr,
        "preprocessing_lines": [
            line
            for line in stdout.splitlines()
            if line.startswith(("% BCE ", "% PE "))
        ],
        "final_clauses": [
            normalize_ids(line) for line in stdout.splitlines() if ",[\'final\'])." in line
        ],
        "counters": {
            name: extract_counter(stdout, name)
            for name in (
                "Parsed axioms",
                "Initial clauses",
                "Removed in clause preprocessing",
                "Initial clauses in saturation",
            )
        },
    }


def run_cases(exe: str, repo: Path, selected: dict[str, dict[str, Any]]) -> dict[str, Any]:
    reports: dict[str, Any] = {}
    for name, case in selected.items():
        command = [exe, *case["options"]]
        if "fixture" in case:
            command.append(str(repo / case["fixture"]))
        completed = subprocess.run(command, check=False, capture_output=True, timeout=60)
        reports[name] = summarize(case["kind"], completed)
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
    thf_names = ("thf_baseline", "thf_bce", "thf_pred_elim", "thf_bce_pred_elim")
    thf_gate_exact = all(
        rust_results[name] == rust_results["thf_baseline"]
        and c_results[name] == c_results["thf_baseline"]
        for name in thf_names
    )
    fo_effects_observed = bool(rust_results["fo_bce"]["preprocessing_lines"]) and bool(
        rust_results["fo_pred_elim"]["preprocessing_lines"]
    )
    report = {
        "case_count": len(selected),
        "cases": rust_results,
        "mismatches": mismatches,
        "thf_fo_preprocessing_gate_exact": thf_gate_exact,
        "fo_preprocessing_effects_observed": fo_effects_observed,
    }
    report["sha256"] = digest(report)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("option-effect comparison differs from the retained reference")
            return 1
    if mismatches or not thf_gate_exact or not fo_effects_observed:
        print(f"mismatches: {mismatches}")
        for name in mismatches:
            print(f"  {name} C: {json.dumps(c_results[name], sort_keys=True)}")
            print(f"  {name} Rust: {json.dumps(rust_results[name], sort_keys=True)}")
        print(f"THF FO-only preprocessing gate exact: {thf_gate_exact}")
        print(f"FO preprocessing effects observed: {fo_effects_observed}")
        return 1
    print(f"validated {len(selected)} exact C/Rust option-effect cases")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
