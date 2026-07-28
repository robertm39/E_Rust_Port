#!/usr/bin/env python3
"""Exercise Umlaut's solution-validation gate on native Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any, Sequence


PROOFCHECK_URL = (
    "https://github.com/AlgorithmicTruth/proofcheck-releases/"
    "releases/download/v1.0/proofcheck-linux-x86_64.zip"
)
PROOFCHECK_SHA256 = (
    "4c4c6f71f9d8235450c6889863963ba242249c2d8d63d0461ea3acb7814b6aaa"
)
PROOFCHECK_TAG = "v1.0"

FIXTURES = (
    ("fof_theorem", "proof", 0),
    ("cnf_unsatisfiable", "proof", 0),
    ("fof_contradictory_axioms", "proof", 2),
    ("fof_counter_satisfiable", "model_gap", 2),
    ("cnf_satisfiable", "model_gap", 2),
    ("tff_theorem", "typed_proof", None),
    ("thf_theorem", "higher_order_proof", 2),
)


def sha256(path: Path) -> str:
    """Return the SHA-256 of one file."""

    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    """Write stable JSON."""

    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: int,
    stdout_path: Path,
    stderr_path: Path,
) -> subprocess.CompletedProcess[bytes]:
    """Run one command and retain its byte streams."""

    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        capture_output=True,
        timeout=timeout,
    )
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    return completed


def run_text(command: Sequence[str], *, cwd: Path, timeout: int = 30) -> str:
    """Run one metadata command and require success."""

    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"{completed.stdout}"
        )
    return completed.stdout.strip()


def download_proofcheck(destination: Path) -> Path:
    """Download, hash-check, and safely extract the pinned ProofCheck release."""

    archive = destination / "proofcheck-linux-x86_64.zip"
    request = urllib.request.Request(
        PROOFCHECK_URL,
        headers={"User-Agent": "Umlaut-soundness-validation/1"},
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        archive.write_bytes(response.read())
    actual_hash = sha256(archive)
    if actual_hash != PROOFCHECK_SHA256:
        raise RuntimeError(
            "ProofCheck archive hash mismatch: "
            f"expected {PROOFCHECK_SHA256}, got {actual_hash}"
        )

    extracted = destination / "proofcheck"
    extracted.mkdir()
    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            path = PurePosixPath(member.filename)
            if path.is_absolute() or ".." in path.parts:
                raise RuntimeError(f"unsafe ProofCheck archive member: {member.filename}")
            target = extracted.joinpath(*path.parts)
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with bundle.open(member) as source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            mode = member.external_attr >> 16
            if mode:
                target.chmod(mode & 0o777)

    candidates = [
        path
        for path in extracted.rglob("proofcheck")
        if path.is_file() and path.name == "proofcheck"
    ]
    if len(candidates) != 1:
        raise RuntimeError(
            f"expected one proofcheck executable, found {len(candidates)}"
        )
    proofcheck = candidates[0]
    proofcheck.chmod(proofcheck.stat().st_mode | stat.S_IXUSR)
    for sibling in proofcheck.parent.iterdir():
        if sibling.is_file() and sibling.suffix not in {".md", ".txt"}:
            sibling.chmod(sibling.stat().st_mode | stat.S_IXUSR)
    return proofcheck


def solution_statuses(text: str) -> list[str]:
    """Return the SZS statuses from one captured solution."""

    return re.findall(
        r"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
        text,
        flags=re.MULTILINE | re.IGNORECASE,
    )


def checker_status(text: str) -> str | None:
    """Return the last checker SZS status."""

    statuses = solution_statuses(text)
    return statuses[-1] if statuses else None


def gate_command(
    *,
    python: str,
    gate: Path,
    problem: Path,
    solution: Path,
    report: Path,
    proofcheck: Path | None,
) -> list[str]:
    """Construct one shell-free validation-gate command."""

    command = [
        python,
        str(gate),
        str(problem),
        str(solution),
        "--report",
        str(report),
        "--timeout-seconds",
        "120",
    ]
    if proofcheck is not None:
        proof_command = [
            str(proofcheck),
            "-j",
            "2",
            "-t",
            "5",
            "-T",
            "120",
            "-p",
            "{problem}",
            "{artifact}",
        ]
        command.extend(
            ["--proof-command-json", json.dumps(proof_command, separators=(",", ":"))]
        )
    return command


def mutate_leaf(solution: str) -> str:
    """Corrupt one copied FOF axiom while preserving a false proof root."""

    mutated, count = re.subn(
        r"(fof\(\s*ax\s*,\s*axiom\s*,\s*)p\(a\)",
        r"\1q(a)",
        solution,
        count=1,
        flags=re.IGNORECASE,
    )
    if count != 1:
        raise RuntimeError("could not locate the FOF axiom leaf to corrupt")
    return mutated


def main() -> int:
    """Run the native validation experiment."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-snapshot-sha256", required=True)
    args = parser.parse_args()

    repo = args.repo.resolve()
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    command_dir = artifact_dir / "commands"
    command_dir.mkdir()
    output_dir = artifact_dir / "solutions"
    output_dir.mkdir()
    report_dir = artifact_dir / "reports"
    report_dir.mkdir()
    external_dir = artifact_dir / "external"
    external_dir.mkdir()

    gate = repo / "tools/validation/validate_tptp_solution.py"
    fixture_dir = (
        repo
        / "experiments/2026-07-27-004-soundness-validation-gates/fixtures"
    )
    binary = repo / "target/release/umlaut"

    proofcheck = download_proofcheck(external_dir)
    self_certify = run(
        [str(proofcheck), "-self-certify"],
        cwd=proofcheck.parent,
        timeout=300,
        stdout_path=command_dir / "proofcheck-self-certify.stdout",
        stderr_path=command_dir / "proofcheck-self-certify.stderr",
    )
    self_certify_text = (
        (command_dir / "proofcheck-self-certify.stdout").read_text(
            encoding="utf-8", errors="replace"
        )
        + (command_dir / "proofcheck-self-certify.stderr").read_text(
            encoding="utf-8", errors="replace"
        )
    )
    if self_certify.returncode != 0 or "117 passed" not in self_certify_text:
        raise RuntimeError("ProofCheck self-certification did not pass all 117 tests")

    build = run(
        ["cargo", "build", "--locked", "--release", "--bin", "umlaut"],
        cwd=repo,
        timeout=3600,
        stdout_path=command_dir / "cargo-build.stdout",
        stderr_path=command_dir / "cargo-build.stderr",
    )
    if build.returncode != 0:
        raise RuntimeError("release Umlaut build failed")

    cases: list[dict[str, Any]] = []
    for name, category, expected_gate_exit in FIXTURES:
        problem = fixture_dir / f"{name}.p"
        solution = output_dir / f"{name}.s"
        prover_stderr = output_dir / f"{name}.stderr"
        prover = run(
            [
                str(binary),
                "--auto",
                "--tstp-out",
                "--proof-object=1",
                "--cpu-limit=30",
                str(problem),
            ],
            cwd=repo,
            timeout=120,
            stdout_path=solution,
            stderr_path=prover_stderr,
        )
        solution_text = solution.read_text(encoding="utf-8", errors="replace")
        statuses = solution_statuses(solution_text)
        if not statuses:
            raise RuntimeError(f"{name} did not emit an SZS status")

        use_proofcheck = category in {"proof", "typed_proof"}
        report_path = report_dir / f"{name}.json"
        gate_result = run(
            gate_command(
                python=sys.executable,
                gate=gate,
                problem=problem,
                solution=solution,
                report=report_path,
                proofcheck=proofcheck if use_proofcheck else None,
            ),
            cwd=repo,
            timeout=180,
            stdout_path=command_dir / f"{name}-gate.stdout",
            stderr_path=command_dir / f"{name}-gate.stderr",
        )
        report = json.loads(report_path.read_text(encoding="utf-8"))
        if expected_gate_exit is not None and gate_result.returncode != expected_gate_exit:
            raise RuntimeError(
                f"{name} gate exit {gate_result.returncode}, "
                f"expected {expected_gate_exit}: {report}"
            )
        cases.append(
            {
                "name": name,
                "category": category,
                "prover_returncode": prover.returncode,
                "solution_statuses": statuses,
                "solution_sha256": sha256(solution),
                "solution_bytes": solution.stat().st_size,
                "gate_returncode": gate_result.returncode,
                "gate_verdict": report["verdict"],
                "gate_reasons": report["reasons"],
            }
        )

    theorem_solution = output_dir / "fof_theorem.s"
    corrupt_solution = output_dir / "fof_theorem-corrupt-leaf.s"
    corrupt_solution.write_text(
        mutate_leaf(theorem_solution.read_text(encoding="utf-8")),
        encoding="utf-8",
    )
    corrupt_report_path = report_dir / "fof_theorem-corrupt-leaf.json"
    corrupt_gate = run(
        gate_command(
            python=sys.executable,
            gate=gate,
            problem=fixture_dir / "fof_theorem.p",
            solution=corrupt_solution,
            report=corrupt_report_path,
            proofcheck=proofcheck,
        ),
        cwd=repo,
        timeout=180,
        stdout_path=command_dir / "corrupt-proof-gate.stdout",
        stderr_path=command_dir / "corrupt-proof-gate.stderr",
    )
    corrupt_report = json.loads(corrupt_report_path.read_text(encoding="utf-8"))
    if corrupt_gate.returncode != 1 or corrupt_report["verdict"] != "rejected":
        raise RuntimeError("corrupted proof was not rejected")
    proof_checks = [
        check
        for check in corrupt_report["checks"]
        if check.get("name") == "external_proof_checker"
    ]
    if not proof_checks or checker_status(
        proof_checks[-1]["stdout"] + "\n" + proof_checks[-1]["stderr"]
    ) != "VerifiedBad":
        raise RuntimeError("ProofCheck did not report VerifiedBad for corrupted proof")

    forged_solution = output_dir / "known-nontheorem-forged-proof.s"
    forged_solution.write_bytes(theorem_solution.read_bytes())
    forged_report_path = report_dir / "known-nontheorem-forged-proof.json"
    forged_gate = run(
        gate_command(
            python=sys.executable,
            gate=gate,
            problem=fixture_dir / "fof_counter_satisfiable.p",
            solution=forged_solution,
            report=forged_report_path,
            proofcheck=proofcheck,
        ),
        cwd=repo,
        timeout=180,
        stdout_path=command_dir / "known-nontheorem-gate.stdout",
        stderr_path=command_dir / "known-nontheorem-gate.stderr",
    )
    forged_report = json.loads(forged_report_path.read_text(encoding="utf-8"))
    if forged_gate.returncode != 1 or forged_report["verdict"] != "rejected":
        raise RuntimeError("known non-theorem accepted a forged theorem claim")

    summary = {
        "schema": "umlaut.soundness-validation-gates",
        "source_commit": args.source_commit,
        "source_snapshot_sha256": args.source_snapshot_sha256,
        "host": {
            "platform": platform.platform(),
            "uname": list(platform.uname()),
            "python": sys.version,
            "rustc": run_text(["rustc", "--version"], cwd=repo),
            "cargo": run_text(["cargo", "--version"], cwd=repo),
        },
        "umlaut_binary_sha256": sha256(binary),
        "proofcheck": {
            "tag": PROOFCHECK_TAG,
            "url": PROOFCHECK_URL,
            "archive_sha256": PROOFCHECK_SHA256,
            "archive_bytes": (external_dir / "proofcheck-linux-x86_64.zip").stat().st_size,
            "executable_sha256": sha256(proofcheck),
            "self_certify_returncode": self_certify.returncode,
            "self_certify_stdout_sha256": sha256(
                command_dir / "proofcheck-self-certify.stdout"
            ),
        },
        "cases": cases,
        "adversarial": {
            "corrupt_proof": {
                "gate_returncode": corrupt_gate.returncode,
                "verdict": corrupt_report["verdict"],
                "reason": corrupt_report["reasons"],
                "solution_sha256": sha256(corrupt_solution),
                "external_status": "VerifiedBad",
            },
            "known_nontheorem_forged_proof": {
                "gate_returncode": forged_gate.returncode,
                "verdict": forged_report["verdict"],
                "reason": forged_report["reasons"],
                "solution_sha256": sha256(forged_solution),
            },
        },
    }
    write_json(artifact_dir / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
