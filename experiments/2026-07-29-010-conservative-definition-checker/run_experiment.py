#!/usr/bin/env python3
"""Evaluate an external checker for used conservative definitions."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


PROOFGUARD_COMMIT = "18fc573131648c9d1ed81e818f52f704c435033e"
PROOFGUARD_REMOTE = "https://github.com/ValueAchooMatthew/ATP-Research-Project.git"
PROOFCHECK_SHA256 = (
    "92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e"
)
SZS_STATUS_RE = re.compile(
    r"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
SUCCESS_STATUSES = {"unsatisfiable", "contradictoryaxioms", "theorem"}


class ExperimentError(RuntimeError):
    """A frozen integrity or correctness gate failed."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def parse_status(output: str) -> str:
    matches = SZS_STATUS_RE.findall(output)
    return matches[-1].lower() if matches else "missing"


def run_capture(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: float,
    env: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[bytes], float]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        env=env,
    )
    return completed, time.monotonic() - started


def write_process(
    root: Path,
    stem: str,
    completed: subprocess.CompletedProcess[bytes],
) -> None:
    (root / f"{stem}.stdout.txt").write_bytes(completed.stdout)
    (root / f"{stem}.stderr.txt").write_bytes(completed.stderr)


def checked_git_output(root: Path, *arguments: str) -> str:
    completed, _ = run_capture(
        ["git", "-C", str(root), *arguments],
        cwd=root,
        timeout=30,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise ExperimentError(f"git {' '.join(arguments)} failed: {detail}")
    return completed.stdout.decode("utf-8", errors="replace").strip()


def verify_proofguard_checkout(root: Path) -> dict[str, str]:
    commit = checked_git_output(root, "rev-parse", "HEAD")
    if commit != PROOFGUARD_COMMIT:
        raise ExperimentError(
            f"ProofGuard commit mismatch: expected {PROOFGUARD_COMMIT}, got {commit}"
        )
    remote = checked_git_output(root, "remote", "get-url", "origin")
    if remote.rstrip("/") != PROOFGUARD_REMOTE.rstrip("/"):
        raise ExperimentError(
            f"ProofGuard remote mismatch: expected {PROOFGUARD_REMOTE}, got {remote}"
        )
    if checked_git_output(root, "status", "--porcelain"):
        raise ExperimentError("ProofGuard checkout is dirty")
    checker = root / "proover-check"
    engine = root / "proover.py"
    if not checker.is_file() or not engine.is_file():
        raise ExperimentError("ProofGuard checkout is incomplete")
    return {
        "commit": commit,
        "remote": remote,
        "checker_sha256": sha256_file(checker),
        "engine_sha256": sha256_file(engine),
    }


def checker_environment(eprover: Path) -> dict[str, str]:
    environment = os.environ.copy()
    environment["ATP_EPROVER_BIN"] = str(eprover)
    return environment


def run_proofguard(
    checker: Path,
    eprover: Path,
    problem: Path,
    proof: Path,
    root: Path,
    stem: str,
    *,
    timeout: int = 120,
) -> dict[str, Any]:
    completed, wall_seconds = run_capture(
        [
            sys.executable,
            str(checker),
            "--quiet",
            "--time-limit",
            str(timeout),
            str(problem),
            str(proof),
        ],
        cwd=checker.parent,
        timeout=timeout + 10,
        env=checker_environment(eprover),
    )
    write_process(root, stem, completed)
    text = (completed.stdout + completed.stderr).decode(
        "utf-8", errors="replace"
    )
    return {
        "returncode": completed.returncode,
        "status": parse_status(text),
        "wall_seconds": wall_seconds,
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(completed.stderr).hexdigest(),
    }


def run_proofcheck(
    checker: Path,
    problem: Path,
    proof: Path,
    root: Path,
    stem: str,
    *,
    timeout: int = 120,
) -> dict[str, Any]:
    completed, wall_seconds = run_capture(
        [str(checker), "-p", str(problem), str(proof)],
        cwd=checker.parent,
        timeout=timeout,
    )
    write_process(root, stem, completed)
    text = (completed.stdout + completed.stderr).decode(
        "utf-8", errors="replace"
    )
    return {
        "returncode": completed.returncode,
        "status": parse_status(text),
        "wall_seconds": wall_seconds,
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(completed.stderr).hexdigest(),
    }


def run_positive_gate(
    validator: Path,
    checker: Path,
    eprover: Path,
    problem: Path,
    proof: Path,
    root: Path,
    stem: str,
    *,
    expected_returncode: int,
    expected_verdict: str,
) -> dict[str, Any]:
    report = root / f"{stem}.report.json"
    checker_command = json.dumps(
        [
            sys.executable,
            str(checker),
            "--quiet",
            "--time-limit",
            "120",
            "{problem}",
            "{artifact}",
        ],
        separators=(",", ":"),
    )
    completed, wall_seconds = run_capture(
        [
            sys.executable,
            str(validator),
            str(problem),
            str(proof),
            "--proof-command-json",
            checker_command,
            "--timeout-seconds",
            "130",
            "--report",
            str(report),
        ],
        cwd=validator.parent,
        timeout=150,
        env=checker_environment(eprover),
    )
    write_process(root, stem, completed)
    if completed.returncode != expected_returncode:
        raise ExperimentError(
            f"{stem}: validation gate returned {completed.returncode}, "
            f"expected {expected_returncode}"
        )
    payload = json.loads(report.read_text(encoding="utf-8"))
    if payload.get("verdict") != expected_verdict:
        raise ExperimentError(
            f"{stem}: validation verdict {payload.get('verdict')!r}, "
            f"expected {expected_verdict!r}"
        )
    return {
        "returncode": completed.returncode,
        "verdict": payload["verdict"],
        "wall_seconds": wall_seconds,
        "report_sha256": sha256_file(report),
    }


def mutation_cases(problem_text: str, proof_text: str) -> dict[str, tuple[str, str]]:
    definition = "(epred1_0<=>q)"
    if definition not in proof_text:
        raise ExperimentError("minimized proof definition fixture changed")
    parent = (
        "inference(split_equiv,[status(thm)],[test_definition])"
    )
    if parent not in proof_text:
        raise ExperimentError("minimized proof parent fixture changed")

    return {
        "reused-symbol": (
            problem_text + "\ncnf(prior_epred,axiom,epred1_0).\n",
            proof_text,
        ),
        "circular-definition": (
            problem_text,
            proof_text.replace(
                definition,
                "(epred1_0<=>(epred1_0|q))",
                1,
            ),
        ),
        "altered-body": (
            problem_text,
            proof_text.replace(definition, "(epred1_0<=>p)", 1),
        ),
        "omitted-ancestry": (
            problem_text,
            proof_text.replace(
                parent,
                "inference(split_equiv,[status(thm)],[q_source])",
                1,
            ),
        ),
    }


def run_umlaut(
    binary: Path,
    problem: Path,
    root: Path,
) -> tuple[Path, dict[str, Any]]:
    proof = root / "solution.txt"
    completed, wall_seconds = run_capture(
        [
            str(binary),
            "--auto",
            "--silent",
            "--tstp-out",
            "--proof-object=1",
            "--cpu-limit=20",
            "--memory-limit=2048",
            "--split-clauses=7",
            "--split-method=2",
            "--split-aggressive",
            "--split-reuse-defs",
            str(problem),
        ],
        cwd=binary.parent,
        timeout=60,
    )
    proof.write_bytes(completed.stdout)
    (root / "prover.stderr.txt").write_bytes(completed.stderr)
    output = completed.stdout.decode("utf-8", errors="replace")
    status = parse_status(output)
    if status not in SUCCESS_STATUSES:
        raise ExperimentError(f"PUZ008-2 prover status was {status}")
    if "% SZS output end CNFRefutation" not in output:
        raise ExperimentError("PUZ008-2 proof block is incomplete")
    return proof, {
        "returncode": completed.returncode,
        "status": status,
        "wall_seconds": wall_seconds,
        "proof_sha256": sha256_file(proof),
        "proof_bytes": proof.stat().st_size,
    }


def stable_report_id(payload: dict[str, Any]) -> str:
    stable = json.loads(json.dumps(payload))
    for case in stable.get("cases", {}).values():
        for checker in case.get("checkers", {}).values():
            checker.pop("wall_seconds", None)
        if "gate" in case:
            case["gate"].pop("wall_seconds", None)
        if "prover" in case:
            case["prover"].pop("wall_seconds", None)
    encoded = json.dumps(
        stable, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--eprover", type=Path, required=True)
    parser.add_argument("--proofguard-root", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--puz-problem", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    artifact_root = args.artifact_root.resolve()
    artifact_root.mkdir(parents=True, exist_ok=False)

    validator = repo_root / "tools/validation/validate_tptp_solution.py"
    fixture_root = (
        repo_root
        / "experiments/2026-07-29-009-tstp-input-leaf-provenance/fixtures"
    )
    minimized_problem = fixture_root / "used-definition-problem.p"
    minimized_proof = fixture_root / "used-definition-proof.s"
    proofguard = args.proofguard_root.resolve() / "proover-check"
    proofcheck = args.proofcheck.resolve()
    eprover = args.eprover.resolve()
    umlaut = args.umlaut.resolve()
    puz_problem = args.puz_problem.resolve()
    required = [
        validator,
        minimized_problem,
        minimized_proof,
        proofguard,
        proofcheck,
        eprover,
        umlaut,
        puz_problem,
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise ExperimentError(f"missing required files: {missing}")

    checkout = verify_proofguard_checkout(args.proofguard_root.resolve())
    if sha256_file(proofcheck) != PROOFCHECK_SHA256:
        raise ExperimentError("ProofCheck executable hash mismatch")

    environment = checker_environment(eprover)
    proofguard_tests, _ = run_capture(
        [
            sys.executable,
            "tests/run_tests.py",
            "--skip-slow",
        ],
        cwd=args.proofguard_root.resolve(),
        timeout=300,
        env=environment,
    )
    write_process(artifact_root, "proofguard-tests", proofguard_tests)
    if proofguard_tests.returncode != 0:
        raise ExperimentError("ProofGuard upstream test suite failed")

    proofcheck_tests, _ = run_capture(
        [str(proofcheck), "-self-certify"],
        cwd=proofcheck.parent,
        timeout=300,
    )
    write_process(artifact_root, "proofcheck-self-certify", proofcheck_tests)
    self_text = (proofcheck_tests.stdout + proofcheck_tests.stderr).decode(
        "utf-8", errors="replace"
    )
    if (
        proofcheck_tests.returncode != 0
        or "Tests: 117 run, 117 passed, 0 failed" not in self_text
    ):
        raise ExperimentError("ProofCheck self-certification failed")

    cases: dict[str, Any] = {}
    minimized_root = artifact_root / "minimized-valid"
    minimized_root.mkdir()
    minimized_checkers = {
        "proofguard": run_proofguard(
            proofguard,
            eprover,
            minimized_problem,
            minimized_proof,
            minimized_root,
            "proofguard",
        ),
        "proofcheck": run_proofcheck(
            proofcheck,
            minimized_problem,
            minimized_proof,
            minimized_root,
            "proofcheck",
        ),
    }
    if minimized_checkers["proofguard"]["status"] != "verifiedgood":
        raise ExperimentError("ProofGuard did not verify the minimized proof")
    if minimized_checkers["proofcheck"]["status"] != "unknown":
        raise ExperimentError("ProofCheck coverage control changed")
    cases["minimized-valid"] = {
        "problem_sha256": sha256_file(minimized_problem),
        "proof_sha256": sha256_file(minimized_proof),
        "checkers": minimized_checkers,
        "gate": run_positive_gate(
            validator,
            proofguard,
            eprover,
            minimized_problem,
            minimized_proof,
            minimized_root,
            "positive-gate",
            expected_returncode=0,
            expected_verdict="verified",
        ),
    }

    problem_text = minimized_problem.read_text(encoding="utf-8")
    proof_text = minimized_proof.read_text(encoding="utf-8")
    for name, (mutant_problem_text, mutant_proof_text) in mutation_cases(
        problem_text, proof_text
    ).items():
        case_root = artifact_root / name
        case_root.mkdir()
        problem_path = case_root / "problem.p"
        proof_path = case_root / "proof.s"
        problem_path.write_text(mutant_problem_text, encoding="utf-8")
        proof_path.write_text(mutant_proof_text, encoding="utf-8")
        proofguard_result = run_proofguard(
            proofguard,
            eprover,
            problem_path,
            proof_path,
            case_root,
            "proofguard",
        )
        if proofguard_result["status"] != "verifiedbad":
            raise ExperimentError(
                f"{name}: ProofGuard returned {proofguard_result['status']}"
            )
        cases[name] = {
            "problem_sha256": sha256_file(problem_path),
            "proof_sha256": sha256_file(proof_path),
            "checkers": {"proofguard": proofguard_result},
            "gate": run_positive_gate(
                validator,
                proofguard,
                eprover,
                problem_path,
                proof_path,
                case_root,
                "positive-gate",
                expected_returncode=1,
                expected_verdict="rejected",
            ),
        }

    puz_root = artifact_root / "puz008-2-static"
    puz_root.mkdir()
    puz_proof, prover = run_umlaut(umlaut, puz_problem, puz_root)
    puz_checkers = {
        "proofguard": run_proofguard(
            proofguard,
            eprover,
            puz_problem,
            puz_proof,
            puz_root,
            "proofguard",
        ),
        "proofcheck": run_proofcheck(
            proofcheck,
            puz_problem,
            puz_proof,
            puz_root,
            "proofcheck",
        ),
    }
    if puz_checkers["proofguard"]["status"] != "verifiedgood":
        raise ExperimentError("ProofGuard did not verify PUZ008-2 static split")
    if puz_checkers["proofcheck"]["status"] != "unknown":
        raise ExperimentError("PUZ008-2 ProofCheck coverage control changed")
    cases["puz008-2-static"] = {
        "problem_sha256": sha256_file(puz_problem),
        "prover": prover,
        "checkers": puz_checkers,
        "gate": run_positive_gate(
            validator,
            proofguard,
            eprover,
            puz_problem,
            puz_proof,
            puz_root,
            "positive-gate",
            expected_returncode=0,
            expected_verdict="verified",
        ),
    }

    report: dict[str, Any] = {
        "schema_version": 1,
        "proofguard": checkout,
        "proofcheck": {
            "executable_sha256": sha256_file(proofcheck),
            "self_certified_tests": 117,
        },
        "binaries": {
            "umlaut_sha256": sha256_file(umlaut),
            "eprover_sha256": sha256_file(eprover),
        },
        "controller_sha256": sha256_file(Path(__file__).resolve()),
        "cases": cases,
        "decision": {
            "advance": True,
            "path": "caller-supplied ProofGuard external command",
            "proofguard_redistributed": False,
        },
    }
    report["report_id"] = stable_report_id(report)
    report_path = artifact_root / "report.json"
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report["decision"], sort_keys=True))
    print(f"report_id={report['report_id']}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"experiment error: {error}", file=sys.stderr)
        raise SystemExit(1)
