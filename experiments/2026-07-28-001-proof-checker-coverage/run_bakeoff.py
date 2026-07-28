#!/usr/bin/env python3
"""Bake off independent checkers on Umlaut proof-output coverage gaps."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path
from typing import Any, Sequence


NORG_ARCHIVE = {
    "name": "Nörgler",
    "version": "1.1",
    "license": "MIT",
    "url": "https://tptp.org/CASC/J13/SystemSources/Norgler---1.1.tgz",
    "bytes": 2_482_928,
    "sha256": "22cd1042af79ae1947e8478367c24a1d4b1e0208e78a49b3d8f66a222c5b9aaf",
}
NORG_JAR_SHA256 = (
    "29e9f5210fe9908c50cdc15f305bf08ae6930c0e768cd9eb42ae1ccd8ae1c6bf"
)
GAPT_ARCHIVE = {
    "name": "GAPT",
    "version": "2.20",
    "license": "GPL-3.0-only",
    "url": "https://tptp.org/CASC/J13/SystemSources/GAPT---2.20.tgz",
    "bytes": 113_746_748,
    "sha256": "3d99d26201f6b892a167f4b8e8d8fc95b6ee76cb154155ad3854b9ea8c44b94c",
}
GAPT_JAR_SHA256 = (
    "4532d97f9a56bd1c57bd7b127d6c1c9b8efc228faf4bd43017cfefcdea88afff"
)
SBT_LAUNCHER = {
    "version": "1.11.5",
    "url": (
        "https://repo1.maven.org/maven2/org/scala-sbt/sbt-launch/1.11.5/"
        "sbt-launch-1.11.5.jar"
    ),
    "bytes": 3_847_512,
    "sha256": "da3424478bb0c91428bdbe621b69b4b4e86ce8d468b403656020e7ebe5f7ed84",
}
E_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
E_HO_SHA256 = (
    "50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a"
)

CASES = (
    ("fof_contradictory_axioms", 0),
    ("tff_theorem", 0),
    ("thf_theorem", 0),
)
SZS_STATUS = re.compile(r"(?im)^%+\s*SZS status\s+([A-Za-z][A-Za-z0-9_]*)")


def sha256(path: Path) -> str:
    """Return one file's SHA-256 digest."""

    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    """Write deterministic JSON."""

    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def require_file(
    path: Path,
    *,
    expected_sha256: str,
    expected_bytes: int | None = None,
) -> dict[str, Any]:
    """Require a pinned input and return its identity."""

    path = path.resolve()
    if not path.is_file():
        raise RuntimeError(f"required file is missing: {path}")
    actual_hash = sha256(path)
    actual_bytes = path.stat().st_size
    if actual_hash != expected_sha256:
        raise RuntimeError(
            f"hash mismatch for {path}: expected {expected_sha256}, "
            f"got {actual_hash}"
        )
    if expected_bytes is not None and actual_bytes != expected_bytes:
        raise RuntimeError(
            f"size mismatch for {path}: expected {expected_bytes}, "
            f"got {actual_bytes}"
        )
    return {
        "path": str(path),
        "bytes": actual_bytes,
        "sha256": actual_hash,
    }


def preserve_license(archive: Path, member_suffix: str, output: Path) -> None:
    """Copy one bounded regular-file license from a pinned source archive."""

    with tarfile.open(archive, "r:gz") as source:
        matches = [
            member
            for member in source.getmembers()
            if member.name.endswith(member_suffix) and member.isfile()
        ]
        if len(matches) != 1:
            raise RuntimeError(
                f"expected one {member_suffix} in {archive}, found {len(matches)}"
            )
        member = matches[0]
        if member.size > 1024 * 1024:
            raise RuntimeError(f"refusing unexpectedly large license: {member.name}")
        extracted = source.extractfile(member)
        if extracted is None:
            raise RuntimeError(f"could not read license: {member.name}")
        output.write_bytes(extracted.read())


