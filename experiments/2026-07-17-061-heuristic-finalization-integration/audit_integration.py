#!/usr/bin/env python3
"""Audit and compare heuristic lookup/finalization integration paths."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
from pathlib import Path


NAMED_STRATEGY = "G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN"


def production_text(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    marker = "#[cfg(test)]"
    return text.split(marker, maxsplit=1)[0]


def noncomment_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("//")
    ]


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
        "stderr": completed.stderr.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
    }


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = str(result["stdout"])
    stderr = str(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stdout_sha256": digest(stdout),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "stderr_sha256": digest(stderr),
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def source_audit(root: Path) -> dict[str, object]:
    c_source = root / "eprover/HEURISTICS/che_heuristics.c"
    rust_lookup = root / "src/heuristics/heuristic_lookup.rs"
    proofcontrol = root / "src/heuristics/proofcontrol.rs"
    executable = root / "src/prover/eprover.rs"

    c_finalize_lines = [
        line for line in noncomment_lines(c_source) if "finalize_auto_parms(" in line
    ]
    c_finalize_calls = [
        line for line in c_finalize_lines if not line.startswith("void finalize_auto_parms(")
    ]
    rust_lookup_production = production_text(rust_lookup)
    rust_finalize_calls = [
        line.strip()
        for line in rust_lookup_production.splitlines()
        if "finalize_auto_parms(" in line and not line.strip().startswith("pub fn ")
    ]
    proofcontrol_production = production_text(proofcontrol)
    executable_text = executable.read_text(encoding="utf-8")

    previous_strategy = json.loads(
        (
            root
            / "experiments/2026-07-17-050-reporting-strategy-limit-matrix/results-summary.json"
        ).read_text(encoding="utf-8")
    )
    previous_banked = json.loads(
        (
            root
            / "experiments/2026-07-17-060-banked-wfcb-production-audit/results-summary.json"
        ).read_text(encoding="utf-8")
    )
    scheduler_findings = (
        root / "experiments/2026-07-16-034-multicore-fork-compatibility/FINDINGS.md"
    ).read_text(encoding="utf-8")

    checks = {
        "c_finalize_definition_count": len(c_finalize_lines),
        "c_finalize_call_count": len(c_finalize_calls),
        "rust_finalize_production_call_count": len(rust_finalize_calls),
        "executable_proof_control_init_call_count": executable_text.count(
            "proof_control_init_with_formula_axioms("
        ),
        "default_weight_install_call_count": proofcontrol_production.count(
            "install_default_weight_functions(control, context)?;"
        ),
        "default_heuristic_install_call_count": proofcontrol_production.count(
            "install_default_heuristics(control, context)?;"
        ),
        "proof_control_parameter_copy_count": proofcontrol_production.count(
            "control.heuristic_parms = params.clone();"
        ),
        "active_hcb_lookup_count": proofcontrol_production.count(
            "control.active_hcb = Some(get_heuristic_handle_with_context("
        ),
        "prior_strategy_exact_count": previous_strategy["exact_count"],
        "prior_strategy_case_count": previous_strategy["case_count"],
        "prior_banked_forbidden_count": previous_banked[
            "forbidden_immutable_call_count"
        ],
        "prior_banked_proof_control_call_count": previous_banked[
            "proof_control_banked_call_count"
        ],
        "scheduler_safe_state_transfer_decision_recorded": (
            "## Safe state-transfer decision" in scheduler_findings
        ),
        "scheduler_cpu_contract_recorded": (
            "Schedule initialization uses self CPU" in scheduler_findings
        ),
    }
    expected = {
        "c_finalize_definition_count": 1,
        "c_finalize_call_count": 0,
        "rust_finalize_production_call_count": 0,
        "executable_proof_control_init_call_count": 1,
        "default_weight_install_call_count": 1,
        "default_heuristic_install_call_count": 1,
        "proof_control_parameter_copy_count": 1,
        "active_hcb_lookup_count": 1,
        "prior_strategy_exact_count": 11,
        "prior_strategy_case_count": 11,
        "prior_banked_forbidden_count": 0,
        "prior_banked_proof_control_call_count": 8,
        "scheduler_safe_state_transfer_decision_recorded": True,
        "scheduler_cpu_contract_recorded": True,
    }
    return {"checks": checks, "expected": expected, "passed": checks == expected}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = (
        args.root.resolve()
        if args.root is not None
        else Path(__file__).resolve().parents[2]
    )
    fixture = Path(__file__).resolve().parent / "unsat.lop"
    cases = [
        (
            "anonymous_inline_heuristic",
            ["--lop-in", "--expert-heuristic=(1*FIFOWeight(ConstPrio))"],
        ),
        (
            "named_custom_definitions",
            [
                "--lop-in",
                "--define-weight-function=custom_fifo=FIFOWeight(ConstPrio)",
                "--define-heuristic=CustomSearch=(1*custom_fifo)",
                "--expert-heuristic=CustomSearch",
            ],
        ),
        (
            "selected_predefined_strategy",
            ["--lop-in", f"--select-strategy={NAMED_STRATEGY}"],
        ),
        ("generated_auto_strategy", ["--lop-in", "--auto"]),
    ]

    comparisons: list[dict[str, object]] = []
    for name, common_args in cases:
        rust = run([str(args.rust_exe.resolve()), *common_args, str(fixture)])
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(shlex.quote(arg) for arg in common_args),
                shlex.quote(wsl_path(fixture)),
            ]
        )
        exact_match = rust == c
        comparison: dict[str, object] = {
            "case": name,
            "exact_match": exact_match,
            "rust": summarize(rust),
            "c": summarize(c),
        }
        if not exact_match:
            comparison["mismatch"] = {"rust": rust, "c": c}
        comparisons.append(comparison)

    audit = source_audit(root)
    result = {
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "source_audit": audit,
        "case_count": len(comparisons),
        "exact_count": sum(bool(case["exact_match"]) for case in comparisons),
        "comparisons": comparisons,
    }
    rendered = json.dumps(result, indent=2) + "\n"
    args.output.write_text(rendered, encoding="utf-8")
    if not args.quiet:
        print(rendered, end="")
    if not audit["passed"] or result["exact_count"] != result["case_count"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
