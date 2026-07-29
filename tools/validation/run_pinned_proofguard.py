#!/usr/bin/env python3
"""Run a caller-supplied, integrity-pinned ProofGuard checkout.

ProofGuard is not bundled or downloaded by this adapter. Its pinned upstream
revision has no license declaration, so callers must obtain any required
permission and keep the checkout outside Umlaut's source and runtime packages.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


PROOFGUARD_COMMIT = "18fc573131648c9d1ed81e818f52f704c435033e"
PROOFGUARD_REMOTE = "https://github.com/ValueAchooMatthew/ATP-Research-Project.git"
PROOFGUARD_CHECKER_SHA256 = (
    "4da81bc5fb1651e01b2d5e5ae233b044ee20c58b8b67aa9644887cd42498471c"
)
PROOFGUARD_ENGINE_SHA256 = (
    "1441ed3a18702a97f83d9dccd5c2ef1fd9b0832a846bba709d4260bba19e8863"
)
SZS_STATUS_RE = re.compile(
    r"^%\s*SZS\s+status\s+(VerifiedGood|VerifiedBad|Unknown|Timeout)"
    r"(?:\s*:\s*.*)?$",
    re.IGNORECASE,
)


class AdapterError(RuntimeError):
    """The external-checker integrity or process contract failed."""


@dataclass(frozen=True)
class CheckoutIdentity:
    commit: str
    remote: str
    checker_sha256: str
    engine_sha256: str


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git_output(root: Path, *arguments: str) -> str:
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), *arguments],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            errors="replace",
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise AdapterError(f"could not inspect ProofGuard checkout: {error}") from error
    if completed.returncode != 0:
        detail = " ".join(completed.stderr.split())
        raise AdapterError(
            f"git {' '.join(arguments)} failed for ProofGuard: {detail}"
        )
    return completed.stdout.strip()


def verify_checkout(
    root: Path,
    *,
    expected_commit: str = PROOFGUARD_COMMIT,
    expected_remote: str = PROOFGUARD_REMOTE,
    expected_checker_sha256: str = PROOFGUARD_CHECKER_SHA256,
    expected_engine_sha256: str = PROOFGUARD_ENGINE_SHA256,
) -> CheckoutIdentity:
    root = root.resolve()
    checker = root / "proover-check"
    engine = root / "proover.py"
    if not checker.is_file() or not engine.is_file():
        raise AdapterError("ProofGuard checkout is missing proover-check or proover.py")

    commit = git_output(root, "rev-parse", "HEAD")
    if commit != expected_commit:
        raise AdapterError(
            f"ProofGuard commit mismatch: expected {expected_commit}, got {commit}"
        )
    remote = git_output(root, "remote", "get-url", "origin")
    if remote != expected_remote:
        raise AdapterError(
            f"ProofGuard remote mismatch: expected {expected_remote}, got {remote}"
        )
    if git_output(root, "status", "--porcelain", "--untracked-files=all"):
        raise AdapterError("ProofGuard checkout is dirty")

    checker_hash = sha256_file(checker)
    engine_hash = sha256_file(engine)
    if checker_hash != expected_checker_sha256:
        raise AdapterError(
            "ProofGuard proover-check hash mismatch: "
            f"expected {expected_checker_sha256}, got {checker_hash}"
        )
    if engine_hash != expected_engine_sha256:
        raise AdapterError(
            "ProofGuard proover.py hash mismatch: "
            f"expected {expected_engine_sha256}, got {engine_hash}"
        )
    return CheckoutIdentity(commit, remote, checker_hash, engine_hash)


def checker_status(stdout: str) -> str:
    nonempty = [line.strip() for line in stdout.splitlines() if line.strip()]
    if len(nonempty) != 1:
        raise AdapterError("ProofGuard did not emit exactly one nonempty output line")
    match = SZS_STATUS_RE.fullmatch(nonempty[0])
    if match is None:
        raise AdapterError("ProofGuard emitted an unrecognized status line")
    return match.group(1).lower()


def run_checker(
    checker: Path,
    eprover: Path,
    problem: Path,
    proof: Path,
    *,
    time_limit: float,
) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["ATP_EPROVER_BIN"] = str(eprover)
    try:
        completed = subprocess.run(
            [
                sys.executable,
                str(checker),
                "--quiet",
                "--time-limit",
                str(time_limit),
                str(problem),
                str(proof),
            ],
            cwd=checker.parent,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            errors="replace",
            timeout=time_limit + 5,
            env=environment,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise AdapterError(f"ProofGuard process failed: {error}") from error
    if completed.returncode != 0:
        raise AdapterError(f"ProofGuard exited with status {completed.returncode}")
    checker_status(completed.stdout)
    return completed


def normalized_sha256(value: str) -> str:
    normalized = value.lower()
    if re.fullmatch(r"[0-9a-f]{64}", normalized) is None:
        raise argparse.ArgumentTypeError("expected a 64-digit SHA-256 value")
    return normalized


def parse_args(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Verify a TSTP proof with a caller-supplied, pinned ProofGuard checkout"
        )
    )
    parser.add_argument("--proofguard-root", type=Path, required=True)
    parser.add_argument("--eprover", type=Path, required=True)
    parser.add_argument(
        "--expected-eprover-sha256",
        type=normalized_sha256,
        required=True,
    )
    parser.add_argument("--time-limit", type=float, default=120.0)
    parser.add_argument("problem", type=Path)
    parser.add_argument("proof", type=Path)
    parsed = parser.parse_args(arguments)
    if parsed.time_limit <= 1:
        parser.error("--time-limit must be greater than one second")
    return parsed


def concise_reason(error: Exception) -> str:
    reason = " ".join(str(error).split())
    return reason[:180] if reason else type(error).__name__


def main(arguments: Sequence[str] | None = None) -> int:
    args = parse_args(arguments)
    try:
        root = args.proofguard_root.resolve()
        eprover = args.eprover.resolve()
        problem = args.problem.resolve()
        proof = args.proof.resolve()
        verify_checkout(root)
        for label, path in (
            ("E backend", eprover),
            ("problem", problem),
            ("proof", proof),
        ):
            if not path.is_file():
                raise AdapterError(f"{label} file is missing: {path}")
        observed_eprover_hash = sha256_file(eprover)
        if observed_eprover_hash != args.expected_eprover_sha256:
            raise AdapterError(
                "E backend hash mismatch: "
                f"expected {args.expected_eprover_sha256}, "
                f"got {observed_eprover_hash}"
            )
        completed = run_checker(
            root / "proover-check",
            eprover,
            problem,
            proof,
            time_limit=args.time_limit,
        )
    except AdapterError as error:
        print(f"% SZS status Unknown : {concise_reason(error)}")
        return 3

    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
