#!/usr/bin/env python3
"""Independently verify every reproducible larger-budget proof claim."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
BASE_ANALYZE_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "analyze.py"
)
PROOFCHECK_HELPER = (
    EXPERIMENT_ROOT.parent
    / "2026-07-27-004-soundness-validation-gates"
    / "run_validation.py"
)
UEQ_ADAPTER_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "proof_adapter.py"
)
SKOLEM_ADAPTER_PATH = EXPERIMENT_ROOT / "proof_adapter.py"
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ANALYZE = load_module(
    "stronger_redundancy_verify_analyze", BASE_ANALYZE_PATH
)
PROOFCHECK = load_module(
    "stronger_redundancy_proofcheck_helper", PROOFCHECK_HELPER
)
UEQ_ADAPTER = load_module(
    "stronger_redundancy_ueq_adapter", UEQ_ADAPTER_PATH
)
SKOLEM_ADAPTER = load_module(
    "stronger_redundancy_skolem_adapter", SKOLEM_ADAPTER_PATH
)


class VerificationError(RuntimeError):
    """A checker setup or proof-validation contract failure."""


def run_command(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: int,
    environment: dict[str, str],
    stdout_path: Path,
    stderr_path: Path,
) -> subprocess.CompletedProcess[bytes]:
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        env=environment,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    return completed


def find_or_download_proofcheck(
    output_root: Path, explicit: Path | None
) -> Path:
    if explicit is not None:
        proofcheck = explicit.resolve()
        if not proofcheck.is_file() or not os.access(proofcheck, os.X_OK):
            raise VerificationError(
                f"ProofCheck is missing or not executable: {proofcheck}"
            )
        return proofcheck
    external = output_root / "external"
    candidates = (
        [
            path
            for path in external.rglob("proofcheck")
            if path.is_file() and path.name == "proofcheck"
        ]
        if external.is_dir()
        else []
    )
    if len(candidates) == 1:
        return candidates[0]
    if candidates:
        raise VerificationError(
            f"expected at most one cached ProofCheck, found {len(candidates)}"
        )
    external.mkdir(parents=True, exist_ok=True)
    return PROOFCHECK.download_proofcheck(external)


def proof_claims(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> list[tuple[str, str, dict[str, Any]]]:
    claims: list[tuple[str, str, dict[str, Any]]] = []
    for strategy in contract["strategies"]:
        coverage = ANALYZE.reproducible_coverage(
            results, strategy, "larger", contract["repetitions"]
        )
        for problem_id in sorted(coverage):
            representative = next(
                result
                for result in results
                if result["strategy"] == strategy
                and result["budget"] == "larger"
                and result["problem_id"] == problem_id
                and result["repetition"] == 1
            )
            if representative["szs_status"] in PROOF_STATUSES:
                claims.append((strategy, problem_id, representative))
    return claims


def verify_claims(
    *,
    repo: Path,
    experiment_root: Path,
    problem_root: Path,
    output_root: Path,
    proofcheck: Path,
) -> dict[str, Any]:
    contract, results = ANALYZE.load_phase(experiment_root, "test")
    if "larger" not in contract["budgets"]:
        raise VerificationError("test contract has no larger budget")
    claims = proof_claims(contract, results)
    output_root.mkdir(parents=True, exist_ok=True)
    commands_dir = output_root / "commands"
    reports_dir = output_root / "reports"
    adapted_dir = output_root / "adapted"
    commands_dir.mkdir(exist_ok=True)
    reports_dir.mkdir(exist_ok=True)
    adapted_dir.mkdir(exist_ok=True)
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")

    self_certify = run_command(
        [str(proofcheck), "-self-certify"],
        cwd=proofcheck.parent,
        timeout=300,
        environment=environment,
        stdout_path=commands_dir / "proofcheck-self-certify.stdout",
        stderr_path=commands_dir / "proofcheck-self-certify.stderr",
    )
    self_certify_text = (
        (commands_dir / "proofcheck-self-certify.stdout").read_text(
            encoding="utf-8", errors="replace"
        )
        + (commands_dir / "proofcheck-self-certify.stderr").read_text(
            encoding="utf-8", errors="replace"
        )
    )
    if self_certify.returncode != 0 or "117 passed" not in self_certify_text:
        raise VerificationError(
            "ProofCheck self-certification did not pass all 117 tests"
        )

    gate = repo / "tools" / "validation" / "validate_tptp_solution.py"
    cases = []
    for strategy, problem_id, result in claims:
        result_path = Path(result["_path"])
        if not result_path.is_absolute():
            result_path = experiment_root / "test" / result_path
        solution_path = result_path.parent / "stdout.txt"
        problem_path = problem_root / result["problem_path"]
        case_name = f"{strategy}--{problem_id}"
        report_path = reports_dir / f"{case_name}.json"
        stdout_path = commands_dir / f"{case_name}.stdout"
        stderr_path = commands_dir / f"{case_name}.stderr"
        checker_problem_path = problem_path
        checker_solution_path = solution_path
        adapter_report = None
        adapter_report_path = None
        if result["category"] == "UEQ":
            checker_solution_path = adapted_dir / f"{case_name}.proof.p"
            checker_problem_path = adapted_dir / f"{case_name}.problem.p"
            adapter_report_path = reports_dir / f"{case_name}.adapter.json"
            adapter_report = UEQ_ADAPTER.write_proofcheck_view(
                solution_path=solution_path,
                prepared_path=checker_solution_path,
                controller_path=checker_problem_path,
            )
            adapter_report_path.write_bytes(
                ANALYZE.canonical_json(adapter_report) + b"\n"
            )
        else:
            checker_solution_path = adapted_dir / f"{case_name}.proof.p"
            adapter_report_path = reports_dir / f"{case_name}.adapter.json"
            adapter_report = SKOLEM_ADAPTER.write_proofcheck_view(
                solution_path=solution_path,
                prepared_path=checker_solution_path,
                report_path=adapter_report_path,
            )
        proof_command = [
            str(proofcheck),
            "-j",
            "2",
            "-t",
            "5",
            "-T",
            "120",
            "-p",
            str(checker_problem_path),
            str(checker_solution_path),
        ]
        command = [
            sys.executable,
            str(gate),
            str(problem_path),
            str(solution_path),
            "--report",
            str(report_path),
            "--timeout-seconds",
            "120",
            "--proof-command-json",
            json.dumps(proof_command, separators=(",", ":")),
        ]
        completed = run_command(
            command,
            cwd=repo,
            timeout=180,
            environment=environment,
            stdout_path=stdout_path,
            stderr_path=stderr_path,
        )
        report = json.loads(report_path.read_text(encoding="utf-8"))
        cases.append(
            {
                "strategy": strategy,
                "problem_id": problem_id,
                "category": result["category"],
                "checker_view": (
                    "alpha_audited_ueq_controller"
                    if result["category"] == "UEQ"
                    else "skolem_metadata_audited_proof"
                ),
                "solution_sha256": ANALYZE.sha256_file(solution_path),
                "checker_problem_sha256": ANALYZE.sha256_file(
                    checker_problem_path
                ),
                "checker_solution_sha256": ANALYZE.sha256_file(
                    checker_solution_path
                ),
                "adapter_report_sha256": (
                    ANALYZE.sha256_file(adapter_report_path)
                    if adapter_report_path is not None
                    else None
                ),
                "gate_returncode": completed.returncode,
                "gate_verdict": report["verdict"],
                "gate_reasons": report["reasons"],
                "gate_report_sha256": ANALYZE.sha256_file(report_path),
                "gate_stdout_sha256": ANALYZE.sha256_file(stdout_path),
                "gate_stderr_sha256": ANALYZE.sha256_file(stderr_path),
            }
        )
        print(
            f"{len(cases)}/{len(claims)} proof claims: "
            f"{strategy}/{problem_id} -> {report['verdict']}",
            flush=True,
        )
    verified_cases = sum(
        case["gate_returncode"] == 0
        and case["gate_verdict"] == "verified"
        for case in cases
    )
    body = {
        "schema_version": 1,
        "test_contract_id": contract["contract_id"],
        "test_binary_sha256": contract["binary_sha256"],
        "proofcheck": {
            "tag": PROOFCHECK.PROOFCHECK_TAG,
            "release_archive_sha256": PROOFCHECK.PROOFCHECK_SHA256,
            "executable_sha256": ANALYZE.sha256_file(proofcheck),
            "self_certify_returncode": self_certify.returncode,
            "self_certify_stdout_sha256": ANALYZE.sha256_file(
                commands_dir / "proofcheck-self-certify.stdout"
            ),
            "self_certify_stderr_sha256": ANALYZE.sha256_file(
                commands_dir / "proofcheck-self-certify.stderr"
            ),
        },
        "checker": {
            "name": "ProofCheck",
            "version": PROOFCHECK.PROOFCHECK_TAG,
            "source_archive_sha256": PROOFCHECK.PROOFCHECK_SHA256,
            "executable_sha256": ANALYZE.sha256_file(proofcheck),
        },
        "ueq_adapter": {
            "name": "proofcheck-1.0-alpha-source-controller",
            "source_sha256": ANALYZE.sha256_file(UEQ_ADAPTER_PATH),
            "logical_proof_fields_unchanged": True,
        },
        "skolem_metadata_adapter": {
            "name": "proofcheck-skolem-records-v1",
            "source_sha256": ANALYZE.sha256_file(SKOLEM_ADAPTER_PATH),
            "logical_formula_fields_unchanged": True,
        },
        "expected_cases": len(claims),
        "verified_cases": verified_cases,
        "all_verified": verified_cases == len(claims),
        "cases": cases,
    }
    return {
        **body,
        "report_id": hashlib.sha256(
            ANALYZE.canonical_json(body)
        ).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise VerificationError("independent proof checking requires Linux")
    output_root = arguments.output_root.resolve()
    proofcheck = find_or_download_proofcheck(
        output_root, arguments.proofcheck
    )
    report = verify_claims(
        repo=arguments.repo.resolve(),
        experiment_root=arguments.experiment_root.resolve(),
        problem_root=arguments.problem_root.resolve(),
        output_root=output_root,
        proofcheck=proofcheck,
    )
    report_path = output_root / "proof-validation.json"
    report_path.write_bytes(ANALYZE.canonical_json(report) + b"\n")
    print(
        f"OK: {report['verified_cases']}/{report['expected_cases']} "
        f"proof claims verified; report {report['report_id']}"
    )
    return 0 if report["all_verified"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        VerificationError,
        ANALYZE.AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
