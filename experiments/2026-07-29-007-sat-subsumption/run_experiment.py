#!/usr/bin/env python3
"""Run the preregistered SAT-subsumption experiment on Ubuntu 24.04."""

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


class ExperimentError(RuntimeError):
    """An environment, command, or evidence-integrity failure."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def safe_extract(archive_path: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    resolved_destination = destination.resolve()
    with tarfile.open(archive_path, "r:gz") as archive:
        for member in archive.getmembers():
            target = (destination / member.name).resolve()
            if (
                target != resolved_destination
                and resolved_destination not in target.parents
            ):
                raise ExperimentError(
                    f"corpus archive escapes destination: {member.name}"
                )
            if member.issym() or member.islnk():
                raise ExperimentError(
                    f"corpus archive contains a link: {member.name}"
                )
        archive.extractall(destination, filter="data")


def write_instrumented_source_diff(
    experiment_root: Path, repo_root: Path, output_path: Path
) -> None:
    """Preserve the complete instrumentation without repository metadata."""
    patch = (experiment_root / "capture.patch").read_bytes()
    prototype = (experiment_root / "sat_subsumption.rs").relative_to(repo_root)
    completed = subprocess.run(
        [
            "git",
            "diff",
            "--no-index",
            "--binary",
            "--",
            os.devnull,
            str(prototype),
        ],
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 1:
        raise ExperimentError(
            "cannot render experiment prototype as a new-file diff: "
            + completed.stderr.decode("utf-8", errors="replace")
        )
    separator = b"" if patch.endswith(b"\n") else b"\n"
    output_path.write_bytes(patch + separator + completed.stdout)


def run_command(
    command: Sequence[str],
    *,
    cwd: Path,
    log_root: Path,
    name: str,
    timeout: int,
) -> dict[str, Any]:
    stdout_path = log_root / f"{name}.stdout.txt"
    stderr_path = log_root / f"{name}.stderr.txt"
    started_at = datetime.now(UTC).isoformat(timespec="seconds")
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    duration = time.monotonic() - started
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
        "duration_seconds": duration,
        "return_code": completed.returncode,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }
    if completed.returncode != 0:
        raise ExperimentError(
            f"{name} failed with exit code {completed.returncode}"
        )
    return record


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--corpus-archive", type=Path, required=True)
    parser.add_argument("--corpus-report", type=Path, required=True)
    parser.add_argument("--repo-head", required=True)
    parser.add_argument("--workers", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("this controller may run only on Linux")
    if arguments.workers < 1:
        raise ExperimentError("--workers must be positive")
    if len(arguments.repo_head) != 40 or any(
        character not in "0123456789abcdef"
        for character in arguments.repo_head
    ):
        raise ExperimentError("--repo-head must be a lowercase 40-digit Git hash")
    repo_root = arguments.repo_root.resolve()
    artifact_root = arguments.artifact_root.resolve()
    corpus_archive = arguments.corpus_archive.resolve()
    corpus_report = arguments.corpus_report.resolve()
    experiment_root = (
        repo_root / "experiments" / "2026-07-29-007-sat-subsumption"
    )
    if not (repo_root / "Cargo.toml").is_file():
        raise ExperimentError(f"invalid repository root: {repo_root}")
    if not corpus_archive.is_file() or not corpus_report.is_file():
        raise ExperimentError("missing corpus archive or report")
    corpus_metadata = json.loads(corpus_report.read_text(encoding="utf-8"))
    if sha256_file(corpus_archive) != corpus_metadata["archive_sha256"]:
        raise ExperimentError("corpus archive hash mismatch")

    artifact_root.mkdir(parents=True, exist_ok=True)
    log_root = artifact_root / "logs"
    result_root = artifact_root / "results"
    log_root.mkdir(parents=True, exist_ok=True)
    result_root.mkdir(parents=True, exist_ok=True)
    shutil.copy2(corpus_report, artifact_root / "corpus-report.json")
    safe_extract(corpus_archive, repo_root)

    commands: list[dict[str, Any]] = []

    def run(name: str, command: Sequence[str], timeout: int) -> None:
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
            *(
                str(experiment_root / name)
                for name in (
                    "analyze.py",
                    "capture.py",
                    "oracle.py",
                    "prepare_corpus.py",
                    "run_experiment.py",
                    "test_scripts.py",
                )
            ),
        ],
        120,
    )
    run(
        "oracle",
        [
            sys.executable,
            str(experiment_root / "oracle.py"),
            "--cases",
            "10000",
            "--output",
            str(result_root / "oracle.json"),
        ],
        300,
    )
    run(
        "patch-check",
        [
            "git",
            "apply",
            "--check",
            "--ignore-space-change",
            "--ignore-whitespace",
            str(experiment_root / "capture.patch"),
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
            str(experiment_root / "capture.patch"),
        ],
        60,
    )
    run("cargo-fmt", ["cargo", "fmt", "--all", "--", "--check"], 300)
    run(
        "cargo-test",
        [
            "cargo",
            "test",
            "--lib",
            "sat_subsumption_experiment",
        ],
        1_800,
    )
    run(
        "cargo-clippy",
        [
            "cargo",
            "clippy",
            "--lib",
            "--",
            "-D",
            "warnings",
            "-D",
            "clippy::pedantic",
        ],
        1_800,
    )
    run(
        "cargo-build-release",
        ["cargo", "build", "--release", "--bin", "umlaut"],
        1_800,
    )
    binary = repo_root / "target" / "release" / "umlaut"
    if not binary.is_file():
        raise ExperimentError("release build did not produce umlaut")

    manifest = repo_root / "benchmarks" / "casc_2025_manifest.jsonl"
    for phase in ("calibration", "validation", "test"):
        run(
            f"capture-{phase}",
            [
                sys.executable,
                str(experiment_root / "capture.py"),
                "--phase",
                phase,
                "--manifest",
                str(manifest),
                "--problem-root",
                str(repo_root),
                "--binary",
                str(binary),
                "--output-root",
                str(result_root / "captures"),
                "--workers",
                str(arguments.workers),
            ],
            3_600,
        )
        analysis_command = [
            sys.executable,
            str(experiment_root / "analyze.py"),
            "--phase-root",
            str(result_root / "captures" / phase),
            "--phase",
            phase,
            "--output",
            str(result_root / f"{phase}-analysis.json"),
        ]
        if phase == "calibration":
            analysis_command.extend(
                [
                    "--selection-output",
                    str(result_root / "selection.json"),
                ]
            )
        else:
            analysis_command.extend(
                ["--selection", str(result_root / "selection.json")]
            )
        run(f"analyze-{phase}", analysis_command, 300)

    git_diff_path = artifact_root / "instrumented-source.diff"
    write_instrumented_source_diff(experiment_root, repo_root, git_diff_path)

    analyses = {
        phase: json.loads(
            (result_root / f"{phase}-analysis.json").read_text(
                encoding="utf-8"
            )
        )
        for phase in ("calibration", "validation", "test")
    }
    evidence_path = artifact_root / "evidence.tar.gz"
    with tarfile.open(evidence_path, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        archive.add(log_root, arcname="logs")
        archive.add(result_root, arcname="results")
        archive.add(
            artifact_root / "corpus-report.json",
            arcname="corpus-report.json",
        )
        archive.add(
            git_diff_path,
            arcname="instrumented-source.diff",
        )

    report = {
        "schema_version": 1,
        "completed_at": datetime.now(UTC).isoformat(timespec="seconds"),
        "platform": platform.platform(),
        "python": sys.version,
        "cpu_count": os.cpu_count(),
        "repo_head": arguments.repo_head,
        "binary_sha256": sha256_file(binary),
        "corpus_archive_sha256": sha256_file(corpus_archive),
        "corpus_report_id": corpus_metadata["report_id"],
        "commands": commands,
        "analyses": {
            phase: {
                "report_id": report["report_id"],
                "records": report["summary"]["records"],
                "unique_pairs": report["summary"]["unique_pairs"],
                "ordinary_disagreements": report["summary"][
                    "ordinary_disagreements"
                ],
                "resolution_true": report["summary"]["resolution_true"],
                "decision": report["decision"]["decision"],
            }
            for phase, report in analyses.items()
        },
        "evidence": {
            "path": str(evidence_path),
            "bytes": evidence_path.stat().st_size,
            "sha256": sha256_file(evidence_path),
        },
        "prototype_sha256": sha256_file(
            experiment_root / "sat_subsumption.rs"
        ),
        "patch_sha256": sha256_file(experiment_root / "capture.patch"),
    }
    report["report_id"] = hashlib.sha256(canonical_json(report)).hexdigest()
    report_path = artifact_root / "report.json"
    report_path.write_bytes(canonical_json(report) + b"\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        ExperimentError,
        OSError,
        subprocess.TimeoutExpired,
        ValueError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
