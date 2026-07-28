#!/usr/bin/env python3
"""Independently verify held-out and focused higher-order proof claims."""

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
ADAPTER_PATH = EXPERIMENT_ROOT / "norgler_adapter.py"
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
NORG_ARCHIVE_SHA256 = (
    "22cd1042af79ae1947e8478367c24a1d4b1e0208e78a49b3d8f66a222c5b9aaf"
)
NORG_JAR_SHA256 = (
    "29e9f5210fe9908c50cdc15f305bf08ae6930c0e768cd9eb42ae1ccd8ae1c6bf"
)
E_HO_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
E_HO_SHA256 = (
    "50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a"
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ANALYZE = load_module("higher_order_gap_verify_analyze", BASE_ANALYZE_PATH)
ADAPTER = load_module("higher_order_gap_verify_adapter", ADAPTER_PATH)


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


def norgler_command(
    *,
    java: str,
    jar: Path,
    eprover_ho: Path,
    problem_path: Path,
    proof_path: Path,
    checker_seconds: int,
) -> list[str]:
    return [
        java,
        "-Xms256m",
        "-Xmx4g",
        "-jar",
        str(jar),
        "--problem",
        str(problem_path),
        "--verbosity",
        "4",
        "--eprover-path",
        str(eprover_ho),
        "--mace4-path",
        "/bin/false",
        "--parallel-mode",
        "steps",
        "--timeout",
        str(checker_seconds),
        "--relax-annotation-format",
        str(proof_path),
    ]


def run_gate(
    *,
    repo: Path,
    java: str,
    jar: Path,
    eprover_ho: Path,
    problem_path: Path,
    solution_path: Path,
    checker_solution_path: Path,
    case_name: str,
    commands_dir: Path,
    reports_dir: Path,
    environment: dict[str, str],
    checker_seconds: int,
) -> dict[str, Any]:
    report_path = reports_dir / f"{case_name}.gate.json"
    stdout_path = commands_dir / f"{case_name}.gate.stdout"
    stderr_path = commands_dir / f"{case_name}.gate.stderr"
    proof_command = norgler_command(
        java=java,
        jar=jar,
        eprover_ho=eprover_ho,
        problem_path=problem_path,
        proof_path=checker_solution_path,
        checker_seconds=checker_seconds,
    )
    command = [
        sys.executable,
        str(repo / "tools" / "validation" / "validate_tptp_solution.py"),
        str(problem_path),
        str(solution_path),
        "--report",
        str(report_path),
        "--timeout-seconds",
        str(checker_seconds + 20),
        "--proof-command-json",
        json.dumps(proof_command, separators=(",", ":")),
    ]
    completed = run_command(
        command,
        cwd=repo,
        timeout=checker_seconds + 40,
        environment=environment,
        stdout_path=stdout_path,
        stderr_path=stderr_path,
    )
    report = json.loads(report_path.read_text(encoding="utf-8"))
    return {
        "problem_sha256": ANALYZE.sha256_file(problem_path),
        "solution_sha256": ANALYZE.sha256_file(solution_path),
        "checker_solution_sha256": ANALYZE.sha256_file(
            checker_solution_path
        ),
        "gate_returncode": completed.returncode,
        "gate_verdict": report["verdict"],
        "gate_reasons": report["reasons"],
        "gate_report_sha256": ANALYZE.sha256_file(report_path),
        "gate_stdout_sha256": ANALYZE.sha256_file(stdout_path),
        "gate_stderr_sha256": ANALYZE.sha256_file(stderr_path),
    }


def adapt_solution(
    *,
    solution_path: Path,
    case_name: str,
    adapted_dir: Path,
    reports_dir: Path,
) -> tuple[Path, Path, dict[str, Any]]:
    prepared_path = adapted_dir / f"{case_name}.proof.p"
    report_path = reports_dir / f"{case_name}.adapter.json"
    report = ADAPTER.write_norgler_view(
        solution_path=solution_path,
        prepared_path=prepared_path,
        report_path=report_path,
    )
    return prepared_path, report_path, report


def run_focused_case(
    *,
    repo: Path,
    binary: Path,
    focused_problem: Path,
    output_root: Path,
    java: str,
    jar: Path,
    eprover_ho: Path,
    adapted_dir: Path,
    commands_dir: Path,
    reports_dir: Path,
    environment: dict[str, str],
    checker_seconds: int,
) -> dict[str, Any]:
    focused_dir = output_root / "focused-pos-ext"
    focused_dir.mkdir(exist_ok=True)
    solution_path = focused_dir / "solution.p"
    stderr_path = focused_dir / "umlaut.stderr"
    telemetry_path = focused_dir / "telemetry.json"
    completed = run_command(
        [
            str(binary),
            "--pos-ext=all",
            "--neg-ext=off",
            "--arg-cong=off",
            "--tstp-out",
            "--proof-object=1",
            f"--search-telemetry={telemetry_path}",
            str(focused_problem),
        ],
        cwd=repo,
        timeout=60,
        environment=environment,
        stdout_path=solution_path,
        stderr_path=stderr_path,
    )
    solution_text = solution_path.read_text(encoding="utf-8", errors="replace")
    telemetry = json.loads(telemetry_path.read_text(encoding="utf-8"))
    pos_ext = ANALYZE.metric(
        {"_telemetry": telemetry},
        "inferences",
        "positive_extensionality",
    )
    neg_ext = ANALYZE.metric(
        {"_telemetry": telemetry},
        "inferences",
        "negative_extensionality",
    )
    if completed.returncode != 0:
        raise VerificationError(
            f"focused positive-extensionality run failed: {completed.returncode}"
        )
    if "SZS status Unsatisfiable" not in solution_text:
        raise VerificationError("focused run did not report Unsatisfiable")
    if "inference(pos_ext,[status(thm)]" not in solution_text:
        raise VerificationError("focused proof did not cite pos_ext")
    if pos_ext != 1 or neg_ext != 0:
        raise VerificationError(
            "focused inference counters are not PosExt=1 and NegExt=0"
        )
    case_name = "focused-pos-ext-only"
    prepared_path, adapter_report_path, adapter_report = adapt_solution(
        solution_path=solution_path,
        case_name=case_name,
        adapted_dir=adapted_dir,
        reports_dir=reports_dir,
    )
    gate = run_gate(
        repo=repo,
        java=java,
        jar=jar,
        eprover_ho=eprover_ho,
        problem_path=focused_problem,
        solution_path=solution_path,
        checker_solution_path=prepared_path,
        case_name=case_name,
        commands_dir=commands_dir,
        reports_dir=reports_dir,
        environment=environment,
        checker_seconds=checker_seconds,
    )
    return {
        "scope": "focused_positive_extensionality",
        "strategy": "pos_ext_all_neg_ext_off",
        "problem_id": focused_problem.stem,
        "category": "THF",
        "umlaut_returncode": completed.returncode,
        "umlaut_stderr_sha256": ANALYZE.sha256_file(stderr_path),
        "telemetry_sha256": ANALYZE.sha256_file(telemetry_path),
        "positive_extensionality": pos_ext,
        "negative_extensionality": neg_ext,
        "proof_contains_pos_ext": True,
        "adapter_report_id": adapter_report["report_id"],
        "adapter_report_sha256": ANALYZE.sha256_file(adapter_report_path),
        **gate,
    }


def verify_claims(
    *,
    repo: Path,
    experiment_root: Path,
    problem_root: Path,
    output_root: Path,
    java: str,
    jar: Path,
    eprover_ho: Path,
    binary: Path,
    focused_problem: Path,
    checker_seconds: int,
) -> dict[str, Any]:
    contract, results = ANALYZE.load_phase(experiment_root, "test")
    if "larger" not in contract["budgets"]:
        raise VerificationError("test contract has no larger budget")
    if ANALYZE.sha256_file(binary) != contract["binary_sha256"]:
        raise VerificationError("focused binary differs from the test binary")
    if ANALYZE.sha256_file(jar) != NORG_JAR_SHA256:
        raise VerificationError("Nörgler JAR hash differs from the pin")
    if ANALYZE.sha256_file(eprover_ho) != E_HO_SHA256:
        raise VerificationError("original E higher-order binary hash differs")

    claims = proof_claims(contract, results)
    output_root.mkdir(parents=True, exist_ok=True)
    adapted_dir = output_root / "adapted"
    commands_dir = output_root / "commands"
    reports_dir = output_root / "reports"
    adapted_dir.mkdir(exist_ok=True)
    commands_dir.mkdir(exist_ok=True)
    reports_dir.mkdir(exist_ok=True)
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")

    version_stdout = commands_dir / "norgler-version.stdout"
    version_stderr = commands_dir / "norgler-version.stderr"
    version = run_command(
        [java, "-jar", str(jar), "--version"],
        cwd=jar.parent,
        timeout=30,
        environment=environment,
        stdout_path=version_stdout,
        stderr_path=version_stderr,
    )
    version_text = (
        version_stdout.read_text(encoding="utf-8", errors="replace")
        + version_stderr.read_text(encoding="utf-8", errors="replace")
    )
    if version.returncode != 0 or "noergler 1.1" not in version_text:
        raise VerificationError("Nörgler version check failed")

    cases: list[dict[str, Any]] = []
    for strategy, problem_id, result in claims:
        result_path = Path(result["_path"])
        if not result_path.is_absolute():
            result_path = experiment_root / "test" / result_path
        solution_path = result_path.parent / "stdout.txt"
        problem_path = problem_root / result["problem_path"]
        case_name = f"{strategy}--{problem_id}"
        try:
            prepared_path, adapter_report_path, adapter_report = adapt_solution(
                solution_path=solution_path,
                case_name=case_name,
                adapted_dir=adapted_dir,
                reports_dir=reports_dir,
            )
            gate = run_gate(
                repo=repo,
                java=java,
                jar=jar,
                eprover_ho=eprover_ho,
                problem_path=problem_path,
                solution_path=solution_path,
                checker_solution_path=prepared_path,
                case_name=case_name,
                commands_dir=commands_dir,
                reports_dir=reports_dir,
                environment=environment,
                checker_seconds=checker_seconds,
            )
            case = {
                "scope": "held_out_larger_budget",
                "strategy": strategy,
                "problem_id": problem_id,
                "category": result["category"],
                "adapter_report_id": adapter_report["report_id"],
                "adapter_report_sha256": ANALYZE.sha256_file(
                    adapter_report_path
                ),
                **gate,
            }
        except (ADAPTER.AdapterError, OSError, subprocess.SubprocessError) as error:
            case = {
                "scope": "held_out_larger_budget",
                "strategy": strategy,
                "problem_id": problem_id,
                "category": result["category"],
                "gate_returncode": 3,
                "gate_verdict": "error",
                "gate_reasons": [f"{type(error).__name__}: {error}"],
            }
        cases.append(case)
        print(
            f"{len(cases)}/{len(claims)} held-out proof claims: "
            f"{strategy}/{problem_id} -> {case['gate_verdict']}",
            flush=True,
        )

    focused = run_focused_case(
        repo=repo,
        binary=binary,
        focused_problem=focused_problem,
        output_root=output_root,
        java=java,
        jar=jar,
        eprover_ho=eprover_ho,
        adapted_dir=adapted_dir,
        commands_dir=commands_dir,
        reports_dir=reports_dir,
        environment=environment,
        checker_seconds=checker_seconds,
    )
    cases.append(focused)
    verified_cases = sum(
        case["gate_returncode"] == 0
        and case["gate_verdict"] == "verified"
        for case in cases
    )
    body = {
        "schema_version": 1,
        "test_contract_id": contract["contract_id"],
        "test_binary_sha256": contract["binary_sha256"],
        "checker": {
            "name": "Nörgler",
            "version": "1.1",
            "license": "MIT",
            "source_archive_sha256": NORG_ARCHIVE_SHA256,
            "executable_jar_sha256": NORG_JAR_SHA256,
            "version_returncode": version.returncode,
            "version_stdout_sha256": ANALYZE.sha256_file(version_stdout),
            "version_stderr_sha256": ANALYZE.sha256_file(version_stderr),
            "semantic_backend": {
                "name": "E higher-order",
                "source_commit": E_HO_COMMIT,
                "executable_sha256": E_HO_SHA256,
            },
        },
        "adapter": {
            "name": "norgler-1.1-thf-exact-source-v1",
            "source_sha256": ANALYZE.sha256_file(ADAPTER_PATH),
            "changed_fields": ["input_leaf.body.exact_cited_source"],
            "non_parenthesis_token_stream_unchanged": True,
            "inference_sources_unchanged": True,
        },
        "checker_seconds_per_case": checker_seconds,
        "expected_held_out_cases": len(claims),
        "expected_focused_cases": 1,
        "expected_cases": len(cases),
        "verified_cases": verified_cases,
        "all_verified": verified_cases == len(cases),
        "cases": cases,
    }
    return {
        **body,
        "report_id": hashlib.sha256(ANALYZE.canonical_json(body)).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--norgler-jar", type=Path, required=True)
    parser.add_argument("--eprover-ho", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--focused-problem", type=Path, required=True)
    parser.add_argument("--checker-seconds", type=int, default=30)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise VerificationError("independent proof checking requires Linux")
    report = verify_claims(
        repo=arguments.repo.resolve(),
        experiment_root=arguments.experiment_root.resolve(),
        problem_root=arguments.problem_root.resolve(),
        output_root=arguments.output_root.resolve(),
        java=arguments.java,
        jar=arguments.norgler_jar.resolve(),
        eprover_ho=arguments.eprover_ho.resolve(),
        binary=arguments.binary.resolve(),
        focused_problem=arguments.focused_problem.resolve(),
        checker_seconds=arguments.checker_seconds,
    )
    report_path = arguments.output_root.resolve() / "proof-validation.json"
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