def run(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: int,
    stdout_path: Path,
    stderr_path: Path,
) -> dict[str, Any]:
    """Run one shell-free command and preserve both byte streams."""

    started = time.monotonic()
    try:
        completed = subprocess.run(
            list(command),
            cwd=cwd,
            check=False,
            capture_output=True,
            timeout=timeout,
        )
        returncode: int | None = completed.returncode
        timed_out = False
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        returncode = None
        timed_out = True
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    elapsed = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    return {
        "command": list(command),
        "returncode": returncode,
        "timed_out": timed_out,
        "wall_seconds": elapsed,
        "stdout": stdout_path.name,
        "stderr": stderr_path.name,
        "stdout_sha256": sha256(stdout_path),
        "stderr_sha256": sha256(stderr_path),
    }


def run_text(command: Sequence[str], *, cwd: Path) -> str:
    """Run a short metadata command and combine its output."""

    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        capture_output=True,
        timeout=30,
    )
    text = (completed.stdout + completed.stderr).decode(
        "utf-8", errors="replace"
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"metadata command failed ({completed.returncode}): "
            f"{' '.join(command)}\n{text}"
        )
    return text.strip()


def statuses(path: Path) -> list[str]:
    """Return all SZS statuses in one captured stream."""

    text = path.read_text(encoding="utf-8", errors="replace")
    return SZS_STATUS.findall(text)


def checker_status(stdout_path: Path, stderr_path: Path) -> str | None:
    """Return the last SZS status emitted by a checker."""

    combined = (
        stdout_path.read_text(encoding="utf-8", errors="replace")
        + "\n"
        + stderr_path.read_text(encoding="utf-8", errors="replace")
    )
    found = SZS_STATUS.findall(combined)
    return found[-1] if found else None


def hard_timeout(command: Sequence[str], seconds: int = 180) -> list[str]:
    """Wrap a command in a GNU timeout with a five-second kill grace."""

    return [
        "timeout",
        "--kill-after=5s",
        f"{seconds}s",
        *command,
    ]


def gapt_command(java: str, jar: Path, proof: Path) -> list[str]:
    """Return the resource-bounded GAPT invocation."""

    return hard_timeout(
        [
            java,
            "-Xss16m",
            "-Xms1g",
            "-Xmx8g",
            "-jar",
            str(jar),
            str(proof),
        ]
    )


def norgler_command(
    java: str,
    jar: Path,
    eprover: Path,
    problem: Path,
    proof: Path,
) -> list[str]:
    """Return the resource-bounded Nörgler invocation."""

    return hard_timeout(
        [
            java,
            "-Xms256m",
            "-Xmx4g",
            "-jar",
            str(jar),
            "--problem",
            str(problem),
            "--verbosity",
            "6",
            "--eprover-path",
            str(eprover),
            "--mace4-path",
            "/bin/false",
            "--parallel-mode",
            "steps",
            "--timeout",
            "120",
            "--relax-annotation-format",
            str(proof),
        ]
    )


