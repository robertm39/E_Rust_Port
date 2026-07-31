#!/usr/bin/env python3
"""Run the frozen E-matching/MBQI comparison matrix."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path
from types import ModuleType
from typing import Any


METHODS = ("clausify", "ematch", "mbqi")
BUDGET_SECONDS = 4.0
MAX_INSTANCES = 100_000
MAX_STEPS = 250_000


def load_module(name: str, path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load module {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_environment(
    *,
    output_root: Path,
    repo_root: Path,
    z3_root: Path,
    repo_commit: str,
    z3_commit: str,
    here: Path,
    corpus: Path,
    adapter: Path,
    drat_trim: Path,
) -> None:
    environment = {
        "schema_version": 1,
        "platform": platform.platform(),
        "python": sys.version,
        "cpu_count": os.cpu_count(),
        "repo_commit": repo_commit,
        "z3_commit": z3_commit,
        "z3_quantifier_source_sha256": sha256_file(
            z3_root / "src/smt/smt_quantifier.cpp"
        ),
        "z3_model_finder_source_sha256": sha256_file(
            z3_root / "src/smt/smt_model_finder.cpp"
        ),
        "corpus_sha256": sha256_file(corpus),
        "preregistration_sha256": sha256_file(
            here / "PREREGISTRATION.md"
        ),
        "worker_sha256": sha256_file(here / "quantifier_worker.py"),
        "verifier_sha256": sha256_file(here / "verify_certificate.py"),
        "analyzer_sha256": sha256_file(here / "analyze.py"),
        "cadical_driver_sha256": sha256_file(adapter),
        "drat_trim_sha256": sha256_file(drat_trim),
        "budget_seconds": BUDGET_SECONDS,
        "max_instances": MAX_INSTANCES,
        "max_steps": MAX_STEPS,
    }
    (output_root / "environment.json").write_text(
        json.dumps(environment, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def run_checked(
    *,
    command: list[str],
    stdout_path: Path,
    stderr_path: Path,
    timeout: float,
) -> subprocess.CompletedProcess[str]:
    started = time.monotonic()
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    stdout_path.write_text(completed.stdout, encoding="utf-8", newline="\n")
    stderr_path.write_text(completed.stderr, encoding="utf-8", newline="\n")
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed after {time.monotonic() - started:.3f}s: "
            + " ".join(command)
            + "\n"
            + (completed.stdout + completed.stderr)[-4000:]
        )
    return completed


def run_one(
    *,
    python: str,
    worker: Path,
    verifier: Path,
    repo_root: Path,
    problem: Path,
    adapter: Path,
    drat_trim: Path,
    output: Path,
    method: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    output.mkdir(parents=True)
    run_checked(
        command=[
            python,
            str(worker),
            "--method",
            method,
            "--repo-root",
            str(repo_root),
            "--problem",
            str(problem),
            "--cadical-driver",
            str(adapter),
            "--drat-trim",
            str(drat_trim),
            "--output-root",
            str(output),
            "--budget-seconds",
            str(BUDGET_SECONDS),
            "--max-instances",
            str(MAX_INSTANCES),
            "--max-steps",
            str(MAX_STEPS),
        ],
        stdout_path=output / "worker.stdout.txt",
        stderr_path=output / "worker.stderr.txt",
        timeout=BUDGET_SECONDS + 150,
    )
    certificate_path = output / "certificate.json"
    completed = run_checked(
        command=[
            python,
            str(verifier),
            "--certificate",
            str(certificate_path),
            "--problem",
            str(problem),
            "--repo-root",
            str(repo_root),
            "--drat-trim",
            str(drat_trim),
        ],
        stdout_path=output / "verify.stdout.txt",
        stderr_path=output / "verify.stderr.txt",
        timeout=180,
    )
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    validation = json.loads(completed.stdout)
    return certificate, validation


def recover_completed_output(
    output: Path,
) -> tuple[dict[str, Any], dict[str, Any]] | None:
    certificate_path = output / "certificate.json"
    verification_path = output / "verify.stdout.txt"
    if not certificate_path.is_file() or not verification_path.is_file():
        return None
    try:
        certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
        validation = json.loads(verification_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return certificate, validation


def record_result(
    *,
    run_id: str,
    kind: str,
    problem_id: str,
    partition: str,
    family: str,
    repetition: int,
    method: str,
    expected_status: str,
    source_path: str,
    output_path: str,
    certificate: dict[str, Any],
    validation: dict[str, Any],
) -> dict[str, Any]:
    status = certificate["status"]
    terminal_checked = bool(
        validation.get("proof_checked") or validation.get("model_checked")
    )
    return {
        "schema_version": 1,
        "run_id": run_id,
        "kind": kind,
        "problem_id": problem_id,
        "partition": partition,
        "family": family,
        "repetition": repetition,
        "method": method,
        "expected_status": expected_status,
        "source_path": source_path,
        "output_path": output_path,
        "status": status,
        "termination_reason": certificate["termination_reason"],
        "verified": (
            status in {"sat", "unsat"}
            and status == expected_status
            and terminal_checked
        ),
        "validation_passed": True,
        "validation": validation,
        "semantic_sha256": certificate["semantic_sha256"],
        "instances_sha256": certificate["instances_sha256"],
        "generated_instances": certificate["generated_instances"],
        "ground_instance_count": certificate["ground_instance_count"],
        "search_wall_seconds": certificate["search_wall_seconds"],
        "search_user_seconds": certificate["search_user_seconds"],
        "search_system_seconds": certificate["search_system_seconds"],
        "search_max_rss_kib": certificate["search_max_rss_kib"],
        "sat_calls": certificate["sat_calls"],
        "sat_ns": certificate["sat_ns"],
        "refinement_iterations": certificate["refinement_iterations"],
        "enumerated_substitutions": certificate["enumerated_substitutions"],
        "method_data": certificate["method_data"],
        "proof_bytes": (
            certificate["proof"]["proof_bytes"]
            if certificate["proof"] is not None
            else 0
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--z3-root", type=Path, required=True)
    parser.add_argument("--repo-commit", required=True)
    parser.add_argument("--z3-commit", required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--resume", action="store_true")
    arguments = parser.parse_args()
    repo_root = arguments.repo_root.resolve()
    z3_root = arguments.z3_root.resolve()
    problem_root = arguments.problem_root.resolve()
    corpus_path = arguments.corpus.resolve()
    adapter = arguments.cadical_driver.resolve()
    drat_trim = arguments.drat_trim.resolve()
    output_root = arguments.output_root.resolve()
    if (
        output_root.exists()
        and any(output_root.iterdir())
        and not arguments.resume
    ):
        raise ValueError(f"output root is not empty: {output_root}")
    output_root.mkdir(parents=True, exist_ok=True)
    here = Path(__file__).resolve().parent
    worker = here / "quantifier_worker.py"
    verifier = here / "verify_certificate.py"
    python = sys.executable

    environment_path = output_root / "environment.json"
    if not environment_path.exists():
        write_environment(
            output_root=output_root,
            repo_root=repo_root,
            z3_root=z3_root,
            repo_commit=arguments.repo_commit,
            z3_commit=arguments.z3_commit,
            here=here,
            corpus=corpus_path,
            adapter=adapter,
            drat_trim=drat_trim,
        )
    records = [
        json.loads(line)
        for line in corpus_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    manifest = records[0]
    problems = records[1:]
    (output_root / "corpus-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    results_path = output_root / "results.jsonl"

    hand_expectations = {
        "ground-unsat": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "unary-chain-unsat": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "unary-chain-sat": {
            "clausify": "sat",
            "ematch": "sat",
            "mbqi": "sat",
        },
        "multipattern-unsat": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "incomplete-trigger-sat": {
            "clausify": "sat",
            "ematch": "unknown",
            "mbqi": "sat",
        },
    }
    run_specs: list[dict[str, Any]] = []
    for problem_id, expected_by_method in hand_expectations.items():
        for method in METHODS:
            run_specs.append(
                {
                    "kind": "hand",
                    "problem_id": problem_id,
                    "partition": "hand",
                    "family": "hand",
                    "repetition": 1,
                    "method": method,
                    "expected_status": expected_by_method[method],
                    "problem": here / "hand" / f"{problem_id}.p",
                    "source_path": str(
                        Path("experiments")
                        / here.name
                        / "hand"
                        / f"{problem_id}.p"
                    ),
                }
            )
    for record in problems:
        repetitions = 1 if record["holdout_split"] == "train" else 2
        expected = (
            "sat"
            if record["expected_class"] == "satisfiable"
            else "unsat"
        )
        for repetition in range(1, repetitions + 1):
            for method in METHODS:
                run_specs.append(
                    {
                        "kind": "corpus",
                        "problem_id": record["problem_id"],
                        "partition": record["holdout_split"],
                        "family": record["family"],
                        "repetition": repetition,
                        "method": method,
                        "expected_status": expected,
                        "problem": problem_root / record["path"],
                        "source_path": record["path"],
                    }
                )

    completed_run_ids: set[str] = set()
    if arguments.resume and results_path.exists():
        completed_run_ids = {
            json.loads(line)["run_id"]
            for line in results_path.read_text(encoding="utf-8").splitlines()
            if line
        }
    mode = "a" if arguments.resume else "w"
    with results_path.open(mode, encoding="utf-8", newline="\n") as results:
        for index, specification in enumerate(run_specs, start=1):
            coordinate = (
                f"{specification['problem_id']}-r"
                f"{specification['repetition']}"
            )
            run_id = f"{coordinate}/{specification['method']}"
            if run_id in completed_run_ids:
                continue
            print(f"[{index}/{len(run_specs)}] {run_id}", flush=True)
            canonical_output = (
                output_root
                / "runs"
                / coordinate
                / specification["method"]
            )
            recovered = recover_completed_output(canonical_output)
            output = canonical_output
            if recovered is None:
                attempt = 1
                while output.exists():
                    output = canonical_output.with_name(
                        f"{specification['method']}-resume-{attempt}"
                    )
                    attempt += 1
                certificate, validation = run_one(
                    python=python,
                    worker=worker,
                    verifier=verifier,
                    repo_root=repo_root,
                    problem=specification["problem"],
                    adapter=adapter,
                    drat_trim=drat_trim,
                    output=output,
                    method=specification["method"],
                )
            else:
                certificate, validation = recovered
            record = record_result(
                run_id=run_id,
                kind=specification["kind"],
                problem_id=specification["problem_id"],
                partition=specification["partition"],
                family=specification["family"],
                repetition=specification["repetition"],
                method=specification["method"],
                expected_status=specification["expected_status"],
                source_path=specification["source_path"],
                output_path=str(output.relative_to(output_root)),
                certificate=certificate,
                validation=validation,
            )
            results.write(json.dumps(record, sort_keys=True) + "\n")
            results.flush()

    analyzer = load_module("ematching_mbqi_analysis", here / "analyze.py")
    analysis = analyzer.analyze(output_root)
    print(
        json.dumps(
            {
                "runs": analysis["runs"],
                "correctness": analysis["correctness"]["passed"],
                "decision": analysis["decision"]["result"],
                "analysis_sha256": analysis["analysis_sha256"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
