#!/usr/bin/env python3
"""Guarded Ubuntu controller for the bounded AVATAR experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence


PROOFCHECK_SHA256 = (
    "92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e"
)


class ExperimentError(RuntimeError):
    """The remote experiment failed a hard integrity or quality gate."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def safe_extract(archive_path: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    resolved = destination.resolve()
    with tarfile.open(archive_path, "r:gz") as archive:
        for member in archive.getmembers():
            target = (destination / member.name).resolve()
            if target != resolved and resolved not in target.parents:
                raise ExperimentError(f"archive escapes repository: {member.name}")
            if member.issym() or member.islnk():
                raise ExperimentError(f"archive contains a link: {member.name}")
        archive.extractall(destination, filter="data")


def run_command(
    command: Sequence[str],
    *,
    cwd: Path,
    log_root: Path,
    name: str,
    timeout: float,
) -> dict[str, Any]:
    stdout_path = log_root / f"{name}.stdout.txt"
    stderr_path = log_root / f"{name}.stderr.txt"
    started_at = datetime.now(UTC).isoformat(timespec="seconds")
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    elapsed = time.monotonic() - started
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    sys.stdout.buffer.write(completed.stdout)
    sys.stderr.buffer.write(completed.stderr)
    sys.stdout.flush()
    sys.stderr.flush()
    record = {
        "name": name,
        "command": list(command),
        "started_at": started_at,
        "wall_seconds": elapsed,
        "return_code": completed.returncode,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }
    if completed.returncode != 0:
        raise ExperimentError(
            f"{name} failed with exit code {completed.returncode}"
        )
    return record


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--corpus-archive", type=Path, required=True)
    parser.add_argument("--corpus-report", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--repo-head", required=True)
    parser.add_argument("--workers", type=int, default=4)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    if sys.platform != "linux":
        raise ExperimentError("controller must run on Linux")
    if arguments.workers < 1:
        raise ExperimentError("workers must be positive")
    repo_root = arguments.repo_root.resolve()
    artifact_root = arguments.artifact_root.resolve()
    corpus_archive = arguments.corpus_archive.resolve()
    corpus_report_path = arguments.corpus_report.resolve()
    proofcheck = arguments.proofcheck.resolve()
    experiment_root = (
        repo_root / "experiments/2026-07-29-008-avatar-restart-prototype"
    )
    if not (repo_root / "Cargo.toml").is_file():
        raise ExperimentError("invalid repository root")
    corpus_report = json.loads(
        corpus_report_path.read_text(encoding="utf-8")
    )
    if sha256_file(corpus_archive) != corpus_report["archive_sha256"]:
        raise ExperimentError("corpus archive hash mismatch")
    if not proofcheck.is_file() or sha256_file(proofcheck) != PROOFCHECK_SHA256:
        raise ExperimentError("ProofCheck hash mismatch")
    proofcheck.chmod(0o555)
    safe_extract(corpus_archive, repo_root)

    log_root = artifact_root / "logs"
    result_root = artifact_root / "results"
    log_root.mkdir(parents=True, exist_ok=True)
    result_root.mkdir(parents=True, exist_ok=True)
    shutil.copy2(corpus_report_path, artifact_root / "corpus-report.json")
    commands: list[dict[str, Any]] = []

    def run(name: str, command: Sequence[str], timeout: float) -> None:
        commands.append(
            run_command(
                command,
                cwd=repo_root,
                log_root=log_root,
                name=name,
                timeout=timeout,
            )
        )

    run(
        "proofcheck-self-certify",
        [str(proofcheck), "-self-certify"],
        300,
    )
    python_files = [
        "analyze.py",
        "avatar_replay.py",
        "driver_integration.py",
        "prepare_corpus.py",
        "run_experiment.py",
        "select_corpus.py",
        "test_scripts.py",
        "tptp_split.py",
        "verify_certificate.py",
    ]
    run(
        "python-tests",
        [sys.executable, str(experiment_root / "test_scripts.py")],
        120,
    )
    run(
        "python-compile",
        [
            sys.executable,
            "-m",
            "py_compile",
            *(str(experiment_root / name) for name in python_files),
        ],
        120,
    )
    patch_path = experiment_root / "cargo-bin.patch"
    run(
        "patch-check",
        [
            "git",
            "apply",
            "--check",
            "--ignore-space-change",
            "--ignore-whitespace",
            str(patch_path),
        ],
        60,
    )
    run(
        "patch-apply",
        [
            "git",
            "apply",
            "--ignore-space-change",
            "--ignore-whitespace",
            str(patch_path),
        ],
        60,
    )
    run("cargo-fmt", ["cargo", "fmt", "--all", "--", "--check"], 300)
    run(
        "cargo-test-driver",
        [
            "cargo",
            "test",
            "--locked",
            "--bin",
            "avatar-sat-driver",
        ],
        1800,
    )
    run(
        "cargo-clippy-driver",
        [
            "cargo",
            "clippy",
            "--locked",
            "--bin",
            "avatar-sat-driver",
            "--",
            "-D",
            "warnings",
            "-D",
            "clippy::pedantic",
        ],
        1800,
    )
    run(
        "cargo-build-release",
        [
            "cargo",
            "build",
            "--locked",
            "--release",
            "--bin",
            "umlaut",
            "--bin",
            "avatar-sat-driver",
        ],
        3600,
    )
    umlaut = repo_root / "target/release/umlaut"
    sat_driver = repo_root / "target/release/avatar-sat-driver"
    if not umlaut.is_file() or not sat_driver.is_file():
        raise ExperimentError("release build omitted experiment binaries")
    driver_report = result_root / "driver-integration.json"
    run(
        "driver-integration",
        [
            sys.executable,
            str(experiment_root / "driver_integration.py"),
            "--driver",
            str(sat_driver),
            "--output",
            str(driver_report),
        ],
        120,
    )
    comparison_root = result_root / "comparison"
    run(
        "comparison",
        [
            sys.executable,
            str(experiment_root / "avatar_replay.py"),
            "--repo-root",
            str(repo_root),
            "--corpus",
            str(experiment_root / "corpus.jsonl"),
            "--artifact-root",
            str(comparison_root),
            "--umlaut",
            str(umlaut),
            "--sat-driver",
            str(sat_driver),
            "--proofcheck",
            str(proofcheck),
            "--phase",
            "all",
            "--workers",
            str(arguments.workers),
        ],
        7200,
    )
    analysis_path = result_root / "analysis.json"
    run(
        "analysis",
        [
            sys.executable,
            str(experiment_root / "analyze.py"),
            "--results",
            str(comparison_root / "results.jsonl"),
            "--driver-report",
            str(driver_report),
            "--output",
            str(analysis_path),
        ],
        300,
    )
    analysis = json.loads(analysis_path.read_text(encoding="utf-8"))
    evidence_path = artifact_root / "evidence.tar.gz"
    with tarfile.open(
        evidence_path, "w:gz", format=tarfile.PAX_FORMAT
    ) as archive:
        archive.add(log_root, arcname="logs")
        archive.add(result_root, arcname="results")
        archive.add(
            artifact_root / "corpus-report.json",
            arcname="corpus-report.json",
        )
    report = {
        "schema_version": 1,
        "completed_at": datetime.now(UTC).isoformat(timespec="seconds"),
        "platform": platform.platform(),
        "python": sys.version,
        "cpu_count": os.cpu_count(),
        "repo_head": arguments.repo_head,
        "proofcheck_sha256": sha256_file(proofcheck),
        "umlaut_sha256": sha256_file(umlaut),
        "sat_driver_sha256": sha256_file(sat_driver),
        "corpus_report_id": corpus_report["report_id"],
        "commands": commands,
        "analysis_report_id": analysis["report_id"],
        "advance": analysis["heldout_decision"]["advance"],
        "soundness_gates_passed": analysis["soundness"]["all_gates_passed"],
        "prototype_hashes": {
            name: sha256_file(experiment_root / name)
            for name in python_files
        },
        "rust_driver_sha256": sha256_file(
            experiment_root / "avatar_sat_driver.rs"
        ),
        "patch_sha256": sha256_file(patch_path),
        "evidence": {
            "path": str(evidence_path),
            "bytes": evidence_path.stat().st_size,
            "sha256": sha256_file(evidence_path),
        },
    }
    report["report_id"] = hashlib.sha256(canonical_json(report)).hexdigest()
    (artifact_root / "report.json").write_bytes(
        canonical_json(report) + b"\n"
    )
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (
        ExperimentError,
        OSError,
        subprocess.SubprocessError,
        ValueError,
    ) as error:
        print(f"experiment error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