def main() -> int:
    """Run the checker bake-off and require all adopted claims."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-snapshot-sha256", required=True)
    parser.add_argument("--norgler-archive", type=Path, required=True)
    parser.add_argument("--norgler-jar", type=Path, required=True)
    parser.add_argument("--gapt-archive", type=Path, required=True)
    parser.add_argument("--gapt-jar", type=Path, required=True)
    parser.add_argument("--sbt-launcher", type=Path, required=True)
    parser.add_argument("--eprover-ho", type=Path, required=True)
    parser.add_argument("--java", default="java")
    args = parser.parse_args()

    repo = args.repo.resolve()
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=False)
    raw_dir = artifact_dir / "raw"
    raw_dir.mkdir()
    solution_dir = artifact_dir / "solutions"
    solution_dir.mkdir()
    report_dir = artifact_dir / "reports"
    report_dir.mkdir()
    license_dir = artifact_dir / "licenses"
    license_dir.mkdir()

    norgler_archive = require_file(
        args.norgler_archive,
        expected_sha256=NORG_ARCHIVE["sha256"],
        expected_bytes=NORG_ARCHIVE["bytes"],
    )
    norgler_jar_path = args.norgler_jar.resolve()
    norgler_jar = require_file(
        norgler_jar_path,
        expected_sha256=NORG_JAR_SHA256,
    )
    gapt_archive = require_file(
        args.gapt_archive,
        expected_sha256=GAPT_ARCHIVE["sha256"],
        expected_bytes=GAPT_ARCHIVE["bytes"],
    )
    gapt_jar_path = args.gapt_jar.resolve()
    gapt_jar = require_file(
        gapt_jar_path,
        expected_sha256=GAPT_JAR_SHA256,
    )
    sbt_launcher = require_file(
        args.sbt_launcher,
        expected_sha256=SBT_LAUNCHER["sha256"],
        expected_bytes=SBT_LAUNCHER["bytes"],
    )
    eprover_path = args.eprover_ho.resolve()
    eprover = require_file(
        eprover_path,
        expected_sha256=E_HO_SHA256,
    )
    preserve_license(
        Path(args.norgler_archive).resolve(),
        "/LICENSE",
        license_dir / "Norgler-1.1-LICENSE",
    )
    preserve_license(
        Path(args.gapt_archive).resolve(),
        "/COPYING",
        license_dir / "GAPT-2.20-COPYING",
    )

    build = run(
        ["cargo", "build", "--locked", "--release", "--bin", "umlaut"],
        cwd=repo,
        timeout=3600,
        stdout_path=raw_dir / "cargo-build.stdout",
        stderr_path=raw_dir / "cargo-build.stderr",
    )
    if build["returncode"] != 0:
        raise RuntimeError("release Umlaut build failed")

    fixture_dir = (
        repo
        / "experiments/2026-07-27-004-soundness-validation-gates/fixtures"
    )
    umlaut = repo / "target/release/umlaut"
    generated: dict[str, dict[str, Any]] = {}
    for name, expected_returncode in CASES:
        problem = (fixture_dir / f"{name}.p").resolve()
        solution = solution_dir / f"{name}.s"
        invocation = run(
            [
                str(umlaut),
                "--auto",
                "--tstp-out",
                "--proof-object=1",
                "--cpu-limit=30",
                "--memory-limit=2048",
                str(problem),
            ],
            cwd=repo,
            timeout=120,
            stdout_path=solution,
            stderr_path=solution_dir / f"{name}.stderr",
        )
        if invocation["returncode"] != expected_returncode:
            raise RuntimeError(
                f"{name} exited {invocation['returncode']}, "
                f"expected {expected_returncode}"
            )
        proof_statuses = statuses(solution)
        if not proof_statuses:
            raise RuntimeError(f"{name} emitted no SZS status")
        text = solution.read_text(encoding="utf-8", errors="replace")
        if "$false" not in text or "% SZS output end CNFRefutation" not in text:
            raise RuntimeError(f"{name} emitted no framed false refutation")
        if name in {"tff_theorem", "thf_theorem"} and (
            "inference(assume_negation,[status(cth)],[goal])" not in text
        ):
            raise RuntimeError(f"{name} lost the explicit conjecture-negation step")
        invocation["solution_sha256"] = sha256(solution)
        invocation["solution_bytes"] = solution.stat().st_size
        invocation["szs_statuses"] = proof_statuses
        generated[name] = invocation

    contradiction = solution_dir / "fof_contradictory_axioms.s"
    corrupt = solution_dir / "fof_contradictory_axioms-corrupt-derived.s"
    corrupt_text = contradiction.read_text(encoding="utf-8")
    old = "cnf(c_0_4, plain, (p(a)),"
    new = "cnf(c_0_4, plain, (q(a)),"
    if corrupt_text.count(old) != 1:
        raise RuntimeError("could not identify the contradiction proof mutation")
    corrupt.write_text(
        corrupt_text.replace(old, new),
        encoding="utf-8",
        newline="\n",
    )

    gapt_results: dict[str, dict[str, Any]] = {}
    for name, _ in CASES:
        proof = solution_dir / f"{name}.s"
        stdout_path = raw_dir / f"gapt-{name}.stdout"
        stderr_path = raw_dir / f"gapt-{name}.stderr"
        result = run(
            gapt_command(args.java, gapt_jar_path, proof),
            cwd=repo,
            timeout=200,
            stdout_path=stdout_path,
            stderr_path=stderr_path,
        )
        result["szs_status"] = checker_status(stdout_path, stderr_path)
        gapt_results[name] = result

    corrupt_stdout = raw_dir / "gapt-fof-corrupt.stdout"
    corrupt_stderr = raw_dir / "gapt-fof-corrupt.stderr"
    gapt_corrupt = run(
        gapt_command(args.java, gapt_jar_path, corrupt),
        cwd=repo,
        timeout=200,
        stdout_path=corrupt_stdout,
        stderr_path=corrupt_stderr,
    )
    gapt_corrupt["szs_status"] = checker_status(
        corrupt_stdout, corrupt_stderr
    )

    norgler_results: dict[str, dict[str, Any]] = {}
    for name, _ in CASES:
        problem = (fixture_dir / f"{name}.p").resolve()
        proof = solution_dir / f"{name}.s"
        stdout_path = raw_dir / f"norgler-{name}.stdout"
        stderr_path = raw_dir / f"norgler-{name}.stderr"
        result = run(
            norgler_command(
                args.java,
                norgler_jar_path,
                eprover_path,
                problem,
                proof,
            ),
            cwd=repo,
            timeout=200,
            stdout_path=stdout_path,
            stderr_path=stderr_path,
        )
        result["szs_status"] = checker_status(stdout_path, stderr_path)
        norgler_results[name] = result

    gate = repo / "tools/validation/validate_tptp_solution.py"
    gate_checker = json.dumps(
        [
            args.java,
            "-Xss16m",
            "-Xms1g",
            "-Xmx8g",
            "-jar",
            str(gapt_jar_path),
            "{artifact}",
        ]
    )
    problem = (fixture_dir / "fof_contradictory_axioms.p").resolve()
    gate_results: dict[str, dict[str, Any]] = {}
    for label, proof, expected_returncode in (
        ("positive", contradiction, 0),
        ("corrupt", corrupt, 1),
    ):
        report = report_dir / f"gate-{label}.json"
        result = run(
            [
                sys.executable,
                str(gate),
                str(problem),
                str(proof),
                "--proof-command-json",
                gate_checker,
                "--timeout-seconds",
                "180",
                "--report",
                str(report),
            ],
            cwd=repo,
            timeout=200,
            stdout_path=raw_dir / f"gate-{label}.stdout",
            stderr_path=raw_dir / f"gate-{label}.stderr",
        )
        if result["returncode"] != expected_returncode:
            raise RuntimeError(
                f"{label} validation gate exited {result['returncode']}, "
                f"expected {expected_returncode}"
            )
        report_value = json.loads(report.read_text(encoding="utf-8"))
        result["verdict"] = report_value["verdict"]
        result["reasons"] = report_value["reasons"]
        gate_results[label] = result

    if gapt_results["fof_contradictory_axioms"]["szs_status"] != "VerifiedGood":
        raise RuntimeError("GAPT did not verify the ContradictoryAxioms proof")
    if gapt_corrupt["szs_status"] != "VerifiedBad":
        raise RuntimeError("GAPT did not reject the corrupted FOF proof")
    if gate_results["positive"]["verdict"] != "verified":
        raise RuntimeError("the validation gate did not accept the checked proof")
    if gate_results["corrupt"]["verdict"] != "rejected":
        raise RuntimeError("the validation gate did not reject the corrupt proof")
    for name in ("tff_theorem", "thf_theorem"):
        if gapt_results[name]["szs_status"] == "VerifiedGood":
            raise RuntimeError(f"unexpectedly stale typed gap for GAPT: {name}")
        if norgler_results[name]["szs_status"] == "VerifiedGood":
            raise RuntimeError(f"unexpectedly stale typed gap for Nörgler: {name}")

    raw_files = sorted(
        path
        for path in artifact_dir.rglob("*")
        if path.is_file() and path.name != "results.json"
    )
    result = {
        "schema_version": 1,
        "source": {
            "commit": args.source_commit,
            "snapshot_sha256": args.source_snapshot_sha256,
        },
        "platform": {
            "platform": platform.platform(),
            "python": platform.python_version(),
            "java": run_text([args.java, "-version"], cwd=repo),
        },
        "resource_limits": {
            "umlaut_cpu_seconds": 30,
            "umlaut_memory_mib": 2048,
            "umlaut_hard_wall_seconds": 120,
            "checker_hard_wall_seconds": 180,
            "checker_kill_grace_seconds": 5,
            "gapt_heap_mib": 8192,
            "gapt_stack_mib": 16,
            "norgler_heap_mib": 4096,
            "norgler_soft_step_seconds": 120,
        },
        "oracles": {
            "gapt": {
                **GAPT_ARCHIVE,
                "source_archive": gapt_archive,
                "jar": gapt_jar,
                "version_evidence": "CASC source release GAPT 2.20",
                "build_command": [
                    args.java,
                    "-Xms1g",
                    "-Xmx8g",
                    "-jar",
                    str(Path(args.sbt_launcher).resolve()),
                    "cli/ProoVerCLI/assembly",
                ],
            },
            "norgler": {
                **NORG_ARCHIVE,
                "source_archive": norgler_archive,
                "jar": norgler_jar,
                "version_output": run_text(
                    [
                        args.java,
                        "-jar",
                        str(norgler_jar_path),
                        "--version",
                    ],
                    cwd=repo,
                ),
                "build_command": [
                    args.java,
                    "-jar",
                    str(Path(args.sbt_launcher).resolve()),
                    "assembly",
                ],
            },
            "sbt_launcher": {
                **SBT_LAUNCHER,
                "artifact": sbt_launcher,
            },
            "e_backend": {
                "name": "E",
                "version": "3.3.5-ho",
                "license": "GPL-3.0-or-later",
                "source_commit": E_COMMIT,
                "binary": eprover,
                "version_output": run_text(
                    [str(eprover_path), "--version"], cwd=repo
                ),
                "build_command": [
                    "python3",
                    "tools/linode-runner/linux_compat.py",
                    "build-reference",
                    "--repo-root",
                    str(repo),
                    "--eprover-commit",
                    E_COMMIT,
                ],
            },
        },
        "umlaut": {
            "binary_sha256": sha256(umlaut),
            "binary_bytes": umlaut.stat().st_size,
            "build": build,
            "generated": generated,
        },
        "gapt": {
            "positive": gapt_results,
            "corrupt_fof": gapt_corrupt,
        },
        "norgler": norgler_results,
        "validation_gate": gate_results,
        "corruption": {
            "kind": "derived-clause formula mutation",
            "original": old,
            "replacement": new,
            "solution_sha256": sha256(corrupt),
            "solution_bytes": corrupt.stat().st_size,
        },
        "artifacts": {
            "file_count": len(raw_files),
            "total_bytes": sum(path.stat().st_size for path in raw_files),
            "files": {
                str(path.relative_to(artifact_dir)).replace("\\", "/"): {
                    "bytes": path.stat().st_size,
                    "sha256": sha256(path),
                }
                for path in raw_files
            },
        },
    }
    write_json(artifact_dir / "results.json", result)
    shutil.copy2(
        artifact_dir / "results.json",
        repo
        / "experiments/2026-07-28-001-proof-checker-coverage/results.json",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
